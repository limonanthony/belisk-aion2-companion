//! The two places the window takes typing: the "+ Add a to-do" form at the
//! end of the list, and the field an amount is typed in.

use std::rc::Rc;

use gpui::{Focusable as _, MouseButton};

use super::rows::{gutter, small_button};
use super::*;
use crate::todos::{AmountView, OWN_COUNT_RANGE};

impl TodoView {
    /// "+ Add a to-do", or the form in its place.
    pub(super) fn add_area(&mut self, s: Scale, window: &Window, cx: &mut Context<Self>) -> Stateful<Div> {
        if self.adding.is_some() {
            return self.add_form(s, window, cx);
        }
        let focus = self.focus_of("add-line", true, window, cx);
        let line = div()
            .id("add-line")
            .debug_selector(|| "add-line".to_string())
            .group("add-line")
            .role(Role::Button)
            .aria_label(fl!("todos-window-add"))
            .flex_none()
            .mt(s.px(6.))
            .h(s.px(26.))
            .pl(s.px(4.))
            .pr(s.px(3.))
            .rounded(s.px(6.))
            .flex()
            .items_center()
            .gap(s.px(6.))
            .cursor_pointer()
            .text_size(s.px(12.5))
            .font_weight(FontWeight::MEDIUM)
            .text_color(rgba(MUTED))
            .hover(|line| line.bg(rgba(SOFT)).text_color(rgba(TEXT)))
            .child(gutter(
                Some(
                    chrome::icon(Icon::Plus, 12., rgba(FAINT), s)
                        .group_hover("add-line", |glyph| glyph.text_color(rgba(TEXT)))
                        .into_any_element(),
                ),
                s,
            ))
            .child(fl!("todos-window-add"))
            .on_click(cx.listener(|this, _, window, cx| this.open_form(window, cx)));
        focusable(line, &focus, 6., s)
    }

    fn add_form(&mut self, s: Scale, window: &Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let (counted, count, cadence) = match &self.adding {
            Some(form) => (form.counted, form.count, form.cadence),
            None => (false, 3, Cadence::Daily),
        };
        let typed = self.name_input.read(cx).text().to_string();
        let blank = typed.trim().is_empty();
        let name_focused = self.name_input.read(cx).focus_handle(cx).is_focused(window);
        let name = chrome::input_box(name_focused, s)
            .flex_1()
            .min_w_0()
            .id("add-name")
            .debug_selector(|| "add-name".to_string())
            .role(Role::TextInput)
            .aria_label(fl!("todos-window-new"))
            .aria_placeholder(fl!("todos-window-name"))
            .aria_value(SharedString::from(typed))
            .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.take_keyboard(window, cx)))
            .child(self.name_input.clone());

        let add = div()
            .id("add-submit")
            .debug_selector(|| "add-submit".to_string())
            .role(Role::Button)
            .aria_label(fl!("todos-window-add-button"))
            .flex_none()
            .h(s.px(22.))
            .px(s.px(10.))
            .flex()
            .items_center()
            .gap(s.px(5.))
            .rounded_full()
            .bg(rgba(SOFT))
            .text_color(rgba(TEXT))
            .text_size(s.px(12.))
            .font_weight(FontWeight::SEMIBOLD)
            .child(chrome::icon(Icon::Plus, 11., rgba(TEXT), s))
            .child(fl!("todos-window-add-button"));
        let add = if blank {
            add.opacity(0.4).into_any_element()
        } else {
            let focus = self.focus_of("add-submit", true, window, cx);
            focusable(
                add.cursor_pointer()
                    .hover(|add| add.bg(rgba(HOT)))
                    .on_click(cx.listener(|this, _, window, cx| this.submit_form(window, cx))),
                &focus,
                11.,
                s,
            )
            .into_any_element()
        };
        let cancel_focus = self.focus_of("add-cancel", true, window, cx);
        let cancel = focusable(
            small_button("add-cancel", Icon::Close, s)
                .role(Role::Button)
                .aria_label(fl!("common-cancel"))
                .on_click(cx.listener(|this, _, window, cx| this.close_form(true, window, cx))),
            &cancel_focus,
            5.,
            s,
        );

        let kind = self.segmented(
            "add-kind",
            fl!("todos-window-kind"),
            [fl!("todos-window-tick"), fl!("todos-window-count")],
            usize::from(counted),
            s,
            window,
            cx,
            |this, index, cx| {
                if let Some(form) = this.adding.as_mut() {
                    form.counted = index == 1;
                }
                cx.notify();
            },
        );
        let resets = self.segmented(
            "add-cadence",
            fl!("todos-window-resets"),
            [fl!("todos-window-daily"), fl!("todos-window-weekly")],
            usize::from(cadence == Cadence::Weekly),
            s,
            window,
            cx,
            |this, index, cx| {
                if let Some(form) = this.adding.as_mut() {
                    form.cadence = if index == 0 { Cadence::Daily } else { Cadence::Weekly };
                }
                cx.notify();
            },
        );
        let how_many = counted.then(|| self.stepper(count, s, window, cx));

        div()
            .id("add-form")
            .debug_selector(|| "add-form".to_string())
            .role(Role::Group)
            .aria_label(fl!("todos-window-add"))
            .track_focus(&self.form_group)
            .relative()
            .child(self.form_mark.marker())
            .flex_none()
            .mt(s.px(6.))
            .p(s.px(6.))
            .rounded(s.px(6.))
            .bg(rgba(FORM_PLATE))
            .flex()
            .flex_col()
            .gap(s.px(6.))
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                if event.keystroke.key == "escape" {
                    cx.stop_propagation();
                    this.close_form(true, window, cx);
                }
            }))
            .child(div().flex().items_center().gap(s.px(6.)).child(name).child(add).child(cancel))
            .child(
                div()
                    .flex()
                    .items_center()
                    .gap(s.px(6.))
                    .child(kind)
                    .children(how_many)
                    .child(div().flex_1())
                    .child(resets),
            )
    }

    /// "of" and the count's arrows around its figure, 2 to 99.
    fn stepper(&mut self, count: u32, s: Scale, window: &Window, cx: &mut Context<Self>) -> Div {
        let (min, max) = (*OWN_COUNT_RANGE.start(), *OWN_COUNT_RANGE.end());
        let set = move |this: &mut Self, delta: i32, cx: &mut Context<Self>| {
            if let Some(form) = this.adding.as_mut() {
                form.count = form.count.saturating_add_signed(delta).clamp(min, max);
            }
            cx.notify();
        };
        let less = self.arrow("add-count-less", "add-count", Icon::ChevronDown, fl!("todos-window-one-less"), count > min, count <= min + 1, true, s, window, cx, move |this, _, cx| {
            set(this, -1, cx)
        });
        let more = self.arrow("add-count-more", "add-count", Icon::ChevronUp, fl!("todos-window-one-more"), count < max, count + 1 >= max, true, s, window, cx, move |this, _, cx| {
            set(this, 1, cx)
        });
        let focus = self.focus_of("add-count", true, window, cx);
        let figure = div()
            .id("add-count")
            .debug_selector(|| "add-count".to_string())
            .role(Role::SpinButton)
            .aria_label(fl!("todos-window-how-many"))
            .aria_numeric_value(f64::from(count))
            .aria_min_numeric_value(f64::from(min))
            .aria_max_numeric_value(f64::from(max))
            .flex_none()
            .w(s.px(26.))
            .h(s.px(20.))
            .flex()
            .items_center()
            .justify_center()
            .text_size(s.px(12.5))
            .font_weight(FontWeight::SEMIBOLD)
            .child(count.to_string())
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, window, cx| {
                let delta = match event.keystroke.key.as_str() {
                    "up" => 1,
                    "down" => -1,
                    _ => return,
                };
                cx.stop_propagation();
                window.prevent_default();
                set(this, delta, cx);
            }));
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(s.px(1.))
            .child(div().mr(s.px(4.)).text_size(s.px(11.5)).text_color(rgba(MUTED)).child(fl!("todos-window-of")))
            .child(less)
            .child(focusable(figure, &focus, 5., s))
            .child(more)
    }

    /// Two pill options in a dark track. One tab stop; the arrow keys, Home
    /// and End move the choice, as in a radio group.
    #[allow(clippy::too_many_arguments)]
    fn segmented(
        &mut self,
        key: &'static str,
        label: String,
        options: [String; 2],
        selected: usize,
        s: Scale,
        window: &Window,
        cx: &mut Context<Self>,
        on_pick: impl Fn(&mut Self, usize, &mut Context<Self>) + 'static,
    ) -> Stateful<Div> {
        let focus = self.focus_of(key, true, window, cx);
        let on_pick = Rc::new(on_pick);
        let on_key = on_pick.clone();
        let last = options.len() - 1;
        let element = div()
            .id(key)
            .debug_selector(move || key.to_string())
            .role(Role::RadioGroup)
            .aria_label(label)
            .aria_value(options[selected].clone())
            .flex_none()
            .flex()
            .gap(s.px(2.))
            .p(s.px(2.))
            .rounded(s.px(7.))
            .bg(rgba(SEGMENT_TRACK))
            .on_key_down(cx.listener(move |this, event: &KeyDownEvent, _, cx| {
                if event.keystroke.modifiers.modified() {
                    return;
                }
                let next = match event.keystroke.key.as_str() {
                    "left" | "up" => selected.saturating_sub(1),
                    "right" | "down" => (selected + 1).min(last),
                    "home" => 0,
                    "end" => last,
                    _ => return,
                };
                cx.stop_propagation();
                if next != selected {
                    on_key(this, next, cx);
                }
            }))
            .children(options.into_iter().enumerate().map(|(index, option)| {
                let on = index == selected;
                let on_pick = on_pick.clone();
                div()
                    .id(index)
                    .debug_selector(move || format!("{key}-{index}"))
                    .role(Role::RadioButton)
                    .aria_label(option.clone())
                    .aria_toggled(if on { gpui::Toggled::True } else { gpui::Toggled::False })
                    .h(s.px(20.))
                    .px(s.px(8.))
                    .flex()
                    .items_center()
                    .rounded(s.px(5.))
                    .text_size(s.px(11.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .cursor_pointer()
                    .when(on, |option| option.bg(rgba(HOT)).text_color(rgba(TEXT)))
                    .when(!on, |option| option.text_color(rgba(MUTED)).hover(|option| option.bg(rgba(SOFT)).text_color(rgba(TEXT))))
                    .on_click(cx.listener(move |this, _, _, cx| {
                        if !on {
                            on_pick(this, index, cx);
                        }
                    }))
                    .child(option)
            }));
        focusable(element, &focus, 7., s)
    }

    /// The field an amount is typed in, in place of the arrows and figure:
    /// the figure on the right, "/cap", and the button that stops tracking.
    pub(super) fn amount_field(&mut self, amount: &AmountView, label: &str, s: Scale, window: &Window, cx: &mut Context<Self>) -> Stateful<Div> {
        let typed = self.amount_input.read(cx).text().to_string();
        let clear_focus = self.focus_of("amount-clear", true, window, cx);
        let clear = focusable(
            small_button("amount-clear", Icon::Close, s)
                .size(s.px(18.))
                .rounded(s.px(4.))
                .role(Role::Button)
                .aria_label(SharedString::from(fl!("todos-window-stop-tracking", label = label)))
                .on_click(cx.listener(|this, _, window, cx| this.stop_tracking(window, cx))),
            &clear_focus,
            4.,
            s,
        );
        div()
            .id("amount-field")
            .debug_selector(|| "amount-field".to_string())
            .track_focus(&self.amount_group)
            .relative()
            .child(self.amount_mark.marker())
            .flex_none()
            .w(s.px(112.))
            .h(s.px(22.))
            .pl(s.px(7.))
            .pr(s.px(2.))
            .flex()
            .items_center()
            .gap(s.px(2.))
            .rounded(s.px(6.))
            .bg(rgba(TRACK))
            .border_1()
            .border_color(rgba(FIELD_BORDER))
            .child(
                div()
                    .id("amount-input")
                    .role(Role::TextInput)
                    .aria_label(SharedString::from(fl!("todos-window-amount-input", label = label)))
                    .aria_value(SharedString::from(typed))
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .text_size(s.px(12.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .on_mouse_down(MouseButton::Left, cx.listener(|this, _, window, cx| this.take_keyboard(window, cx)))
                    .child(self.amount_input.clone()),
            )
            .child(div().flex_none().text_size(s.px(11.)).text_color(rgba(FAINT)).child(format!("/{}", amount.cap)))
            .child(clear)
    }
}
