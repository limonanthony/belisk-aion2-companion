//! What's new after an update: at launch, the versions newer than the last one
//! the player ran are shown once in their own window ([`view`]), as soon as no
//! fight is on. A fresh install, a version already seen, or one whose notes
//! have no bullets opens nothing; replays and dev captures, which never read
//! the player's settings, skip the check.

mod view;

use std::time::Duration;

use gpui::{App, AsyncApp, Entity};

use crate::changelog::{self, WhatsNew};
use crate::i18n;
use crate::meter::MeterModel;
use crate::secondary;
use crate::settings;

pub use view::{OpenChangelog, WhatsNewView, EXPECTED_SIZE};

/// How often a fight under way is asked whether it is over.
const FIGHT_CHECK: Duration = Duration::from_secs(2);

/// Opens What's new once this launch, if there is something new, once you are
/// not fighting.
pub fn install(meter_model: &Entity<MeterModel>, cx: &mut App) {
    let meter = meter_model.read(cx).meter().cloned();
    let model = meter_model.clone();
    schedule(
        move |_| meter.as_ref().is_some_and(|meter| meter.in_fight()),
        move |whats_new, cx| secondary::open_whats_new(&model, whats_new, cx),
        cx,
    );
}

fn schedule(in_fight: impl Fn(&App) -> bool + 'static, open: impl FnOnce(WhatsNew, &mut App) + 'static, cx: &mut App) {
    let store = settings::global(cx);
    let stored = store.read(cx);
    if stored.data_dir().is_none() {
        return;
    }
    let Some(whats_new) = changelog::whats_new(stored.get().last_seen_version, i18n::language()) else {
        // A version whose notes have no bullets announces nothing; it is seen
        // all the same, so later launches parse nothing.
        settings::mark_version_seen(cx);
        return;
    };
    cx.spawn(async move |cx: &mut AsyncApp| {
        while cx.update(|cx| in_fight(cx)) {
            cx.background_executor().timer(FIGHT_CHECK).await;
        }
        cx.update(|cx| open(whats_new, cx));
    })
    .detach();
}

#[cfg(test)]
mod tests {
    use std::cell::{Cell, RefCell};
    use std::path::PathBuf;
    use std::rc::Rc;
    use std::time::{SystemTime, UNIX_EPOCH};

    use gpui::{AppContext as _, TestAppContext};

    use super::*;
    use crate::changelog::Version;
    use crate::i18n::{set_language, Language};
    use crate::settings::Settings;

    struct TestDir(PathBuf);

    impl TestDir {
        fn new(name: &str) -> Self {
            let nanos = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos();
            let dir = std::env::temp_dir().join(format!("aion-whats-new-{name}-{}-{nanos}", std::process::id()));
            std::fs::create_dir_all(&dir).unwrap();
            Self(dir)
        }

        /// A settings file from an earlier run, which last ran `seen`.
        fn with_settings(name: &str, seen: Option<&str>) -> Self {
            let dir = Self::new(name);
            let seen = seen.map_or(String::new(), |seen| format!(r#", "lastSeenVersion": "{seen}""#));
            std::fs::write(dir.0.join("settings.json"), format!(r#"{{ "version": 2{seen} }}"#)).unwrap();
            dir
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    struct Launch {
        fighting: Rc<Cell<bool>>,
        opened: Rc<RefCell<Vec<WhatsNew>>>,
    }

    /// A launch on the settings `dir` holds (none: in memory), as the app
    /// starts, with a fight that is on while `fighting` says so.
    fn launch(dir: Option<&TestDir>, fighting: bool, cx: &mut TestAppContext) -> Launch {
        set_language(Language::English);
        let launch = Launch { fighting: Rc::new(Cell::new(fighting)), opened: Rc::default() };
        let (fight, opened) = (launch.fighting.clone(), launch.opened.clone());
        cx.update(|cx| {
            let store = cx.new(|_| match dir {
                Some(dir) => Settings::load(Some(&dir.0)),
                None => Settings::in_memory(settings::AppSettings::default()),
            });
            settings::install(&store, cx);
            schedule(move |_| fight.get(), move |whats_new, _| opened.borrow_mut().push(whats_new), cx);
        });
        cx.run_until_parked();
        launch
    }

    fn opened_versions(launch: &Launch) -> Vec<String> {
        launch.opened.borrow().iter().flat_map(|shown| shown.releases.iter().map(|release| release.version.to_string())).collect()
    }

    #[gpui::test]
    fn a_version_jump_opens_the_new_versions_notes_once(cx: &mut TestAppContext) {
        let dir = TestDir::with_settings("jump", Some("0.9.0"));
        let launch = launch(Some(&dir), false, cx);
        assert_eq!(launch.opened.borrow().len(), 1);
        let shown = launch.opened.borrow()[0].clone();
        assert_eq!(shown.since, Some(Version::new(0, 9, 0)));
        assert_eq!(shown.current, Version::current());
        assert_eq!(opened_versions(&launch), [Version::current().to_string()]);
    }

    #[gpui::test]
    fn a_settings_file_from_before_the_reset_sees_the_first_notes(cx: &mut TestAppContext) {
        let dir = TestDir::with_settings("pre-reset", None);
        let launch = launch(Some(&dir), false, cx);
        assert_eq!(opened_versions(&launch), ["1.0.0"]);
        assert_eq!(launch.opened.borrow()[0].since, None);
    }

    #[gpui::test]
    fn a_fresh_install_opens_nothing(cx: &mut TestAppContext) {
        let dir = TestDir::new("fresh");
        let launch = launch(Some(&dir), false, cx);
        assert!(launch.opened.borrow().is_empty());
    }

    #[gpui::test]
    fn the_same_version_again_opens_nothing(cx: &mut TestAppContext) {
        let dir = TestDir::with_settings("same", Some(&Version::current().to_string()));
        let launch = launch(Some(&dir), false, cx);
        assert!(launch.opened.borrow().is_empty());
    }

    #[gpui::test]
    fn replays_and_captures_never_open_it(cx: &mut TestAppContext) {
        let launch = launch(None, false, cx);
        assert!(launch.opened.borrow().is_empty(), "settings in memory have no data folder");
    }

    #[gpui::test]
    fn during_a_fight_it_waits_and_opens_once_the_fight_is_over(cx: &mut TestAppContext) {
        let dir = TestDir::with_settings("fight", Some("0.9.0"));
        let launch = launch(Some(&dir), true, cx);
        cx.executor().advance_clock(FIGHT_CHECK * 5);
        cx.run_until_parked();
        assert!(launch.opened.borrow().is_empty(), "nothing during the fight");

        launch.fighting.set(false);
        cx.executor().advance_clock(FIGHT_CHECK);
        cx.run_until_parked();
        assert_eq!(launch.opened.borrow().len(), 1, "opened once the fight ended");
        cx.executor().advance_clock(FIGHT_CHECK * 5);
        cx.run_until_parked();
        assert_eq!(launch.opened.borrow().len(), 1, "and only once");
    }
}
