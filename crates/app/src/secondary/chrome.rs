//! The approved overlay language the history, player detail, PvP and reminder
//! windows share (docs/gpui-mockups/approved/overlay.css, values in
//! tokens.md): the card the window is exactly as large as, a header that drags
//! the window, tool and text buttons, tabs, inputs, popovers, empty states and
//! the thin scroll thumb. The user's content scale and card opacity are the
//! meter's.

use std::cell::Cell;
use std::rc::Rc;
use std::sync::Arc;

use gpui::{
    canvas, div, fill, img, point, prelude::*, relative, rgb, rgba, size, svg, AnyElement, AnyView, App, Bounds,
    Context, Div, ElementId, FontFeatures, FontWeight, Hsla, IntoElement, Pixels, Render, ScrollHandle,
    SharedString, Stateful, Svg, Window,
};

use crate::assets::Icon;
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
    div()
        .flex_none()
        .w(s.px(width))
        .flex()
        .flex_col()
        .pt(s.px(6.))
        .px(s.px(8.))
        .pb(s.px(8.))
        .rounded(s.px(14.))
        .border_1()
        .border_color(rgba(CARD_BORDER))
        .bg(rgba((CARD_RGB << 8) | u32::from(look.alpha)))
}

/// The header row: `title` drags the window; `tools` and the close button sit
/// at the top right.
pub fn header(title: impl IntoElement, tools: Vec<AnyElement>, drag: &overlay::WindowDrag, s: Scale) -> Div {
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
        .child(close_button(s))
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

fn close_button(s: Scale) -> Stateful<Div> {
    tool_button("close", Icon::Close, false, s).on_click(|_, window, _| window.remove_window())
}

pub fn icon(glyph: Icon, size: f32, colour: impl Into<Hsla>, s: Scale) -> Svg {
    svg().path(glyph.path()).flex_none().size(s.px(size)).text_color(colour.into())
}

/// 22 px square button with a 13 px icon; `hot` while what it opens is shown.
pub fn tool_button(id: impl Into<ElementId>, glyph: Icon, hot: bool, s: Scale) -> Stateful<Div> {
    square_button(id, glyph, hot, 22., 13., s)
}

/// The 20 px variant inside rows, with a 12 px icon.
pub fn row_button(id: impl Into<ElementId>, glyph: Icon, s: Scale) -> Stateful<Div> {
    square_button(id, glyph, false, 20., 12., s)
}

fn square_button(id: impl Into<ElementId>, glyph: Icon, hot: bool, side: f32, glyph_size: f32, s: Scale) -> Stateful<Div> {
    let id = id.into();
    let group = SharedString::from(format!("button-{id}"));
    div()
        .id(id)
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
}

pub fn chip(glyph: Option<Icon>, label: impl Into<SharedString>, tone: Tone, s: Scale) -> Div {
    let (fill, colour) = match tone {
        Tone::Good => (CHIP_GOOD, GOOD),
        Tone::Warn => (CHIP_WARN, WARN),
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

/// A list that scrolls past `max_height`, with the thin thumb GPUI does not
/// draw by itself. `overflows` reserves the thumb's 8 px on the right; the
/// callers know their row heights, so it is decided before layout.
pub fn scroll_list(
    id: impl Into<ElementId>,
    handle: &ScrollHandle,
    max_height: f32,
    overflows: bool,
    content: impl IntoElement,
    s: Scale,
) -> Div {
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
                .when(overflows, |list| list.pr(s.px(8.)))
                .child(content),
        )
        .when(overflows, |list| list.child(thumb(handle.clone(), s)))
}

/// Painted from the scroll state of the same frame, so scrolling redraws it
/// without another render.
fn thumb(handle: ScrollHandle, s: Scale) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let max = handle.max_offset().y;
            let viewport = handle.bounds().size.height;
            if max <= Pixels::ZERO || viewport <= Pixels::ZERO {
                return;
            }
            let length = (viewport * (viewport / (viewport + max))).max(s.px(16.));
            let travel = viewport - s.px(4.) - length;
            let progress = (-handle.offset().y / max).clamp(0., 1.);
            let origin = point(bounds.right() - s.px(4.), bounds.top() + s.px(2.) + travel * progress);
            window.paint_quad(fill(Bounds::new(origin, size(s.px(3.), length)), rgba(THUMB)).corner_radii(s.px(2.)));
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
    move |_, cx| cx.new(|_| Tip { text: text.clone(), s }).into()
}

struct Tip {
    text: SharedString,
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
    }
}
