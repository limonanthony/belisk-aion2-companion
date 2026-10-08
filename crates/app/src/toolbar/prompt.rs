//! The first-open prompt (docs/gpui-mockups/approved/prompt.html, values in
//! tokens.md, "Hub"): the first time the meter opens from the toolbar, a card
//! above it asks whether the meter should open at launch and names where the
//! setting lives. Yes turns it on and says so for 4 s; Not now closes at
//! once; either answer is final. Unanswered, the card stays while the meter
//! is open and goes with it. It never takes the keyboard and is never
//! click-through.

use std::cell::Cell;
use std::rc::Rc;
use std::time::Duration;

use gpui::{
    div, prelude::*, rgba, size, AnyElement, Animation, AnimationExt as _, Context, Div, FontWeight, Pixels, Render,
    Role, SharedString, Size, Stateful, Task, Window,
};

use crate::assets::Icon;
use crate::i18n::fl;
use crate::meter::{board_width, Layout, OverlaySettings};
use crate::reminders::GAP;
use crate::secondary::chrome::{self, Look, Scale, FAINT, HOT, MUTED, SOFT, TEXT};
use crate::secondary::Stacking;
use crate::settings;

const CAPSULE_WIDTH: f32 = 220.0;
/// The card before its first frame measures it, at 100%.
const EXPECTED_HEIGHT: f32 = 126.0;
const DONE_FOR: Duration = Duration::from_secs(4);
const FADE: Duration = Duration::from_millis(150);
/// The card follows the meter as it moves.
const TICK: Duration = Duration::from_millis(100);

/// The meter's width, or 220 px over the capsule.
fn card_width(overlay: &OverlaySettings) -> f32 {
    if overlay.layout == Layout::Capsule {
        CAPSULE_WIDTH
    } else {
        board_width(overlay)
    }
}

/// The window's size before its first frame, in logical pixels.
pub fn prompt_size(overlay: &OverlaySettings) -> Size<Pixels> {
    let scale = overlay.scale();
    size(gpui::px(card_width(overlay) * scale), gpui::px(EXPECTED_HEIGHT * scale))
}

/// "Not now" (quiet) or "Yes, at launch" (tonal): 22 px pills; under the
/// pointer both take the hot fill and the text colour. A press takes neither
/// the keyboard nor a drag.
fn answer_button(id: &'static str, label: String, tonal: bool, s: Scale) -> Stateful<Div> {
    div()
        .id(id)
        .debug_selector(move || id.to_string())
        .role(Role::Button)
        .aria_label(SharedString::from(label.clone()))
        .flex_none()
        .h(s.px(22.))
        .px(s.px(10.))
        .flex()
        .items_center()
        .rounded_full()
        .when(tonal, |button| button.bg(rgba(SOFT)))
        .text_color(rgba(if tonal { TEXT } else { MUTED }))
        .text_size(s.px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .cursor_pointer()
        .hover(|button| button.bg(rgba(HOT)).text_color(rgba(TEXT)))
        .on_mouse_down(gpui::MouseButton::Left, |_, window, _| window.prevent_default())
        .child(label)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Answer {
    Waiting,
    /// Yes: the good note shows, then the card fades out.
    Done,
    Leaving,
}

pub struct PromptView {
    answer: Answer,
    /// None in tests, where there is no meter to sit on.
    stacking: Option<Rc<Stacking>>,
    card_size: Rc<Cell<Option<Size<Pixels>>>>,
    _ticker: Option<Task<()>>,
    _leaving: Option<Task<()>>,
}

impl PromptView {
    pub fn new(stacking: Rc<Stacking>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let ticker = cx.spawn_in(window, async move |this, cx| loop {
            cx.background_executor().timer(TICK).await;
            if this.update_in(cx, |this, window, cx| this.keep_in_place(window, cx)).is_err() {
                break;
            }
        });
        Self::built(Some(stacking), Some(ticker), cx)
    }

    #[cfg(test)]
    pub fn unplaced(cx: &mut Context<Self>) -> Self {
        Self::built(None, None, cx)
    }

    fn built(stacking: Option<Rc<Stacking>>, ticker: Option<Task<()>>, cx: &mut Context<Self>) -> Self {
        let store = settings::global(cx);
        cx.observe(&store, |_, _, cx| cx.notify()).detach();
        Self { answer: Answer::Waiting, stacking, card_size: Rc::default(), _ticker: ticker, _leaving: None }
    }

    fn keep_in_place(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let (Some(stacking), Some(card)) = (&self.stacking, self.card_size.get()) else {
            return;
        };
        let scale = settings::global(cx).read(cx).get().overlay.scale();
        stacking.place(card, GAP * scale, window, cx);
    }

    fn answer(&mut self, yes: bool, window: &mut Window, cx: &mut Context<Self>) {
        if self.answer != Answer::Waiting {
            return;
        }
        settings::global(cx).update(cx, |settings, cx| {
            settings.update(cx, |all| {
                all.windows.launch_prompt_answered = true;
                if yes {
                    all.windows.open_meter_at_launch = true;
                }
            })
        });
        if !yes {
            window.remove_window();
            return;
        }
        self.answer = Answer::Done;
        self._leaving = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(DONE_FOR).await;
            let reduce_motion = this
                .update(cx, |this, cx| {
                    this.answer = Answer::Leaving;
                    cx.notify();
                    cx.reduce_motion()
                })
                .unwrap_or(true);
            if !reduce_motion {
                cx.background_executor().timer(FADE).await;
            }
            let _ = this.update_in(cx, |_, window, _| window.remove_window());
        }));
        cx.notify();
    }

    #[cfg(test)]
    pub fn is_done(&self) -> bool {
        self.answer != Answer::Waiting
    }

    fn path(s: Scale) -> impl IntoElement {
        let steps = [fl!("toolbar-settings"), fl!("settings-section-overlay"), fl!("settings-overlay-open-meter")];
        let last = steps.len() - 1;
        let mut path = div()
            .mt(s.px(6.))
            .pl(s.px(23.))
            .pr(s.px(4.))
            .flex()
            .flex_wrap()
            .items_center()
            .gap(s.px(4.))
            .text_size(s.px(11.5))
            .font_weight(FontWeight::MEDIUM)
            .line_height(s.px(15.))
            .text_color(rgba(TEXT))
            .child(chrome::icon(Icon::Settings, 12., rgba(FAINT), s).mr(s.px(1.)));
        for (index, step) in steps.into_iter().enumerate() {
            if index > 0 {
                path = path.child(chrome::icon(Icon::ChevronRight, 10., rgba(FAINT), s));
            }
            path = path.child(div().when(index == last, |step| step.font_weight(FontWeight::SEMIBOLD)).child(step));
        }
        path
    }
}

impl Render for PromptView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let overlay = settings::global(cx).read(cx).get().overlay.clone();
        let look = Look::of(&overlay);
        let s = look.s;
        let title: SharedString = fl!("prompt-title").into();
        let head = div()
            .flex_none()
            .h(s.px(22.))
            .pl(s.px(4.))
            .flex()
            .items_center()
            .gap(s.px(6.))
            .child(chrome::icon(Icon::Swords, 13., rgba(MUTED), s))
            .child(div().min_w_0().truncate().text_size(s.px(12.5)).font_weight(FontWeight::SEMIBOLD).child(title.clone()));
        let body: Vec<AnyElement> = if self.answer == Answer::Waiting {
            let no = answer_button("prompt-no", fl!("prompt-no"), false, s)
                .on_click(cx.listener(|this, _, window, cx| this.answer(false, window, cx)));
            let yes = answer_button("prompt-yes", fl!("prompt-yes"), true, s)
                .on_click(cx.listener(|this, _, window, cx| this.answer(true, window, cx)));
            vec![
                div()
                    .mt(s.px(3.))
                    .pl(s.px(23.))
                    .pr(s.px(4.))
                    .text_size(s.px(11.5))
                    .font_weight(FontWeight::MEDIUM)
                    .line_height(s.px(15.))
                    .text_color(rgba(MUTED))
                    .child(fl!("prompt-body"))
                    .into_any_element(),
                Self::path(s).into_any_element(),
                div().mt(s.px(9.)).flex().justify_end().gap(s.px(6.)).child(no).child(yes).into_any_element(),
            ]
        } else {
            vec![
                chrome::note(Icon::Check, fl!("prompt-done"), true, s)
                    .mt(s.px(6.))
                    .id("prompt-done")
                    .role(Role::Status)
                    .debug_selector(|| "prompt-done".to_string())
                    .into_any_element(),
                Self::path(s).into_any_element(),
            ]
        };
        let card = chrome::frame(look, 14.)
            .flex_none()
            .w(s.px(card_width(&overlay)))
            .p(s.px(8.))
            .flex()
            .flex_col()
            .id("prompt")
            .debug_selector(|| "prompt".to_string())
            .role(Role::Dialog)
            .aria_label(title)
            .child(head)
            .children(body);
        let card: AnyElement = if self.answer == Answer::Leaving {
            card.with_animation("prompt-out", Animation::new(FADE), |card, t| card.opacity(1. - t)).into_any_element()
        } else {
            card.into_any_element()
        };
        let card_size = self.card_size.clone();
        let stacking = self.stacking.clone();
        let gap = GAP * overlay.scale();
        chrome::surface(look)
            .on_children_prepainted(move |bounds, window, cx| {
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
