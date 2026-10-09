use std::sync::mpsc::{channel, Sender};
use std::sync::Arc;
use std::thread;
use std::time::SystemTime;

use futures::channel::mpsc::{unbounded, UnboundedReceiver};
use futures::StreamExt;
use gpui::{App, AppContext as _, Context, Entity, Global, Subscription, Task};
use meter_core::engine::meter::DpsMeter;

use super::apply;
use super::engine::{Command, Engine, Parts, SystemClock};
use super::http::{Http, UreqHttp};
use super::repo::Repo;
use super::stage::Layout;
use super::state::{Failure, Snapshot, UpdateState};
use super::verify;
use crate::changelog::Version;
use crate::settings::Settings;

/// The one entity the update UI observes: the toolbar's icon and Settings ›
/// About › Updates read [`Updates::snapshot`] and call its actions. All
/// network and disk work runs on the worker thread; this only relays.
pub struct Updates {
    snapshot: Snapshot,
    worker: Option<Worker>,
    restart_failure: Option<Failure>,
    download_automatically: bool,
    _pump: Option<Task<()>>,
    _settings: Option<Subscription>,
    /// At quit the worker stops, deleting a partial download.
    _quit: Option<Subscription>,
}

/// The worker thread's door, and what a restart checks the staged update with.
pub struct Worker {
    commands: Sender<Command>,
    layout: Layout,
    key: ed25519_dalek::VerifyingKey,
    /// Held for the life of the app: see [`apply::ImageLock`].
    image: apply::ImageLock,
}

type Started = (Worker, UnboundedReceiver<Snapshot>);

struct UpdatesHandle(Entity<Updates>);

impl Global for UpdatesHandle {}

/// Makes the app's updates. `live` is false for replays and captures, which
/// never contact anything. Without `UPDATE_REPO` at build time the state is
/// [`UpdateState::Disabled`] and no thread starts.
pub fn install(settings: &Entity<Settings>, live: bool, meter: Option<Arc<DpsMeter>>, cx: &mut App) {
    let download_automatically = settings.read(cx).get().updates.download_automatically;
    let worker = if live { start_worker_for_this_build(meter, download_automatically) } else { None };
    let updates = cx.new(|cx| Updates::new(settings, worker, cx));
    cx.set_global(UpdatesHandle(updates));
}

pub fn global(cx: &App) -> Entity<Updates> {
    cx.global::<UpdatesHandle>().0.clone()
}

/// Makes updates that sit in `snapshot` with no worker: for the update UI's
/// tests and `--capture` fixtures, which must never contact anything. When
/// updates exist already (a capture run), they take `snapshot` in place, so
/// the windows observing them follow.
#[cfg(any(test, feature = "probes"))]
pub fn install_fixture(settings: &Entity<Settings>, snapshot: Snapshot, cx: &mut App) {
    if let Some(updates) = cx.try_global::<UpdatesHandle>().map(|handle| handle.0.clone()) {
        updates.update(cx, |updates, cx| {
            updates.snapshot = snapshot;
            cx.notify();
        });
        return;
    }
    let updates = cx.new(|cx| {
        let mut updates = Updates::new(settings, None, cx);
        updates.snapshot = snapshot;
        updates
    });
    cx.set_global(UpdatesHandle(updates));
}

/// Updates sitting in `snapshot` whose actions reach the returned receiver
/// instead of a worker, for the UI's tests of what a click asks. Their
/// restart looks for a staged exe in a folder that does not exist, so it
/// fails and says so instead of quitting.
#[cfg(test)]
pub fn install_recording_fixture(settings: &Entity<Settings>, snapshot: Snapshot, cx: &mut App) -> std::sync::mpsc::Receiver<Command> {
    let (commands, received) = channel();
    let worker = Worker {
        commands,
        layout: Layout::new(std::env::temp_dir().join(format!("belisk-update-ui-tests-{}", std::process::id()))),
        key: super::stage::tests::key(),
        image: apply::ImageLock::of_this_test_exe(),
    };
    let updates = cx.new(|cx| {
        let mut updates = Updates::new(settings, Some((worker, unbounded().1)), cx);
        updates.snapshot = snapshot;
        updates
    });
    cx.set_global(UpdatesHandle(updates));
    received
}

fn start_worker_for_this_build(meter: Option<Arc<DpsMeter>>, download_automatically: bool) -> Option<Started> {
    let repo = Repo::of_this_build()?;
    let key = match verify::trusted_key() {
        Ok(key) => key,
        Err(error) => {
            crate::app_log::error(format!("updates are off: {error}"));
            return None;
        }
    };
    let install = match std::env::current_exe().map_err(|error| error.to_string()).and_then(|exe| apply::Install::of_exe(&exe)) {
        Ok(install) => install,
        Err(error) => {
            crate::app_log::error(format!("updates are off: {error}"));
            return None;
        }
    };
    let in_fight = move || meter.as_ref().is_some_and(|meter| meter.in_fight());
    let image = match std::env::current_exe().map_err(|error| error.to_string()).and_then(|exe| apply::ImageLock::acquire(&exe)) {
        Ok(image) => image,
        Err(error) => {
            crate::app_log::error(format!("updates are off: {error}"));
            return None;
        }
    };
    let worker = spawn_worker(repo, key, install.layout(), Arc::new(UreqHttp::new()), Box::new(in_fight), download_automatically, image);
    match worker {
        Ok(started) => Some(started),
        Err(error) => {
            crate::app_log::error(format!("updates are off: {error}"));
            None
        }
    }
}

fn spawn_worker(
    repo: Repo,
    key: ed25519_dalek::VerifyingKey,
    layout: Layout,
    http: Arc<dyn Http>,
    in_fight: Box<dyn Fn() -> bool + Send>,
    auto_download: bool,
    image: apply::ImageLock,
) -> Result<Started, String> {
    let (commands, receiver) = channel();
    let (publisher, snapshots) = unbounded();
    let engine = Engine::new(Parts {
        repo,
        running: Version::current(),
        key,
        layout: layout.clone(),
        http,
        clock: Arc::new(SystemClock),
        in_fight,
        publish: Box::new(move |snapshot| {
            let _ = publisher.unbounded_send(snapshot);
        }),
        commands: receiver,
        auto_download,
    });
    thread::Builder::new()
        .name("update-checker".to_owned())
        .spawn(move || engine.run())
        .map_err(|error| format!("cannot start the update thread: {error}"))?;
    Ok((Worker { commands, layout, key, image }, snapshots))
}

impl Updates {
    fn new(settings: &Entity<Settings>, started: Option<Started>, cx: &mut Context<Self>) -> Self {
        let download_automatically = settings.read(cx).get().updates.download_automatically;
        let Some((worker, snapshots)) = started else {
            return Self {
                snapshot: Snapshot::disabled(),
                worker: None,
                restart_failure: None,
                download_automatically,
                _pump: None,
                _settings: None,
                _quit: None,
            };
        };
        Self {
            snapshot: Snapshot { state: UpdateState::Checking, last_checked: None, check_again_at: SystemTime::UNIX_EPOCH },
            worker: Some(worker),
            restart_failure: None,
            download_automatically,
            _pump: Some(Self::spawn_pump(snapshots, cx)),
            _settings: Some(cx.observe(settings, |this, settings, cx| {
                let wanted = settings.read(cx).get().updates.download_automatically;
                if wanted != this.download_automatically {
                    this.download_automatically = wanted;
                    this.send(Command::AutoDownload(wanted));
                }
            })),
            _quit: Some(cx.on_app_quit(|this, _| {
                this.send(Command::Shutdown);
                async {}
            })),
        }
    }

    /// The worker publishes in bursts (a download reports every percent); what
    /// piled up while the UI was busy is shown as its last state.
    fn spawn_pump(mut snapshots: UnboundedReceiver<Snapshot>, cx: &mut Context<Self>) -> Task<()> {
        cx.spawn(async move |this, cx| {
            while let Some(mut latest) = snapshots.next().await {
                while let Ok(next) = snapshots.try_recv() {
                    latest = next;
                }
                let alive = this
                    .update(cx, |this, cx| {
                        if this.snapshot != latest {
                            this.snapshot = latest;
                            cx.notify();
                        }
                    })
                    .is_ok();
                if !alive {
                    return;
                }
            }
        })
    }

    pub fn snapshot(&self) -> &Snapshot {
        &self.snapshot
    }

    pub fn state(&self) -> &UpdateState {
        &self.snapshot.state
    }

    /// Why the last restart did not start, while the update is still ready.
    pub fn restart_failure(&self) -> Option<&Failure> {
        self.restart_failure.as_ref()
    }

    /// Settings › About › Check now. The worker ignores it earlier than
    /// `snapshot().check_again_at`, so the button is disabled until then.
    pub fn check_now(&self) {
        self.send(Command::CheckNow);
    }

    /// The Update button, and the toolbar icon's click while an update is available.
    pub fn start_download(&self) {
        self.send(Command::StartDownload);
    }

    /// Cancel while downloading: the partial file is deleted and the update is
    /// available again; automatic download leaves that version alone.
    pub fn cancel(&self) {
        self.send(Command::Cancel);
    }

    /// Restart to update, only while [`UpdateState::Ready`]. Checks the staged
    /// update once more (hash and signature), starts a copy of this exe as the
    /// helper that puts it in place after this process ends (`apply.rs`), then
    /// quits through the normal quit path (capture stopped, everything saved).
    /// When the check or the start fails nothing has changed: the app keeps
    /// running and [`Updates::restart_failure`] says why. The check reads and
    /// hashes the archive (about 16 MB), a tenth of a second on this click.
    pub fn restart(&mut self, cx: &mut Context<Self>) {
        let Some(worker) = &self.worker else {
            return;
        };
        if !matches!(self.snapshot.state, UpdateState::Ready { .. }) {
            return;
        }
        match apply::start_helper(&worker.layout, &worker.key, &worker.image) {
            Ok(()) => {
                crate::app_log::info("restarting into the staged update");
                cx.quit();
            }
            Err(detail) => {
                crate::app_log::error(format!("the restart did not start: {detail}"));
                self.restart_failure = Some(Failure::Restart(detail));
                cx.notify();
            }
        }
    }

    fn send(&self, command: Command) {
        if let Some(worker) = &self.worker {
            let _ = worker.commands.send(command);
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui::TestAppContext;

    use super::*;
    use crate::settings::AppSettings;

    fn settings(cx: &mut TestAppContext, download_automatically: bool) -> Entity<Settings> {
        cx.update(|cx| {
            let mut all = AppSettings::default();
            all.updates.download_automatically = download_automatically;
            cx.new(|_| Settings::in_memory(all))
        })
    }

    fn worker_with(commands: Sender<Command>) -> Worker {
        Worker {
            commands,
            layout: Layout::new(std::env::temp_dir().join("belisk-updates-service-test")),
            key: crate::update::stage::tests::key(),
            image: apply::ImageLock::of_this_test_exe(),
        }
    }

    fn started_with(commands: Sender<Command>) -> Started {
        (worker_with(commands), unbounded().1)
    }

    #[gpui::test]
    fn without_a_worker_the_state_is_disabled_and_every_action_does_nothing(cx: &mut TestAppContext) {
        let settings = settings(cx, true);
        let updates = cx.update(|cx| cx.new(|cx| Updates::new(&settings, None, cx)));

        updates.read_with(cx, |updates, _| {
            assert_eq!(updates.state(), &UpdateState::Disabled);
            updates.check_now();
            updates.start_download();
            updates.cancel();
        });
        updates.update(cx, |updates, cx| updates.restart(cx));
        updates.read_with(cx, |updates, _| assert!(updates.restart_failure().is_none()));
    }

    #[gpui::test]
    fn the_actions_reach_the_worker_and_the_snapshots_reach_the_entity(cx: &mut TestAppContext) {
        let settings = settings(cx, true);
        let (commands, received) = channel();
        let (publisher, snapshots) = unbounded();
        let updates = cx.update(|cx| cx.new(|cx| Updates::new(&settings, Some((worker_with(commands), snapshots)), cx)));

        updates.read_with(cx, |updates, _| {
            updates.check_now();
            updates.start_download();
            updates.cancel();
        });
        assert!(matches!(received.try_recv(), Ok(Command::CheckNow)));
        assert!(matches!(received.try_recv(), Ok(Command::StartDownload)));
        assert!(matches!(received.try_recv(), Ok(Command::Cancel)));

        let ready = Snapshot {
            state: UpdateState::Ready { version: Version::new(1, 1, 0) },
            last_checked: None,
            check_again_at: SystemTime::UNIX_EPOCH,
        };
        publisher.unbounded_send(ready.clone()).unwrap();
        cx.run_until_parked();
        updates.read_with(cx, |updates, _| assert_eq!(updates.snapshot(), &ready));
    }

    #[gpui::test]
    fn turning_automatic_download_off_or_on_in_settings_tells_the_worker_once(cx: &mut TestAppContext) {
        let settings = settings(cx, true);
        let (commands, received) = channel();
        let _updates = cx.update(|cx| cx.new(|cx| Updates::new(&settings, Some(started_with(commands)), cx)));

        settings.update(cx, |settings, cx| settings.update(cx, |all| all.updates.download_automatically = false));
        settings.update(cx, |settings, cx| settings.update(cx, |all| all.overlay.show_dps = false));
        settings.update(cx, |settings, cx| settings.update(cx, |all| all.updates.download_automatically = true));

        assert!(matches!(received.try_recv(), Ok(Command::AutoDownload(false))));
        assert!(matches!(received.try_recv(), Ok(Command::AutoDownload(true))));
        assert!(received.try_recv().is_err(), "an unrelated setting sends nothing");
    }

    #[gpui::test]
    fn a_restart_that_cannot_start_the_staged_exe_keeps_the_app_running_and_says_why(cx: &mut TestAppContext) {
        let settings = settings(cx, true);
        let (commands, _received) = channel();
        let ready = Snapshot {
            state: UpdateState::Ready { version: Version::new(1, 1, 0) },
            last_checked: None,
            check_again_at: SystemTime::UNIX_EPOCH,
        };
        let updates = cx.update(|cx| cx.new(|cx| Updates::new(&settings, Some(started_with(commands)), cx)));
        updates.update(cx, |updates, _| updates.snapshot = ready);

        updates.update(cx, |updates, cx| updates.restart(cx));

        updates.read_with(cx, |updates, _| {
            assert!(matches!(updates.restart_failure(), Some(Failure::Restart(_))));
            assert!(matches!(updates.state(), UpdateState::Ready { .. }), "still ready: nothing was changed");
        });
    }

    #[gpui::test]
    fn restart_does_nothing_unless_an_update_is_ready(cx: &mut TestAppContext) {
        let settings = settings(cx, true);
        let (commands, _received) = channel();
        let updates = cx.update(|cx| cx.new(|cx| Updates::new(&settings, Some(started_with(commands)), cx)));

        updates.update(cx, |updates, cx| updates.restart(cx));

        updates.read_with(cx, |updates, _| assert!(updates.restart_failure().is_none()));
    }

    #[gpui::test]
    fn a_run_that_is_not_live_never_starts_the_worker_so_nothing_is_contacted(cx: &mut TestAppContext) {
        let settings = settings(cx, true);

        cx.update(|cx| install(&settings, false, None, cx));

        let updates = cx.update(|cx| global(cx));
        updates.read_with(cx, |updates, _| assert_eq!(updates.state(), &UpdateState::Disabled));
    }

    #[gpui::test]
    fn a_build_without_a_repository_has_disabled_updates(cx: &mut TestAppContext) {
        if option_env!("UPDATE_REPO").is_some() {
            return;
        }
        let settings = settings(cx, true);

        cx.update(|cx| install(&settings, true, None, cx));

        let updates = cx.update(|cx| global(cx));
        updates.read_with(cx, |updates, _| assert_eq!(updates.state(), &UpdateState::Disabled));
    }
}
