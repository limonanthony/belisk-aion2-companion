//! The approved overlay language the history, player detail, PvP and reminder
//! windows share (docs/gpui-mockups/approved/overlay.css, values in
//! tokens.md): the card the window is exactly as large as, a header that drags
//! the window, tool and text buttons, tabs, inputs, popovers, empty states and
//! the thin scroll thumb. The user's content scale and card opacity are the
//! meter's.

use std::cell::{Cell, RefCell};
use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    canvas, div, fill, img, point, prelude::*, relative, rgb, rgba, size, svg, AnyElement, AnyView, App, Bounds,
    Context, DispatchPhase, Div, ElementId, FocusHandle, FontFeatures, FontWeight, HitboxBehavior, Hsla, IntoElement,
    List, ListAlignment, ListOffset, ListState, MouseButton, MouseDownEvent, MouseMoveEvent, MouseUpEvent, Pixels,
    Render, Role, ScrollHandle, SharedString, Stateful, Svg, Window,
};

use crate::assets::Icon;
use crate::i18n::fl;
use crate::fonts::OVERLAY_FAMILY;
use crate::game_data::PlayerClass;
use crate::meter::OverlaySettings;
use crate::overlay;
pub use crate::style::Scale;

pub const TEXT: u32 = 0xfff6ecf5;
pub const MUTED: u32 = 0xfff6eca8;
pub const FAINT: u32 = 0xfff6ec70;
pub const CREAM: u32 = 0xfff6ece6;
pub const SOFT: u32 = 0xfff0e114;
pub const HOT: u32 = 0xfff0e121;
pub const GOOD: u32 = 0x86e3a8;
pub const WARN: u32 = 0xf2c66d;
pub const BAD: u32 = 0xf7a8a8;
pub const INK: u32 = 0x17120d;
pub const GAUGE_EMPTY: u32 = 0xfff0e11f;
const CARD_RGB: u32 = 0x100e0c;
const CARD_BORDER: u32 = 0xffecd61a;
const RANK_BADGE: u32 = 0xfff0e117;
const YOU_RIM: u32 = 0xfff6ec73;
const POPOVER: u32 = 0x1e1a16fa;
const POPOVER_BORDER: u32 = 0xffecd624;
const INPUT_FILL: u32 = 0x00000057;
const INPUT_BORDER: u32 = 0xfff0e11f;
const INPUT_FOCUS: u32 = 0xfff6ec8c;
const TABS_TRACK: u32 = 0x0000004d;
const THUMB: u32 = 0xfff0e147;
const CHIP_GOOD: u32 = 0x86e3a824;
const CHIP_WARN: u32 = 0xf2c66d21;
const NOTE_GOOD: u32 = 0x86e3a81f;
const DANGER_FILL: u32 = 0xf7a8a824;

/// How the user wants the overlays drawn: the meter's content scale and card
/// opacity.
#[derive(Clone, Copy)]
pub struct Look {
    pub s: Scale,
    pub alpha: u8,
}

impl Look {
    pub fn of(settings: &OverlaySettings) -> Self {
        Self { s: Scale(settings.scale()), alpha: settings.background_alpha }
    }
}

/// The window's root: the card at the top left, then anything drawn over it
/// (a popover and the scrim that dismisses it). The window is fitted to all
/// of them, so it grows while a popover hangs below the card.
pub fn root(look: Look) -> Div {
    surface(look).on_children_prepainted(|bounds, window, cx| overlay::fit_to_children(&bounds, window, cx))
}

/// [`root`] for a window that sizes and places itself.
pub fn surface(look: Look) -> Div {
    div()
        .size_full()
        .relative()
        .flex()
        .flex_col()
        .items_start()
        .font_family(OVERLAY_FAMILY)
        .font_features(FontFeatures(Arc::new(vec![("tnum".into(), 1)])))
        .line_height(relative(1.))
        .text_size(look.s.px(13.))
        .text_color(rgba(TEXT))
}

pub fn card(width: f32, look: Look) -> Div {
    let s = look.s;
    frame(look, 14.)
        .flex_none()
        .w(s.px(width))
        .flex()
        .flex_col()
        .pt(s.px(6.))
        .px(s.px(8.))
        .pb(s.px(8.))
}

/// The card's fill at the user's opacity and its border, at `radius`.
pub fn frame(look: Look, radius: f32) -> Div {
    div()
        .rounded(look.s.px(radius))
        .border_1()
        .border_color(rgba(CARD_BORDER))
        .bg(rgba((CARD_RGB << 8) | u32::from(look.alpha)))
}

/// The header row: `title` drags the window; `tools` and the close button sit
/// at the top right.
pub fn header(title: impl IntoElement, tools: Vec<AnyElement>, drag: &overlay::WindowDrag, s: Scale) -> Div {
    header_closed_by(title, tools, close_button(s), drag, s)
}

/// [`header`] with a close button of the window's own.
pub fn header_closed_by(
    title: impl IntoElement,
    tools: Vec<AnyElement>,
    close: impl IntoElement,
    drag: &overlay::WindowDrag,
    s: Scale,
) -> Div {
    let handle = div().flex_1().min_w_0().h_full().flex().items_center().gap(s.px(8.));
    div()
        .flex_none()
        .h(s.px(28.))
        .pl(s.px(4.))
        .flex()
        .items_center()
        .gap(s.px(8.))
        .child(drag.handle(handle).child(title))
        .children(tools)
        .child(close)
        .child(drag.listener())
}

/// Icon and name of a window, with an optional count after it.
pub fn title(glyph: Icon, text: impl Into<SharedString>, count: Option<SharedString>, s: Scale) -> Div {
    div()
        .min_w_0()
        .flex()
        .items_center()
        .gap(s.px(8.))
        .child(
            div()
                .min_w_0()
                .flex()
                .items_center()
                .gap(s.px(7.))
                .text_size(s.px(13.5))
                .font_weight(FontWeight::SEMIBOLD)
                .child(icon(glyph, 13., rgba(MUTED), s))
                .child(div().min_w_0().truncate().child(text.into())),
        )
        .children(count.map(|count| self::count(count, s)))
}

pub fn count(text: impl Into<SharedString>, s: Scale) -> Div {
    div()
        .flex_none()
        .text_size(s.px(11.5))
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgba(FAINT))
        .child(text.into())
}

pub fn close_button(s: Scale) -> Stateful<Div> {
    tool_button("close", Icon::Close, fl!("common-close"), false, s).on_click(|_, window, _| window.remove_window())
}

pub fn icon(glyph: Icon, size: f32, colour: impl Into<Hsla>, s: Scale) -> Svg {
    svg().path(glyph.path()).flex_none().size(s.px(size)).text_color(colour.into())
}

/// 22 px square button with a 13 px icon; `hot` while what it opens is shown.
pub fn tool_button(id: impl Into<ElementId>, glyph: Icon, label: impl Into<SharedString>, hot: bool, s: Scale) -> Stateful<Div> {
    square_button(id, glyph, label.into(), hot, 22., 13., s)
}

/// The 20 px variant inside rows, with a 12 px icon.
pub fn row_button(id: impl Into<ElementId>, glyph: Icon, label: impl Into<SharedString>, s: Scale) -> Stateful<Div> {
    square_button(id, glyph, label.into(), false, 20., 12., s)
}

fn square_button(id: impl Into<ElementId>, glyph: Icon, label: SharedString, hot: bool, side: f32, glyph_size: f32, s: Scale) -> Stateful<Div> {
    let id = id.into();
    let group = SharedString::from(format!("button-{id}"));
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .group(group.clone())
        .flex_none()
        .size(s.px(side))
        .flex()
        .items_center()
        .justify_center()
        .rounded(s.px(7.))
        .cursor_pointer()
        .when(hot, |button| button.bg(rgba(HOT)))
        .hover(|button| button.bg(rgba(HOT)))
        .child(
            icon(glyph, glyph_size, rgba(if hot { TEXT } else { MUTED }), s)
                .group_hover(group, |svg| svg.text_color(rgba(TEXT))),
        )
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Quiet,
    Danger,
    /// The quiet look at 40%: nothing to act on.
    Disabled,
}

/// A pill with a label and an optional 12 px icon, `height` 20 or 22 px.
pub fn text_button(
    id: impl Into<ElementId>,
    glyph: Option<Icon>,
    label: impl Into<SharedString>,
    kind: ButtonKind,
    height: f32,
    s: Scale,
) -> Stateful<Div> {
    let (fill, colour, hover) = match kind {
        ButtonKind::Quiet => (None, rgba(MUTED), Some(rgba(SOFT))),
        ButtonKind::Danger => (Some(rgba(DANGER_FILL)), rgb(BAD), Some(rgba(0xf7a8a83d))),
        ButtonKind::Disabled => (None, rgba(MUTED), None),
    };
    let colour: Hsla = colour.into();
    div()
        .id(id)
        .flex_none()
        .h(s.px(height))
        .px(s.px(if height < 22. { 8. } else { 10. }))
        .flex()
        .items_center()
        .gap(s.px(5.))
        .rounded_full()
        .when_some(fill, |button, fill| button.bg(fill))
        .text_color(colour)
        .text_size(s.px(if height < 22. { 11.5 } else { 12. }))
        .font_weight(FontWeight::SEMIBOLD)
        .when_some(hover, |button, hover| button.cursor_pointer().hover(move |button| button.bg(hover)))
        .when(hover.is_none(), |button| button.opacity(0.4))
        .children(glyph.map(|glyph| icon(glyph, 12., colour, s)))
        .child(label.into())
}

#[derive(Clone, Copy)]
pub enum Tone {
    Good,
    Warn,
    Bad,
}

pub fn chip(glyph: Option<Icon>, label: impl Into<SharedString>, tone: Tone, s: Scale) -> Div {
    let (fill, colour) = match tone {
        Tone::Good => (CHIP_GOOD, GOOD),
        Tone::Warn => (CHIP_WARN, WARN),
        Tone::Bad => (DANGER_FILL, BAD),
    };
    div()
        .flex_none()
        .h(s.px(20.))
        .px(s.px(8.))
        .flex()
        .items_center()
        .gap(s.px(5.))
        .rounded_full()
        .bg(rgba(fill))
        .text_color(rgb(colour))
        .text_size(s.px(11.5))
        .font_weight(FontWeight::SEMIBOLD)
        .children(glyph.map(|glyph| icon(glyph, 12., rgb(colour), s)))
        .child(label.into())
}

pub fn chip_note(text: impl Into<SharedString>, s: Scale) -> Div {
    div().text_size(s.px(11.)).font_weight(FontWeight::MEDIUM).text_color(rgba(MUTED)).child(text.into())
}

/// A section label row: 24 px, 6 px above.
pub fn section(s: Scale) -> Div {
    div()
        .flex_none()
        .h(s.px(24.))
        .mt(s.px(6.))
        .px(s.px(4.))
        .flex()
        .items_center()
        .gap(s.px(8.))
        .text_color(rgba(MUTED))
        .text_size(s.px(12.))
        .font_weight(FontWeight::SEMIBOLD)
}

/// The segmented control's track; children are [`tab`]s.
pub fn tabs(s: Scale) -> Div {
    div()
        .flex_none()
        .flex()
        .gap(s.px(2.))
        .mt(s.px(2.))
        .mb(s.px(6.))
        .p(s.px(2.))
        .rounded(s.px(8.))
        .bg(rgba(TABS_TRACK))
}

pub fn tab(id: impl Into<ElementId>, label: impl Into<SharedString>, count: Option<usize>, on: bool, s: Scale) -> Stateful<Div> {
    div()
        .id(id)
        .flex_1()
        .h(s.px(22.))
        .flex()
        .items_center()
        .justify_center()
        .gap(s.px(5.))
        .rounded(s.px(6.))
        .text_size(s.px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgba(if on { TEXT } else { MUTED }))
        .when(on, |tab| tab.bg(rgba(HOT)))
        .when(!on, |tab| tab.cursor_pointer().hover(|tab| tab.bg(rgba(SOFT)).text_color(rgba(TEXT))))
        .child(label.into())
        .children(count.map(|count| {
            div().text_size(s.px(11.)).font_weight(FontWeight::MEDIUM).text_color(rgba(FAINT)).child(count.to_string())
        }))
}

/// A table header row; columns are laid out by the caller.
pub fn table_head(s: Scale) -> Div {
    div()
        .flex_none()
        .h(s.px(20.))
        .flex()
        .items_center()
        .gap(s.px(6.))
        .text_size(s.px(11.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgba(FAINT))
}

/// A row of a list: optional rank badge, the bar, then whatever follows it.
pub fn row(s: Scale) -> Div {
    div().flex_none().h(s.px(22.)).flex().items_center().gap(s.px(5.))
}

pub fn rank_badge(rank: usize, you: bool, s: Scale) -> Div {
    div()
        .flex_none()
        .size(s.px(18.))
        .mr(s.px(3.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(s.px(4.))
        .bg(rgba(if you { CREAM } else { RANK_BADGE }))
        .text_color(if you { rgb(INK) } else { rgba(MUTED) })
        .text_size(s.px(11.5))
        .font_weight(FontWeight::SEMIBOLD)
        .child(rank.to_string())
}

/// What sits on a bar: 4 px in from the left (8 px without an icon), 9 px
/// from the right, 6 px between items.
pub fn bar_content(plain: bool, s: Scale) -> Div {
    div()
        .size_full()
        .flex()
        .items_center()
        .gap(s.px(6.))
        .pl(s.px(if plain { 8. } else { 4. }))
        .pr(s.px(9.))
}

/// The 1 px cream rim inside your bar.
pub fn you_rim(s: Scale) -> Div {
    div().absolute().inset_0().rounded(s.px(5.)).border_1().border_color(rgba(YOU_RIM))
}

pub fn class_icon(class: PlayerClass, size: f32, s: Scale) -> impl IntoElement {
    img(class.icon()).flex_none().size(s.px(size)).rounded(s.px(size / 2.))
}

/// A name that cuts off with an ellipsis, with an optional quieter note after
/// it (server, "Not in range").
pub fn name(text: impl Into<SharedString>, note: Option<SharedString>, weight: FontWeight, s: Scale) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .items_baseline()
        .gap(s.px(5.))
        .child(div().min_w_0().truncate().font_weight(weight).child(text.into()))
        .children(note.map(|note| {
            div().flex_none().text_size(s.px(11.)).text_color(rgba(MUTED)).child(note)
        }))
}

/// A right-aligned figure column.
pub fn figure(text: impl Into<SharedString>, width: Option<f32>, s: Scale) -> Div {
    div()
        .flex_none()
        .when_some(width, |cell, width| cell.w(s.px(width)))
        .flex()
        .justify_end()
        .child(text.into())
}

pub fn strong(text: impl Into<SharedString>, s: Scale) -> Div {
    div().flex_none().text_size(s.px(13.5)).font_weight(FontWeight::SEMIBOLD).child(text.into())
}

/// The 38 px percent column beside a bar.
pub fn percent(text: impl Into<SharedString>, s: Scale) -> Div {
    div()
        .flex_none()
        .w(s.px(38.))
        .flex()
        .justify_end()
        .text_size(s.px(12.))
        .text_color(rgba(MUTED))
        .child(text.into())
}

/// A text field's or select's box, 26 px tall.
pub fn input_box(focused: bool, s: Scale) -> Div {
    div()
        .flex_none()
        .h(s.px(26.))
        .pl(s.px(9.))
        .pr(s.px(4.))
        .flex()
        .items_center()
        .gap(s.px(6.))
        .rounded(s.px(7.))
        .border_1()
        .border_color(rgba(if focused { INPUT_FOCUS } else { INPUT_BORDER }))
        .bg(rgba(INPUT_FILL))
        .text_size(s.px(12.5))
}

/// A dropdown: its label above, the chosen value and a chevron in the box.
pub fn select(id: impl Into<ElementId>, label: impl Into<SharedString>, value: SharedString, open: bool, s: Scale) -> Stateful<Div> {
    div()
        .id(id)
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(s.px(3.))
        .child(div().pl(s.px(2.)).text_size(s.px(11.)).text_color(rgba(FAINT)).child(label.into()))
        .child(
            input_box(open, s)
                .pr(s.px(6.))
                .child(div().flex_1().min_w_0().truncate().font_weight(FontWeight::MEDIUM).child(value))
                .child(icon(if open { Icon::ChevronUp } else { Icon::ChevronDown }, 13., rgba(MUTED), s)),
        )
}

/// A menu, dropdown or suggestion list, placed by the caller; it swallows
/// the presses that land on it.
pub fn popover(s: Scale) -> Div {
    div()
        .absolute()
        .occlude()
        .p(s.px(4.))
        .flex()
        .flex_col()
        .rounded(s.px(9.))
        .border_1()
        .border_color(rgba(POPOVER_BORDER))
        .bg(rgba(POPOVER))
        .text_size(s.px(12.5))
}

pub fn menu_item(id: impl Into<ElementId>, hot: bool, s: Scale) -> Stateful<Div> {
    div()
        .id(id)
        .flex_none()
        .h(s.px(26.))
        .px(s.px(8.))
        .flex()
        .items_center()
        .gap(s.px(8.))
        .rounded(s.px(6.))
        .cursor_pointer()
        .when(hot, |item| item.bg(rgba(HOT)))
        .hover(|item| item.bg(rgba(HOT)))
}

/// A 14 px title and one sentence saying what will fill the space.
pub fn empty(title: Option<SharedString>, line: impl Into<SharedString>, s: Scale) -> Div {
    div()
        .flex_none()
        .pt(s.px(14.))
        .px(s.px(6.))
        .pb(s.px(12.))
        .flex()
        .flex_col()
        .gap(s.px(3.))
        .whitespace_normal()
        .line_height(relative(1.4))
        .text_size(s.px(12.5))
        .text_color(rgba(MUTED))
        .children(title.map(|title| {
            div().text_size(s.px(14.)).font_weight(FontWeight::SEMIBOLD).text_color(rgba(TEXT)).child(title)
        }))
        .child(line.into())
}

/// A note in the window; `good` for a success.
pub fn note(glyph: Icon, text: impl Into<SharedString>, good: bool, s: Scale) -> Div {
    let colour: Hsla = if good { rgb(GOOD).into() } else { rgba(MUTED).into() };
    div()
        .flex_none()
        .flex()
        .items_start()
        .gap(s.px(7.))
        .py(s.px(7.))
        .px(s.px(9.))
        .rounded(s.px(8.))
        .bg(rgba(if good { NOTE_GOOD } else { SOFT }))
        .text_color(colour)
        .text_size(s.px(11.5))
        .line_height(relative(1.35))
        .whitespace_normal()
        .child(icon(glyph, 13., colour, s).mt(s.px(1.)))
        .child(div().flex_1().min_w_0().child(text.into()))
}

/// The window footer row: 26 px, 6 px above.
pub fn footer(s: Scale) -> Div {
    div()
        .flex_none()
        .h(s.px(26.))
        .mt(s.px(6.))
        .pl(s.px(4.))
        .pr(s.px(2.))
        .flex()
        .items_center()
        .gap(s.px(8.))
        .text_size(s.px(11.5))
        .text_color(rgba(MUTED))
}

/// The thumb's gutter on the right of a list that scrolls.
pub const GUTTER: f32 = 8.;

/// A list that scrolls past `max_height`, with the thin thumb GPUI does not
/// draw by itself. `overflows` reserves the thumb's gutter on the right; the
/// callers know their row heights, so it is decided before layout.
pub fn scroll_list(
    id: impl Into<ElementId>,
    handle: &ScrollHandle,
    max_height: f32,
    overflows: bool,
    content: impl IntoElement,
    s: Scale,
) -> Div {
    let id = id.into();
    let thumb_key = ElementId::Name(SharedString::from(format!("{id}-thumb")));
    div()
        .flex_none()
        .relative()
        .w_full()
        .child(
            div()
                .id(id)
                .w_full()
                .max_h(s.px(max_height))
                .overflow_y_scroll()
                .track_scroll(handle)
                .when(overflows, |list| list.pr(s.px(GUTTER)))
                .child(content),
        )
        .when(overflows, |list| list.child(scrollbar(thumb_key, Scroller::Div(handle.clone()), s)))
}

/// [`Rows`] drawn by `list`, as tall as its items up to `max_height`, with
/// the thumb while they run past it. The items leave the thumb's gutter
/// themselves while [`Rows::overflows`].
pub fn scroll_rows(key: &str, rows: &Rows, list: List, max_height: f32, s: Scale) -> Div {
    let height = rows.content_height().min(s.px(max_height));
    div()
        .flex_none()
        .relative()
        .w_full()
        .h(height)
        .child(list.size_full())
        .when(rows.overflows(max_height, s), |area| {
            area.child(scrollbar(SharedString::from(format!("{key}-thumb")), Scroller::Rows(rows.clone()), s))
        })
}

/// GPUI's `list` over items whose heights are known before layout: it builds
/// only the items in and near the view, and the thumb and the keyboard work
/// in exact pixels rather than in the heights the list has measured so far.
#[derive(Clone)]
pub struct Rows {
    pub state: ListState,
    items: Rc<RefCell<RowsItems>>,
}

#[derive(Default)]
struct RowsItems {
    /// Each item's top, then the bottom of the last.
    tops: Vec<Pixels>,
    focus: Vec<Option<FocusHandle>>,
}

impl RowsItems {
    fn same(&self, items: &[(Pixels, Option<FocusHandle>)]) -> bool {
        self.focus.len() == items.len()
            && items.iter().enumerate().all(|(ix, (height, focus))| self.tops[ix + 1] - self.tops[ix] == *height && self.focus[ix] == *focus)
    }
}

impl Rows {
    /// `overdraw`: how far beyond the view items are measured ahead.
    pub fn new(overdraw: Pixels) -> Self {
        Self { state: ListState::new(0, ListAlignment::Top, overdraw), items: Rc::default() }
    }

    /// The items, each as its height and the focus handle of the control it
    /// holds, if any: the list keeps drawing an item whose control has the
    /// keyboard wherever it is scrolled. Items that differ from the last ones
    /// replace them, at the same scroll in pixels.
    pub fn set(&self, items: &[(Pixels, Option<FocusHandle>)]) {
        if self.items.borrow().same(items) {
            return;
        }
        let scrolled = self.scrolled();
        let old = {
            let mut stored = self.items.borrow_mut();
            let old = stored.focus.len();
            stored.tops.clear();
            stored.tops.push(Pixels::ZERO);
            let mut bottom = Pixels::ZERO;
            for (height, _) in items {
                bottom += *height;
                stored.tops.push(bottom);
            }
            stored.focus = items.iter().map(|(_, focus)| focus.clone()).collect();
            old
        };
        self.state.splice_focusable(0..old, items.iter().map(|(_, focus)| focus.clone()));
        self.scroll_to(scrolled);
    }

    pub fn content_height(&self) -> Pixels {
        self.items.borrow().tops.last().copied().unwrap_or_default()
    }

    pub fn overflows(&self, max_height: f32, s: Scale) -> bool {
        self.content_height() > s.px(max_height)
    }

    pub fn top_of(&self, ix: usize) -> Option<Pixels> {
        self.items.borrow().tops.get(ix).copied()
    }

    pub fn viewport(&self) -> Bounds<Pixels> {
        self.state.viewport_bounds()
    }

    pub fn max(&self) -> Pixels {
        (self.content_height() - self.viewport().size.height).max(Pixels::ZERO)
    }

    /// How far the list is scrolled, down from its top.
    pub fn scrolled(&self) -> Pixels {
        let top = self.state.logical_scroll_top();
        let at = self.items.borrow().tops.get(top.item_ix).copied().unwrap_or_else(|| self.content_height());
        (at + top.offset_in_item).min(self.max())
    }

    pub fn scroll_to(&self, y: Pixels) {
        let y = y.clamp(Pixels::ZERO, self.max());
        let (item_ix, top) = {
            let items = self.items.borrow();
            let count = items.focus.len();
            let ix = items.tops.partition_point(|top| *top <= y).saturating_sub(1).min(count.saturating_sub(1));
            (ix, items.tops.get(ix).copied().unwrap_or_default())
        };
        self.state.scroll_to(ListOffset { item_ix, offset_in_item: y - top });
    }

    /// Scrolls the least that shows item `ix` with `margin` around it.
    pub fn reveal(&self, ix: usize, margin: Pixels) {
        let (Some(top), Some(bottom)) = (self.top_of(ix), self.top_of(ix + 1)) else {
            return;
        };
        let (scrolled, height) = (self.scrolled(), self.viewport().size.height);
        if top - margin < scrolled {
            self.scroll_to(top - margin);
        } else if height > Pixels::ZERO && bottom + margin > scrolled + height {
            self.scroll_to(bottom + margin - height);
        }
    }

    /// Whether item `ix` is at least partly in view.
    pub fn shows(&self, ix: usize) -> bool {
        let (Some(top), Some(bottom)) = (self.top_of(ix), self.top_of(ix + 1)) else {
            return false;
        };
        let scrolled = self.scrolled();
        let height = Some(self.viewport().size.height).filter(|height| *height > Pixels::ZERO).unwrap_or_else(|| self.content_height());
        bottom > scrolled && top < scrolled + height
    }
}

/// What a scrollbar scrolls.
#[derive(Clone)]
pub enum Scroller {
    /// A div with `overflow_y_scroll`.
    Div(ScrollHandle),
    Rows(Rows),
}

impl Scroller {
    fn viewport_height(&self) -> Pixels {
        match self {
            Self::Div(handle) => handle.bounds().size.height,
            Self::Rows(rows) => rows.viewport().size.height,
        }
    }

    fn max(&self) -> Pixels {
        match self {
            Self::Div(handle) => handle.max_offset().y.max(Pixels::ZERO),
            Self::Rows(rows) => rows.max(),
        }
    }

    fn scrolled(&self) -> Pixels {
        match self {
            Self::Div(handle) => (-handle.offset().y).clamp(Pixels::ZERO, self.max()),
            Self::Rows(rows) => rows.scrolled(),
        }
    }

    fn scroll_to(&self, y: Pixels) {
        match self {
            Self::Div(handle) => handle.set_offset(point(handle.offset().x, -y.clamp(Pixels::ZERO, self.max()))),
            Self::Rows(rows) => rows.scroll_to(y),
        }
    }
}

/// Where the thumb runs in its gutter.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Thumb {
    /// The top of the thumb's run.
    top: Pixels,
    length: Pixels,
    travel: Pixels,
    max: Pixels,
}

impl Thumb {
    fn of(scroller: &Scroller, bounds: Bounds<Pixels>, s: Scale) -> Option<Self> {
        let (max, viewport) = (scroller.max(), scroller.viewport_height());
        if max <= Pixels::ZERO || viewport <= Pixels::ZERO {
            return None;
        }
        let length = (viewport * (viewport / (viewport + max))).max(s.px(16.));
        let travel = (viewport - s.px(4.) - length).max(Pixels::ZERO);
        Some(Self { top: bounds.top() + s.px(2.), length, travel, max })
    }

    fn top_at(self, scrolled: Pixels) -> Pixels {
        self.top + self.travel * (scrolled / self.max).clamp(0., 1.)
    }

    /// The scroll that puts the thumb's top at `top`.
    fn scroll_for(self, top: Pixels) -> Pixels {
        if self.travel <= Pixels::ZERO {
            return Pixels::ZERO;
        }
        self.max * ((top - self.top) / self.travel)
    }
}

/// A press that drags the thumb: the pointer's height and the scroll then.
#[derive(Clone, Copy, Debug, PartialEq)]
struct ThumbGrab {
    y: Pixels,
    scrolled: Pixels,
}

/// The thin thumb GPUI does not draw, painted from the scroll state of the
/// same frame in the gutter on the right, taking the pointer as Zed's
/// scrollbar does. A press in the gutter is the thumb's alone (no window drag,
/// no focus): on the thumb it grabs it, beside it the thumb's middle comes to
/// the pointer and is grabbed there. While it is held, the moves anywhere in
/// the window drive the scroll and the release ends it, both heard in the
/// capture phase, before any element. The wheel still scrolls the list.
pub fn scrollbar(key: impl Into<ElementId>, scroller: Scroller, s: Scale) -> impl IntoElement {
    let key = key.into();
    canvas(
        move |bounds, window, cx| {
            let grab = window.use_keyed_state(key, cx, |_, _| None::<ThumbGrab>);
            let gutter = Bounds::new(point(bounds.right() - s.px(GUTTER), bounds.top()), size(s.px(GUTTER), bounds.size.height));
            (grab, window.insert_hitbox(gutter, HitboxBehavior::BlockMouseExceptScroll))
        },
        move |bounds, (grab, gutter), window, _| {
            let Some(thumb) = Thumb::of(&scroller, bounds, s) else {
                return;
            };
            let origin = point(bounds.right() - s.px(4.), thumb.top_at(scroller.scrolled()));
            window.paint_quad(fill(Bounds::new(origin, size(s.px(3.), thumb.length)), rgba(THUMB)).corner_radii(s.px(2.)));
            let view = window.current_view();

            window.on_mouse_event({
                let (grab, scroller) = (grab.clone(), scroller.clone());
                move |event: &MouseDownEvent, phase, window, cx| {
                    if phase != DispatchPhase::Capture || event.button != MouseButton::Left || !gutter.should_handle_scroll(window) {
                        return;
                    }
                    let top = thumb.top_at(scroller.scrolled());
                    if !(top..=top + thumb.length).contains(&event.position.y) {
                        scroller.scroll_to(thumb.scroll_for(event.position.y - thumb.length / 2.));
                    }
                    let held = ThumbGrab { y: event.position.y, scrolled: scroller.scrolled() };
                    grab.update(cx, |grab, _| *grab = Some(held));
                    window.prevent_default();
                    cx.stop_propagation();
                    cx.notify(view);
                }
            });
            window.on_mouse_event({
                let grab = grab.clone();
                move |event: &MouseMoveEvent, phase, _, cx| {
                    let Some(from) = *grab.read(cx) else {
                        return;
                    };
                    if phase != DispatchPhase::Capture {
                        return;
                    }
                    if event.pressed_button != Some(MouseButton::Left) {
                        grab.update(cx, |grab, _| *grab = None);
                        return;
                    }
                    let moved = thumb.scroll_for(thumb.top + event.position.y - from.y);
                    scroller.scroll_to(from.scrolled + moved);
                    cx.stop_propagation();
                    cx.notify(view);
                }
            });
            window.on_mouse_event(move |event: &MouseUpEvent, phase, _, cx| {
                if phase == DispatchPhase::Capture && event.button == MouseButton::Left && grab.read(cx).is_some() {
                    grab.update(cx, |grab, _| *grab = None);
                }
            });
        },
    )
    .absolute()
    .inset_0()
}

/// Where an element sits in the window, kept from frame to frame, so a
/// popover can hang from it as the window's own child.
#[derive(Clone, Default)]
pub struct Anchor(Rc<Cell<Option<Bounds<Pixels>>>>);

impl Anchor {
    pub fn bounds(&self) -> Option<Bounds<Pixels>> {
        self.0.get()
    }

    /// Put inside the anchored element (which must be `relative`): it fills
    /// it and records where it is. A move asks for one more frame, so the
    /// popover follows it; nothing is drawn otherwise.
    pub fn marker(&self) -> impl IntoElement {
        let cell = self.0.clone();
        canvas(
            move |bounds, window, _| {
                if cell.get() != Some(bounds) {
                    cell.set(Some(bounds));
                    window.on_next_frame(|window, _| window.refresh());
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .inset_0()
    }
}

/// Covers the window while a popover is open: a press anywhere else only
/// closes it and reaches nothing below.
pub fn scrim(id: &'static str) -> Stateful<Div> {
    div().id(id).absolute().inset_0().occlude()
}

/// A GPUI tooltip in the popover colours, for `.tooltip(...)`.
pub fn tooltip(text: SharedString, s: Scale) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    move |_, cx| cx.new(|_| Tip { text: text.clone(), detail: None, s }).into()
}

/// [`tooltip`] with a second line in `faint`, both wrapping at 250 px.
pub fn tooltip_with_detail(text: SharedString, detail: Option<SharedString>, s: Scale) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    move |_, cx| cx.new(|_| Tip { text: text.clone(), detail: detail.clone(), s }).into()
}

struct Tip {
    text: SharedString,
    detail: Option<SharedString>,
    s: Scale,
}

impl Render for Tip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let s = self.s;
        div()
            .font_family(OVERLAY_FAMILY)
            .py(s.px(6.))
            .px(s.px(9.))
            .rounded(s.px(8.))
            .border_1()
            .border_color(rgba(POPOVER_BORDER))
            .bg(rgba(POPOVER))
            .text_size(s.px(11.5))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgba(TEXT))
            .child(self.text.clone())
            .when_some(self.detail.clone(), |tip, detail| {
                tip.flex().flex_col().max_w(s.px(250.)).whitespace_normal().child(div().text_color(rgba(FAINT)).child(detail))
            })
    }
}
