//! The list: section labels, the four kinds of row, the hidden rows listed by
//! Show and the line that lists them. The Resources window draws the
//! Recharges rows alone, without × or Show.

use super::*;
use crate::todos::{AmountView, FigureTip, GroupView, RowView, SectionView};

impl TodoView {
    pub(super) fn list_body(&mut self, list: &TodoList, s: Scale, window: &Window, cx: &mut Context<Self>) -> Div {
        let character = list.character.id;
        let mut content = div().relative().flex().flex_col().p(s.px(3.)).child(self.list_size.marker());
        match self.contents {
            Contents::Todos => {
                for (index, section) in list.sections.iter().enumerate() {
                    content = content.child(self.section(character, section, index == 0, s, window, cx));
                }
                content = content.child(self.add_area(s, window, cx));
            }
            Contents::Resources => {
                let place = HiddenIn::Section(Section::Recharges);
                let mut rows = Vec::new();
                for row in list.sections.iter().filter(|section| section.section == Section::Recharges).flat_map(|section| &section.rows) {
                    self.push_row(&mut rows, character, row, place, None, s, window, cx);
                }
                content = content.child(div().flex().flex_col().gap(s.px(2.)).children(rows));
            }
        }
        let overflows = self.list_size.bounds().is_some_and(|bounds| bounds.size.height > s.px(LIST_MAX));
        // The rows reach 3 px past the card's padding on each side, so a
        // focus ring around them is never clipped by the scrolling area.
        chrome::scroll_list("todo-list", &self.list_scroll, LIST_MAX, overflows, content, s)
            .w(s.px(WIDTH - 10.) - px(2.))
            .mx(s.px(-3.))
            .when(self.contents == Contents::Resources, |list| list.mt(s.px(2.)))
    }

    fn section(&mut self, character: CharacterId, section: &SectionView, first: bool, s: Scale, window: &Window, cx: &mut Context<Self>) -> Div {
        let place = HiddenIn::Section(section.section);
        let mut block = div().flex().flex_col().gap(s.px(2.)).child(self.section_label(section, first, s, window, cx));
        if section.folded {
            return block;
        }
        let mut rows = Vec::new();
        for row in &section.rows {
            self.push_row(&mut rows, character, row, place, None, s, window, cx);
        }
        block = block.children(rows);
        if section.hidden_rows > 0 {
            block = block.child(self.hidden_line(place, section.hidden_rows, false, s, window, cx));
        }
        block
    }

    fn section_label(&mut self, section: &SectionView, first: bool, s: Scale, window: &Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let key = section_key(section.section);
        let focus = self.focus_of(&format!("section:{key}"), true, window, cx);
        let (which, folded) = (section.section, section.folded);
        let group = SharedString::from(format!("section-{key}"));
        let when = section.when_text.clone();
        let time = section.when_time_at.map(|at| at..when.len());
        let when = StyledText::new(when).with_highlights(time.map(|range| {
            (range, HighlightStyle { color: Some(rgba(MUTED).into()), font_weight: Some(FontWeight::SEMIBOLD), ..Default::default() })
        }));
        let label = div()
            .id(element_id("section", key))
            .debug_selector(move || format!("section-{key}"))
            .group(group.clone())
            .role(Role::Button)
            .aria_label(section.title.clone())
            .aria_expanded(!folded)
            .flex_none()
            .h(s.px(24.))
            .when(!first, |label| label.mt(s.px(6.)))
            .pl(s.px(4.))
            .pr(s.px(6.))
            .rounded(s.px(6.))
            .flex()
            .items_center()
            .gap(s.px(6.))
            .cursor_pointer()
            .child(chrome::icon(if folded { Icon::ChevronRight } else { Icon::ChevronDown }, 12., rgba(FAINT), s))
            .child(
                div()
                    .flex_none()
                    .text_size(s.px(12.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgba(MUTED))
                    .group_hover(group, |title| title.text_color(rgba(TEXT)))
                    .child(section.title.clone()),
            )
            .when(!section.count_text.is_empty(), |label| {
                label.child(
                    div()
                        .flex_none()
                        .text_size(s.px(11.))
                        .font_weight(FontWeight::MEDIUM)
                        .when(section.count_is_done, |count| count.text_color(rgb(GOOD)))
                        .when(!section.count_is_done, |count| count.text_color(rgba(FAINT)))
                        .child(section.count_text.clone()),
                )
            })
            .children(section.full_text.clone().map(|text| warn_chip(text, s)))
            .child(div().flex_1())
            .child(div().flex_none().text_size(s.px(11.)).font_weight(FontWeight::MEDIUM).text_color(rgba(FAINT)).child(when))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.todos.update(cx, |todos, cx| todos.set_section_folded(which, !folded, cx));
            }));
        focusable(label, &focus, 6., s)
    }

    /// A row as the window draws it: shown, listed by Show while hidden, or
    /// left out; a group brings its open rows.
    #[allow(clippy::too_many_arguments)]
    fn push_row(
        &mut self,
        out: &mut Vec<AnyElement>,
        character: CharacterId,
        row: &RowView,
        place: HiddenIn,
        group: Option<RowId>,
        s: Scale,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        if row.hidden {
            if self.listing_hidden.contains(&place) {
                if let Some(listed) = self.listed_row(character, row, group, s, window, cx) {
                    out.push(listed);
                }
            }
            return;
        }
        match &row.kind {
            RowKind::Tick { done } => out.push(self.tick_row(character, row, *done, group, s, window, cx)),
            RowKind::Count { count, target, done } => {
                out.push(self.count_row(character, row, (*count, *target), *done, group, s, window, cx))
            }
            RowKind::Amount(amount) => {
                if let Some(element) = self.amount_row(character, row, amount, None, s, window, cx) {
                    out.push(element);
                }
            }
            RowKind::Group(view) => self.group_rows(out, character, row, view, s, window, cx),
        }
    }

    /// The row's frame: at least 26 px, the 12 px gutter first; a group's
    /// rows sit 18 px further in. The pointer entering and leaving it is all
    /// it reports.
    fn shell(&self, id: RowId, in_group: bool, s: Scale, cx: &Context<Self>) -> Stateful<Div> {
        let key = row_key(id);
        let shell = div()
            .id(element_id("row", &key))
            .debug_selector({
                let key = key.clone();
                move || format!("row-{key}")
            })
            .flex_none()
            .min_h(s.px(26.))
            .pl(s.px(if in_group { 22. } else { 4. }))
            .pr(s.px(3.))
            .flex()
            .items_center()
            .gap(s.px(6.))
            .rounded(s.px(6.))
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| this.pointer_on_row(id, *hovered, cx)));
        #[cfg(feature = "probes")]
        let shell = shell.relative().child(self.row_marks.borrow_mut().entry(key).or_default().marker());
        shell
    }

    /// Delete on any control of the row does what its × does; the Resources
    /// window has no ×.
    fn deletable(&self, shell: Stateful<Div>, character: CharacterId, id: RowId, label: String, cx: &Context<Self>) -> Stateful<Div> {
        if self.contents == Contents::Resources {
            return shell;
        }
        shell.on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
            if event.keystroke.key == "delete" && !event.keystroke.modifiers.modified() && this.amount_edit.is_none() {
                cx.stop_propagation();
                this.take_away(character, id, label.clone(), true, window, cx);
            }
        }))
    }

    /// Check and × show while the pointer is in the row, from its enter to
    /// its leave, or while one of its controls has the keyboard.
    fn is_revealed(&self, id: RowId, keys: &[String], window: &Window) -> bool {
        (self.pointer_row == Some(id) && self.held_still.is_none())
            || (window.last_input_was_keyboard() && keys.iter().any(|key| self.is_focused(key, window)))
    }

    /// The 20 px × slot every row of the to-do window keeps, with the button
    /// while revealed; none in the Resources window.
    fn x_slot(&self, revealed: bool, character: CharacterId, row: &RowView, s: Scale, cx: &Context<Self>) -> Option<Div> {
        if self.contents == Contents::Resources {
            return None;
        }
        let slot = div().flex_none().size(s.px(20.));
        if !revealed {
            return Some(slot);
        }
        let (id, label) = (row.id, row.accessible_name());
        let name = if row.yours {
            fl!("todos-window-remove", label = label.as_str())
        } else {
            fl!("todos-window-hide", label = label.as_str())
        };
        let key = row_key(id);
        Some(
            slot.child(
                small_button(&format!("x-{key}"), Icon::Close, s)
                    .role(Role::Button)
                    .aria_label(SharedString::from(name))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        cx.stop_propagation();
                        this.take_away(character, id, label.clone(), false, window, cx);
                    })),
            ),
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn tick_row(&mut self, character: CharacterId, row: &RowView, done: bool, group: Option<RowId>, s: Scale, window: &Window, cx: &mut Context<Self>) -> AnyElement {
        let (id, label) = (row.id, row.accessible_name());
        let key = row_key(id);
        let focus_key = format!("row:{key}");
        let focus = self.focus_of(&focus_key, true, window, cx);
        let revealed = self.is_revealed(id, std::slice::from_ref(&focus_key), window);
        self.drawn.push(Drawn { id, group, focus_key });
        let from_game = is_game_row(&row.source);
        let typed_over = row.source == Source::Typed;
        // A tick the game fills is typed over: the opposite of the game's
        // value, and a second press goes back to it.
        let toggle = {
            let label = label.clone();
            move |this: &mut Self, cx: &mut Context<Self>| {
                let change = match id {
                    RowId::Builtin(builtin) if from_game => {
                        let value = (!typed_over).then_some(u32::from(!done));
                        let changed = this.todos.update(cx, |todos, cx| todos.set_progress(character, builtin, value, cx));
                        changed.then_some(super::super::ProgressChange { was_done: done, is_done: !done })
                    }
                    _ => this.todos.update(cx, |todos, cx| todos.toggle_tick(character, id, cx)),
                };
                this.announce_progress(label.clone(), change);
            }
        };
        let check = revealed.then(|| {
            let (glyph, name, tip) = if done {
                (Icon::Undo, fl!("todos-window-undo-row", label = label.as_str()), fl!("todos-window-undo"))
            } else {
                (Icon::Check, fl!("todos-window-mark-done-row", label = label.as_str()), fl!("todos-window-mark-done"))
            };
            let toggle = toggle.clone();
            let group_name = SharedString::from(format!("check-{key}"));
            div()
                .id(element_id("check", &key))
                .debug_selector({
                    let key = key.clone();
                    move || format!("check-{key}")
                })
                .group(group_name.clone())
                .role(Role::Button)
                .aria_label(SharedString::from(name))
                .tooltip(chrome::tooltip(tip.into(), s))
                .flex_none()
                .size(s.px(20.))
                .rounded(s.px(5.))
                .flex()
                .items_center()
                .justify_center()
                .cursor_pointer()
                .when(!done, |check| check.bg(rgba(SOFT)).hover(|check| check.bg(rgba(HOT))))
                .when(done, |check| check.hover(|check| check.bg(rgba(SOFT))))
                .child(
                    chrome::icon(glyph, 12., rgba(if done { MUTED } else { TEXT }), s)
                        .group_hover(group_name, |glyph| glyph.text_color(rgba(TEXT))),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    toggle(this, cx);
                }))
        });
        let state = match (done, from_game) {
            (true, false) => fl!("todos-window-row-done", label = label.as_str()),
            (false, false) => fl!("todos-window-row-not-done", label = label.as_str()),
            (true, true) => fl!("todos-window-row-done-from-game", label = label.as_str()),
            (false, true) => fl!("todos-window-row-not-done-from-game", label = label.as_str()),
        };
        let shell = self
            .shell(id, group.is_some(), s, cx)
            .aria_label(SharedString::from(state))
            .role(Role::Button)
            .cursor_pointer()
            .when(revealed, |row| row.bg(rgba(SOFT)))
            .on_click(cx.listener(move |this, _, _, cx| toggle(this, cx)));
        let shell = self.deletable(shell, character, id, label, cx);
        let x = self.x_slot(revealed, character, row, s, cx);
        focusable(shell, &focus, 6., s)
            .child(gutter(typed_over.then(|| pencil(s)), s))
            .child(name_block(row, done, false, s))
            .children(check)
            .children(x)
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn count_row(
        &mut self,
        character: CharacterId,
        row: &RowView,
        (count, target): (u32, u32),
        done: bool,
        group: Option<RowId>,
        s: Scale,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let (id, label) = (row.id, row.accessible_name());
        let key = row_key(id);
        let keys = [format!("less:{key}"), format!("fig:{key}"), format!("more:{key}")];
        let pointer_in = self.pointer_row == Some(id);
        let revealed = self.is_revealed(id, &keys, window);
        self.drawn.push(Drawn { id, group, focus_key: keys[1].clone() });
        if is_game_row(&row.source) {
            let RowId::Builtin(builtin) = id else {
                return div().into_any_element();
            };
            let typed_over = row.source == Source::Typed;
            let editing = self.amount_edit.as_ref().is_some_and(|edit| edit.row == builtin);
            let controls = if editing {
                self.amount_field(target, true, &label, s, window, cx).into_any_element()
            } else {
                let focus = self.focus_of(&keys[1], true, window, cx);
                let progress = copy::done_of(count as usize, target as usize);
                let name = if typed_over {
                    fl!("todos-window-figure-typed", label = label.as_str(), value = progress)
                } else {
                    fl!("todos-window-figure-from-game", label = label.as_str(), value = progress)
                };
                let figure = figure_box(&keys[1], s)
                    .role(Role::Button)
                    .aria_label(SharedString::from(name))
                    .cursor_text()
                    .border_1()
                    .border_color(gpui::transparent_black())
                    .hover(|figure| figure.bg(rgba(TRACK)).border_color(rgba(FIGURE_RIM)))
                    .active(|figure| figure.bg(rgba(PRESS)))
                    .child(figure_text(count.to_string(), format!("/{target}"), rgba(if done { MUTED } else { TEXT }), rgba(FAINT), s))
                    .when_some(row.tip.clone(), |figure, tip| figure.tooltip(figure_tooltip(tip, s)))
                    .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                        this.open_amount(builtin, Typing::Progress, Some(count), matches!(event, ClickEvent::Keyboard(_)), window, cx);
                    }));
                game_block(focusable(figure, &focus, 5., s), row.extra, s).into_any_element()
            };
            let shell = self.deletable(self.shell(id, group.is_some(), s, cx), character, id, label, cx);
            let x = self.x_slot(revealed, character, row, s, cx);
            return shell
                .child(gutter(typed_over.then(|| pencil(s)), s))
                .child(name_block(row, done, false, s))
                .child(controls)
                .children(x)
                .into_any_element();
        }
        let adjust = {
            let label = label.clone();
            move |this: &mut Self, delta: i32, cx: &mut Context<Self>| {
                let change = this.todos.update(cx, |todos, cx| todos.adjust_count(character, id, delta, cx));
                this.announce_progress(label.clone(), change);
            }
        };
        let less = {
            let adjust = adjust.clone();
            self.arrow(&keys[0], &keys[1], Icon::ChevronDown, fl!("todos-window-one-less-row", label = label.as_str()), count > 0, count <= 1, pointer_in, s, window, cx, move |this, _, cx| {
                adjust(this, -1, cx)
            })
        };
        let more = {
            let adjust = adjust.clone();
            self.arrow(&keys[2], &keys[1], Icon::ChevronUp, fl!("todos-window-one-more-row", label = label.as_str()), count < target, count + 1 >= target, pointer_in, s, window, cx, move |this, _, cx| {
                adjust(this, 1, cx)
            })
        };
        let focus = self.focus_of(&keys[1], true, window, cx);
        let value_text = if done { fl!("todos-window-count-done", target = target) } else { copy::value_of(count, target) };
        let figure = figure_box(&keys[1], s)
            .role(Role::SpinButton)
            .aria_label(SharedString::from(label.clone()))
            .aria_numeric_value(f64::from(count))
            .aria_min_numeric_value(0.)
            .aria_max_numeric_value(f64::from(target))
            .aria_value(SharedString::from(value_text))
            .hover(|figure| figure.bg(rgba(SOFT)))
            .active(|figure| figure.bg(rgba(PRESS)))
            .child(figure_text(count.to_string(), format!("/{target}"), rgba(if done { MUTED } else { TEXT }), rgba(FAINT), s))
            .on_click({
                let adjust = adjust.clone();
                cx.listener(move |this, _, _, cx| adjust(this, 1, cx))
            })
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                let delta = match event.keystroke.key.as_str() {
                    "up" => 1,
                    "down" => -1,
                    _ => return,
                };
                cx.stop_propagation();
                window.prevent_default();
                adjust(this, delta, cx);
            }));
        let figure = focusable(figure, &focus, 5., s);
        let shell = self.deletable(self.shell(id, group.is_some(), s, cx), character, id, label, cx);
        let x = self.x_slot(revealed, character, row, s, cx);
        shell
            .child(gutter(None, s))
            .child(name_block(row, done, false, s))
            .child(div().flex_none().flex().items_center().gap(s.px(1.)).child(less).child(figure).child(more))
            .children(x)
            .into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn amount_row(
        &mut self,
        character: CharacterId,
        row: &RowView,
        amount: &AmountView,
        group: Option<RowId>,
        s: Scale,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let RowId::Builtin(builtin) = row.id else {
            return None;
        };
        let (id, label) = (row.id, row.accessible_name());
        let keys = [format!("less:{builtin}"), format!("fig:{builtin}"), format!("more:{builtin}")];
        let pointer_in = self.pointer_row == Some(id);
        let revealed = self.is_revealed(id, &keys, window);
        self.drawn.push(Drawn { id, group, focus_key: keys[1].clone() });
        let editing = self.amount_edit.as_ref().is_some_and(|edit| edit.row == builtin);
        let controls: AnyElement = if editing {
            self.amount_field(amount.cap, amount.game_value.is_some(), &label, s, window, cx).into_any_element()
        } else {
            self.amount_controls(character, builtin, row, amount, &keys, pointer_in, s, window, cx)
        };
        let status = (!editing).then(|| amount.status_text.clone()).flatten().map(|status| {
            div()
                .flex_none()
                .text_size(s.px(11.5))
                .when(amount.full, |status| status.text_color(rgb(WARN)).font_weight(FontWeight::SEMIBOLD))
                .when(!amount.full, |status| status.text_color(rgba(MUTED)).font_weight(FontWeight::MEDIUM))
                .child(status)
        });
        let shell = self.deletable(self.shell(id, group.is_some(), s, cx), character, id, label, cx);
        let x = self.x_slot(revealed, character, row, s, cx);
        let typed_over = row.source == Source::Typed && amount.game_value.is_some();
        let mark = if amount.full {
            Some(chrome::icon(Icon::TriangleAlert, 12., rgb(WARN), s).into_any_element())
        } else {
            typed_over.then(|| pencil(s))
        };
        Some(
            shell
                .when(amount.full, |row| row.bg(rgba(FULL_FILL)))
                .child(gutter(mark, s))
                .child(name_block(row, false, !amount.tracked, s))
                .children(status)
                .child(controls)
                .children(x)
                .into_any_element(),
        )
    }

    /// The arrows and the figure, "—" between empty arrow places while the
    /// amount is not tracked, or a value the game gave: its figure alone, a
    /// click on it types over it.
    #[allow(clippy::too_many_arguments)]
    fn amount_controls(
        &mut self,
        character: CharacterId,
        builtin: &'static str,
        row: &RowView,
        amount: &AmountView,
        keys: &[String; 3],
        pointer_in: bool,
        s: Scale,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let label = row.accessible_name();
        let label = label.as_str();
        let focus = self.focus_of(&keys[1], true, window, cx);
        let tracked = amount.tracked.then_some(amount.value);
        let figure = figure_box(&keys[1], s).on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
            this.open_amount(builtin, Typing::Amount, tracked, matches!(event, ClickEvent::Keyboard(_)), window, cx);
        }));
        let (figure_colour, cap_colour): (Hsla, Hsla) =
            if amount.full { (rgb(WARN).into(), rgba(WARN_SOFT).into()) } else { (rgba(TEXT).into(), rgba(FAINT).into()) };
        if amount.game_value.is_some() {
            let value = match row.extra {
                0 => amount.value_text(),
                extra => fl!("todos-window-with-extra", value = amount.value_text(), extra = extra),
            };
            let name = if row.source == Source::Typed {
                fl!("todos-window-figure-typed", label = label, value = value)
            } else {
                fl!("todos-window-figure-from-game", label = label, value = value)
            };
            let figure = figure
                .role(Role::Button)
                .aria_label(SharedString::from(name))
                .cursor_text()
                .border_1()
                .border_color(gpui::transparent_black())
                .hover(|figure| figure.bg(rgba(TRACK)).border_color(rgba(FIGURE_RIM)))
                .active(|figure| figure.bg(rgba(PRESS)))
                .child(figure_text(amount.value.to_string(), format!("/{}", amount.cap), figure_colour, cap_colour, s))
                .when_some(row.tip.clone(), |figure, tip| figure.tooltip(figure_tooltip(tip, s)));
            return game_block(focusable(figure, &focus, 5., s), row.extra, s).into_any_element();
        }
        let figure = figure
            .role(Role::SpinButton)
            .aria_label(SharedString::from(label.to_string()))
            .aria_value(SharedString::from(amount.value_text()));
        if !amount.tracked {
            let figure = figure.child(
                div().text_size(s.px(12.5)).font_weight(FontWeight::MEDIUM).text_color(rgba(FAINT)).child("\u{2014}"),
            );
            return div()
                .flex_none()
                .flex()
                .items_center()
                .gap(s.px(1.))
                .child(div().size(s.px(20.)))
                .child(focusable(figure, &focus, 5., s))
                .child(div().size(s.px(20.)))
                .into_any_element();
        }
        let step = amount.step;
        let (less_name, more_name) = if step == 1 {
            (fl!("todos-window-one-less-row", label = label), fl!("todos-window-one-more-row", label = label))
        } else {
            (fl!("todos-window-step-less-row", step = step, label = label), fl!("todos-window-step-more-row", step = step, label = label))
        };
        let step_by = move |this: &mut Self, direction: i32, cx: &mut Context<Self>| {
            this.todos.update(cx, |todos, cx| todos.step_amount(character, builtin, direction, cx));
        };
        let (value, cap) = (amount.value, amount.cap);
        let less = self.arrow(&keys[0], &keys[1], Icon::ChevronDown, less_name, value > 0, value <= step, pointer_in, s, window, cx, move |this, _, cx| {
            step_by(this, -1, cx)
        });
        let more = self.arrow(&keys[2], &keys[1], Icon::ChevronUp, more_name, value < cap, value + step >= cap, pointer_in, s, window, cx, move |this, _, cx| {
            step_by(this, 1, cx)
        });
        let figure = figure
            .aria_numeric_value(f64::from(value))
            .aria_min_numeric_value(0.)
            .aria_max_numeric_value(f64::from(cap))
            .border_1()
            .border_color(gpui::transparent_black())
            .hover(|figure| figure.bg(rgba(TRACK)).border_color(rgba(FIGURE_RIM)))
            .active(|figure| figure.bg(rgba(PRESS)))
            .child(figure_text(value.to_string(), format!("/{cap}"), figure_colour, cap_colour, s))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                let direction = match event.keystroke.key.as_str() {
                    "up" => 1,
                    "down" => -1,
                    _ => return,
                };
                cx.stop_propagation();
                window.prevent_default();
                step_by(this, direction, cx);
            }));
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(s.px(1.))
            .child(less)
            .child(focusable(figure, &focus, 5., s))
            .child(more)
            .into_any_element()
    }

    /// A 20 px arrow: a tab stop while it can act, dimmed and inert at the
    /// end it moves towards. `last` is a press that brings it to that end;
    /// pressed from the keyboard, the figure (`figure_key`) takes the focus.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn arrow(
        &mut self,
        key: &str,
        figure_key: &str,
        glyph: Icon,
        name: String,
        enabled: bool,
        last: bool,
        bright: bool,
        s: Scale,
        window: &Window,
        cx: &mut Context<Self>,
        act: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> AnyElement {
        let base = div().flex_none().size(s.px(20.)).rounded(s.px(5.)).flex().items_center().justify_center();
        if !enabled {
            return base.opacity(DISABLED_OPACITY).child(chrome::icon(glyph, 12., rgba(FAINT), s)).into_any_element();
        }
        let focus = self.focus_of(key, true, window, cx);
        let group = SharedString::from(format!("arrow-{key}"));
        let figure_key = figure_key.to_string();
        let selector = key.replace(':', "-");
        let element = base
            .id(element_id("arrow", key))
            .debug_selector(move || selector.clone())
            .group(group.clone())
            .role(Role::Button)
            .aria_label(SharedString::from(name))
            .cursor_pointer()
            .hover(|arrow| arrow.bg(rgba(SOFT)))
            .active(|arrow| arrow.bg(rgba(PRESS)))
            .child(
                chrome::icon(glyph, 12., rgba(if bright { TEXT } else { MUTED }), s)
                    .group_hover(group, |glyph| glyph.text_color(rgba(TEXT))),
            )
            .on_click(cx.listener(move |this, event: &ClickEvent, window, cx| {
                act(this, window, cx);
                if last && matches!(event, ClickEvent::Keyboard(_)) {
                    this.focus_control(&figure_key, window, cx);
                }
            }));
        focusable(element, &focus, 5., s).into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn group_rows(
        &mut self,
        out: &mut Vec<AnyElement>,
        character: CharacterId,
        row: &RowView,
        view: &GroupView,
        s: Scale,
        window: &Window,
        cx: &mut Context<Self>,
    ) {
        let RowId::Builtin(builtin) = row.id else {
            return;
        };
        let (id, label) = (row.id, row.accessible_name());
        let focus_key = format!("row:{builtin}");
        let focus = self.focus_of(&focus_key, true, window, cx);
        let revealed = self.is_revealed(id, std::slice::from_ref(&focus_key), window);
        self.drawn.push(Drawn { id, group: None, focus_key });
        let open = view.open;
        let shell = self
            .shell(id, false, s, cx)
            .role(Role::Button)
            .aria_label(SharedString::from(format!("{label}, {}", view.summary)))
            .aria_expanded(open)
            .cursor_pointer()
            .when(revealed, |row| row.bg(rgba(SOFT)))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.todos.update(cx, |todos, cx| todos.set_group_open(builtin, !open, cx));
            }));
        let shell = self.deletable(shell, character, id, label, cx);
        let x = self.x_slot(revealed, character, row, s, cx);
        let chevron = chrome::icon(if open { Icon::ChevronDown } else { Icon::ChevronRight }, 12., rgba(FAINT), s);
        out.push(
            focusable(shell, &focus, 6., s)
                .child(gutter(Some(chevron.into_any_element()), s))
                .child(name_block(row, view.all_done, false, s))
                .child(
                    div()
                        .flex_none()
                        .text_size(s.px(11.5))
                        .font_weight(if view.full_children > 0 { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
                        .text_color(if view.all_done {
                            rgb(GOOD).into()
                        } else if view.full_children > 0 {
                            rgb(WARN).into()
                        } else {
                            gpui::Hsla::from(rgba(MUTED))
                        })
                        .child(view.summary.clone()),
                )
                .children(x)
                .into_any_element(),
        );
        if !open {
            return;
        }
        let place = HiddenIn::Group(builtin);
        for child in &view.children {
            match &child.kind {
                RowKind::Tick { done } if !child.hidden => {
                    out.push(self.tick_row(character, child, *done, Some(id), s, window, cx));
                }
                RowKind::Count { count, target, done } if !child.hidden => {
                    out.push(self.count_row(character, child, (*count, *target), *done, Some(id), s, window, cx));
                }
                RowKind::Amount(amount) if !child.hidden => {
                    out.extend(self.amount_row(character, child, amount, Some(id), s, window, cx));
                }
                _ if child.hidden && self.listing_hidden.contains(&place) => {
                    if let Some(listed) = self.listed_row(character, child, Some(id), s, window, cx) {
                        out.push(listed);
                    }
                }
                _ => {}
            }
        }
        if view.hidden_children > 0 && self.contents == Contents::Todos {
            out.push(self.hidden_line(place, view.hidden_children, true, s, window, cx).into_any_element());
        }
    }

    /// A hidden row listed by Show: faded, with only its figure, and the
    /// put-back button in the × slot.
    fn listed_row(&mut self, character: CharacterId, row: &RowView, group: Option<RowId>, s: Scale, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let RowId::Builtin(builtin) = row.id else {
            return None;
        };
        let label = row.accessible_name();
        let focus_key = format!("put:{builtin}");
        let focus = self.focus_of(&focus_key, true, window, cx);
        self.drawn.push(Drawn { id: row.id, group, focus_key });
        let figure: Option<AnyElement> = match &row.kind {
            RowKind::Tick { .. } => None,
            RowKind::Count { count, target, .. } => Some(
                div().w(s.px(46.)).flex().justify_center().child(figure_text(count.to_string(), format!("/{target}"), rgba(TEXT), rgba(FAINT), s)).into_any_element(),
            ),
            RowKind::Amount(amount) if amount.tracked => Some(
                div()
                    .w(s.px(46.))
                    .flex()
                    .justify_center()
                    .child(figure_text(amount.value.to_string(), format!("/{}", amount.cap), rgba(TEXT), rgba(FAINT), s))
                    .into_any_element(),
            ),
            RowKind::Amount(_) => Some(
                div()
                    .w(s.px(46.))
                    .flex()
                    .justify_center()
                    .text_size(s.px(12.5))
                    .text_color(rgba(FAINT))
                    .child("\u{2014}")
                    .into_any_element(),
            ),
            RowKind::Group(view) => Some(
                div().text_size(s.px(11.5)).text_color(rgba(MUTED)).child(view.summary.clone()).into_any_element(),
            ),
        };
        let put = focusable(
            small_button(&format!("put-{builtin}"), Icon::Eye, s)
                .role(Role::Button)
                .aria_label(SharedString::from(fl!("todos-window-put-back", label = label.as_str())))
                .on_click(cx.listener(move |this, _, window, cx| this.put_back(character, builtin, label.clone(), window, cx))),
            &focus,
            5.,
            s,
        );
        Some(
            self.shell(row.id, group.is_some(), s, cx)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .items_center()
                        .gap(s.px(6.))
                        .opacity(HIDDEN_OPACITY)
                        .child(gutter(None, s))
                        .child(name_block(row, row.is_done(), false, s))
                        .children(figure),
                )
                .child(put)
                .into_any_element(),
        )
    }

    /// "2 hidden · Show", or "· Hide again" while they are listed.
    fn hidden_line(&mut self, place: HiddenIn, count: usize, in_group: bool, s: Scale, window: &Window, cx: &mut Context<Self>) -> Div {
        let key = match place {
            HiddenIn::Section(section) => section_key(section).to_string(),
            HiddenIn::Group(group) => group.to_string(),
        };
        let listing = self.listing_hidden.contains(&place);
        let focus = self.focus_of(&format!("show:{key}"), true, window, cx);
        let (text, name) = if listing {
            (fl!("todos-window-hide-again"), fl!("todos-window-hide-again-name", hidden = copy::hidden(count)))
        } else {
            (fl!("todos-window-show"), fl!("todos-window-show-name", hidden = copy::hidden(count)))
        };
        let toggle = focusable(
            link(&format!("show-{key}"), text, s)
                .role(Role::Button)
                .aria_label(SharedString::from(name))
                .aria_expanded(listing)
                .on_click(cx.listener(move |this, _, _, cx| {
                    if !this.listing_hidden.remove(&place) {
                        this.listing_hidden.insert(place);
                    }
                    cx.notify();
                })),
            &focus,
            4.,
            s,
        );
        div()
            .flex_none()
            .h(s.px(20.))
            .pl(s.px(if in_group { 40. } else { 22. }))
            .flex()
            .items_center()
            .gap(s.px(5.))
            .text_size(s.px(11.5))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgba(FAINT))
            .child(copy::hidden(count))
            .child("\u{b7}")
            .child(toggle)
    }
}

pub(super) fn section_key(section: Section) -> &'static str {
    match section {
        Section::Today => "today",
        Section::Recharges => "recharges",
        Section::ThisWeek => "week",
    }
}

/// The 12 px gutter before a name: a group's chevron, a full amount's
/// warning, a typed-over value's pencil, or nothing.
pub(super) fn gutter(content: Option<AnyElement>, s: Scale) -> Div {
    div().flex_none().w(s.px(12.)).flex().items_center().justify_center().children(content)
}

/// A tick or count the game fills: given by the game, itself or through
/// another character, or typed over the game's value (the player's own
/// counts are `Manual` or `NoneYet`).
fn is_game_row(source: &Source) -> bool {
    matches!(source, Source::GameLive | Source::GameEarlier { .. } | Source::Shared { .. } | Source::Typed)
}

/// The gutter's mark for a value typed over the game's.
fn pencil(s: Scale) -> AnyElement {
    chrome::icon(Icon::Pencil, 12., rgba(FAINT), s).into_any_element()
}

/// A figure the game gave, in the place of a count's arrows and figure: a
/// 14 px gap, the figure, then the extra pool ("+3") in 26 px, so the block
/// keeps the stepper's 88 px and nothing else in the row moves.
fn game_block(figure: impl IntoElement, extra: u32, s: Scale) -> Div {
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(s.px(1.))
        .child(div().flex_none().w(s.px(14.)))
        .child(figure)
        .child(
            div()
                .flex_none()
                .w(s.px(26.))
                .pl(s.px(1.))
                .text_size(s.px(11.5))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgba(MUTED))
                .when(extra > 0, |slot| slot.child(format!("+{extra}"))),
        )
}

fn figure_tooltip(tip: FigureTip, s: Scale) -> impl Fn(&mut Window, &mut App) -> gpui::AnyView + 'static {
    chrome::tooltip_with_detail(tip.text.into(), tip.detail.map(SharedString::from), s)
}

/// The name, its quieter second part and the "Account" or "Yours" tag. Done:
/// `muted` with a line through, "Yours" not struck.
fn name_block(row: &RowView, done: bool, quiet: bool, s: Scale) -> Div {
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .items_baseline()
        .gap(s.px(5.))
        .overflow_hidden()
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_size(s.px(13.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgba(if done || quiet { MUTED } else { TEXT }))
                .when(done, |name| name.line_through())
                .child(row.label.clone()),
        )
        .children(row.sub.clone().map(|sub| {
            div()
                .flex_none()
                .text_size(s.px(11.))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgba(if done { SUB_DONE } else { FAINT }))
                .when(done, |sub| sub.line_through())
                .child(sub)
        }))
        .when(row.account, |name| {
            name.child(
                div()
                    .flex_none()
                    .ml(s.px(1.))
                    .flex()
                    .items_center()
                    .gap(s.px(3.))
                    .text_size(s.px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgba(FAINT))
                    .child(chrome::icon(Icon::Users, 11., rgba(FAINT), s))
                    .child(fl!("todos-window-account")),
            )
        })
        .when(row.yours, |name| {
            name.child(div().flex_none().text_size(s.px(11.)).font_weight(FontWeight::MEDIUM).text_color(rgba(FAINT)).child(fl!("todos-window-yours")))
        })
}

/// The 46 × 20 figure of a count or an amount.
pub(super) fn figure_box(key: &str, s: Scale) -> Stateful<Div> {
    let selector = key.replace(':', "-");
    div()
        .id(element_id("figure", key))
        .debug_selector(move || selector.clone())
        .flex_none()
        .w(s.px(46.))
        .h(s.px(20.))
        .rounded(s.px(5.))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
}

/// "9" at 12.5 px / 600 and "/14" at 11 px, 1 px apart.
pub(super) fn figure_text(value: String, of: String, value_colour: impl Into<Hsla>, of_colour: impl Into<Hsla>, s: Scale) -> Div {
    div()
        .flex()
        .items_baseline()
        .gap(s.px(1.))
        .child(div().text_size(s.px(12.5)).font_weight(FontWeight::SEMIBOLD).text_color(value_colour.into()).child(value))
        .child(div().text_size(s.px(11.)).font_weight(FontWeight::MEDIUM).text_color(of_colour.into()).child(of))
}

/// A 20 × 20 icon button inside a row: `muted`, `soft` and `text` under the
/// pointer.
pub(super) fn small_button(key: &str, glyph: Icon, s: Scale) -> Stateful<Div> {
    let group = SharedString::from(format!("button-{key}"));
    let selector = key.to_string();
    div()
        .id(element_id("button", key))
        .debug_selector(move || selector.clone())
        .group(group.clone())
        .flex_none()
        .size(s.px(20.))
        .rounded(s.px(5.))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .hover(|button| button.bg(rgba(SOFT)))
        .child(chrome::icon(glyph, 12., rgba(MUTED), s).group_hover(group, |glyph| glyph.text_color(rgba(TEXT))))
}
