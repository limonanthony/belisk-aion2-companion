//! Settings → To-dos (docs/gpui-mockups/approved/settings.html, tokens.md
//! "Settings → To-dos"): the reset hour, the membership, and the two ways to
//! bring the built-in to-dos back, for every character. The rest of the
//! to-dos lives in the to-do window.

use super::*;

/// How long "6 to-dos back" or "To-dos reset" stays beside its button.
const TODOS_DONE_FOR: Duration = Duration::from_secs(4);

impl SettingsView {
    pub(super) fn todos_page(&mut self, settings: &AppSettings, window: &Window, cx: &mut Context<Self>) -> Div {
        let reset_hour = self.select_control(Select::ResetHour, settings, true, window, cx);
        let weekly = div()
            .debug_selector(|| "weekly-reset".to_string())
            .font_family(DISPLAY_FONT)
            .text_size(px(13.5))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgba(MUTED))
            .child(fl!("settings-todos-weekly-at", time = hour_label(settings.todos.reset_hour).to_string()));
        let resets = group(vec![
            row(fl!("todos-window-daily-reset"), Some(description(fl!("settings-todos-daily-reset-text"))), reset_hour),
            row(
                fl!("settings-todos-weekly-reset"),
                Some(description(fl!("settings-todos-weekly-reset-text"))),
                weekly,
            ),
        ]);
        let membership_focus = self.focus_of("membership", true, window, cx);
        let membership_on = todos::global(cx).read(cx).membership();
        let set_membership: Handler<bool> =
            Rc::new(|on, _, cx| todos::global(cx).update(cx, |todos, cx| todos.set_membership(on, cx)));
        let membership_label = fl!("settings-todos-membership");
        let membership = switch("membership", membership_label.clone(), membership_on, &membership_focus, Some(set_membership));
        let recharges = group(vec![row(membership_label, Some(description(fl!("settings-todos-membership-text"))), membership)]);

        let hidden = self.todos_seen.hidden;
        let restore_focus = self.focus_of("todos-restore", true, window, cx);
        let restore = (hidden.hidden > 0).then(|| self.button_handler(cx, |this, _, cx| this.restore_default_todos(cx)));
        let restore = button(
            "todos-restore",
            fl!("settings-todos-restore"),
            Some(Icon::Reset),
            ButtonKind::Secondary,
            false,
            &restore_focus,
            restore,
        );
        let reset_focus = self.focus_of("todos-reset", true, window, cx);
        let reset = self.button_handler(cx, |this, window, cx| this.open_dialog(Dialog::ResetTodos, window, cx));
        let reset = button(
            "todos-reset",
            fl!("settings-todos-reset-completely"),
            None,
            ButtonKind::Danger,
            false,
            &reset_focus,
            Some(reset),
        );
        let defaults = group(vec![
            row(
                fl!("settings-todos-hidden"),
                Some(description(todos::copy::hidden_description(hidden))),
                self.with_done_badge(|done| matches!(done, TodosDone::Restored(_)), restore, cx),
            ),
            row(
                fl!("settings-todos-start-over"),
                Some(description(fl!("settings-todos-start-over-text"))),
                self.with_done_badge(|done| done == TodosDone::Reset, reset, cx),
            ),
        ]);

        let notices = {
            let todos = todos::global(cx).read(cx);
            let mut notices = Vec::new();
            if let Some(notice) = todos.load_notice() {
                notices.push(alert(fl!("settings-todos-set-aside"), notice.to_string(), Tone::Warn));
            }
            if let Some(error) = todos.save_error() {
                notices.push(alert(fl!("settings-todos-not-saved"), error.to_string(), Tone::Bad));
            }
            notices
        };
        div()
            .flex()
            .flex_col()
            .child(page_head(fl!("settings-section-todos"), Some(fl!("settings-todos-lead").into())))
            .children(notices)
            .child(group_title(fl!("settings-todos-resets"), true))
            .child(resets)
            .child(group_title(fl!("todos-section-recharges"), false))
            .child(recharges)
            .child(group_title(fl!("settings-todos-defaults"), false))
            .child(defaults)
    }

    /// The button, with the good badge saying what it just did before it.
    fn with_done_badge(&self, belongs: impl Fn(TodosDone) -> bool, control: AnyElement, cx: &App) -> Div {
        let shown = self.todos_done.as_ref().map(|(done, _)| *done).filter(|done| belongs(*done)).map(|done| {
            let text = match done {
                TodosDone::Restored(count) => todos::copy::to_dos_back(count),
                TodosDone::Reset => fl!("settings-todos-reset-done"),
            };
            let note = badge(Some(icon(Icon::Check, 12., rgb(widgets::GOOD)).into_any_element()), text.clone(), Tone::Good)
                .id("todos-done")
                .debug_selector(|| "todos-done".to_string())
                .role(Role::Status)
                .aria_label(SharedString::from(text));
            if cx.reduce_motion() {
                note.into_any_element()
            } else {
                note.with_animation("todos-done-fade", Animation::new(CHIP_FADE), |note, t| note.opacity(t)).into_any_element()
            }
        });
        div().flex().items_center().gap(px(10.)).children(shown).child(control)
    }

    fn restore_default_todos(&mut self, cx: &mut Context<Self>) {
        let restored = todos::global(cx).update(cx, |todos, cx| todos.restore_defaults(cx));
        if restored > 0 {
            self.say_todos_done(TodosDone::Restored(restored), cx);
        }
    }

    pub(super) fn say_todos_done(&mut self, done: TodosDone, cx: &mut Context<Self>) {
        let hide = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(TODOS_DONE_FOR).await;
            let _ = this.update(cx, |this, cx| {
                this.todos_done = None;
                cx.notify();
            });
        });
        self.todos_done = Some((done, hide));
        cx.notify();
    }

    #[cfg(any(test, feature = "probes"))]
    pub fn todos_done_text(&self) -> Option<String> {
        self.todos_done.as_ref().map(|(done, _)| match done {
            TodosDone::Restored(count) => todos::copy::to_dos_back(*count),
            TodosDone::Reset => fl!("settings-todos-reset-done"),
        })
    }
}
