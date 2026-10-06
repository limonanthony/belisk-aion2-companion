//! A one-line text field: typing, IME composition, caret movement, paste,
//! Enter and Escape. It is adapted from GPUI's `examples/input.rs` (Apache-2.0)
//! without mouse selections or dragging; the only selection is the whole text,
//! as a field opened on a value has it, so typing replaces it. GPUI Kit's
//! `Input` was not added for these few fields: it would bring the Kit into the
//! overlay build, with a theme of its own to override.

use std::ops::Range;

use gpui::{
    actions, div, fill, point, prelude::*, px, relative, rgba, size, App, Bounds, Context, CursorStyle,
    Element, ElementId, ElementInputHandler, Entity, EntityInputHandler, EventEmitter, FocusHandle, Focusable,
    GlobalElementId, Hsla, InspectorElementId, KeyBinding, LayoutId, MouseButton, MouseDownEvent, PaintQuad,
    Pixels, Point, ShapedLine, SharedString, Style, TextRun, UTF16Selection, UnderlineStyle, Window,
};

actions!(text_input, [Backspace, Delete, Left, Right, Home, End, Paste, Submit, Cancel]);

const CONTEXT: &str = "TextInput";
const MAX_CHARS: usize = 32;
const PLACEHOLDER: u32 = 0xfff6ec70;
const SELECTION: u32 = 0xfff6ec47;

pub fn bind_keys(cx: &mut App) {
    cx.bind_keys([
        KeyBinding::new("backspace", Backspace, Some(CONTEXT)),
        KeyBinding::new("delete", Delete, Some(CONTEXT)),
        KeyBinding::new("left", Left, Some(CONTEXT)),
        KeyBinding::new("right", Right, Some(CONTEXT)),
        KeyBinding::new("home", Home, Some(CONTEXT)),
        KeyBinding::new("end", End, Some(CONTEXT)),
        KeyBinding::new("ctrl-v", Paste, Some(CONTEXT)),
        KeyBinding::new("enter", Submit, Some(CONTEXT)),
        KeyBinding::new("escape", Cancel, Some(CONTEXT)),
    ]);
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TextInputEvent {
    Submitted(String),
    Cancelled,
}

pub struct TextInput {
    focus_handle: FocusHandle,
    content: String,
    placeholder: SharedString,
    max_chars: usize,
    digits_only: bool,
    right_aligned: bool,
    /// Byte offset of the caret.
    cursor: usize,
    /// The whole text is selected: typing replaces it, an arrow key drops the
    /// selection.
    all_selected: bool,
    marked_range: Option<Range<usize>>,
    last_layout: Option<ShapedLine>,
    last_bounds: Option<Bounds<Pixels>>,
}

impl EventEmitter<TextInputEvent> for TextInput {}

impl Focusable for TextInput {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl TextInput {
    pub fn new(placeholder: impl Into<SharedString>, cx: &mut Context<Self>) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            content: String::new(),
            placeholder: placeholder.into(),
            max_chars: MAX_CHARS,
            digits_only: false,
            right_aligned: false,
            cursor: 0,
            all_selected: false,
            marked_range: None,
            last_layout: None,
            last_bounds: None,
        }
    }

    pub fn with_max_chars(mut self, max_chars: usize) -> Self {
        self.max_chars = max_chars;
        self
    }

    /// Anything typed or pasted other than the digits 0 to 9 is left out.
    pub fn digits_only(mut self) -> Self {
        self.digits_only = true;
        self
    }

    pub fn right_aligned(mut self) -> Self {
        self.right_aligned = true;
        self
    }

    pub fn set_placeholder(&mut self, placeholder: impl Into<SharedString>, cx: &mut Context<Self>) {
        self.placeholder = placeholder.into();
        cx.notify();
    }

    pub fn text(&self) -> &str {
        &self.content
    }

    pub fn clear(&mut self, cx: &mut Context<Self>) {
        self.content.clear();
        self.cursor = 0;
        self.all_selected = false;
        self.marked_range = None;
        cx.notify();
    }

    /// Shows `text` selected, so what is typed next replaces it.
    pub fn set_text_selected(&mut self, text: &str, cx: &mut Context<Self>) {
        self.content = text.chars().take(self.max_chars).collect();
        self.cursor = self.content.len();
        self.all_selected = !self.content.is_empty();
        self.marked_range = None;
        cx.notify();
    }

    fn move_to(&mut self, offset: usize, cx: &mut Context<Self>) {
        self.cursor = offset;
        self.all_selected = false;
        cx.notify();
    }

    fn previous_boundary(&self) -> usize {
        self.content[..self.cursor].char_indices().next_back().map_or(0, |(index, _)| index)
    }

    fn next_boundary(&self) -> usize {
        self.content[self.cursor..]
            .chars()
            .next()
            .map_or(self.cursor, |next| self.cursor + next.len_utf8())
    }

    fn backspace(&mut self, _: &Backspace, window: &mut Window, cx: &mut Context<Self>) {
        if self.all_selected {
            self.replace(0..self.content.len(), "", cx);
            return;
        }
        let start = self.previous_boundary();
        if start < self.cursor {
            self.replace(start..self.cursor, "", cx);
        } else {
            window.play_system_bell();
        }
    }

    fn delete(&mut self, _: &Delete, window: &mut Window, cx: &mut Context<Self>) {
        if self.all_selected {
            self.replace(0..self.content.len(), "", cx);
            return;
        }
        let end = self.next_boundary();
        if end > self.cursor {
            self.replace(self.cursor..end, "", cx);
        } else {
            window.play_system_bell();
        }
    }

    fn left(&mut self, _: &Left, _: &mut Window, cx: &mut Context<Self>) {
        let to = if self.all_selected { 0 } else { self.previous_boundary() };
        self.move_to(to, cx);
    }

    fn right(&mut self, _: &Right, _: &mut Window, cx: &mut Context<Self>) {
        let to = if self.all_selected { self.content.len() } else { self.next_boundary() };
        self.move_to(to, cx);
    }

    fn home(&mut self, _: &Home, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(0, cx);
    }

    fn end(&mut self, _: &End, _: &mut Window, cx: &mut Context<Self>) {
        self.move_to(self.content.len(), cx);
    }

    fn paste(&mut self, _: &Paste, _: &mut Window, cx: &mut Context<Self>) {
        if let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) {
            let single_line = text.replace(['\r', '\n'], " ");
            let range = self.typing_range();
            self.replace(range, &single_line, cx);
        }
    }

    fn submit(&mut self, _: &Submit, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(TextInputEvent::Submitted(self.content.clone()));
    }

    fn cancel(&mut self, _: &Cancel, _: &mut Window, cx: &mut Context<Self>) {
        cx.emit(TextInputEvent::Cancelled);
    }

    fn on_mouse_down(&mut self, event: &MouseDownEvent, _: &mut Window, cx: &mut Context<Self>) {
        let offset = self.index_for_mouse_position(event.position);
        self.move_to(offset, cx);
    }

    fn index_for_mouse_position(&self, position: Point<Pixels>) -> usize {
        match (&self.last_bounds, &self.last_layout) {
            (Some(bounds), Some(line)) if !self.content.is_empty() => {
                line.closest_index_for_x(position.x - bounds.left() - self.text_offset(bounds, line))
            }
            _ => 0,
        }
    }

    /// How far right of the field's left edge the text starts.
    fn text_offset(&self, bounds: &Bounds<Pixels>, line: &ShapedLine) -> Pixels {
        if self.right_aligned {
            (bounds.size.width - line.width).max(Pixels::ZERO)
        } else {
            Pixels::ZERO
        }
    }

    /// Where typed text goes: over the selection, else at the caret.
    fn typing_range(&self) -> Range<usize> {
        if self.all_selected {
            0..self.content.len()
        } else {
            self.cursor..self.cursor
        }
    }

    /// Replaces `range`, keeping the field within its length limit and its
    /// characters, and puts the caret after the new text.
    fn replace(&mut self, range: Range<usize>, new_text: &str, cx: &mut Context<Self>) {
        let kept = self.content.chars().count() - self.content[range.clone()].chars().count();
        let room = self.max_chars.saturating_sub(kept);
        let new_text: String =
            new_text.chars().filter(|ch| !self.digits_only || ch.is_ascii_digit()).take(room).collect();
        self.content.replace_range(range.clone(), &new_text);
        self.cursor = range.start + new_text.len();
        self.all_selected = false;
        self.marked_range = None;
        cx.notify();
    }

    fn offset_from_utf16(&self, offset: usize) -> usize {
        let mut utf8_offset = 0;
        let mut utf16_count = 0;
        for ch in self.content.chars() {
            if utf16_count >= offset {
                break;
            }
            utf16_count += ch.len_utf16();
            utf8_offset += ch.len_utf8();
        }
        utf8_offset
    }

    fn offset_to_utf16(&self, offset: usize) -> usize {
        let mut utf16_offset = 0;
        let mut utf8_count = 0;
        for ch in self.content.chars() {
            if utf8_count >= offset {
                break;
            }
            utf8_count += ch.len_utf8();
            utf16_offset += ch.len_utf16();
        }
        utf16_offset
    }

    fn range_to_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_to_utf16(range.start)..self.offset_to_utf16(range.end)
    }

    fn range_from_utf16(&self, range: &Range<usize>) -> Range<usize> {
        self.offset_from_utf16(range.start)..self.offset_from_utf16(range.end)
    }
}

impl EntityInputHandler for TextInput {
    fn text_for_range(
        &mut self,
        range_utf16: Range<usize>,
        actual_range: &mut Option<Range<usize>>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<String> {
        let range = self.range_from_utf16(&range_utf16);
        actual_range.replace(self.range_to_utf16(&range));
        Some(self.content[range].to_string())
    }

    fn selected_text_range(&mut self, _: bool, _: &mut Window, _: &mut Context<Self>) -> Option<UTF16Selection> {
        let range = self.typing_range();
        Some(UTF16Selection { range: self.range_to_utf16(&range), reversed: false })
    }

    fn marked_text_range(&self, _: &mut Window, _: &mut Context<Self>) -> Option<Range<usize>> {
        self.marked_range.as_ref().map(|range| self.range_to_utf16(range))
    }

    fn unmark_text(&mut self, _: &mut Window, _: &mut Context<Self>) {
        self.marked_range = None;
    }

    fn replace_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or(self.marked_range.clone())
            .unwrap_or_else(|| self.typing_range());
        self.replace(range, new_text, cx);
    }

    fn replace_and_mark_text_in_range(
        &mut self,
        range_utf16: Option<Range<usize>>,
        new_text: &str,
        new_selected_range_utf16: Option<Range<usize>>,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let range = range_utf16
            .as_ref()
            .map(|range| self.range_from_utf16(range))
            .or(self.marked_range.clone())
            .unwrap_or_else(|| self.typing_range());
        self.replace(range.clone(), new_text, cx);
        if !new_text.is_empty() {
            self.marked_range = Some(range.start..range.start + new_text.len());
        }
        if let Some(selected) = new_selected_range_utf16 {
            self.cursor = range.start + self.range_from_utf16(&selected).end;
        }
    }

    fn bounds_for_range(
        &mut self,
        range_utf16: Range<usize>,
        bounds: Bounds<Pixels>,
        _: &mut Window,
        _: &mut Context<Self>,
    ) -> Option<Bounds<Pixels>> {
        let layout = self.last_layout.as_ref()?;
        let range = self.range_from_utf16(&range_utf16);
        let left = bounds.left() + self.text_offset(&bounds, layout);
        Some(Bounds::from_corners(
            point(left + layout.x_for_index(range.start), bounds.top()),
            point(left + layout.x_for_index(range.end), bounds.bottom()),
        ))
    }

    fn character_index_for_point(&mut self, point: Point<Pixels>, _: &mut Window, _: &mut Context<Self>) -> Option<usize> {
        let bounds = self.last_bounds?;
        let line_point = bounds.localize(&point)?;
        let layout = self.last_layout.as_ref()?;
        let index = layout.index_for_x(line_point.x - self.text_offset(&bounds, layout))?;
        Some(self.offset_to_utf16(index))
    }
}

struct TextElement {
    input: Entity<TextInput>,
}

struct Prepaint {
    line: ShapedLine,
    /// Where the line starts, right of the left edge when right-aligned.
    origin: Point<Pixels>,
    caret: PaintQuad,
    selection: Option<PaintQuad>,
}

impl IntoElement for TextElement {
    type Element = Self;

    fn into_element(self) -> Self::Element {
        self
    }
}

impl Element for TextElement {
    type RequestLayoutState = ();
    type PrepaintState = Prepaint;

    fn id(&self) -> Option<ElementId> {
        None
    }

    fn source_location(&self) -> Option<&'static core::panic::Location<'static>> {
        None
    }

    fn request_layout(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        window: &mut Window,
        cx: &mut App,
    ) -> (LayoutId, Self::RequestLayoutState) {
        let mut style = Style::default();
        style.size.width = relative(1.).into();
        style.size.height = window.line_height().into();
        (window.request_layout(style, [], cx), ())
    }

    fn prepaint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        window: &mut Window,
        cx: &mut App,
    ) -> Self::PrepaintState {
        let input = self.input.read(cx);
        let style = window.text_style();
        let (shown, colour) = if input.content.is_empty() {
            (input.placeholder.clone(), Hsla::from(rgba(PLACEHOLDER)))
        } else {
            (SharedString::from(input.content.clone()), style.color)
        };
        let run = TextRun {
            len: shown.len(),
            font: style.font(),
            color: colour,
            background_color: None,
            underline: None,
            strikethrough: None,
        };
        let runs = match input.marked_range.as_ref().filter(|_| !input.content.is_empty()) {
            Some(marked) => [
                TextRun { len: marked.start, ..run.clone() },
                TextRun {
                    len: marked.end - marked.start,
                    underline: Some(UnderlineStyle { color: Some(run.color), thickness: px(1.), wavy: false }),
                    ..run.clone()
                },
                TextRun { len: shown.len() - marked.end, ..run },
            ]
            .into_iter()
            .filter(|run| run.len > 0)
            .collect(),
            None => vec![run],
        };
        let font_size = style.font_size.to_pixels(window.rem_size());
        let line = window.text_system().shape_line(shown, font_size, &runs, None);
        let origin = point(bounds.left() + input.text_offset(&bounds, &line), bounds.top());
        let caret_x = line.x_for_index(if input.content.is_empty() { 0 } else { input.cursor });
        let caret = fill(
            Bounds::new(point((origin.x + caret_x).min(bounds.right() - px(1.)), bounds.top()), size(px(1.), bounds.size.height)),
            style.color,
        );
        let selection = (input.all_selected && !input.content.is_empty())
            .then(|| fill(Bounds::new(origin, size(line.width, bounds.size.height)), rgba(SELECTION)));
        Prepaint { line, origin, caret, selection }
    }

    fn paint(
        &mut self,
        _: Option<&GlobalElementId>,
        _: Option<&InspectorElementId>,
        bounds: Bounds<Pixels>,
        _: &mut Self::RequestLayoutState,
        prepaint: &mut Self::PrepaintState,
        window: &mut Window,
        cx: &mut App,
    ) {
        let focus_handle = self.input.read(cx).focus_handle.clone();
        window.handle_input(&focus_handle, ElementInputHandler::new(bounds, self.input.clone()), cx);
        if let Some(selection) = prepaint.selection.clone().filter(|_| focus_handle.is_focused(window)) {
            window.paint_quad(selection);
        }
        let line = prepaint.line.clone();
        let _ = line.paint(prepaint.origin, window.line_height(), gpui::TextAlign::Left, None, window, cx);
        if focus_handle.is_focused(window) && prepaint.selection.is_none() {
            window.paint_quad(prepaint.caret.clone());
        }
        let line = prepaint.line.clone();
        self.input.update(cx, |input, _| {
            input.last_layout = Some(line);
            input.last_bounds = Some(bounds);
        });
    }
}

/// The text and caret only; the caller draws the box around them, so the
/// field matches the window's other inputs.
impl Render for TextInput {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div()
            .key_context(CONTEXT)
            .track_focus(&self.focus_handle)
            .cursor(CursorStyle::IBeam)
            .on_action(cx.listener(Self::backspace))
            .on_action(cx.listener(Self::delete))
            .on_action(cx.listener(Self::left))
            .on_action(cx.listener(Self::right))
            .on_action(cx.listener(Self::home))
            .on_action(cx.listener(Self::end))
            .on_action(cx.listener(Self::paste))
            .on_action(cx.listener(Self::submit))
            .on_action(cx.listener(Self::cancel))
            .on_mouse_down(MouseButton::Left, cx.listener(Self::on_mouse_down))
            .flex_1()
            .min_w_0()
            .flex()
            .items_center()
            .child(div().w_full().child(TextElement { input: cx.entity() }))
    }
}
