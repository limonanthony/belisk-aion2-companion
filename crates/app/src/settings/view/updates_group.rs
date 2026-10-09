//! Settings › About › Updates (docs/gpui-mockups/approved/settings.html,
//! tokens.md "Settings › About › Updates"): the status row with its one
//! action on the right edge, "Download updates automatically" and the
//! Changelog row, under a title that says what is contacted. Errors are
//! field messages under the status, never a pop-up.

use std::time::{Duration, SystemTime, UNIX_EPOCH};

use super::super::widgets::progress_bar;
use super::*;
use crate::clock::{clock_time, LocalZone};
use crate::update::{Failure, Snapshot, UpdateState};

/// What the status row shows for one state of the updates.
#[derive(Clone, Debug, PartialEq)]
pub(super) struct Status {
    pub label: String,
    pub text: Option<String>,
    pub progress: Option<Progress>,
    pub message: Option<Message>,
    /// The spinner badge while GitHub or the download is being checked.
    pub busy: bool,
    pub actions: Vec<Action>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Progress {
    pub fraction: f32,
    pub paused: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct Message {
    pub tone: MessageTone,
    pub icon: Icon,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum MessageTone {
    Bad,
    Warn,
}

/// The status row's buttons, left to right; at most one is primary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) enum Action {
    ReleaseNotes(String),
    Update(Version),
    TryDownloadAgain,
    TryCheckAgain { enabled: bool },
    CheckNow { enabled: bool },
    Cancel,
    Restart,
}

impl Status {
    /// None when updates are off in this build: About says so in one line.
    /// `can_check`: Check now is open again (see [`check_now_opens_at`]).
    pub fn of(snapshot: &Snapshot, restart_failure: Option<&Failure>, can_check: bool, running: Version) -> Option<Self> {
        let status = |label: String, text: Option<String>| Self {
            label,
            text,
            progress: None,
            message: None,
            busy: false,
            actions: Vec::new(),
        };
        Some(match &snapshot.state {
            UpdateState::Disabled => return None,
            UpdateState::UpToDate { checked_at } => {
                let version = running.to_string();
                let text = if can_check {
                    fl!("settings-updates-up-to-date-text", version = version, time = time_words(*checked_at))
                } else {
                    fl!("settings-updates-just-checked-text", version = version)
                };
                Self { actions: vec![Action::CheckNow { enabled: can_check }], ..status(fl!("settings-updates-up-to-date"), Some(text)) }
            }
            UpdateState::Checking => Self {
                busy: true,
                actions: vec![Action::CheckNow { enabled: false }],
                ..status(fl!("settings-updates-checking"), Some(fl!("settings-updates-checking-text")))
            },
            UpdateState::Available { release, last_failure } => {
                let size = megabytes(release.size);
                let text = match release.published {
                    Some(day) => fl!("settings-updates-available-text", date = i18n::day_month(day.day, day.month), size = size),
                    None => fl!("settings-updates-available-undated-text", size = size),
                };
                let primary = if last_failure.is_some() { Action::TryDownloadAgain } else { Action::Update(release.version) };
                Self {
                    message: last_failure.as_ref().map(failure_message),
                    actions: vec![Action::ReleaseNotes(release.notes_url.clone()), primary],
                    ..status(fl!("settings-updates-available", version = release.version.to_string()), Some(text))
                }
            }
            &UpdateState::Downloading { version, downloaded, total, paused_for_fight } => {
                let (done, total_words) = (megabyte_figure(downloaded.min(total)), megabytes(total));
                let (label, text) = if paused_for_fight {
                    (fl!("settings-updates-paused"), fl!("settings-updates-paused-text", done = done, total = total_words))
                } else {
                    (
                        fl!("settings-updates-downloading", version = version.to_string()),
                        fl!("settings-updates-downloading-text", done = done, total = total_words),
                    )
                };
                let fraction = if total == 0 { 0. } else { downloaded.min(total) as f32 / total as f32 };
                Self {
                    progress: Some(Progress { fraction, paused: paused_for_fight }),
                    actions: vec![Action::Cancel],
                    ..status(label, Some(text))
                }
            }
            &UpdateState::Verifying { version } => Self {
                progress: Some(Progress { fraction: 1., paused: false }),
                busy: true,
                ..status(
                    fl!("settings-updates-verifying"),
                    Some(fl!("settings-updates-verifying-text", version = version.to_string())),
                )
            },
            &UpdateState::Ready { version } => Self {
                message: restart_failure.map(failure_message),
                actions: vec![Action::Restart],
                ..status(
                    fl!("settings-updates-ready", version = version.to_string()),
                    Some(fl!("settings-updates-ready-text", version = version.to_string())),
                )
            },
            UpdateState::Failed { failure } => Self {
                message: Some(failure_message(failure)),
                actions: vec![Action::TryCheckAgain { enabled: can_check }],
                ..status(fl!("settings-updates-not-checked"), last_checked_words(snapshot))
            },
            &UpdateState::Unreachable { next_try } => Self {
                message: Some(Message {
                    tone: MessageTone::Warn,
                    icon: Icon::CloudOff,
                    text: fl!("settings-updates-unreachable", time = time_words(next_try)),
                }),
                actions: vec![Action::CheckNow { enabled: can_check }],
                ..status(fl!("settings-updates-not-checked"), last_checked_words(snapshot))
            },
            &UpdateState::RateLimited { until } => Self {
                message: Some(Message {
                    tone: MessageTone::Warn,
                    icon: Icon::Clock,
                    text: fl!("settings-updates-rate-limited", time = time_words(until)),
                }),
                actions: vec![Action::CheckNow { enabled: can_check }],
                ..status(fl!("settings-updates-not-checked"), last_checked_words(snapshot))
            },
        })
    }
}

/// Check now (and Try again after a failed check) does nothing before this:
/// a minute after any check, and while GitHub limits, until its reset.
pub(super) fn check_now_opens_at(snapshot: &Snapshot) -> SystemTime {
    match snapshot.state {
        UpdateState::RateLimited { until } => until.max(snapshot.check_again_at),
        _ => snapshot.check_again_at,
    }
}

fn failure_message(failure: &Failure) -> Message {
    let text = match failure {
        Failure::Check(_) => fl!("settings-updates-failed-check"),
        Failure::Download(_) => fl!("settings-updates-failed-download"),
        Failure::ChecksumMismatch => fl!("settings-updates-failed-hash"),
        Failure::SignatureMismatch => fl!("settings-updates-failed-signature"),
        Failure::MissingChecksum => fl!("settings-updates-failed-no-hash"),
        Failure::MissingSignature => fl!("settings-updates-failed-no-signature"),
        Failure::Stage(_) => fl!("settings-updates-failed-unpack"),
        Failure::Restart(_) => fl!("settings-updates-failed-restart"),
    };
    Message { tone: MessageTone::Bad, icon: Icon::TriangleAlert, text }
}

fn last_checked_words(snapshot: &Snapshot) -> Option<String> {
    snapshot.last_checked.map(|at| fl!("settings-updates-last-checked", time = time_words(at)))
}

/// "21:02" on this PC's clock.
fn time_words(at: SystemTime) -> String {
    let ms = at.duration_since(UNIX_EPOCH).map_or(0, |since| since.as_millis() as i64);
    clock_time(&LocalZone, ms).to_string()
}

/// "4.2 MB" (French: "4,2 Mo"), in the million bytes of the 1 MB/s cap.
fn megabytes(bytes: u64) -> String {
    fl!("unit-megabytes", value = megabyte_figure(bytes))
}

/// "4.2" (French: "4,2").
fn megabyte_figure(bytes: u64) -> String {
    let tenths = (bytes + 50_000) / 100_000;
    fl!("unit-decimal", whole = (tenths / 10).to_string(), tenth = (tenths % 10).to_string())
}

impl SettingsView {
    /// Follows when Check now opens again while About is shown: the button
    /// stays disabled while the wake is armed, and the wake redraws About when
    /// it opens. Elsewhere nothing is armed.
    pub(super) fn follow_check_now(&mut self, cx: &mut Context<Self>) {
        let opens_at = check_now_opens_at(self.updates.read(cx).snapshot());
        if self.check_now_wake.as_ref().is_some_and(|(at, _)| *at == opens_at) {
            return;
        }
        let wait = opens_at.duration_since(SystemTime::now()).ok().filter(|wait| !wait.is_zero());
        self.check_now_wake = wait.map(|wait: Duration| {
            let wake = cx.spawn(async move |this, cx| {
                cx.background_executor().timer(wait).await;
                let _ = this.update(cx, |this, cx| {
                    this.check_now_wake = None;
                    cx.notify();
                });
            });
            (opens_at, wake)
        });
    }

    /// The group's title and its rows.
    pub(super) fn updates_group(&mut self, settings: &AppSettings, window: &Window, cx: &mut Context<Self>) -> [Div; 2] {
        let changelog_focus = self.focus_of("open-changelog", true, window, cx);
        let open_changelog = self.button_handler(cx, |this, window, cx| {
            this.open_changelog(ChangelogPage::new(changelog::load(i18n::language()), Version::current()), window, cx)
        });
        let changelog_row = row(
            fl!("settings-about-changelog"),
            Some(description(fl!("settings-about-changelog-text"))),
            page_button("open-changelog", fl!("settings-about-open-changelog"), &changelog_focus, open_changelog),
        );

        let updates = self.updates.read(cx);
        let can_check = self.check_now_wake.is_none();
        let status = Status::of(updates.snapshot(), updates.restart_failure(), can_check, Version::current());
        let Some(status) = status else {
            let off = div()
                .debug_selector(|| "updates-off".to_string())
                .py(px(11.))
                .px(px(16.))
                .text_size(px(12.))
                .text_color(rgba(MUTED))
                .child(fl!("settings-updates-disabled"));
            return [group_title(fl!("settings-about-updates"), false), group(vec![off, changelog_row])];
        };

        let status_row = self.status_row(status, window, cx);
        let auto = self.switch_row(
            "auto-download",
            fl!("settings-updates-auto"),
            Some(fl!("settings-updates-auto-text")),
            settings.updates.download_automatically,
            |all, on| all.updates.download_automatically = on,
            window,
            cx,
        );
        let title = group_title(fl!("settings-about-updates"), false)
            .child(div().flex_1().min_w_0().font_weight(FontWeight::NORMAL).text_color(rgba(FAINT)).child(fl!("settings-updates-note")));
        [title, group(vec![status_row, auto, changelog_row])]
    }

    fn status_row(&mut self, status: Status, window: &Window, cx: &mut Context<Self>) -> Div {
        let animate = !cx.reduce_motion();
        let text = div()
            .flex()
            .flex_col()
            .children(status.progress.map(|progress| {
                progress_bar("update-progress", fl!("settings-updates-progress"), progress.fraction, progress.paused)
            }))
            .children(status.text.map(description))
            .children(status.message.map(|message| {
                let colour = match message.tone {
                    MessageTone::Bad => rgb(widgets::BAD),
                    MessageTone::Warn => rgb(widgets::WARN),
                };
                note(message.icon, message.text, colour).debug_selector(|| "update-message".to_string())
            }))
            .children(self.error_under("release-notes"));
        let mut controls: Vec<AnyElement> = Vec::new();
        if status.busy {
            controls.push(
                badge(Some(spinner("update-busy", 12., rgba(MUTED), animate)), fl!("settings-updates-checking-badge"), Tone::Neutral)
                    .into_any_element(),
            );
        }
        for action in status.actions {
            controls.push(self.action_button(action, window, cx));
        }
        row(status.label, Some(text.into_any_element()), div().flex().items_center().gap(px(8.)).children(controls))
    }

    fn action_button(&mut self, action: Action, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let updates = self.updates.clone();
        let (key, label, glyph, kind, press): (&'static str, String, Option<Icon>, ButtonKind, Option<Handler<()>>) = match action {
            Action::ReleaseNotes(url) => (
                "release-notes",
                fl!("settings-updates-release-notes"),
                Some(Icon::External),
                ButtonKind::Ghost,
                Some(self.button_handler(cx, move |this, _, cx| this.hand_over("release-notes", files::open_web_page(&url), cx))),
            ),
            Action::Update(version) => (
                "update-to",
                fl!("settings-updates-update-to", version = version.to_string()),
                Some(Icon::Download),
                ButtonKind::Primary,
                Some(Rc::new(move |(), _, cx| updates.read(cx).start_download())),
            ),
            Action::TryDownloadAgain => (
                "update-try-again",
                fl!("settings-updates-try-again"),
                Some(Icon::Download),
                ButtonKind::Primary,
                Some(Rc::new(move |(), _, cx| updates.read(cx).start_download())),
            ),
            Action::TryCheckAgain { enabled } => (
                "update-try-again",
                fl!("settings-updates-try-again"),
                Some(Icon::RefreshCw),
                ButtonKind::Primary,
                enabled.then(|| -> Handler<()> { Rc::new(move |(), _, cx| updates.read(cx).check_now()) }),
            ),
            Action::CheckNow { enabled } => (
                "update-check-now",
                fl!("settings-updates-check-now"),
                Some(Icon::RefreshCw),
                ButtonKind::Secondary,
                enabled.then(|| -> Handler<()> { Rc::new(move |(), _, cx| updates.read(cx).check_now()) }),
            ),
            Action::Cancel => (
                "update-cancel",
                fl!("settings-updates-cancel"),
                None,
                ButtonKind::Ghost,
                Some(Rc::new(move |(), _, cx| updates.read(cx).cancel())),
            ),
            Action::Restart => (
                "update-restart",
                fl!("settings-updates-restart"),
                Some(Icon::RotateCw),
                ButtonKind::Primary,
                Some(Rc::new(move |(), _, cx| updates.update(cx, |updates, cx| updates.restart(cx)))),
            ),
        };
        let focus = self.focus_of(key, true, window, cx);
        button(key, label, glyph, kind, false, &focus, press)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changelog::Date;
    use crate::i18n::{set_language, Language};
    use crate::update::ReleaseInfo;

    const RUNNING: Version = Version::new(1, 5, 1);
    const NEXT: Version = Version::new(1, 6, 0);

    fn at(now: SystemTime, state: UpdateState) -> Snapshot {
        Snapshot { state, last_checked: Some(now - Duration::from_secs(600)), check_again_at: now - Duration::from_secs(1) }
    }

    fn release() -> ReleaseInfo {
        ReleaseInfo {
            version: NEXT,
            size: 4_200_000,
            notes_url: "https://github.com/o/r/releases/tag/v1.6.0".into(),
            published: Some(Date { year: 2026, month: 10, day: 21 }),
        }
    }

    fn status(snapshot: &Snapshot, now: SystemTime) -> Status {
        Status::of(snapshot, None, now >= check_now_opens_at(snapshot), RUNNING).expect("updates are on")
    }

    #[test]
    fn each_state_reads_as_drawn_with_one_bright_action_at_most() {
        set_language(Language::English);
        let now = SystemTime::now();
        let checked = time_words(now - Duration::from_secs(600));

        let up_to_date = status(&at(now, UpdateState::UpToDate { checked_at: now - Duration::from_secs(600) }), now);
        assert_eq!(up_to_date.label, "Up to date");
        assert_eq!(
            up_to_date.text.as_deref(),
            Some(format!("1.5.1 is the latest version. Checked at {checked}; checked again at launch and every hour.").as_str())
        );
        assert_eq!(up_to_date.actions, vec![Action::CheckNow { enabled: true }]);

        let checking = status(&at(now, UpdateState::Checking), now);
        assert_eq!((checking.label.as_str(), checking.busy), ("Checking for updates…", true));
        assert_eq!(checking.actions, vec![Action::CheckNow { enabled: false }]);

        let available = status(&at(now, UpdateState::Available { release: release(), last_failure: None }), now);
        assert_eq!(available.label, "Version 1.6.0 is available");
        assert_eq!(
            available.text.as_deref(),
            Some("Released 21 Oct · 4.2 MB. Its notes are on GitHub until you update; then What's new shows them.")
        );
        assert_eq!(available.actions, vec![Action::ReleaseNotes(release().notes_url), Action::Update(NEXT)]);
        assert_eq!(available.message, None);

        let downloading = UpdateState::Downloading { version: NEXT, downloaded: 2_600_000, total: 4_200_000, paused_for_fight: false };
        let downloading = status(&at(now, downloading), now);
        assert_eq!(downloading.label, "Downloading 1.6.0");
        assert_eq!(downloading.text.as_deref(), Some("2.6 of 4.2 MB · at most 1 MB/s, paused while you fight"));
        assert_eq!(downloading.progress.map(|progress| progress.paused), Some(false));
        assert_eq!(downloading.actions, vec![Action::Cancel]);

        let paused = UpdateState::Downloading { version: NEXT, downloaded: 2_600_000, total: 4_200_000, paused_for_fight: true };
        let paused = status(&at(now, paused), now);
        assert_eq!(paused.label, "Download paused");
        assert_eq!(paused.text.as_deref(), Some("2.6 of 4.2 MB · resumes when the fight ends"));
        assert_eq!(paused.progress.map(|progress| progress.paused), Some(true));

        let verifying = status(&at(now, UpdateState::Verifying { version: NEXT }), now);
        assert_eq!(verifying.label, "Checking the download");
        assert_eq!(verifying.progress.map(|progress| progress.fraction), Some(1.));
        assert!(verifying.busy && verifying.actions.is_empty());

        let ready = status(&at(now, UpdateState::Ready { version: NEXT }), now);
        assert_eq!(ready.label, "1.6.0 is ready to install");
        assert_eq!(ready.actions, vec![Action::Restart]);

        let bright = |status: &Status| {
            status
                .actions
                .iter()
                .filter(|action| matches!(action, Action::Update(_) | Action::TryDownloadAgain | Action::TryCheckAgain { .. } | Action::Restart))
                .count()
        };
        for each in [&up_to_date, &checking, &available, &downloading, &paused, &verifying, &ready] {
            assert!(bright(each) <= 1, "{each:?}");
        }
    }

    #[test]
    fn errors_are_field_messages_red_for_a_failed_check_amber_for_github() {
        set_language(Language::English);
        let now = SystemTime::now();

        let failed_download = UpdateState::Available { release: release(), last_failure: Some(Failure::SignatureMismatch) };
        let failed_download = status(&at(now, failed_download), now);
        let message = failed_download.message.expect("a message");
        assert_eq!((message.tone, message.icon), (MessageTone::Bad, Icon::TriangleAlert));
        assert_eq!(
            message.text,
            "The last download did not pass its check (signature mismatch), so it was deleted. Nothing was installed."
        );
        assert_eq!(failed_download.actions.last(), Some(&Action::TryDownloadAgain));

        let failed_check = status(&at(now, UpdateState::Failed { failure: Failure::Check("odd answer".into()) }), now);
        assert_eq!(failed_check.message.map(|message| message.tone), Some(MessageTone::Bad));
        assert_eq!(failed_check.actions, vec![Action::TryCheckAgain { enabled: true }]);

        let next_try = now + Duration::from_secs(3600);
        let unreachable = status(&at(now, UpdateState::Unreachable { next_try }), now);
        let message = unreachable.message.expect("a message");
        assert_eq!((message.tone, message.icon), (MessageTone::Warn, Icon::CloudOff));
        assert_eq!(
            message.text,
            format!("GitHub could not be reached: no internet, or GitHub is down. Next try at {}.", time_words(next_try))
        );
        assert_eq!(unreachable.actions, vec![Action::CheckNow { enabled: true }]);

        let until = now + Duration::from_secs(720);
        let limited = status(&at(now, UpdateState::RateLimited { until }), now);
        assert_eq!(limited.message.as_ref().map(|message| (message.tone, message.icon)), Some((MessageTone::Warn, Icon::Clock)));
        assert_eq!(limited.actions, vec![Action::CheckNow { enabled: false }], "disabled until GitHub's reset");
        assert_eq!(limited.label, "Could not check for updates");

        let ready = Status::of(&at(now, UpdateState::Ready { version: NEXT }), Some(&Failure::Restart("gone".into())), true, RUNNING).unwrap();
        assert_eq!(ready.message.map(|message| message.tone), Some(MessageTone::Bad));
    }

    #[test]
    fn check_now_stays_disabled_until_check_again_at_and_then_says_when_it_was_checked() {
        set_language(Language::English);
        let now = SystemTime::now();
        let mut snapshot = at(now, UpdateState::UpToDate { checked_at: now });
        snapshot.check_again_at = now + Duration::from_secs(60);

        let just_checked = status(&snapshot, now);
        assert_eq!(just_checked.actions, vec![Action::CheckNow { enabled: false }]);
        assert_eq!(
            just_checked.text.as_deref(),
            Some("1.5.1 is the latest version. Checked just now; you can check again in a minute.")
        );

        let a_minute_later = status(&snapshot, now + Duration::from_secs(60));
        assert_eq!(a_minute_later.actions, vec![Action::CheckNow { enabled: true }]);
        assert!(a_minute_later.text.unwrap().contains("Checked at"));
    }

    #[test]
    fn updates_off_in_this_build_have_no_status_row() {
        assert_eq!(Status::of(&Snapshot::disabled(), None, true, RUNNING), None);
    }

    #[test]
    fn sizes_and_dates_follow_the_language() {
        set_language(Language::French);
        let now = SystemTime::now();
        let available = status(&at(now, UpdateState::Available { release: release(), last_failure: None }), now);
        assert!(available.text.unwrap().starts_with("Publiée le 21 oct. · 4,2 Mo."));
        assert_eq!(available.label, "La version 1.6.0 est disponible");
    }
}
