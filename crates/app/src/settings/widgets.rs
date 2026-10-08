//! The Settings window's own controls, in the main-window language of
//! docs/gpui-mockups/approved/settings.html (values in tokens.md, "Main
//! window"). Every control takes the keyboard focus, draws a ring when it was
//! reached from the keyboard, and gives AccessKit its role, label and state.

use std::cell::Cell;
use std::rc::Rc;

use gpui::{
    canvas, div, fill, linear_color_stop, linear_gradient, percentage, point, prelude::*, px, relative, rgb, rgba,
    size, AccessibleAction, Animation, AnimationExt as _, AnyElement, AnyView, App, Bounds, BoxShadow, Context,
    Div, Element, ElementId, FocusHandle, FontWeight, GlobalElementId, Hsla, InspectorElementId, KeyDownEvent, LayoutId,
    MouseButton, MouseDownEvent, Pixels, Render, Role, ScrollHandle, SharedString, Stateful, Toggled, Transformation,
    Window,
};

use crate::assets::Icon;
use crate::fonts::OVERLAY_FAMILY;
use crate::secondary::chrome::{self, Anchor, Scale};

pub const SIDE: u32 = 0x0e0c0b;
pub const CANVAS: u32 = 0x141210;
pub const SURFACE: u32 = 0x1b1815;
pub const RAISED: u32 = 0x241f1b;
pub const LINE: u32 = 0xfff0e114;
pub const LINE_STRONG: u32 = 0xfff0e124;
pub const TEXT: u32 = 0xfff6ecf5;
pub const MUTED: u32 = 0xfff6eca8;
pub const FAINT: u32 = 0xfff6ec70;
pub const CREAM: u32 = 0xf3ebe1;
pub const INK: u32 = 0x17120d;
pub const SELECTED: u32 = 0xfff0e11c;
pub const SOFT: u32 = 0xfff0e112;
pub const GOOD: u32 = 0x86e3a8;
pub const WARN: u32 = 0xf2c66d;
pub const BAD: u32 = 0xf7a8a8;
pub const SCRIM: u32 = 0x0807069e;
pub const POPOVER_BORDER: u32 = 0xffecd624;
const HOVER: u32 = 0xfff0e11c;
const PRESSED: u32 = 0xfff0e12b;
const DANGER_FILL: u32 = 0xf7a8a826;
const DANGER_HOVER: u32 = 0xf7a8a838;
const SWITCH_OFF: u32 = 0xfff0e129;
const SWITCH_OFF_HOVER: u32 = 0xfff0e138;
const SLIDER_TRACK: u32 = 0xfff0e124;
const SLIDER_RANGE: u32 = 0xf3ebe1d9;
const SEGMENT_TRACK: u32 = 0x00000052;
const SELECT_FILL: u32 = 0x0000004d;
const OPEN_BORDER: u32 = 0xfff6ec8c;
const SCROLL_THUMB: u32 = 0xfff0e138;
const KBD_FILL: u32 = 0xfff0e10f;
const KBD_BORDER: u32 = 0xfff0e129;
/// Not in tokens.md: the mockups show no keyboard focus. Cream, as the open
/// select's border, but solid enough to stand out on every surface.
const FOCUS_RING: u32 = 0xf3ebe1e6;

/// Body text: the installed Windows 11 face; GPUI falls back to Segoe UI.
pub const BODY_FONT: &str = "Segoe UI Variable Text";
/// Titles and figures. The mockup's Bahnschrift SemiCondensed cannot be
/// selected in GPUI; the overlays' Barlow Semi Condensed stands in.
pub const DISPLAY_FONT: &str = OVERLAY_FAMILY;

const RING_GAP: f32 = 3.0;
const SPIN: std::time::Duration = std::time::Duration::from_millis(900);
/// A select list's option.
pub const OPTION_HEIGHT: f32 = 28.;

pub type Handler<T> = Rc<dyn Fn(T, &mut Window, &mut App)>;

/// A control's keyboard focus as the view sees it this frame.
#[derive(Clone)]
pub struct Focus {
    pub handle: FocusHandle,
    /// Focused, and the last input came from the keyboard.
    pub ring: bool,
    /// Focused inside the scrolling page, which is to bring it into view.
    pub reveal: Option<Reveal>,
}

/// A request to scroll the page so the focused control shows, honoured once.
#[derive(Clone)]
pub struct Reveal {
    pub scroll: ScrollHandle,
    pub pending: Rc<Cell<bool>>,
}

pub fn icon(glyph: Icon, size: f32, colour: impl Into<Hsla>) -> gpui::Svg {
    chrome::icon(glyph, size, colour, Scale(1.0))
}

/// Makes `element` the focus target of `focus`, with its ring.
fn focusable(element: Stateful<Div>, focus: &Focus, radius: f32) -> Stateful<Div> {
    element
        .relative()
        .track_focus(&focus.handle)
        .when(focus.ring, |element| element.child(ring(radius)))
        .when_some(focus.reveal.clone(), |element, reveal| element.child(reveal_marker(reveal)))
}

fn ring(radius: f32) -> Div {
    div()
        .debug_selector(|| "focus-ring".to_string())
        .absolute()
        .top(px(-RING_GAP))
        .left(px(-RING_GAP))
        .right(px(-RING_GAP))
        .bottom(px(-RING_GAP))
        .rounded(px(radius + RING_GAP))
        .border_2()
        .border_color(rgba(FOCUS_RING))
}

/// Scrolls the page by what the control sticks out of it, plus a margin.
pub fn reveal_marker(reveal: Reveal) -> impl IntoElement {
    canvas(
        move |bounds, window, _| {
            if !reveal.pending.replace(false) {
                return;
            }
            let viewport = reveal.scroll.bounds();
            let margin = px(24.);
            let shift = if bounds.top() < viewport.top() + margin {
                viewport.top() + margin - bounds.top()
            } else if bounds.bottom() > viewport.bottom() - margin {
                viewport.bottom() - margin - bounds.bottom()
            } else {
                return;
            };
            let offset = reveal.scroll.offset();
            let lowest = -reveal.scroll.max_offset().y;
            let y = (offset.y + shift).clamp(lowest, Pixels::ZERO);
            reveal.scroll.set_offset(point(offset.x, y));
            window.on_next_frame(|window, _| window.refresh());
        },
        |_, _, _, _| {},
    )
    .absolute()
    .inset_0()
}

// ---- Page structure ----

pub fn page_head(title: impl Into<SharedString>, lead: Option<SharedString>) -> Div {
    let title = title.into();
    div()
        .flex_none()
        .flex()
        .flex_col()
        .gap(px(5.))
        .child(
            div()
                .id("page-title")
                .role(Role::Heading)
                .aria_level(1)
                .aria_label(title.clone())
                .font_family(DISPLAY_FONT)
                .text_size(px(21.))
                .font_weight(FontWeight::SEMIBOLD)
                .line_height(relative(1.2))
                .child(title),
        )
        .children(lead.map(|lead| div().max_w(px(520.)).text_color(rgba(MUTED)).child(lead)))
}

/// A group's title, 18 px below the page head for the first one.
pub fn group_title(text: impl Into<SharedString>, first: bool) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(px(8.))
        .mt(px(if first { 18. } else { 22. }))
        .mb(px(8.))
        .text_size(px(12.5))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgba(MUTED))
        .child(text.into())
}

/// Rows in a rounded surface, divided by hairlines.
pub fn group(rows: Vec<Div>) -> Div {
    div()
        .flex_none()
        .flex()
        .flex_col()
        .rounded(px(10.))
        .border_1()
        .border_color(rgba(LINE))
        .bg(rgb(SURFACE))
        .children(
            rows.into_iter()
                .enumerate()
                .map(|(index, row)| row.when(index > 0, |row| row.border_t_1().border_color(rgba(LINE)))),
        )
}

/// A label and its sentence on the left, the control on a stable right edge.
pub fn row(label: impl IntoElement, description: Option<AnyElement>, control: impl IntoElement) -> Div {
    row_of(label, description, control, RowLook::Plain)
}

/// A row under a master switch that is off: the label and sentence at half
/// strength; the caller passes the control disabled.
pub fn dimmed_row(label: impl IntoElement, description: Option<AnyElement>, control: impl IntoElement) -> Div {
    row_of(label, description, control, RowLook::Dimmed)
}

/// A row whose control sits under its label and sentence, 10 px apart.
pub fn stacked_row(label: impl IntoElement, description: Option<AnyElement>, control: impl IntoElement) -> Div {
    row_of(label, description, control, RowLook::Stacked)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum RowLook {
    Plain,
    Dimmed,
    Stacked,
}

fn row_of(label: impl IntoElement, description: Option<AnyElement>, control: impl IntoElement, look: RowLook) -> Div {
    div()
        .flex()
        .map(|row| match look {
            RowLook::Stacked => row.flex_col().items_start().gap(px(10.)),
            RowLook::Plain | RowLook::Dimmed => row.items_center().gap(px(20.)),
        })
        .min_h(px(54.))
        .py(px(11.))
        .px(px(16.))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(px(2.))
                .when(look == RowLook::Dimmed, |text| text.opacity(0.5))
                .child(
                    div()
                        .flex()
                        .items_center()
                        .gap(px(6.))
                        .text_size(px(13.5))
                        .font_weight(FontWeight::MEDIUM)
                        .child(label),
                )
                .children(description),
        )
        .child(div().flex_none().flex().items_center().gap(px(8.)).child(control))
}

/// A fixed shortcut as Kbd keys joined by a faint "+", read-only.
pub fn keys(id: &'static str, label: impl Into<SharedString>, keys: Vec<String>) -> AnyElement {
    let mut row = div()
        .id(id)
        .flex()
        .items_center()
        .gap(px(4.))
        .role(Role::Group)
        .aria_label(label.into())
        .text_size(px(11.))
        .text_color(rgba(FAINT));
    for (index, key) in keys.into_iter().enumerate() {
        if index > 0 {
            row = row.child("+");
        }
        row = row.child(
            div()
                .h(px(22.))
                .px(px(7.))
                .flex()
                .items_center()
                .rounded(px(5.))
                .border_1()
                .border_color(rgba(KBD_BORDER))
                .bg(rgba(KBD_FILL))
                .font_family(DISPLAY_FONT)
                .text_size(px(12.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgba(TEXT))
                .child(key),
        );
    }
    row.into_any_element()
}

pub fn description(text: impl Into<SharedString>) -> AnyElement {
    div()
        .max_w(px(400.))
        .text_size(px(12.))
        .line_height(relative(1.4))
        .text_color(rgba(MUTED))
        .child(text.into())
        .into_any_element()
}

/// A line under a row's sentence: `glyph` and `text` in `colour`.
pub fn note(glyph: Icon, text: impl IntoElement, colour: impl Into<Hsla>) -> Div {
    let colour = colour.into();
    div()
        .flex()
        .items_start()
        .gap(px(6.))
        .mt(px(4.))
        .max_w(px(420.))
        .text_size(px(12.))
        .line_height(relative(1.4))
        .text_color(colour)
        .child(icon(glyph, 13., colour).mt(px(2.)))
        .child(div().flex_1().min_w_0().child(text))
}

/// A list row: icon, two lines, actions on the right.
pub fn list_row(glyph: Option<Icon>, title: impl Into<SharedString>, line: impl Into<SharedString>) -> Div {
    div()
        .flex()
        .items_center()
        .gap(px(12.))
        .min_h(px(46.))
        .py(px(8.))
        .px(px(16.))
        .children(glyph.map(|glyph| icon(glyph, 16., rgba(FAINT))))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(div().truncate().font_weight(FontWeight::MEDIUM).child(title.into()))
                .child(div().text_size(px(12.)).text_color(rgba(MUTED)).child(line.into())),
        )
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    Neutral,
    Good,
    Warn,
    Bad,
}

impl Tone {
    fn colour(self) -> Hsla {
        match self {
            Self::Neutral => rgba(FAINT).into(),
            Self::Good => rgb(GOOD).into(),
            Self::Warn => rgb(WARN).into(),
            Self::Bad => rgb(BAD).into(),
        }
    }

    fn tint(self, alpha: u32) -> Hsla {
        match self {
            Self::Neutral => rgba(SOFT).into(),
            Self::Good => rgba((GOOD << 8) | alpha).into(),
            Self::Warn => rgba((WARN << 8) | alpha).into(),
            Self::Bad => rgba((BAD << 8) | alpha).into(),
        }
    }
}

/// A connection tile: tinted icon square, label, value and a quieter line.
pub fn tile(glyph: Icon, label: impl Into<SharedString>, value: impl Into<SharedString>, detail: Option<SharedString>, tone: Tone) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .items_start()
        .gap(px(10.))
        .py(px(11.))
        .px(px(12.))
        .rounded(px(10.))
        .border_1()
        .border_color(rgba(LINE))
        .bg(rgb(SURFACE))
        .child(
            div()
                .flex_none()
                .size(px(30.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(8.))
                .bg(tone.tint(0x1f))
                .child(icon(glyph, 15., tone.colour())),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(div().text_size(px(11.5)).text_color(rgba(FAINT)).child(label.into()))
                .child(div().truncate().text_size(px(13.5)).font_weight(FontWeight::SEMIBOLD).child(value.into()))
                .children(detail.map(|detail| div().text_size(px(11.5)).text_color(rgba(MUTED)).child(detail))),
        )
}

pub fn badge(glyph: Option<AnyElement>, label: impl Into<SharedString>, tone: Tone) -> Div {
    div()
        .flex_none()
        .h(px(22.))
        .px(px(9.))
        .flex()
        .items_center()
        .gap(px(6.))
        .rounded_full()
        .bg(tone.tint(0x24))
        .text_color(if tone == Tone::Neutral { rgba(MUTED).into() } else { tone.colour() })
        .text_size(px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .children(glyph)
        .child(label.into())
}

/// A tinted note at the top of a page: what failed and what was done.
pub fn alert(title: impl Into<SharedString>, text: impl Into<SharedString>, tone: Tone) -> Div {
    div()
        .flex_none()
        .mt(px(14.))
        .flex()
        .items_start()
        .gap(px(10.))
        .py(px(11.))
        .px(px(13.))
        .rounded(px(9.))
        .bg(tone.tint(0x1a))
        .text_size(px(12.5))
        .line_height(relative(1.45))
        .text_color(rgba(MUTED))
        .child(icon(Icon::TriangleAlert, 16., tone.colour()).mt(px(1.)))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .child(div().font_weight(FontWeight::SEMIBOLD).text_color(tone.colour()).child(title.into()))
                .child(text.into()),
        )
}

/// The indeterminate spinner: Lucide `loader-circle` turning, or standing
/// still when Windows' animation effects are off.
pub fn spinner(id: impl Into<ElementId>, size: f32, colour: impl Into<Hsla>, animate: bool) -> AnyElement {
    let glyph = icon(Icon::LoaderCircle, size, colour);
    if animate {
        glyph
            .with_animation(id, Animation::new(SPIN).repeat(), |glyph, t| {
                glyph.with_transformation(Transformation::rotate(percentage(t)))
            })
            .into_any_element()
    } else {
        glyph.into_any_element()
    }
}

/// The thin thumb GPUI does not draw for a scrolling area, painted from the
/// scroll state of the same frame.
pub fn scroll_thumb(handle: ScrollHandle) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let max = handle.max_offset().y;
            let viewport = handle.bounds().size.height;
            if max <= Pixels::ZERO || viewport <= Pixels::ZERO {
                return;
            }
            let length = (viewport * (viewport / (viewport + max))).max(px(24.));
            let travel = viewport - px(16.) - length;
            let progress = (-handle.offset().y / max).clamp(0., 1.);
            let origin = point(bounds.right() - px(7.), bounds.top() + px(8.) + travel * progress);
            window.paint_quad(fill(Bounds::new(origin, size(px(4.), length)), rgba(SCROLL_THUMB)).corner_radii(px(2.)));
        },
    )
    .absolute()
    .inset_0()
}

pub fn popover_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: rgba(0x00000080).into(),
        offset: point(px(0.), px(12.)),
        blur_radius: px(28.),
        spread_radius: px(0.),
        inset: false,
    }]
}

pub fn dialog_shadow() -> Vec<BoxShadow> {
    vec![BoxShadow {
        color: rgba(0x00000099).into(),
        offset: point(px(0.), px(24.)),
        blur_radius: px(60.),
        spread_radius: px(0.),
        inset: false,
    }]
}

// ---- Controls ----

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ButtonKind {
    Secondary,
    Ghost,
    Danger,
}

/// A 30 px button (26 px `small`). Without `on_press` it is disabled: dimmed,
/// out of the tab order, deaf to clicks and announced as unavailable.
pub fn button(
    id: impl Into<SharedString>,
    label: impl Into<SharedString>,
    glyph: Option<Icon>,
    kind: ButtonKind,
    small: bool,
    focus: &Focus,
    on_press: Option<Handler<()>>,
) -> AnyElement {
    let id: SharedString = id.into();
    let label: SharedString = label.into();
    let (fill, hover, border, colour): (Hsla, Hsla, Hsla, Hsla) = match kind {
        ButtonKind::Secondary => (rgba(SOFT).into(), rgba(HOVER).into(), rgba(LINE_STRONG).into(), rgba(TEXT).into()),
        ButtonKind::Ghost => (gpui::transparent_black(), rgba(SOFT).into(), gpui::transparent_black(), rgba(MUTED).into()),
        ButtonKind::Danger => (rgba(DANGER_FILL).into(), rgba(DANGER_HOVER).into(), gpui::transparent_black(), rgb(BAD).into()),
    };
    let element = div()
        .id(ElementId::Name(id.clone()))
        .debug_selector(|| id.to_string())
        .role(Role::Button)
        .aria_label(label.clone())
        .flex_none()
        .h(px(if small { 26. } else { 30. }))
        .px(px(if small { 10. } else { 12. }))
        .flex()
        .items_center()
        .justify_center()
        .gap(px(7.))
        .rounded(px(7.))
        .border_1()
        .border_color(border)
        .bg(fill)
        .text_color(colour)
        .text_size(px(if small { 12. } else { 13. }))
        .font_weight(FontWeight::SEMIBOLD)
        .whitespace_nowrap()
        .children(glyph.map(|glyph| icon(glyph, 14., colour)))
        .when(!label.is_empty(), |button| button.child(label));
    match on_press {
        Some(on_press) => focusable(element, focus, 7.)
            .cursor_pointer()
            .hover(move |style| style.bg(hover))
            .active(|style| style.bg(rgba(PRESSED)))
            .on_click(move |_, window, cx| on_press((), window, cx))
            .into_any_element(),
        None => Disabled(element.opacity(0.45)).into_any_element(),
    }
}

/// 36 × 20; off is a soft track with a cream knob, on is cream with a dark
/// knob. Space and Enter switch it, as a click does.
pub fn switch(
    id: &'static str,
    label: impl Into<SharedString>,
    on: bool,
    focus: &Focus,
    on_toggle: Option<Handler<bool>>,
) -> AnyElement {
    let element = div()
        .id(id)
        .debug_selector(|| id.to_string())
        .role(Role::Switch)
        .aria_label(label.into())
        .aria_toggled(if on { Toggled::True } else { Toggled::False })
        .flex_none()
        .w(px(36.))
        .h(px(20.))
        .rounded_full()
        .bg(if on { Hsla::from(rgb(CREAM)) } else { Hsla::from(rgba(SWITCH_OFF)) })
        .child(
            div()
                .absolute()
                .top(px(2.))
                .left(px(if on { 18. } else { 2. }))
                .size(px(16.))
                .rounded_full()
                .bg(rgb(if on { INK } else { CREAM })),
        );
    match on_toggle {
        Some(on_toggle) => focusable(element, focus, 10.)
            .cursor_pointer()
            .when(!on, |switch| switch.hover(|style| style.bg(rgba(SWITCH_OFF_HOVER))))
            .on_click(move |_, window, cx| on_toggle(!on, window, cx))
            .into_any_element(),
        None => Disabled(element.opacity(0.45)).into_any_element(),
    }
}

/// A control that cannot be used now, announced as unavailable: GPUI's
/// `aria_*` setters have no disabled state, so the flag is added to the
/// node the wrapped element writes.
struct Disabled(Stateful<Div>);

impl IntoElement for Disabled {
    type Element = Self;

    fn into_element(self) -> Self {
        self
    }
}

impl Element for Disabled {
    type RequestLayoutState = <Stateful<Div> as Element>::RequestLayoutState;
    type PrepaintState = <Stateful<Div> as Element>::PrepaintState;

    fn id(&self) -> Option<ElementId> {
        Element::id(&self.0)
    }

    fn source_location(&self) -> Option<&'static std::panic::Location<'static>> {
        self.0.source_location()
    }

    fn request_layout(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        self.0.request_layout(id, inspector_id, window, cx)
    }

    fn prepaint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        self.0.prepaint(id, inspector_id, bounds, request_layout, window, cx)
    }

    fn paint(
        &mut self,
        id: Option<&GlobalElementId>,
        inspector_id: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        request_layout: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        self.0.paint(id, inspector_id, bounds, request_layout, prepaint, window, cx)
    }

    fn a11y_role(&self) -> Option<Role> {
        self.0.a11y_role()
    }

    fn write_a11y_info(&self, node: &mut gpui::accesskit::Node) {
        self.0.write_a11y_info(node);
        node.set_disabled();
    }
}

/// A slider's range and where it stands; the value is in the slider's own
/// units (opacity in percent, scale as a factor, players as a count).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SliderSpec {
    pub min: f64,
    pub max: f64,
    pub step: f64,
    /// PageUp and PageDown move this far.
    pub page: f64,
    pub value: f64,
}

impl SliderSpec {
    /// The value a key moves the slider to; `None` for keys it ignores.
    pub fn after_key(&self, key: &str) -> Option<f64> {
        let target = match key {
            "right" | "up" => self.value + self.step,
            "left" | "down" => self.value - self.step,
            "pageup" => self.value + self.page,
            "pagedown" => self.value - self.page,
            "home" => self.min,
            "end" => self.max,
            _ => return None,
        };
        Some(self.snapped(target))
    }

    /// The value under `fraction` (0 at the left end, 1 at the right).
    pub fn at(&self, fraction: f32) -> f64 {
        self.snapped(self.min + f64::from(fraction.clamp(0., 1.)) * (self.max - self.min))
    }

    fn snapped(&self, value: f64) -> f64 {
        let steps = ((value - self.min) / self.step).round();
        (self.min + steps * self.step).clamp(self.min, self.max)
    }

    fn fraction(&self) -> f32 {
        if self.max > self.min {
            ((self.value - self.min) / (self.max - self.min)).clamp(0., 1.) as f32
        } else {
            0.
        }
    }
}

/// The track (4 px, cream range, 16 px thumb) then the value on the right.
/// The track is the focus target; `track` records where it is so a press or
/// drag can be turned into a value.
#[allow(clippy::too_many_arguments)]
pub fn slider(
    id: &'static str,
    label: impl Into<SharedString>,
    spec: SliderSpec,
    text: SharedString,
    width: f32,
    focus: &Focus,
    track: &Anchor,
    on_change: Handler<f64>,
    on_press: Handler<gpui::Point<Pixels>>,
) -> Div {
    let fraction = spec.fraction();
    let on_key = on_change.clone();
    let on_increment = on_change.clone();
    let on_decrement = on_change.clone();
    let on_set = on_change;
    let control = div()
        .id(id)
        .debug_selector(|| id.to_string())
        .role(Role::Slider)
        .aria_label(label.into())
        .aria_numeric_value(spec.value)
        .aria_min_numeric_value(spec.min)
        .aria_max_numeric_value(spec.max)
        .aria_numeric_value_step(spec.step)
        .aria_value(text.clone())
        .flex_none()
        .w(px(width))
        .h(px(20.))
        .cursor_pointer()
        .child(track.marker())
        .child(
            div()
                .absolute()
                .top(px(8.))
                .left_0()
                .right_0()
                .h(px(4.))
                .rounded(px(2.))
                .bg(rgba(SLIDER_TRACK)),
        )
        .child(
            div()
                .absolute()
                .top(px(8.))
                .left_0()
                .w(relative(fraction))
                .h(px(4.))
                .rounded(px(2.))
                .bg(rgba(SLIDER_RANGE)),
        )
        .child(
            div()
                .absolute()
                .top(px(2.))
                .left(px(fraction * width - 8.))
                .size(px(16.))
                .rounded_full()
                .border_2()
                .border_color(rgb(INK))
                .bg(rgb(CREAM)),
        )
        .on_mouse_down(MouseButton::Left, move |event: &MouseDownEvent, window, cx| {
            on_press(event.position, window, cx)
        })
        .on_key_down(move |event: &KeyDownEvent, window, cx| {
            if event.keystroke.modifiers.modified() {
                return;
            }
            if let Some(value) = spec.after_key(&event.keystroke.key) {
                cx.stop_propagation();
                on_key(value, window, cx);
            }
        })
        .on_a11y_action(AccessibleAction::Increment, move |_, window, cx| {
            if let Some(value) = spec.after_key("right") {
                on_increment(value, window, cx);
            }
        })
        .on_a11y_action(AccessibleAction::Decrement, move |_, window, cx| {
            if let Some(value) = spec.after_key("left") {
                on_decrement(value, window, cx);
            }
        })
        .on_a11y_action(AccessibleAction::SetValue, move |data, window, cx| {
            if let Some(gpui::accesskit::ActionData::NumericValue(value)) = data {
                let fraction = ((value - spec.min) / (spec.max - spec.min)) as f32;
                on_set(spec.at(fraction), window, cx);
            }
        });
    div()
        .flex()
        .items_center()
        .gap(px(12.))
        .child(focusable(control, focus, 10.))
        .child(
            div()
                .w(px(44.))
                .flex()
                .justify_end()
                .font_family(DISPLAY_FONT)
                .text_size(px(13.5))
                .font_weight(FontWeight::SEMIBOLD)
                .child(text),
        )
}

/// Where a press at `x` lands on a track laid out at `bounds`, 0 to 1.
pub fn fraction_at(bounds: Bounds<Pixels>, x: Pixels) -> f32 {
    if bounds.size.width <= Pixels::ZERO {
        return 0.;
    }
    ((x - bounds.left()) / bounds.size.width).clamp(0., 1.)
}

/// Pill options in a dark track. One tab stop; the arrows, Home and End move
/// the choice, as in a radio group.
pub fn segmented(
    id: &'static str,
    label: impl Into<SharedString>,
    options: &[SharedString],
    selected: usize,
    focus: &Focus,
    on_pick: Handler<usize>,
) -> Stateful<Div> {
    let count = options.len();
    let on_key = on_pick.clone();
    let element = div()
        .id(id)
        .debug_selector(|| id.to_string())
        .role(Role::RadioGroup)
        .aria_label(label.into())
        .aria_value(options[selected].clone())
        .flex_none()
        .flex()
        .gap(px(2.))
        .p(px(2.))
        .rounded(px(8.))
        .bg(rgba(SEGMENT_TRACK))
        .on_key_down(move |event: &KeyDownEvent, window, cx| {
            if event.keystroke.modifiers.modified() {
                return;
            }
            let next = match event.keystroke.key.as_str() {
                "left" | "up" => (selected + count - 1) % count,
                "right" | "down" => (selected + 1) % count,
                "home" => 0,
                "end" => count - 1,
                _ => return,
            };
            cx.stop_propagation();
            if next != selected {
                on_key(next, window, cx);
            }
        })
        .children(options.iter().enumerate().map(|(index, option)| {
            let on = index == selected;
            let on_pick = on_pick.clone();
            div()
                .id(index)
                .role(Role::RadioButton)
                .aria_label(option.clone())
                .aria_toggled(if on { Toggled::True } else { Toggled::False })
                .h(px(26.))
                .px(px(11.))
                .flex()
                .items_center()
                .rounded(px(6.))
                .text_size(px(12.5))
                .font_weight(FontWeight::SEMIBOLD)
                .cursor_pointer()
                .when(on, |option| option.bg(rgba(SELECTED)).text_color(rgba(TEXT)))
                .when(!on, |option| {
                    option.text_color(rgba(MUTED)).hover(|style| style.bg(rgba(SOFT)).text_color(rgba(TEXT)))
                })
                .on_click(move |_, window, cx| {
                    if !on {
                        on_pick(index, window, cx);
                    }
                })
                .child(option.clone())
        }));
    focusable(element, focus, 8.)
}

/// The closed select: value and chevron in a 30 px box at least `min_width`
/// wide. The view owns the open state, the keys and the menu.
pub fn select_trigger(
    id: &'static str,
    label: impl Into<SharedString>,
    value: SharedString,
    open: bool,
    min_width: f32,
    focus: &Focus,
    anchor: &Anchor,
) -> Stateful<Div> {
    let element = select_box(id, label, value, open, min_width)
        .cursor_pointer()
        .hover(|style| style.border_color(rgba(OPEN_BORDER)))
        .child(anchor.marker());
    focusable(element, focus, 7.)
}

/// [`select_trigger`] that cannot be opened now: dimmed, out of the tab
/// order and announced as unavailable.
pub fn disabled_select(id: &'static str, label: impl Into<SharedString>, value: SharedString, min_width: f32) -> AnyElement {
    Disabled(select_box(id, label, value, false, min_width).opacity(0.45)).into_any_element()
}

fn select_box(id: &'static str, label: impl Into<SharedString>, value: SharedString, open: bool, min_width: f32) -> Stateful<Div> {
    div()
        .id(id)
        .debug_selector(|| id.to_string())
        .role(Role::ComboBox)
        .aria_label(label.into())
        .aria_value(value.clone())
        .aria_expanded(open)
        .flex_none()
        .min_w(px(min_width))
        .h(px(30.))
        .pl(px(11.))
        .pr(px(9.))
        .flex()
        .items_center()
        .gap(px(8.))
        .rounded(px(7.))
        .border_1()
        .border_color(rgba(if open { OPEN_BORDER } else { LINE_STRONG }))
        .bg(rgba(SELECT_FILL))
        .child(div().flex_1().child(value))
        .child(icon(Icon::ChevronDown, 14., rgba(MUTED)))
}

/// The select's list, hung below the trigger at `below` in window
/// coordinates; it swallows the presses that land on it. With `scroll`, a
/// list longer than `shown` options scrolls, with the thin thumb.
#[allow(clippy::too_many_arguments)]
pub fn select_menu(
    below: Bounds<Pixels>,
    label: impl Into<SharedString>,
    options: &[SharedString],
    current: Option<usize>,
    highlighted: usize,
    on_pick: Handler<usize>,
    on_hover: Handler<usize>,
    animate: bool,
    scroll: Option<(&ScrollHandle, usize)>,
) -> AnyElement {
    let items = options.iter().enumerate().map(|(index, option)| {
        let on_pick = on_pick.clone();
        let on_hover = on_hover.clone();
        let is_current = current == Some(index);
        div()
            .id(("option", index))
            .debug_selector(move || format!("option-{index}"))
            .role(Role::ListBoxOption)
            .aria_label(option.clone())
            .aria_selected(is_current)
            .flex_none()
            .h(px(OPTION_HEIGHT))
            .px(px(8.))
            .flex()
            .items_center()
            .gap(px(8.))
            .rounded(px(6.))
            .cursor_pointer()
            .when(index == highlighted, |item| item.bg(rgba(SELECTED)))
            .on_hover(move |hovered, window, cx| {
                if *hovered {
                    on_hover(index, window, cx);
                }
            })
            .on_click(move |_, window, cx| on_pick(index, window, cx))
            .child(div().flex_1().child(option.clone()))
            .when(is_current, |item| item.child(icon(Icon::Check, 14., rgba(TEXT))))
    });
    let menu = div()
        .id("select-menu")
        .debug_selector(|| "select-menu".to_string())
        .role(Role::ListBox)
        .aria_label(label.into())
        .absolute()
        .left(below.left())
        .top(below.bottom() + px(4.))
        .w(below.size.width)
        .occlude()
        .rounded(px(9.))
        .border_1()
        .border_color(rgba(POPOVER_BORDER))
        .bg(rgb(RAISED))
        .shadow(popover_shadow());
    let menu = match scroll.filter(|(_, shown)| options.len() > *shown) {
        Some((handle, shown)) => menu.relative().child(
            div()
                .id("select-options")
                .max_h(px(shown as f32 * OPTION_HEIGHT + 8.))
                .overflow_y_scroll()
                .track_scroll(handle)
                .p(px(4.))
                .pr(px(10.))
                .flex()
                .flex_col()
                .children(items),
        )
        .child(menu_thumb(handle.clone())),
        None => menu.p(px(4.)).flex().flex_col().children(items),
    };
    if animate {
        menu.with_animation("select-menu-fade", Animation::new(std::time::Duration::from_millis(100)), |menu, t| {
            menu.opacity(t)
        })
        .into_any_element()
    } else {
        menu.into_any_element()
    }
}

/// The 3 px thumb of a scrolling select list.
fn menu_thumb(handle: ScrollHandle) -> impl IntoElement {
    canvas(
        |_, _, _| {},
        move |bounds, _, window, _| {
            let max = handle.max_offset().y;
            let viewport = handle.bounds().size.height;
            if max <= Pixels::ZERO || viewport <= Pixels::ZERO {
                return;
            }
            let length = (viewport * (viewport / (viewport + max))).max(px(16.));
            let travel = viewport - px(8.) - length;
            let progress = (-handle.offset().y / max).clamp(0., 1.);
            let origin = point(bounds.right() - px(6.), bounds.top() + px(4.) + travel * progress);
            window.paint_quad(fill(Bounds::new(origin, size(px(3.), length)), rgba(SCROLL_THUMB)).corner_radii(px(2.)));
        },
    )
    .absolute()
    .inset_0()
}

/// A sidebar entry: 32 px, icon and name; the open section is lit.
pub fn side_item(id: &'static str, glyph: Icon, label: impl Into<SharedString>, selected: bool, focus: &Focus) -> Stateful<Div> {
    let label = label.into();
    let colour: Hsla = rgba(if selected { TEXT } else { MUTED }).into();
    let element = div()
        .id(id)
        .debug_selector(|| id.to_string())
        .role(Role::Tab)
        .aria_label(label.clone())
        .aria_selected(selected)
        .flex_none()
        .h(px(32.))
        .px(px(10.))
        .flex()
        .items_center()
        .gap(px(10.))
        .rounded(px(7.))
        .text_color(colour)
        .font_weight(FontWeight::MEDIUM)
        .cursor_pointer()
        .when(selected, |item| item.bg(rgba(SELECTED)))
        .when(!selected, |item| item.hover(|style| style.bg(rgba(SOFT)).text_color(rgba(TEXT))))
        .child(icon(glyph, 16., colour))
        .child(label);
    focusable(element, focus, 7.)
}

/// A main-window tooltip, for `.tooltip(...)`.
pub fn tooltip(text: impl Into<SharedString>) -> impl Fn(&mut Window, &mut App) -> AnyView + 'static {
    let text = text.into();
    move |_, cx| cx.new(|_| Tip { text: text.clone() }).into()
}

struct Tip {
    text: SharedString,
}

impl Render for Tip {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .font_family(BODY_FONT)
            .max_w(px(250.))
            .py(px(8.))
            .px(px(10.))
            .rounded(px(8.))
            .border_1()
            .border_color(rgba(POPOVER_BORDER))
            .bg(rgb(RAISED))
            .shadow(popover_shadow())
            .text_size(px(12.))
            .line_height(relative(1.4))
            .text_color(rgba(TEXT))
            .child(self.text.clone())
    }
}

/// The preview's game-like backdrop: sky to grass, with a warm and a cold
/// light, so the card's opacity reads as it would in game.
pub fn backdrop() -> Div {
    let light = |colour: u32, left: f32, top: f32, side: f32| {
        div().absolute().left(px(left)).top(px(top)).size(px(side)).rounded_full().bg(rgba(colour))
    };
    div()
        .absolute()
        .inset_0()
        .bg(linear_gradient(
            180.,
            linear_color_stop(rgb(0x6f97bd), 0.),
            linear_color_stop(rgb(0x39421f), 1.),
        ))
        .child(light(0xffd27a8c, 54., 36., 90.))
        .child(light(0x78ebff73, 10., 190., 80.))
        .child(light(0xf046d259, 214., 160., 60.))
}

#[cfg(test)]
mod tests {
    use super::*;

    const OPACITY: SliderSpec = SliderSpec { min: 0., max: 100., step: 1., page: 10., value: 59. };

    #[test]
    fn keys_move_a_slider_by_a_step_a_page_or_to_an_end() {
        assert_eq!(OPACITY.after_key("right"), Some(60.));
        assert_eq!(OPACITY.after_key("left"), Some(58.));
        assert_eq!(OPACITY.after_key("pageup"), Some(69.));
        assert_eq!(OPACITY.after_key("pagedown"), Some(49.));
        assert_eq!(OPACITY.after_key("home"), Some(0.));
        assert_eq!(OPACITY.after_key("end"), Some(100.));
        assert_eq!(OPACITY.after_key("a"), None);
        let top = SliderSpec { value: 100., ..OPACITY };
        assert_eq!(top.after_key("right"), Some(100.));
    }

    #[test]
    fn a_press_on_the_track_snaps_to_the_step() {
        let scale = SliderSpec { min: 0.7, max: 1.5, step: 0.05, page: 0.25, value: 1.0 };
        assert!((scale.at(0.5) - 1.1).abs() < 1e-9);
        assert!((scale.at(0.51) - 1.1).abs() < 1e-9);
        assert_eq!(scale.at(-1.), 0.7);
        let bounds = Bounds::new(point(px(100.), px(0.)), size(px(200.), px(20.)));
        assert_eq!(fraction_at(bounds, px(150.)), 0.25);
        assert_eq!(fraction_at(bounds, px(400.)), 1.);
    }
}
