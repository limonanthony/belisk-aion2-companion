//! The What's new window as approved in docs/gpui-mockups/approved/whatsnew.html
//! (values in tokens.md, "Updates and the changelog"): History's frame, the
//! versions the player has not run yet, newest first and all open, and a
//! footer that leads to the full changelog. It sits under the toolbar like the
//! reminder card while the meter is closed, never takes the keyboard from the
//! game, and stays until it is closed; closing it, with × or "Full
//! changelog", counts the versions as seen.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    div, prelude::*, px, rgba, size, Context, EventEmitter, FocusHandle, FontWeight, HighlightStyle, KeyDownEvent,
    MouseButton, Pixels, Render, Role, ScrollHandle, SharedString, Size, StyledText, Task, Window,
};

use crate::assets::Icon;
use crate::changelog::{self, Release, WhatsNew};
use crate::i18n::fl;
use crate::overlay::WindowDrag;
use crate::reminders::GAP;
use crate::secondary::chrome::{self, Look, Scale, Tone, FAINT, HOT, MUTED, TEXT};
use crate::secondary::Stacking;
use crate::settings;

const WIDTH: f32 = 370.0;
/// The list scrolls past it; the window hugs a shorter one.
const LIST_MAX: f32 = 318.0;
/// The window before its first frame measures the card, at 100%.
pub const EXPECTED_SIZE: (f32, f32) = (WIDTH, 220.0);
/// The card follows the toolbar as it moves.
const TICK: Duration = Duration::from_millis(100);
const FOCUS_RING: u32 = 0xf3ebe1e6;

/// "Full changelog" was chosen: Settings › About › Changelog is to open.
pub struct OpenChangelog;

pub struct WhatsNewView {
    whats_new: WhatsNew,
    /// None in tests, where there is no toolbar to sit under.
    stacking: Option<Rc<Stacking>>,
    card_size: Rc<Cell<Option<Size<Pixels>>>>,
    scroll: ScrollHandle,
    /// The list ran past its height at the last layout, so the thumb's gutter
    /// is kept.
    overflows: Rc<Cell<bool>>,
    drag: WindowDrag,
    root_focus: FocusHandle,
    full_changelog_focus: FocusHandle,
    _ticker: Option<Task<()>>,
}

impl EventEmitter<OpenChangelog> for WhatsNewView {}

impl WhatsNewView {
    pub fn new(whats_new: WhatsNew, stacking: Rc<Stacking>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let ticker = cx.spawn_in(window, async move |this, cx| loop {
            cx.background_executor().timer(TICK).await;
            if this.update_in(cx, |this, window, cx| this.keep_in_place(window, cx)).is_err() {
                break;
            }
        });
        Self::built(whats_new, Some(stacking), Some(ticker), window, cx)
    }

    #[cfg(test)]
    pub fn unplaced(whats_new: WhatsNew, window: &mut Window, cx: &mut Context<Self>) -> Self {
        Self::built(whats_new, None, None, window, cx)
    }

    fn built(
        whats_new: WhatsNew,
        stacking: Option<Rc<Stacking>>,
        ticker: Option<Task<()>>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let store = settings::global(cx);
        cx.observe(&store, |_, _, cx| cx.notify()).detach();
        let root_focus = cx.focus_handle();
        window.focus(&root_focus, cx);
        Self {
            whats_new,
            stacking,
            card_size: Rc::default(),
            scroll: ScrollHandle::new(),
            overflows: Rc::default(),
            drag: WindowDrag::default(),
            root_focus,
            full_changelog_focus: cx.focus_handle().tab_stop(true),
            _ticker: ticker,
        }
    }

    fn gap(cx: &Context<Self>) -> f32 {
        GAP * settings::global(cx).read(cx).get().overlay.scale()
    }

    fn keep_in_place(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(stacking), Some(card)) = (&self.stacking, self.card_size.get()) else {
            return;
        };
        stacking.place(card, Self::gap(cx), window, cx);
    }

    /// The versions shown count as seen, and the window goes.
    fn close(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        settings::mark_version_seen(cx);
        window.remove_window();
    }

    fn open_full_changelog(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        cx.emit(OpenChangelog);
        // Once the event has reached its listeners, which the window's end
        // would drop with the view.
        cx.defer_in(window, |this, window, cx| this.close(window, cx));
    }

    /// Tab reaches "Full changelog"; the window never takes the keyboard from
    /// the game, so this serves accessibility tools.
    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let modifiers = event.keystroke.modifiers;
        if event.keystroke.key != "tab" || modifiers.control || modifiers.alt || modifiers.platform {
            return;
        }
        cx.stop_propagation();
        if modifiers.shift {
            window.focus_prev(cx);
        } else {
            window.focus_next(cx);
        }
        cx.notify();
    }

    /// For dev captures: the list scrolled to its end, as the wheel would.
    #[cfg(feature = "probes")]
    pub fn scroll_to_end(&mut self, cx: &mut Context<Self>) {
        self.scroll.scroll_to_bottom();
        cx.notify();
    }

    fn since_line(&self, s: Scale) -> Option<impl IntoElement> {
        let since = self.whats_new.since.filter(|_| self.whats_new.releases.len() > 1)?;
        let version = since.to_string();
        let text = fl!("whatsnew-since", version = version.as_str());
        let start = text.find(&version)?;
        let end = start + version.len();
        // The line is drawn in `muted` and its words fade to `faint`: GPUI
        // keeps the paragraph's alpha under a run's colour, so the version
        // could not be brighter than a `faint` paragraph.
        let faint_alpha = (FAINT & 0xff) as f32 / (MUTED & 0xff) as f32;
        let words = HighlightStyle { fade_out: Some(1. - faint_alpha), ..Default::default() };
        let highlights = [(0..start, words), (start..end, HighlightStyle { font_weight: Some(FontWeight::SEMIBOLD), ..Default::default() }), (end..text.len(), words)]
            .into_iter()
            .filter(|(range, _)| !range.is_empty());
        let highlights: Vec<_> = highlights.collect();
        Some(
            div()
                .debug_selector(|| "whats-new-since".to_string())
                .flex_none()
                .h(s.px(18.))
                .px(s.px(4.))
                .flex()
                .items_center()
                .text_size(s.px(11.5))
                .text_color(rgba(MUTED))
                .child(StyledText::new(text).with_highlights(highlights)),
        )
    }

    fn full_changelog_button(&self, s: Scale, window: &Window, cx: &Context<Self>) -> impl IntoElement {
        let ring = self.full_changelog_focus.is_focused(window) && window.last_input_was_keyboard();
        let label = fl!("whatsnew-full-changelog");
        let group = "full-changelog";
        let glyph = |icon| chrome::icon(icon, 12., rgba(MUTED), s).group_hover(group, |svg| svg.text_color(rgba(TEXT)));
        div()
            .id("full-changelog")
            .debug_selector(|| "full-changelog".to_string())
            .role(Role::Button)
            .aria_label(SharedString::from(label.clone()))
            .track_focus(&self.full_changelog_focus)
            .group(group)
            .relative()
            .flex_none()
            .h(s.px(22.))
            .px(s.px(10.))
            .flex()
            .items_center()
            .gap(s.px(5.))
            .rounded_full()
            .text_size(s.px(12.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgba(MUTED))
            .cursor_pointer()
            .hover(|button| button.bg(rgba(HOT)).text_color(rgba(TEXT)))
            .on_mouse_down(MouseButton::Left, |_, window, _| window.prevent_default())
            .child(glyph(Icon::Settings))
            .child(label)
            .child(glyph(Icon::ChevronRight))
            .when(ring, |button| {
                button.child(
                    div()
                        .debug_selector(|| "focus-ring".to_string())
                        .absolute()
                        .top(px(-3.))
                        .left(px(-3.))
                        .right(px(-3.))
                        .bottom(px(-3.))
                        .rounded_full()
                        .border_2()
                        .border_color(rgba(FOCUS_RING)),
                )
            })
            .on_click(cx.listener(|this, _, window, cx| this.open_full_changelog(window, cx)))
    }
}

fn version_block(release: &Release, first: bool, s: Scale) -> impl IntoElement {
    let version = release.version.to_string();
    let selector = format!("whats-new-{version}");
    div()
        .debug_selector(move || selector.clone())
        .flex_none()
        .flex()
        .flex_col()
        .when(!first, |block| block.mt(s.px(14.)))
        .child(
            div()
                .h(s.px(20.))
                .flex()
                .items_baseline()
                .gap(s.px(8.))
                .child(div().flex_1().text_size(s.px(13.5)).font_weight(FontWeight::SEMIBOLD).child(version))
                .child(
                    div()
                        .flex_none()
                        .text_size(s.px(11.5))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgba(FAINT))
                        .child(changelog::date_words(release.date)),
                ),
        )
        .child(
            div()
                .mt(s.px(1.))
                .text_size(s.px(12.5))
                .font_weight(FontWeight::MEDIUM)
                .line_height(s.px(16.))
                .child(changelog::styled(&release.summary, changelog::bold())),
        )
        .child(div().mt(s.px(5.)).flex().flex_col().gap(s.px(4.)).children(release.bullets.iter().map(|bullet| {
            div()
                .relative()
                .pl(s.px(11.))
                .text_size(s.px(12.))
                .font_weight(FontWeight::MEDIUM)
                .line_height(s.px(15.5))
                .text_color(rgba(MUTED))
                .child(div().absolute().top(s.px(6.)).left(s.px(2.)).size(s.px(4.)).rounded_full().bg(rgba(FAINT)))
                .child(changelog::styled(bullet, changelog::bold_in_text_colour()))
        })))
}

impl Render for WhatsNewView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let overlay = settings::global(cx).read(cx).get().overlay.clone();
        let look = Look::of(&overlay);
        let s = look.s;
        let title = fl!("whatsnew-title");

        let close = chrome::tool_button("close", Icon::Close, fl!("common-close"), false, s)
            .debug_selector(|| "whats-new-close".to_string())
            .on_click(cx.listener(|this, _, window, cx| this.close(window, cx)));
        let heading = chrome::title(Icon::ScrollText, title.clone(), None, s).child(chrome::chip(
            Some(Icon::Check),
            fl!("whatsnew-updated", version = self.whats_new.current.to_string()),
            Tone::Good,
            s,
        ));
        let header = chrome::header_closed_by(heading, Vec::new(), close, &self.drag, s);
        let releases = div()
            .w_full()
            .px(s.px(4.))
            .pb(s.px(2.))
            .flex()
            .flex_col()
            .children(self.whats_new.releases.iter().enumerate().map(|(index, release)| version_block(release, index == 0, s)));
        let list = chrome::scroll_list("whats-new-list", &self.scroll, LIST_MAX, self.overflows.get(), releases, s).mt(s.px(4.));
        let footer = chrome::footer(s).child(div().flex_1()).child(self.full_changelog_button(s, window, cx));
        let card = chrome::card(WIDTH, look)
            .id("whats-new")
            .debug_selector(|| "whats-new".to_string())
            .role(Role::Dialog)
            .aria_label(SharedString::from(title))
            .child(header)
            .children(self.since_line(s))
            .child(list)
            .child(footer);

        let card_size = self.card_size.clone();
        let stacking = self.stacking.clone();
        let gap = Self::gap(cx);
        let scroll = self.scroll.clone();
        let overflows = self.overflows.clone();
        chrome::surface(look)
            .track_focus(&self.root_focus)
            .on_key_down(cx.listener(Self::on_key))
            .on_children_prepainted(move |bounds, window, cx| {
                let now_overflows = scroll.max_offset().y > Pixels::ZERO;
                if overflows.replace(now_overflows) != now_overflows {
                    window.on_next_frame(|window, _| window.refresh());
                }
                let Some(card) = bounds.first() else {
                    return;
                };
                let measured = size(card.origin.x + card.size.width, card.origin.y + card.size.height);
                card_size.set(Some(measured));
                if let Some(stacking) = &stacking {
                    stacking.place(measured, gap, window, cx);
                }
            })
            .child(card)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use gpui::{AppContext as _, Entity, KeyUpEvent, Keystroke, Modifiers, TestAppContext, VisualTestContext};

    use super::*;
    use crate::changelog::{Date, Span, Version};
    use crate::settings::{install, AppSettings, Settings};

    fn release(version: Version, bullets: usize) -> Release {
        Release {
            version,
            date: Date { year: 2026, month: 10, day: 21 },
            summary: vec![Span { text: "What changed.".into(), bold: false }],
            bullets: (0..bullets)
                .map(|n| vec![Span { text: "Bold".into(), bold: true }, Span { text: format!(": bullet {n}.").into(), bold: false }])
                .collect(),
        }
    }

    fn from(since: Option<Version>, versions: &[Version]) -> WhatsNew {
        WhatsNew { since, current: versions[0], releases: versions.iter().map(|version| release(*version, 2)).collect() }
    }

    struct Opened<'a> {
        settings: Entity<Settings>,
        asked: Rc<RefCell<usize>>,
        cx: &'a mut VisualTestContext,
    }

    impl Opened<'_> {
        fn press(&mut self, key: &str) {
            let keystroke = Keystroke::parse(key).expect("a key name");
            self.cx.simulate_event(KeyDownEvent { keystroke: keystroke.clone(), is_held: false, prefer_character_input: false });
            self.cx.simulate_event(KeyUpEvent { keystroke });
        }

        fn is_open(&self) -> bool {
            !self.cx.windows().is_empty()
        }

        /// Read through the app: the window may be gone.
        fn seen(&mut self) -> Option<Version> {
            let settings = self.settings.clone();
            let app: &mut TestAppContext = &mut **self.cx;
            app.update(|cx| settings.read(cx).get().last_seen_version)
        }
    }

    fn open(cx: &mut TestAppContext, whats_new: WhatsNew) -> Opened<'_> {
        let mut stored = AppSettings::default();
        stored.last_seen_version = Some(Version::new(0, 9, 0));
        let settings = cx.update(|cx| {
            let store = cx.new(|_| Settings::in_memory(stored));
            install(&store, cx);
            store
        });
        let (view, cx) = cx.add_window_view(|window, cx| WhatsNewView::unplaced(whats_new, window, cx));
        let asked = Rc::new(RefCell::new(0));
        let count = asked.clone();
        cx.update(|_, cx| cx.subscribe(&view, move |_, _: &OpenChangelog, _| *count.borrow_mut() += 1).detach());
        Opened { settings, asked, cx }
    }

    const V140: Version = Version::new(1, 4, 0);
    const V150: Version = Version::new(1, 5, 0);
    const V151: Version = Version::new(1, 5, 1);
    const V160: Version = Version::new(1, 6, 0);

    #[gpui::test]
    fn one_version_shows_without_the_since_line(cx: &mut TestAppContext) {
        let window = open(cx, from(Some(V151), &[V160]));
        assert!(window.cx.debug_bounds("whats-new-1.6.0").is_some());
        assert!(window.cx.debug_bounds("whats-new-since").is_none());
    }

    #[gpui::test]
    fn skipped_versions_are_all_listed_under_the_since_line(cx: &mut TestAppContext) {
        let window = open(cx, from(Some(V140), &[V160, V151, V150]));
        let since = window.cx.debug_bounds("whats-new-since").expect("the since line");
        let newest = window.cx.debug_bounds("whats-new-1.6.0").expect("1.6.0");
        let oldest = window.cx.debug_bounds("whats-new-1.5.0").expect("1.5.0");
        assert!(since.bottom() <= newest.top() && newest.bottom() < oldest.top(), "newest first, under the line");
    }

    #[gpui::test]
    fn a_player_from_before_the_reset_gets_no_since_line(cx: &mut TestAppContext) {
        let window = open(cx, from(None, &[V160, V151]));
        assert!(window.cx.debug_bounds("whats-new-since").is_none());
    }

    #[gpui::test]
    fn closing_marks_the_versions_seen(cx: &mut TestAppContext) {
        let mut window = open(cx, from(Some(V151), &[V160]));
        let close = window.cx.debug_bounds("whats-new-close").expect("the close button").center();
        window.cx.simulate_click(close, Modifiers::none());
        window.cx.run_until_parked();

        assert!(!window.is_open());
        assert_eq!(window.seen(), Some(Version::current()));
        assert_eq!(*window.asked.borrow(), 0, "× only closes");
    }

    #[gpui::test]
    fn full_changelog_asks_for_the_changelog_and_closes(cx: &mut TestAppContext) {
        let mut window = open(cx, from(Some(V151), &[V160]));
        let full = window.cx.debug_bounds("full-changelog").expect("the footer button").center();
        window.cx.simulate_click(full, Modifiers::none());
        window.cx.run_until_parked();

        assert_eq!(*window.asked.borrow(), 1);
        assert!(!window.is_open());
        assert_eq!(window.seen(), Some(Version::current()));
    }

    #[gpui::test]
    fn the_keyboard_reaches_full_changelog_only_through_tab(cx: &mut TestAppContext) {
        let mut window = open(cx, from(Some(V151), &[V160]));
        assert!(window.cx.debug_bounds("focus-ring").is_none(), "nothing is focused when it opens");

        window.press("enter");
        assert_eq!(*window.asked.borrow(), 0, "Enter on the window does nothing");
        assert!(window.is_open());

        window.press("tab");
        assert!(window.cx.debug_bounds("focus-ring").is_some(), "the ring on Full changelog");
        window.press("enter");
        window.cx.run_until_parked();
        assert_eq!(*window.asked.borrow(), 1);
        assert!(!window.is_open());
    }
}
