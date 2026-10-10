//! The list: "Pinned" over the pinned pets, then "All pets" or the family
//! over the rest, each pet as a meter row; or what to do when nothing
//! matches.
//!
//! GPUI's `list` draws the lines ([`chrome::Rows`]): each frame builds only
//! those in view, so a drag or a scroll does not build ~200 rows. Every label
//! and row has a height known before layout, so the thumb and the keyboard's
//! reveal work in exact pixels. A pin with the keyboard keeps its row drawn
//! wherever it is scrolled; the pin Tab reaches is one in view.

use gpui::list;

use super::*;
use crate::pets::badge::level_badge;
use crate::pets::card::{figure, pet_label, pet_name};
use crate::pets::list::{Listing, Row};
use crate::style::{class_fill, track};

const ROW_HEIGHT: f32 = 22.;
const ROW_GAP: f32 = 3.;
const LABEL_HEIGHT: f32 = 24.;
const LABEL_GAP: f32 = 6.;
/// Room around the lines, so a pin's focus ring is never clipped by the list.
const RING_ROOM: f32 = 3.;

/// One line of the list, in the order drawn.
pub(super) enum Line {
    Label { glyph: Icon, title: String, count: usize },
    Pet(Row),
}

/// What the lines are drawn with in this frame.
#[derive(Clone, Copy)]
pub(super) struct LinesFrame {
    s: Scale,
    pin_stop: Option<u32>,
    overflows: bool,
}

impl PetsView {
    pub(super) fn list_body(&mut self, listing: Listing, s: Scale, cx: &mut Context<Self>) -> Stateful<Div> {
        let rest_title = match self.tab {
            Tab::All => fl!("pets-all-pets"),
            Tab::Family(family) => family_name(family),
        };
        let (pinned, rest) = (listing.pinned, listing.rest);
        self.lines.clear();
        if !pinned.is_empty() {
            self.lines.push(Line::Label { glyph: Icon::Pin, title: fl!("pets-pinned"), count: pinned.len() });
            self.lines.extend(pinned.into_iter().map(Line::Pet));
            if !rest.is_empty() {
                self.lines.push(Line::Label { glyph: Icon::PawPrint, title: rest_title, count: rest.len() });
            }
        }
        self.lines.extend(rest.into_iter().map(Line::Pet));
        self.drawn = self.lines.iter().filter_map(Line::pet_id).collect();

        let mut items = Vec::with_capacity(self.lines.len());
        for ix in 0..self.lines.len() {
            let focus = self.lines[ix].pet_id().map(|pet_id| self.pin_handle(pet_id, cx));
            items.push((s.px(line_height(&self.lines, ix)), focus));
        }
        self.rows.set(&items);
        #[cfg(feature = "probes")]
        self.built.borrow_mut().clear();
        if let Some(ix) = self.reveal_pet.take().and_then(|pet_id| self.line_of(pet_id)) {
            self.rows.reveal(ix, s.px(RING_ROOM));
        }
        let in_view = |ix: &usize| self.rows.shows(*ix);
        let pin_stop = self
            .pin_cursor
            .and_then(|pet_id| self.line_of(pet_id))
            .filter(in_view)
            .or_else(|| (0..self.lines.len()).filter(|ix| self.lines[*ix].pet_id().is_some()).find(in_view))
            .and_then(|ix| self.lines[ix].pet_id());
        self.lines_frame = Some(LinesFrame { s, pin_stop, overflows: self.rows.overflows(LIST_MAX, s) });

        let lines = list(self.rows.state.clone(), cx.processor(|this: &mut Self, ix: usize, window, cx| this.line(ix, window, cx)));
        // The lines reach 3 px past the card's padding on each side, so a
        // focus ring around a pin is never clipped by the list.
        chrome::scroll_rows("pet-list", &self.rows, lines, LIST_MAX, s)
            .id("pet-rows")
            .role(Role::List)
            .w(s.px(WIDTH - 10.) - px(2.))
            .mx(s.px(-3.))
    }

    fn line_of(&self, pet_id: u32) -> Option<usize> {
        self.lines.iter().position(|line| line.pet_id() == Some(pet_id))
    }

    /// Line `ix`, as the list asks for it: the gap above it and the ring's
    /// room around the list are inside the line.
    fn line(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let (Some(frame), Some(line)) = (self.lines_frame, self.lines.get(ix)) else {
            return div().into_any_element();
        };
        let s = frame.s;
        let item = div()
            .w_full()
            .flex()
            .flex_col()
            .pl(s.px(RING_ROOM))
            .pr(s.px(RING_ROOM + if frame.overflows { chrome::GUTTER } else { 0. }))
            .when(ix == 0, |item| item.pt(s.px(RING_ROOM)))
            .when(ix + 1 == self.lines.len(), |item| item.pb(s.px(RING_ROOM)));
        let row = match line {
            Line::Label { glyph, title, count } => {
                return item.child(section_label(*glyph, title.clone(), *count, ix == 0, s)).into_any_element();
            }
            Line::Pet(row) => *row,
        };
        let after_row = ix > 0 && self.lines[ix - 1].pet_id().is_some();
        #[cfg(feature = "probes")]
        self.built.borrow_mut().push(row.pet_id);
        let pinned = self.pets.read(cx).is_pinned(row.pet_id);
        item.when(after_row, |item| item.pt(s.px(ROW_GAP)))
            .child(self.row(&row, pinned, frame.pin_stop == Some(row.pet_id), s, window, cx))
            .into_any_element()
    }

    /// The level badge, the track with the souls toward the next level, the
    /// name, "27/75", "→ Lv 3" ("Max" at the cap), then the pin.
    fn row(&mut self, row: &Row, pinned: bool, pin_stop: bool, s: Scale, window: &Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let pet_id = row.pet_id;
        let name = pet_name(pet_id);
        let handle = self.pin_handle(pet_id, cx).tab_stop(pin_stop);
        let focus = Focus { ring: handle.is_focused(window) && window.last_input_was_keyboard(), handle };
        let revealed = pinned || focus.ring || (self.pointer_row == Some(pet_id) && self.held_still.is_none());
        let (souls, needed) = row.souls_of_needed();
        let quiet = row.is_maxed() || row.never_obtained();
        let progress = row.progress();
        let fill = if progress > 0. { progress.max(0.04) } else { 0. };

        let name_text = div()
            .min_w_0()
            .truncate()
            .font_weight(FontWeight::MEDIUM)
            .when(row.never_obtained(), |name| name.text_color(rgba(MUTED)))
            .child(name.clone());
        let name_block = if row.family == Some(PetFamily::Special) {
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .items_baseline()
                .gap(s.px(5.))
                .child(name_text)
                .child(div().flex_none().text_size(s.px(11.)).font_weight(FontWeight::MEDIUM).text_color(rgba(FAINT)).child(fl!("pets-no-stats")))
        } else {
            name_text.flex_1()
        };
        let next = div()
            .flex_none()
            .w(s.px(40.))
            .flex()
            .items_center()
            .justify_end()
            .gap(s.px(3.))
            .text_size(s.px(11.5))
            .font_weight(FontWeight::MEDIUM);
        let next = if row.is_maxed() {
            next.text_color(rgba(FAINT)).child(fl!("pets-max"))
        } else {
            let next_level: u32 = row.state.level + 1;
            next.text_color(rgba(MUTED))
                .child(chrome::icon(Icon::ArrowRight, 10., rgba(FAINT), s))
                .child(fl!("pets-level", level = next_level))
        };
        let content = chrome::bar_content(true, s)
            .child(name_block)
            .child(figure(souls, needed, rgba(if quiet { MUTED } else { TEXT }).into(), s).w(s.px(44.)).justify_end())
            .child(next);

        let (label, tip) = if pinned {
            (fl!("pets-unpin", name = name.as_str()), fl!("pets-unpin-tip"))
        } else {
            (fl!("pets-pin", name = name.as_str()), fl!("pets-pin-tip"))
        };
        let group = SharedString::from(format!("pin-{pet_id}"));
        let pin = div()
            .id(("pin", pet_id as usize))
            .debug_selector(move || format!("pin-{pet_id}"))
            .group(group.clone())
            .role(Role::Button)
            .aria_label(SharedString::from(label))
            .aria_toggled(if pinned { Toggled::True } else { Toggled::False })
            .flex_none()
            .size(s.px(20.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(s.px(5.))
            .cursor_pointer()
            .when(revealed, |pin| {
                pin.hover(|pin| pin.bg(rgba(SOFT))).tooltip(chrome::tooltip(tip.into(), s)).child(
                    chrome::icon(Icon::Pin, 12., rgba(if pinned { TEXT } else { MUTED }), s).group_hover(group, |icon| icon.text_color(rgba(TEXT))),
                )
            })
            .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                cx.stop_propagation();
                if !matches!(event, ClickEvent::Keyboard(_)) {
                    this.toggle_pin(pet_id, false, window, cx);
                }
            }))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| this.on_pin_key(pet_id, event, window, cx)));

        let element = div()
            .id(("pet", pet_id as usize))
            .debug_selector(move || format!("pet-{pet_id}"))
            .role(Role::ListItem)
            .aria_label(SharedString::from(pet_label(pet_id, row.state)))
            .flex_none()
            .h(s.px(22.))
            .flex()
            .items_center()
            .gap(s.px(5.))
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| this.pointer_on_row(pet_id, *hovered, cx)))
            .child(level_badge(row.state.level, Some(30.), 18., s).mr(s.px(3.)))
            .child(track(class_fill(None), fill, 22., s).child(content))
            .child(focusable(pin, &focus, 5., s));
        #[cfg(feature = "probes")]
        let element = element.relative().child(self.row_marks.borrow_mut().entry(pet_id).or_default().marker());
        element
    }

    /// Nothing in the list: what the search or the menu left out, with the
    /// way to every family when a family tab is open.
    pub(super) fn no_match(&mut self, query: &str, s: Scale, window: &Window, cx: &mut Context<Self>) -> Div {
        let query = query.trim();
        if query.is_empty() {
            return empty_title(fl!("pets-shown", count = 0), s);
        }
        let title = fl!("pets-no-match", query = query);
        match self.tab {
            Tab::All => chrome::empty(Some(title.into()), fl!("pets-no-match-all"), s),
            Tab::Family(family) => {
                let focus = self.focus_of("search-all", window, cx);
                let search_all = focusable(
                    link("search-all", fl!("pets-search-all"), s)
                        .role(Role::Button)
                        .aria_label(fl!("pets-search-all"))
                        .on_click(cx.listener(|this, _, _, cx| this.select_tab(Tab::All, cx))),
                    &focus,
                    4.,
                    s,
                );
                empty_title(title, s).child(
                    div()
                        .flex()
                        .flex_wrap()
                        .items_baseline()
                        .gap(s.px(5.))
                        .child(fl!("pets-nothing-in", tab = family_name(family)))
                        .child(search_all.text_size(s.px(12.5))),
                )
            }
        }
    }
}

impl Line {
    fn pet_id(&self) -> Option<u32> {
        match self {
            Self::Label { .. } => None,
            Self::Pet(row) => Some(row.pet_id),
        }
    }
}

/// Line `ix`'s height with the gap above it, and the ring's room above the
/// first and below the last.
fn line_height(lines: &[Line], ix: usize) -> f32 {
    let after_row = ix > 0 && lines[ix - 1].pet_id().is_some();
    let own = match lines[ix] {
        Line::Label { .. } => LABEL_HEIGHT + if ix > 0 { LABEL_GAP } else { 0. },
        Line::Pet(_) => ROW_HEIGHT + if after_row { ROW_GAP } else { 0. },
    };
    let first = if ix == 0 { RING_ROOM } else { 0. };
    let last = if ix + 1 == lines.len() { RING_ROOM } else { 0. };
    own + first + last
}

/// "Pinned", "All pets" or the family over its rows, with their count: the
/// to-do window's section label, not clickable.
fn section_label(glyph: Icon, title: String, count: usize, first: bool, s: Scale) -> Div {
    div()
        .flex_none()
        .h(s.px(24.))
        .when(!first, |label| label.mt(s.px(6.)))
        .pl(s.px(4.))
        .pr(s.px(6.))
        .flex()
        .items_center()
        .gap(s.px(6.))
        .child(chrome::icon(glyph, 12., rgba(FAINT), s))
        .child(div().flex_none().text_size(s.px(12.)).font_weight(FontWeight::SEMIBOLD).text_color(rgba(MUTED)).child(title))
        .child(div().flex_none().text_size(s.px(11.)).font_weight(FontWeight::MEDIUM).text_color(rgba(FAINT)).child(count.to_string()))
}

/// [`chrome::empty`]'s title, with what follows it up to the caller.
fn empty_title(title: String, s: Scale) -> Div {
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
        .child(div().text_size(s.px(14.)).font_weight(FontWeight::SEMIBOLD).text_color(rgba(TEXT)).child(title))
}
