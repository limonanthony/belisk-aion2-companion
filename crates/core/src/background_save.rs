//! Saving a store on a thread of its own. The threads that change the store
//! only set a flag: serialising, compressing and syncing to disk never run on
//! the packet-parsing thread, the snapshot thread or the UI thread, and never
//! under the store's own lock.

use std::sync::{Arc, Condvar, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

use crate::logger::AppLogger;

/// After a failed write the writer thread waits this long before trying again.
const RETRY_DELAY: Duration = Duration::from_secs(5);

pub struct BackgroundSave {
    shared: Arc<Shared>,
    writer: Option<JoinHandle<()>>,
}

struct Shared {
    file_name: &'static str,
    logger: Arc<AppLogger>,
    write: Box<dyn Fn() -> Result<(), String> + Send + Sync>,
    pending: Mutex<Pending>,
    wake: Condvar,
    /// One write at a time: the temp file is shared.
    writing: Mutex<()>,
}

#[derive(Default)]
struct Pending {
    dirty: bool,
    shutdown: bool,
}

impl BackgroundSave {
    /// `write` takes the store's current content (copying it under the store's
    /// lock and releasing the lock before it serialises) and puts it on disk.
    /// `file_name` names the file in the log.
    pub fn new(
        file_name: &'static str,
        logger: Arc<AppLogger>,
        write: impl Fn() -> Result<(), String> + Send + Sync + 'static,
    ) -> Self {
        let shared = Arc::new(Shared {
            file_name,
            logger,
            write: Box::new(write),
            pending: Mutex::new(Pending::default()),
            wake: Condvar::new(),
            writing: Mutex::new(()),
        });
        let thread_shared = Arc::clone(&shared);
        let writer = thread::Builder::new()
            .name(format!("{file_name}-writer"))
            .spawn(move || thread_shared.write_when_dirty())
            .map_err(|error| {
                shared.logger.error(format!(
                    "{file_name}: cannot start the writer thread ({error}); changes are saved only on exit"
                ));
            })
            .ok();
        Self { shared, writer }
    }

    /// The store changed: it will be written soon, by the writer thread.
    pub fn request(&self) {
        self.shared.pending.lock().unwrap().dirty = true;
        self.shared.wake.notify_all();
    }

    /// Writes on the calling thread if anything changed since the last write,
    /// for quitting. When it returns, every change requested before the call
    /// is on disk or the failure is logged.
    pub fn flush(&self) {
        self.shared.write_if_dirty();
    }
}

impl Drop for BackgroundSave {
    fn drop(&mut self) {
        self.shared.pending.lock().unwrap().shutdown = true;
        self.shared.wake.notify_all();
        if let Some(writer) = self.writer.take() {
            let _ = writer.join();
        }
        self.shared.write_if_dirty();
    }
}

impl Shared {
    fn write_when_dirty(&self) {
        loop {
            let mut pending = self.pending.lock().unwrap();
            while !pending.dirty && !pending.shutdown {
                pending = self.wake.wait(pending).unwrap();
            }
            if pending.shutdown {
                return;
            }
            drop(pending);

            if !self.write_if_dirty() {
                let pending = self.pending.lock().unwrap();
                let _ = self
                    .wake
                    .wait_timeout_while(pending, RETRY_DELAY, |pending| !pending.shutdown)
                    .unwrap();
            }
        }
    }

    /// Returns whether the content is on disk (or there was nothing to write).
    /// The dirty flag is consumed under the write lock, so a caller that finds
    /// nothing to do knows any write that took the change has finished. A
    /// failed write puts the flag back.
    fn write_if_dirty(&self) -> bool {
        let _writing = self.writing.lock().unwrap();
        {
            let mut pending = self.pending.lock().unwrap();
            if !pending.dirty {
                return true;
            }
            pending.dirty = false;
        }
        match (self.write)() {
            Ok(()) => true,
            Err(error) => {
                self.logger
                    .error(format!("{}: cannot save ({error}); will retry", self.file_name));
                self.pending.lock().unwrap().dirty = true;
                false
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::mpsc;

    use super::*;

    fn quiet_logger() -> Arc<AppLogger> {
        let path = std::env::temp_dir().join(format!("meter-core-background-save-{}.log", std::process::id()));
        Arc::new(AppLogger::new(path, Box::new(|_| {})))
    }

    #[test]
    fn a_requested_save_is_written_by_the_writer_thread_not_the_caller() {
        let (written, receive_written) = mpsc::channel();
        let saver = BackgroundSave::new("test.bin", quiet_logger(), move || {
            written.send(thread::current().id()).unwrap();
            Ok(())
        });

        saver.request();

        let writer = receive_written.recv_timeout(Duration::from_secs(10)).expect("the writer thread wrote");
        assert_ne!(writer, thread::current().id());
    }

    #[test]
    fn requests_made_while_a_write_is_running_cost_one_more_write() {
        let (entered, wait_entered) = mpsc::channel();
        let (release, wait_release) = mpsc::channel::<()>();
        let writes = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&writes);
        let wait_release = Mutex::new(wait_release);
        let saver = BackgroundSave::new("test.bin", quiet_logger(), move || {
            counted.fetch_add(1, Ordering::SeqCst);
            entered.send(()).unwrap();
            wait_release.lock().unwrap().recv().unwrap();
            Ok(())
        });

        saver.request();
        wait_entered.recv_timeout(Duration::from_secs(10)).unwrap();
        saver.request();
        saver.request();
        saver.request();
        release.send(()).unwrap();
        wait_entered.recv_timeout(Duration::from_secs(10)).unwrap();
        release.send(()).unwrap();
        drop(saver);

        assert_eq!(writes.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn a_pending_save_is_flushed_on_the_calling_thread_and_not_written_twice() {
        let writes = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&writes);
        let (entered, wait_entered) = mpsc::channel();
        let saver = BackgroundSave::new("test.bin", quiet_logger(), move || {
            counted.fetch_add(1, Ordering::SeqCst);
            let _ = entered.send(());
            Ok(())
        });

        saver.request();
        saver.flush();
        wait_entered.recv_timeout(Duration::from_secs(10)).unwrap();
        saver.flush();
        drop(saver);

        assert_eq!(writes.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_pending_save_is_written_when_the_saver_is_dropped() {
        let writes = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&writes);
        let saver = BackgroundSave::new("test.bin", quiet_logger(), move || {
            counted.fetch_add(1, Ordering::SeqCst);
            Ok(())
        });
        saver.request();
        drop(saver);

        assert_eq!(writes.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn a_failed_write_stays_pending_so_the_flush_tries_again() {
        let attempts = Arc::new(AtomicUsize::new(0));
        let counted = Arc::clone(&attempts);
        let saver = BackgroundSave::new("test.bin", quiet_logger(), move || {
            if counted.fetch_add(1, Ordering::SeqCst) == 0 {
                Err("disk full".to_string())
            } else {
                Ok(())
            }
        });
        saver.request();
        saver.flush();
        saver.flush();

        assert!(attempts.load(Ordering::SeqCst) >= 2);
    }
}
