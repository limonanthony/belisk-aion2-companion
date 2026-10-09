//! One live meter per Windows session. The first live launch holds a named
//! mutex; a later one finds it, signals the first through a named event and
//! ends. Only the first meter's own window is ever touched: nothing here looks
//! at, opens or enumerates another process.
//!
//! Both objects are in the `Local\` namespace, so a second Windows user (their
//! own session, their own data folder) can run a meter of their own, while two
//! launches by the same user collide. The kernel releases the mutex when the
//! process dies, however it ends, so a crash leaves no stale lock.

use std::ffi::c_void;
use std::thread;

use futures::channel::mpsc::{unbounded, UnboundedReceiver};
use futures::StreamExt;
use gpui::App;
use windows::core::{Owned, HSTRING};
use windows::Win32::Foundation::{GetLastError, ERROR_ALREADY_EXISTS, HANDLE, WAIT_OBJECT_0};
use windows::Win32::System::Threading::{CreateEventW, CreateMutexW, SetEvent, WaitForSingleObject, INFINITE};

const INSTANCE_NAME: &str = "Local\\Belisk.SingleInstance";

pub enum Claim {
    First(Running),
    AlreadyRunning(RunningMeter),
}

/// Holds the lock for as long as it lives.
pub struct Running {
    _lock: Owned<HANDLE>,
    raise_requests: UnboundedReceiver<()>,
}

/// The meter that holds the lock.
pub struct RunningMeter {
    raise: Owned<HANDLE>,
}

pub fn claim() -> Result<Claim, String> {
    claim_named(INSTANCE_NAME)
}

fn claim_named(name: &str) -> Result<Claim, String> {
    // The event is created before the lock, so whoever finds the lock held
    // finds the event too.
    let raise = unsafe { CreateEventW(None, false, false, &HSTRING::from(format!("{name}.raise"))) }
        .map_err(|error| format!("cannot create the single-instance event: {error}"))?;
    let raise = unsafe { Owned::new(raise) };
    let lock = unsafe { CreateMutexW(None, false, &HSTRING::from(format!("{name}.lock"))) }
        .map_err(|error| format!("cannot create the single-instance lock: {error}"))?;
    let lock = unsafe { Owned::new(lock) };
    if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
        return Ok(Claim::AlreadyRunning(RunningMeter { raise }));
    }

    let (requests, raise_requests) = unbounded();
    // A handle cannot cross threads; the listener owns the event until the
    // process ends, so it is forgotten only once the listener runs. When the
    // thread cannot start, `raise` and `lock` close on the way out.
    let listened = raise.0 as usize;
    thread::Builder::new()
        .name("single-instance".to_owned())
        .spawn(move || {
            let raise = HANDLE(listened as *mut c_void);
            while unsafe { WaitForSingleObject(raise, INFINITE) } == WAIT_OBJECT_0 {
                if requests.unbounded_send(()).is_err() {
                    return;
                }
            }
            crate::app_log::error("the single-instance listener stopped: waiting for the event failed");
        })
        .map_err(|error| format!("cannot start the single-instance listener: {error}"))?;
    std::mem::forget(raise);
    Ok(Claim::First(Running { _lock: lock, raise_requests }))
}

impl RunningMeter {
    pub fn ask_to_come_forward(&self) -> Result<(), String> {
        unsafe { SetEvent(*self.raise) }.map_err(|error| format!("cannot signal the running meter: {error}"))
    }
}

impl Running {
    /// Brings the toolbar, and what Alt+Shift+CapsLock hid, forward each time
    /// a later launch asks, as the tray's Show toolbar does. Requests that
    /// arrive before the toolbar exists wait in the queue.
    pub fn serve(self, cx: &App) -> gpui::Task<()> {
        let Running { _lock: lock, mut raise_requests } = self;
        cx.spawn(async move |cx| {
            let _lock = lock;
            while raise_requests.next().await.is_some() {
                while raise_requests.try_recv().is_ok() {}
                cx.update(crate::toolbar::show_toolbar);
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::time::{Duration, Instant};

    use super::*;

    fn unique_name() -> String {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        format!(
            "Local\\Belisk.Test.{}.{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::SeqCst)
        )
    }

    fn first(name: &str) -> Running {
        match claim_named(name).unwrap() {
            Claim::First(running) => running,
            Claim::AlreadyRunning(_) => panic!("{name} was already claimed"),
        }
    }

    #[test]
    fn a_second_claim_finds_the_first() {
        let name = unique_name();
        let _running = first(&name);
        assert!(matches!(claim_named(&name).unwrap(), Claim::AlreadyRunning(_)));
        assert!(matches!(claim_named(&name).unwrap(), Claim::AlreadyRunning(_)));
    }

    #[test]
    fn the_lock_is_free_once_the_first_is_gone() {
        let name = unique_name();
        drop(first(&name));
        drop(first(&name));
    }

    #[test]
    fn claims_under_other_names_do_not_collide() {
        let (a, b) = (unique_name(), unique_name());
        let _first = first(&a);
        let _other = first(&b);
    }

    #[test]
    fn a_later_launch_asks_the_first_to_come_forward() {
        let name = unique_name();
        let mut running = first(&name);
        assert!(running.raise_requests.try_recv().is_err(), "nothing asked yet");

        let Claim::AlreadyRunning(meter) = claim_named(&name).unwrap() else {
            panic!("the lock should be held");
        };
        meter.ask_to_come_forward().unwrap();

        let deadline = Instant::now() + Duration::from_secs(5);
        while running.raise_requests.try_recv().is_err() {
            assert!(Instant::now() < deadline, "the request never arrived");
            thread::sleep(Duration::from_millis(10));
        }
    }
}
