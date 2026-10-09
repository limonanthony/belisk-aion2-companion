//! Settings › About › Changelog (docs/gpui-mockups/approved/settings.html,
//! tokens.md "Settings › About › Changelog"): a page of About with its way
//! back, this build's versions newest first, one open at a time (the newest
//! at first), five shown and five more with each "Show older".
//!
//! The rows shown are all drawn: they differ in height (the open one holds its
//! notes), so GPUI's uniform list does not fit, and its variable-height list
//! would take over the page's own scrolling, keyboard reveal and thumb for
//! this one page. "Show older" keeps the list as short as the player asked.

use gpui::Stateful;

use super::super::widgets::expandable_row;
use super::*;
use crate::changelog::{Changelog, Release};

/// Versions shown at first, and added by each "Show older".
const PAGE_SIZE: usize = 5;

pub(super) struct ChangelogPage {
    /// Newest first, up to this build.
    releases: Vec<Release>,
    current: Version,
    open: Option<usize>,
    shown: usize,
}

impl ChangelogPage {
    pub(super) fn new(changelog: Changelog, current: Version) -> Self {
        let releases = up_to(changelog, current);
        Self { open: (!releases.is_empty()).then_some(0), shown: releases.len().min(PAGE_SIZE), releases, current }
    }

    /// The same versions, in the language now in force.
    pub(super) fn retell(&mut self, changelog: Changelog) {
        self.releases = up_to(changelog, self.current);
        self.shown = self.shown.min(self.releases.len());
        self.open = self.open.filter(|open| *open < self.shown);
    }

    fn toggle(&mut self, index: usize) {
        self.open = if self.open == Some(index) { None } else { Some(index) };
    }

    /// Shows five more; the first of them, if any.
    fn show_older(&mut self) -> Option<Version> {
        let first_new = self.releases.get(self.shown).map(|release| release.version);
        self.shown = (self.shown + PAGE_SIZE).min(self.releases.len());
        first_new
    }
}

fn up_to(changelog: Changelog, current: Version) -> Vec<Release> {
    changelog.releases.into_iter().filter(|release| release.version <= current).collect()
}

fn row_key(version: Version) -> String {
    format!("changelog-{version}")
}

impl SettingsView {
    pub(super) fn open_changelog(&mut self, page: ChangelogPage, window: &mut Window, cx: &mut Context<Self>) {
        self.changelog = Some(page);
        self.scroll.set_offset(point(px(0.), px(0.)));
        self.focus_control("changelog-back", window, cx);
        cx.notify();
    }

    fn close_changelog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.changelog = None;
        self.scroll.set_offset(point(px(0.), px(0.)));
        self.focus_control("open-changelog", window, cx);
        cx.notify();
    }

    /// The open version, how many are shown, and how many there are.
    #[cfg(any(test, feature = "probes"))]
    pub fn changelog_state(&self) -> Option<(Option<Version>, usize, usize)> {
        let page = self.changelog.as_ref()?;
        let open = page.open.map(|index| page.releases[index].version);
        Some((open, page.shown, page.releases.len()))
    }

    /// For dev captures: the page on invented versions, as this build would
    /// show them were it `current`.
    #[cfg(feature = "probes")]
    pub fn show_changelog_of(&mut self, changelog: Changelog, current: Version, window: &mut Window, cx: &mut Context<Self>) {
        self.show_section(Section::About, cx);
        self.open_changelog(ChangelogPage::new(changelog, current), window, cx);
    }

    pub(super) fn changelog_page(&mut self, window: &Window, cx: &mut Context<Self>) -> Div {
        let Some((current, shown_versions)) = self
            .changelog
            .as_ref()
            .map(|page| (page.current, page.releases[..page.shown].iter().map(|release| release.version).collect::<Vec<_>>()))
        else {
            return div();
        };
        let back_focus = self.focus_of("changelog-back", true, window, cx);
        let row_focus: Vec<Focus> = shown_versions.iter().map(|version| self.focus_of(&row_key(*version), true, window, cx)).collect();
        let older_focus = self.focus_of("changelog-older", true, window, cx);
        let back = self.button_handler(cx, |this, window, cx| this.close_changelog(window, cx));
        let older = self.button_handler(cx, |this, window, cx| {
            let first_new = this.changelog.as_mut().and_then(ChangelogPage::show_older);
            if let Some(version) = first_new {
                this.focus_control(&row_key(version), window, cx);
            }
            cx.notify();
        });

        let Some(page) = &self.changelog else {
            return div();
        };
        let mut items: Vec<Div> = page.releases[..page.shown]
            .iter()
            .zip(row_focus)
            .enumerate()
            .map(|(index, (release, focus))| {
                let open = page.open == Some(index);
                let toggle = cx.listener(move |this: &mut Self, _: &gpui::ClickEvent, _, cx| {
                    if let Some(page) = &mut this.changelog {
                        page.toggle(index);
                        cx.notify();
                    }
                });
                div().flex().flex_col().child(version_row(release, open, &focus).on_click(toggle)).when(open, |item| item.child(notes(release)))
            })
            .collect();
        if page.releases.is_empty() {
            items.push(div().py(px(11.)).px(px(16.)).child(description(fl!("settings-changelog-unreadable"))));
        } else {
            let total = page.releases.len();
            let count = if page.shown < total {
                fl!("settings-changelog-shown", shown = page.shown, total = total)
            } else {
                fl!("settings-changelog-all", total = total)
            };
            items.push(
                div()
                    .flex()
                    .items_center()
                    .gap(px(10.))
                    .min_h(px(46.))
                    .py(px(8.))
                    .px(px(16.))
                    .when(page.shown < total, |more| {
                        more.child(button(
                            "changelog-older",
                            fl!("settings-changelog-show-older"),
                            Some(Icon::ChevronDown),
                            ButtonKind::Secondary,
                            true,
                            &older_focus,
                            Some(older),
                        ))
                    })
                    .child(div().debug_selector(|| "changelog-count".to_string()).text_size(px(12.)).text_color(rgba(FAINT)).child(count)),
            );
        }

        div()
            .flex()
            .flex_col()
            .child(div().flex().mt(px(-6.)).mb(px(10.)).ml(px(-10.)).child(button(
                "changelog-back",
                fl!("settings-section-about"),
                Some(Icon::ChevronLeft),
                ButtonKind::Ghost,
                true,
                &back_focus,
                Some(back),
            )))
            .child(page_head(
                fl!("settings-about-changelog"),
                Some(fl!("settings-changelog-lead", version = current.to_string()).into()),
            ))
            .child(group(items).mt(px(18.)))
    }
}

/// Collapsed: the version, the summary cut to one line, the date. Open: the
/// summary moves into the notes under the row.
fn version_row(release: &Release, open: bool, focus: &Focus) -> Stateful<Div> {
    let version = release.version.to_string();
    let date = changelog::date_words(release.date);
    let content = vec![
        div()
            .flex_none()
            .w(px(52.))
            .font_family(DISPLAY_FONT)
            .text_size(px(15.))
            .font_weight(FontWeight::SEMIBOLD)
            .child(version.clone())
            .into_any_element(),
        div()
            .flex_1()
            .min_w_0()
            .truncate()
            .text_size(px(12.))
            .text_color(rgba(MUTED))
            .when(!open, |summary| summary.child(changelog::styled(&release.summary, changelog::bold())))
            .into_any_element(),
        div().flex_none().text_size(px(12.)).text_color(rgba(FAINT)).child(date.clone()).into_any_element(),
    ];
    expandable_row(row_key(release.version), row_name(release, &version, &date), open, focus, content)
}

/// "1.4.0, 7 Oct 2026: A report at the end of every boss fight."
fn row_name(release: &Release, version: &str, date: &str) -> String {
    let summary: String = release.summary.iter().map(|span| span.text.as_str()).collect();
    fl!("settings-changelog-row", version = version, date = date, summary = summary)
}

/// The open row's whole summary and its bullets, under the version's text.
fn notes(release: &Release) -> Div {
    div()
        .debug_selector(|| "changelog-notes".to_string())
        .pl(px(80.))
        .pr(px(46.))
        .pb(px(14.))
        .flex()
        .flex_col()
        .child(div().max_w(px(470.)).text_size(px(13.)).line_height(relative(1.45)).child(changelog::styled(&release.summary, changelog::bold())))
        .child(div().mt(px(6.)).max_w(px(490.)).flex().flex_col().gap(px(4.)).children(release.bullets.iter().map(|bullet| {
            div()
                .relative()
                .pl(px(13.))
                .text_size(px(12.5))
                .line_height(relative(1.45))
                .text_color(rgba(MUTED))
                .child(div().absolute().top(px(7.)).left(px(2.)).size(px(4.)).rounded_full().bg(rgba(FAINT)))
                .child(changelog::styled(bullet, changelog::bold_in_text_colour()))
        })))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::changelog::{Date, Span};
    use crate::i18n::{set_language, Language};

    #[test]
    fn a_row_is_named_by_its_version_date_and_whole_summary() {
        let release = Release {
            version: Version::new(1, 4, 0),
            date: Date { year: 2026, month: 10, day: 7 },
            summary: vec![Span { text: "A report ".into(), bold: false }, Span { text: "at the end".into(), bold: true }, Span { text: ".".into(), bold: false }],
            bullets: Vec::new(),
        };
        set_language(Language::English);
        assert_eq!(row_name(&release, "1.4.0", "7 Oct 2026"), "1.4.0, 7 Oct 2026: A report at the end.");
        set_language(Language::French);
        assert_eq!(row_name(&release, "1.4.0", "7 oct. 2026"), "1.4.0, 7 oct. 2026 : A report at the end.");
    }
}
