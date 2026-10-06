//! The meter overlay as approved in docs/gpui-mockups/approved/meter.html
//! (layout D); values from tokens.md.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use gpui::{
    div, img, prelude::*, px, relative, rgb, rgba, size, svg,
    Animation, AnimationExt as _, AnyElement, App, ClipboardItem, Context, Div, ElementId,
    Entity, EventEmitter, FontFeatures, FontWeight, Global, Hsla, IntoElement, Render, SharedString, Stateful,
    Task, Window, WindowHandle,
};

use super::board::Board;
use super::fight::SummaryKind;
use crate::game_data::PlayerClass;
use super::live::{Gauge, Lead, LiveBoard, PaceLine, PlayerRow};
use super::session_board::SessionBoard;
use super::model::{BossChoice, Footer, MeterModel, PingQuality};
use super::motion::Tween;
use super::settings::{Layout, OverlaySettings};
use super::shades;
use super::summary::{BestLine, FightSummary};
use crate::assets::Icon;
use crate::detail::DetailSource;
use crate::fonts::OVERLAY_FAMILY;
use crate::i18n::fl;
use crate::overlay;
use crate::secondary;
use crate::style::{class_fill, track, Scale};
use crate::todos;

const TEXT: u32 = 0xfff6ecf5;
const SECONDARY: u32 = 0xfff6ecd1;
const MUTED: u32 = 0xfff6eca8;
const FAINT: u32 = 0xfff6ec70;
const INK: u32 = 0x17120d;
const CARD_RGB: u32 = 0x100e0c;
const CARD_BORDER: u32 = 0xffecd61a;
const SOFT: u32 = 0xfff0e114;
const HOT: u32 = 0xfff0e121;
const RANK_BADGE: u32 = 0xfff0e117;
const YOU_BADGE: u32 = 0xfff6ece6;
const YOU_RIM: u32 = 0xfff6ec73;
const GAUGE_EMPTY: u32 = 0xfff0e11f;
const DIVIDER: u32 = 0xfff0e12e;
const SWITCH_TRACK: u32 = 0x0000004d;
const POPOVER: u32 = 0x1e1a16fa;
const POPOVER_BORDER: u32 = 0xffecd624;
const GOOD: u32 = 0x86e3a8;
const WARN: u32 = 0xf2c66d;
const BAD: u32 = 0xf7a8a8;
const HP: u32 = 0xe8735f;
const CHIP_GOOD: u32 = 0x86e3a824;
const CHIP_WARN: u32 = 0xf2c66d21;
const CHIP_BAD: u32 = 0xf7a8a821;
const PB_NEW_FILL: u32 = 0xfacc152b;
const PB_NEW_TEXT: u32 = 0xfde68a;

const CARD_WIDTH: f32 = 310.0;
const CARD_WIDTH_WITH_DAMAGE: f32 = 340.0;
const ROW_HEIGHT: f32 = 22.0;
const ROW_GAP: f32 = 3.0;
const SWITCH_WIDTH: f32 = 100.0;
const SWITCH_INSET: f32 = 2.0;
/// The card's top padding plus the header: where the menu hangs from.
const MENU_TOP: f32 = 30.0;
const MENU_WIDTH: f32 = 240.0;
const COLLAPSE_AFTER: Duration = Duration::from_millis(350);
const MENU_CLOSE_AFTER: Duration = Duration::from_millis(400);
const COPIED_FOR: Duration = Duration::from_millis(1500);
const TIP_DELAY: Duration = Duration::from_millis(500);
const SWAP_FADE: Duration = Duration::from_millis(100);
const SWITCH_SLIDE: Duration = Duration::from_millis(120);
const CHIP_FADE: Duration = Duration::from_millis(150);
const BOARD_FADE: Duration = Duration::from_millis(150);
const EXPAND_FADE: Duration = Duration::from_millis(180);

struct RowMotion {
    slot: Tween,
    bar: Tween,
}

/// The board a card draws.
enum Shown<'a> {
    Session(&'a SessionBoard),
    Target(&'a LiveBoard),
}

impl Shown<'_> {
    fn board(&self) -> Board {
        match self {
            Self::Session(_) => Board::Session,
            Self::Target(_) => Board::Target,
        }
    }

    fn rows(&self) -> &[PlayerRow] {
        match self {
            Self::Session(board) => &board.rows,
            Self::Target(board) => &board.rows,
        }
    }
}

pub struct MeterView {
    model: Entity<MeterModel>,
    figures: FontFeatures,
    locked: bool,
    /// Alt+CapsLock is registered, so a locked overlay can be unlocked.
    lock_shortcut: bool,
    pub(crate) pointer_over: bool,
    /// The capsule is showing the full card.
    expanded: bool,
    expansions: u32,
    collapse: Option<Task<()>>,
    copied: Option<Task<()>>,
    motion: HashMap<u32, RowMotion>,
    boss_hp: Option<Tween>,
    shown_board: Option<Board>,
    board_changes: u32,
    /// The switch's highlight: 0 on Session, 1 on Target.
    switch_slide: Tween,
    switch_hover: Option<Board>,
    /// The switch side whose tooltip is up, once the pointer has rested.
    tip: Option<Board>,
    tip_timer: Option<Task<()>>,
    menu_open: bool,
    menu_close: Option<Task<()>>,
    drag: overlay::WindowDrag,
    /// Dev captures cannot move a real pointer, so they set it here.
    #[cfg(feature = "probes")]
    pub(crate) forced_pointer: Option<bool>,
    /// How many frames this view has built, for the idle-cost check.
    #[cfg(feature = "probes")]
    pub(crate) renders: u64,
    /// Drawn inside the Settings window at this zoom: never under the
    /// pointer, and the window is not fitted to it.
    preview_zoom: Option<f32>,
}

/// The overlay was locked (click-through) or unlocked, by its button, the
/// shortcut or Settings.
pub struct LockChanged(pub bool);

impl EventEmitter<LockChanged> for MeterView {}

struct MeterWindow(WindowHandle<MeterView>);

impl Global for MeterWindow {}

/// Makes `window` the meter's window, for [`window`].
pub fn install_window(window: WindowHandle<MeterView>, cx: &mut App) {
    cx.set_global(MeterWindow(window));
}

/// The meter's window, to read or change the lock from Settings.
pub fn window(cx: &App) -> WindowHandle<MeterView> {
    cx.global::<MeterWindow>().0
}

impl MeterView {
    pub fn new(model: Entity<MeterModel>, cx: &mut Context<Self>) -> Self {
        cx.observe(&model, |_, _, cx| cx.notify()).detach();
        Self {
            model,
            figures: FontFeatures(Arc::new(vec![("tnum".into(), 1)])),
            locked: false,
            lock_shortcut: false,
            pointer_over: false,
            expanded: false,
            expansions: 0,
            collapse: None,
            copied: None,
            motion: HashMap::new(),
            boss_hp: None,
            shown_board: None,
            board_changes: 0,
            switch_slide: Tween::lasting(SWITCH_SLIDE, 0.0, Instant::now()),
            switch_hover: None,
            tip: None,
            tip_timer: None,
            menu_open: false,
            menu_close: None,
            drag: overlay::WindowDrag::default(),
            #[cfg(feature = "probes")]
            forced_pointer: None,
            #[cfg(feature = "probes")]
            renders: 0,
            preview_zoom: None,
        }
    }

    /// The meter as Settings shows it beside the overlay options: the same
    /// view at `zoom`, which takes no pointer and leaves its window's size
    /// alone.
    pub fn preview(model: Entity<MeterModel>, zoom: f32, cx: &mut Context<Self>) -> Self {
        Self { preview_zoom: Some(zoom), ..Self::new(model, cx) }
    }

    pub fn enable_lock_shortcut(&mut self, cx: &mut Context<Self>) {
        self.lock_shortcut = true;
        cx.notify();
    }

    pub fn is_locked(&self) -> bool {
        self.locked
    }

    /// Whether Alt+CapsLock is registered, i.e. whether a locked overlay can
    /// be unlocked again. Without it nothing offers to lock.
    pub fn can_unlock(&self) -> bool {
        self.lock_shortcut
    }

    #[cfg(feature = "probes")]
    pub fn menu_is_open(&self) -> bool {
        self.menu_open
    }

    /// For dev captures: the switch side under the pointer with its tooltip
    /// shown at once, the boss menu, and Copy's check, which captures show
    /// without touching the clipboard.
    #[cfg(feature = "probes")]
    pub fn force_hover(&mut self, switch: Option<Board>, menu_open: bool, copied: bool, cx: &mut Context<Self>) {
        self.switch_hover = switch;
        self.tip = switch;
        self.tip_timer = None;
        self.menu_open = menu_open;
        self.menu_close = None;
        self.copied = copied.then(|| Task::ready(()));
        cx.notify();
    }

    pub fn toggle_lock(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.set_locked(!self.locked, window, cx);
    }

    pub fn set_locked(&mut self, locked: bool, window: &mut Window, cx: &mut Context<Self>) {
        if let Err(error) = overlay::set_click_through(window, locked) {
            eprintln!("{} failed: {error}", if locked { "lock" } else { "unlock" });
            return;
        }
        self.locked = locked;
        cx.emit(LockChanged(locked));
        if locked {
            // Click-through mid-drag: GPUI still holds the capture and would
            // keep moving the window until the release.
            self.drag.stop();
            self.close_menu();
            self.hover_switch_off();
        }
        cx.notify();
    }

    fn pointer_is_over(&self, window: &Window) -> bool {
        if self.preview_zoom.is_some() {
            return false;
        }
        #[cfg(feature = "probes")]
        if let Some(forced) = self.forced_pointer {
            return forced && !self.locked;
        }
        !self.locked && window.is_window_hovered()
    }

    /// GPUI refreshes the window when the pointer enters or leaves it, and the
    /// window is exactly the card, so the change is noticed here.
    fn pointer_moved(&mut self, over: bool, cx: &mut Context<Self>) {
        self.pointer_over = over;
        if over {
            self.collapse = None;
            self.menu_close = None;
            if !self.expanded {
                self.expanded = true;
                self.expansions += 1;
            }
            return;
        }
        self.hover_switch_off();
        self.collapse = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(COLLAPSE_AFTER).await;
            let _ = this.update(cx, |this, cx| {
                this.expanded = false;
                this.collapse = None;
                cx.notify();
            });
        }));
    }

    fn close_menu(&mut self) {
        self.menu_open = false;
        self.menu_close = None;
    }

    /// An open menu with the pointer outside the window closes after 400 ms.
    /// Checked on every frame rather than only on the leave itself, so a
    /// pointer that came and went between two frames still closes it.
    fn arm_menu_close(&mut self, cx: &mut Context<Self>) {
        if !self.menu_open || self.pointer_over || self.menu_close.is_some() {
            return;
        }
        self.menu_close = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(MENU_CLOSE_AFTER).await;
            let _ = this.update(cx, |this, cx| {
                if !this.pointer_over {
                    this.close_menu();
                    cx.notify();
                }
                this.menu_close = None;
            });
        }));
    }

    fn drag_handle(&self, handle: Div) -> Div {
        if self.preview_zoom.is_some() {
            return handle;
        }
        self.drag.handle(handle)
    }

    fn hover_switch_off(&mut self) {
        self.switch_hover = None;
        self.tip = None;
        self.tip_timer = None;
    }

    /// The tooltip waits for the pointer to rest, like GPUI's own tooltips.
    fn hover_switch(&mut self, side: Board, hovered: bool, cx: &mut Context<Self>) {
        if hovered {
            self.switch_hover = Some(side);
            self.tip = None;
            self.tip_timer = Some(cx.spawn(async move |this, cx| {
                cx.background_executor().timer(TIP_DELAY).await;
                let _ = this.update(cx, |this, cx| {
                    this.tip = this.switch_hover;
                    this.tip_timer = None;
                    cx.notify();
                });
            }));
        } else if self.switch_hover == Some(side) {
            self.hover_switch_off();
        }
        cx.notify();
    }

    fn copy_report(&mut self, cx: &mut Context<Self>) {
        let Some(report) = self.model.read(cx).report() else {
            return;
        };
        cx.write_to_clipboard(ClipboardItem::new_string(report));
        self.copied = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(COPIED_FOR).await;
            let _ = this.update(cx, |this, cx| {
                this.copied = None;
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// Under the pointer: reset · capsule · lock · copy · … · close. Waiting,
    /// there is nothing to reset or copy.
    fn tools(&self, model: &MeterModel, has_figures: bool, s: Scale, cx: &Context<Self>) -> Div {
        let capsule_on = model.settings.layout == Layout::Capsule;
        let copied = self.copied.is_some();
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(s.px(2.))
            .mr(s.px(-2.))
            .when(has_figures && model.can_reset(), |row| {
                row.child(tool_button("reset", Icon::Reset, false, s).on_click(cx.listener(
                    |this, _, _, cx| this.model.update(cx, |model, cx| {
                        model.reset();
                        cx.notify();
                    }),
                )))
            })
            .child(
                tool_button("capsule", Icon::Capsule, capsule_on, s).on_click(cx.listener(
                    |_, _, _, cx| crate::settings::toggle_layout(cx),
                )),
            )
            .when(self.lock_shortcut, |row| {
                row.child(
                    tool_button("lock", Icon::LockOpen, false, s)
                        .on_click(cx.listener(|this, _, window, cx| this.set_locked(true, window, cx))),
                )
            })
            .when(has_figures, |row| {
                row.child(
                    tool_button("copy", if copied { Icon::Check } else { Icon::Copy }, copied, s)
                        .on_click(cx.listener(|this, _, _, cx| this.copy_report(cx))),
                )
            })
            .child(tool_button("more", Icon::More, self.menu_open, s).on_click(cx.listener(
                |this, _, _, cx| {
                    if this.menu_open {
                        this.close_menu();
                    } else {
                        this.menu_open = true;
                    }
                    cx.notify();
                },
            )))
            .child(
                tool_button("close", Icon::Close, false, s)
                    .on_click(cx.listener(|_, _, _, cx| cx.quit())),
            )
    }

    /// The name on the left, the time (or a label) on the right; under the
    /// pointer the tools take the time's place and the name keeps the rest.
    fn head(
        &self,
        model: &MeterModel,
        name: SharedString,
        bright: bool,
        at_rest: Div,
        has_figures: bool,
        s: Scale,
        cx: &Context<Self>,
    ) -> Div {
        let right = if self.pointer_over {
            self.tools(model, has_figures, s, cx)
                .with_animation("tools", Animation::new(SWAP_FADE), |tools, t| tools.opacity(t))
                .into_any_element()
        } else {
            at_rest
                .with_animation("at-rest", Animation::new(SWAP_FADE), |right, t| right.opacity(t))
                .into_any_element()
        };
        div()
            .h(s.px(22.))
            .flex()
            .items_center()
            .gap(s.px(6.))
            .pl(s.px(4.))
            .pr(s.px(2.))
            .child(
                self.drag_handle(div().flex_1().min_w_0().h_full().flex().items_center()).child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_color(rgba(if bright { TEXT } else { MUTED }))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(name),
                ),
            )
            .child(right)
    }

    /// The boss gauge drains continuously between snapshots.
    fn drained_hp(&mut self, gauge: Option<&Gauge>, window: &Window, cx: &Context<Self>) -> f32 {
        let Some(gauge) = gauge else {
            self.boss_hp = None;
            return 0.0;
        };
        let now = Instant::now();
        let tween = self.boss_hp.get_or_insert_with(|| Tween::at(gauge.fraction, now));
        tween.retarget(gauge.fraction, now, !cx.reduce_motion());
        if tween.is_running(now) {
            window.request_animation_frame();
        }
        tween.value(now)
    }

    fn copy_button(&self, s: Scale, cx: &Context<Self>) -> Stateful<Div> {
        let copied = self.copied.is_some();
        div()
            .id("copy-text")
            .flex_none()
            .h(s.px(20.))
            .px(s.px(9.))
            .flex()
            .items_center()
            .gap(s.px(5.))
            .rounded_full()
            .bg(rgba(SOFT))
            .hover(|button| button.bg(rgba(HOT)))
            .cursor_pointer()
            .text_color(rgba(TEXT))
            .text_size(s.px(11.5))
            .font_weight(FontWeight::SEMIBOLD)
            .child(icon(if copied { Icon::Check } else { Icon::Copy }, 12., rgba(TEXT), s))
            .child(if copied { fl!("meter-copied") } else { fl!("meter-copy") })
            .on_click(cx.listener(|this, _, _, cx| this.copy_report(cx)))
    }

    /// Session or Target: header, (gauge), rows, the not-identified line, the
    /// footer with the switch, and the switch's tooltip over it all.
    fn board_card(
        &mut self,
        model: &MeterModel,
        shown: Shown,
        s: Scale,
        window: &Window,
        cx: &Context<Self>,
    ) -> Div {
        let board = shown.board();
        if self.shown_board != Some(board) {
            self.shown_board = Some(board);
            self.board_changes += 1;
            self.motion.clear();
            self.boss_hp = None;
        }
        let width = board_width(&model.settings);
        let name = model.header_name().cloned().unwrap_or_else(|| SharedString::new_static("DPS Meter"));
        let time = div()
            .flex_none()
            .text_size(s.px(14.))
            .font_weight(FontWeight::SEMIBOLD)
            .children(model.elapsed().cloned());
        let head = self.head(model, name, board == Board::Target, time, true, s, cx);

        let gauge = match &shown {
            Shown::Target(live) => {
                let drawn = self.drained_hp(live.boss_hp.as_ref(), window, cx);
                Some(gauge_bar(live.boss_hp.as_ref(), drawn, s))
            }
            Shown::Session(_) => None,
        };
        let row_count = shown.rows().len();
        let rows = self.rows(shown.rows(), board, &model.settings, s, window, cx);
        let body = div().flex().flex_col().children(gauge).child(rows);
        let body = if self.board_changes > 1 {
            body.with_animation(
                ElementId::NamedInteger("board".into(), u64::from(self.board_changes)),
                Animation::new(BOARD_FADE),
                |body, t| body.opacity(t),
            )
            .into_any_element()
        } else {
            body.into_any_element()
        };

        let footer = (model.active_board() == board).then(|| model.footer()).flatten();
        let foot = self.board_footer(model, board, footer, s, window, cx);
        let tip = self.tip.and_then(|side| switch_tip(side, board, model.is_board_pinned()));

        card(model.settings.background_alpha, width, s)
            .child(head)
            .child(body)
            .when(!model.identified(), |card| card.child(unidentified_note(s)))
            .child(foot)
            .when_some(tip, |card, (title, line)| card.child(tooltip(title, line, row_count < 2, s)))
    }

    fn board_footer(
        &mut self,
        model: &MeterModel,
        board: Board,
        footer: Option<Footer>,
        s: Scale,
        window: &Window,
        cx: &Context<Self>,
    ) -> Div {
        let left: Vec<AnyElement> = match footer {
            Some(Footer::Session { damage, targets }) => vec![
                label(fl!("common-damage"), s).into_any_element(),
                value(damage, s).into_any_element(),
                div()
                    .flex_none()
                    .ml(s.px(4.))
                    .flex()
                    .items_center()
                    .gap(s.px(3.))
                    .child(icon(Icon::Target, 12., rgba(FAINT), s))
                    .child(targets.to_string())
                    .into_any_element(),
            ],
            Some(Footer::TargetPace(pace)) => {
                vec![pace_figure(&pace, &pace.versus_best, 12., s).into_any_element()]
            }
            Some(Footer::TargetPartyDamage(damage)) => vec![
                label(fl!("common-damage"), s).into_any_element(),
                value(damage, s).into_any_element(),
            ],
            None => Vec::new(),
        };
        div()
            .h(s.px(20.))
            .mt(s.px(5.))
            .pl(s.px(4.))
            .flex()
            .items_center()
            .gap(s.px(4.))
            .text_size(s.px(11.5))
            .text_color(rgba(MUTED))
            .children(left)
            .child(div().flex_1())
            .children(ping(model, s))
            .child(self.switch(model, board, s, window, cx).ml(s.px(6.)))
    }

    /// Session | Target. A click on the other side pins it; a click on the
    /// pinned side hands the choice back. Locked, it shows and does nothing.
    fn switch(
        &mut self,
        model: &MeterModel,
        active: Board,
        s: Scale,
        window: &Window,
        cx: &Context<Self>,
    ) -> Div {
        let now = Instant::now();
        let at = match active {
            Board::Session => 0.0,
            Board::Target => 1.0,
        };
        self.switch_slide.retarget(at, now, !cx.reduce_motion());
        if self.switch_slide.is_running(now) {
            window.request_animation_frame();
        }
        let half = (SWITCH_WIDTH - 2.0 * SWITCH_INSET) / 2.0;
        let pinned = model.is_board_pinned();
        let option = |side: Board, text: String| {
            let on = side == active;
            let hovered = !self.locked && !on && self.switch_hover == Some(side);
            let id = match side {
                Board::Session => "switch-session",
                Board::Target => "switch-target",
            };
            div()
                .id(id)
                .flex_1()
                .min_w_0()
                .h_full()
                .flex()
                .items_center()
                .justify_center()
                .gap(s.px(3.))
                .rounded(s.px(4.))
                .text_size(s.px(11.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgba(if on || hovered { TEXT } else { MUTED }))
                .when(hovered, |option| option.bg(rgba(SOFT)))
                .when(on && pinned, |option| {
                    option.child(div().flex_none().size(s.px(4.)).rounded(s.px(2.)).bg(rgba(TEXT)))
                })
                .child(text)
                .when(!self.locked, |option| {
                    option
                        .cursor_pointer()
                        .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                            this.hover_switch(side, *hovered, cx)
                        }))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.tip = None;
                            this.model.update(cx, |model, cx| {
                                model.select_board(side);
                                cx.notify();
                            });
                        }))
                })
        };
        div()
            .relative()
            .flex_none()
            .w(s.px(SWITCH_WIDTH))
            .h(s.px(18.))
            .p(s.px(SWITCH_INSET))
            .rounded(s.px(6.))
            .bg(rgba(SWITCH_TRACK))
            .flex()
            .child(
                div()
                    .absolute()
                    .top(s.px(SWITCH_INSET))
                    .left(s.px(SWITCH_INSET + half * self.switch_slide.value(now)))
                    .w(s.px(half))
                    .h(s.px(18. - 2.0 * SWITCH_INSET))
                    .rounded(s.px(4.))
                    .bg(rgba(HOT)),
            )
            .child(option(Board::Session, fl!("common-session")))
            .child(option(Board::Target, fl!("common-target")))
    }

    fn rows(
        &mut self,
        rows: &[PlayerRow],
        board: Board,
        settings: &OverlaySettings,
        s: Scale,
        window: &Window,
        cx: &Context<Self>,
    ) -> Div {
        let now = Instant::now();
        let animate = !cx.reduce_motion();
        let count = rows.len() as f32;
        let mut list = div()
            .relative()
            .w_full()
            .h(s.px(count * ROW_HEIGHT + (count - 1.0).max(0.0) * ROW_GAP));
        let mut moving = false;
        for (slot, row) in rows.iter().enumerate() {
            let slot = slot as f32;
            let motion = self.motion.entry(row.actor_id).or_insert_with(|| RowMotion {
                slot: Tween::at(slot, now),
                bar: Tween::at(row.bar, now),
            });
            motion.slot.retarget(slot, now, animate);
            motion.bar.retarget(row.bar, now, animate);
            moving |= motion.slot.is_running(now) || motion.bar.is_running(now);
            let actor_id = row.actor_id;
            list = list.child(
                player_row(row, board, motion.bar.value(now), settings, s)
                    .id(ElementId::NamedInteger("row".into(), u64::from(actor_id)))
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| this.open_detail(actor_id, board, cx)))
                    .absolute()
                    .left_0()
                    .right_0()
                    .top(s.px(motion.slot.value(now) * (ROW_HEIGHT + ROW_GAP))),
            );
        }
        self.motion.retain(|actor, _| rows.iter().any(|row| row.actor_id == *actor));
        if moving {
            window.request_animation_frame();
        }
        list
    }

    /// The Target board's row opens that player's skills on the board's
    /// target; the Session board's opens their totals across every mob.
    fn open_detail(&mut self, actor_id: u32, board: Board, cx: &mut Context<Self>) {
        let source = match board {
            Board::Session => DetailSource::Session { actor_id },
            Board::Target => DetailSource::Target { actor_id, target_id: self.model.read(cx).board_target_id() },
        };
        secondary::open_detail(&self.model, source, cx);
    }

    fn waiting_card(&self, model: &MeterModel, s: Scale, cx: &Context<Self>) -> Div {
        let (title, hint, status) = match model.unavailable() {
            Some(reason) => (fl!("meter-stopped-title"), reason.to_string(), fl!("meter-status-stopped")),
            None => (fl!("meter-waiting-title"), fl!("meter-lock-hint"), fl!("meter-status-standing-by")),
        };
        let head = self.head(model, SharedString::new_static("DPS Meter"), false, label(status, s), false, s, cx);
        card(model.settings.background_alpha, CARD_WIDTH, s)
            .child(head)
            .child(
                div()
                    .pt(s.px(6.))
                    .px(s.px(4.))
                    .pb(s.px(8.))
                    .flex()
                    .flex_col()
                    .gap(s.px(5.))
                    .child(div().text_size(s.px(15.)).font_weight(FontWeight::SEMIBOLD).child(title))
                    .child(div().text_size(s.px(11.5)).text_color(rgba(MUTED)).child(hint)),
            )
            .child(
                footer(s)
                    .mt(s.px(2.))
                    .child(div().flex_1())
                    .children(system_figures(model, s)),
            )
    }

    /// The boss correction for the mob the header names. Drawn in the window
    /// over the card; the window grows downward while it does not fit.
    fn menu(&self, model: &MeterModel, s: Scale, cx: &Context<Self>) -> impl IntoElement {
        let target = model.correction_target();
        let choice = target.as_ref().map(|target| target.choice);
        let boss_line: SharedString = match &target {
            Some(target) => fl!("meter-menu-treat-as-boss", name = target.name.to_string()).into(),
            None => fl!("meter-menu-treat-as-boss-unnamed").into(),
        };
        let item = |id: &'static str, text: SharedString, item_choice: BossChoice| {
            let enabled = target.is_some();
            let checked = choice == Some(item_choice);
            div()
                .id(id)
                .h(s.px(26.))
                .px(s.px(8.))
                .flex()
                .items_center()
                .gap(s.px(8.))
                .rounded(s.px(6.))
                .text_color(rgba(if enabled { TEXT } else { FAINT }))
                .child(
                    div()
                        .flex_none()
                        .size(s.px(12.))
                        .when(checked, |tick| tick.child(icon(Icon::Check, 12., rgba(MUTED), s))),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .font_weight(if checked { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
                        .child(text),
                )
                .when(enabled, |item| {
                    item.cursor_pointer()
                        .hover(|item| item.bg(rgba(HOT)))
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.close_menu();
                            this.model.update(cx, |model, cx| {
                                if model.set_boss_choice(item_choice) {
                                    cx.notify();
                                }
                            });
                            cx.notify();
                        }))
                })
        };
        let link = |id: &'static str, text: String, trailer: Option<String>, open: fn(&Entity<MeterModel>, &mut App)| {
            div()
                .id(id)
                .h(s.px(26.))
                .px(s.px(8.))
                .flex()
                .items_center()
                .gap(s.px(8.))
                .rounded(s.px(6.))
                .text_color(rgba(TEXT))
                .cursor_pointer()
                .hover(|item| item.bg(rgba(HOT)))
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.close_menu();
                    open(&this.model, cx);
                    cx.notify();
                }))
                .child(div().flex_none().size(s.px(12.)))
                .child(div().flex_1().min_w_0().truncate().font_weight(FontWeight::MEDIUM).child(text))
                .children(trailer.map(|trailer| {
                    div().flex_none().text_size(s.px(11.)).text_color(rgba(FAINT)).child(trailer)
                }))
        };
        let full = todos::global(cx).read(cx).full_count();
        div()
            .absolute()
            .top(s.px(MENU_TOP))
            .right(s.px(8.))
            .w(s.px(MENU_WIDTH))
            .occlude()
            .p(s.px(4.))
            .flex()
            .flex_col()
            .rounded(s.px(9.))
            .border_1()
            .border_color(rgba(POPOVER_BORDER))
            .bg(rgba(POPOVER))
            .text_size(s.px(12.5))
            .child(link("menu-history", fl!("meter-menu-fight-history"), None, secondary::open_history))
            .child(link("menu-pvp", fl!("meter-menu-pvp"), None, secondary::open_pvp))
            .child(link("menu-todos", fl!("meter-menu-todos"), (full > 0).then(|| todos::copy::full(full)), secondary::open_todos))
            .child(link("menu-settings", fl!("meter-menu-settings"), None, secondary::open_settings))
            .child(div().h(s.px(1.)).mx(s.px(4.)).my(s.px(4.)).bg(rgba(DIVIDER)))
            .when(target.is_none(), |menu| {
                menu.child(
                    div()
                        .h(s.px(24.))
                        .px(s.px(8.))
                        .flex()
                        .items_center()
                        .text_size(s.px(11.5))
                        .text_color(rgba(MUTED))
                        .child(fl!("meter-menu-hit-a-mob")),
                )
            })
            .child(item("menu-boss", boss_line, BossChoice::Boss))
            .child(item("menu-not-boss", fl!("meter-menu-not-a-boss").into(), BossChoice::NotBoss))
            .child(item("menu-automatic", fl!("meter-menu-automatic").into(), BossChoice::Automatic))
            .with_animation("menu", Animation::new(SWAP_FADE), |menu, t| menu.opacity(t))
    }

    fn summary_card(&self, model: &MeterModel, summary: &FightSummary, s: Scale, cx: &Context<Self>) -> Div {
        let fin = summary.kind == SummaryKind::Final;
        let (chip_fill, chip_text, chip_icon, outcome) = if fin {
            (CHIP_GOOD, GOOD, Icon::Check, fl!("meter-summary-defeated"))
        } else {
            (CHIP_WARN, WARN, Icon::Hourglass, fl!("meter-summary-ended"))
        };
        let chip = div()
            .flex_none()
            .h(s.px(22.))
            .pl(s.px(7.))
            .pr(s.px(9.))
            .flex()
            .items_center()
            .gap(s.px(6.))
            .rounded_full()
            .bg(rgba(chip_fill))
            .text_color(rgb(chip_text))
            .text_size(s.px(12.5))
            .font_weight(FontWeight::SEMIBOLD)
            .child(icon(chip_icon, 13., rgb(chip_text), s))
            .child(outcome)
            .when(!fin, |chip| {
                chip.child(
                    div()
                        .text_color(rgba(MUTED))
                        .text_size(s.px(11.))
                        .font_weight(FontWeight::MEDIUM)
                        .child(fl!("meter-summary-may-resume")),
                )
            })
            .with_animation("outcome", Animation::new(CHIP_FADE), |chip, t| chip.opacity(t));

        let head = div()
            .h(s.px(22.))
            .flex()
            .items_center()
            .gap(s.px(8.))
            .pr(s.px(2.))
            .child(
                self.drag_handle(div().flex_1().h_full().flex().items_center().gap(s.px(8.)))
                    .child(chip)
                    .child(div().flex_1())
                    .child(
                        div()
                            .text_size(s.px(14.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(summary.elapsed.clone()),
                    ),
            )
            .when(!self.locked, |head| {
                head.child(
                    tool_button("dismiss", Icon::Close, false, s)
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.model.update(cx, |model, cx| model.dismiss_summary(cx))
                        })),
                )
            });

        let key_value = |name: String, figure: SharedString| {
            div()
                .w(s.px(104.))
                .flex()
                .items_center()
                .justify_between()
                .gap(s.px(12.))
                .child(
                    div()
                        .text_size(s.px(11.))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(rgba(MUTED))
                        .child(name),
                )
                .child(value(figure, s))
        };
        let hero = div()
            .mt(s.px(9.))
            .flex()
            .items_end()
            .gap(s.px(12.))
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap(s.px(4.))
                    .child(
                        div()
                            .text_size(s.px(36.))
                            .line_height(relative(0.9))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(summary.dps.clone()),
                    )
                    .child(
                        div()
                            .text_size(s.px(12.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgba(FAINT))
                            .child(if summary.personal { fl!("common-dps") } else { fl!("meter-summary-party-dps") }),
                    ),
            )
            .child(
                div()
                    .ml_auto()
                    .flex()
                    .flex_col()
                    .gap(s.px(3.))
                    .when_some(summary.rank.clone(), |list, rank| list.child(key_value(fl!("meter-summary-rank"), rank)))
                    .when_some(summary.share.clone(), |list, share| list.child(key_value(fl!("meter-summary-share"), share)))
                    .when(!summary.personal, |list| {
                        list.child(key_value(fl!("common-players"), summary.players.clone()))
                    })
                    .when_some(summary.crit.clone(), |list, crit| {
                        list.child(key_value(fl!("meter-summary-crit"), crit))
                    }),
            );

        let skills = div()
            .pt(s.px(8.))
            .flex()
            .flex_col()
            .gap(s.px(3.))
            .children(summary.skills.iter().map(|skill| {
                track(class_fill(summary.class.map(PlayerClass::rgb)), skill.bar, 18., s).child(
                    div()
                        .size_full()
                        .flex()
                        .items_center()
                        .gap(s.px(6.))
                        .pl(s.px(9.))
                        .pr(s.px(9.))
                        .text_size(s.px(12.))
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .font_weight(FontWeight::MEDIUM)
                                .child(skill.name.clone()),
                        )
                        .child(value(skill.share.clone(), s)),
                )
            }));

        let body = div()
            .pt(s.px(if fin { 7. } else { 4. }))
            .px(s.px(4.))
            .pb(s.px(2.))
            .flex()
            .flex_col()
            .child(
                div()
                    .truncate()
                    .text_size(s.px(14.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(summary.boss.clone()),
            )
            .when_some(summary.place.clone(), |body, place| {
                body.child(
                    div()
                        .mt(s.px(3.))
                        .text_size(s.px(11.5))
                        .text_color(rgba(MUTED))
                        .child(place),
                )
            })
            .child(hero)
            .when_some(summary.best.as_ref(), |body, best| body.child(best_chip(best, s)))
            .when(!summary.skills.is_empty(), |body| body.child(skills));

        let foot = footer(s)
            .mt(s.px(2.))
            .when_some(summary.party_dps.clone(), |row, party| {
                row.child(label(fl!("meter-summary-party"), s)).child(value(party, s))
            })
            .child(label(fl!("common-damage"), s))
            .child(value(summary.damage.clone(), s))
            .child(div().flex_1())
            .when(!self.locked, |row| row.child(self.copy_button(s, cx)));

        card(model.settings.background_alpha, CARD_WIDTH, s)
            .child(head)
            .when(!fin, |card| {
                let gauge = summary.boss_hp.as_ref();
                card.child(gauge_bar(gauge, gauge.map_or(0.0, |gauge| gauge.fraction), s))
            })
            .child(body)
            .child(foot)
    }

    fn capsule(&self, model: &MeterModel, s: Scale) -> Div {
        let pill = div()
            .flex_none()
            .h(s.px(30.))
            .flex()
            .items_center()
            .gap(s.px(8.))
            .pr(s.px(12.))
            .rounded_full()
            .border_1()
            .border_color(rgba(CARD_BORDER))
            .bg(card_fill(model.settings.background_alpha));
        let session = model.session().filter(|_| model.active_board() == Board::Session || model.live().is_none());
        let divider = || div().w(px(1.)).h(s.px(12.)).bg(rgba(DIVIDER));
        let figure = |text: SharedString| div().text_size(s.px(17.)).font_weight(FontWeight::SEMIBOLD).child(text);
        let clock = |text: SharedString| div().font_weight(FontWeight::SEMIBOLD).text_color(rgba(MUTED)).child(text);
        if let Some(session) = session {
            let lead = match &session.lead {
                Lead::You { rank, colour, damage, .. } => pill
                    .pl(s.px(4.))
                    .child(capsule_badge(rank.clone(), *colour, s))
                    .child(figure(damage.clone()))
                    .child(label(fl!("meter-capsule-damage"), s)),
                Lead::Party { .. } => pill
                    .pl(s.px(12.))
                    .child(figure(session.damage.clone()))
                    .child(label(fl!("meter-capsule-party-damage"), s)),
            };
            return lead
                .child(divider())
                .child(label(fl!("common-session"), s))
                .child(clock(session.elapsed.clone()));
        }
        let Some(live) = model.live() else {
            return pill
                .pl(s.px(12.))
                .child(div().font_weight(FontWeight::SEMIBOLD).child("DPS Meter"))
                .child(label(fl!("meter-status-standing-by"), s));
        };
        let lead = match &live.lead {
            Lead::You { rank, colour, dps, .. } => pill
                .pl(s.px(4.))
                .child(capsule_badge(rank.clone(), *colour, s))
                .child(figure(dps.clone()))
                .child(label(fl!("common-dps"), s)),
            Lead::Party { dps } => pill
                .pl(s.px(12.))
                .child(figure(dps.clone()))
                .child(label(fl!("meter-capsule-party-dps"), s)),
        };
        lead.child(divider())
            .child(clock(live.elapsed.clone()))
            .when_some(live.pace.as_ref(), |pill, pace| {
                pill.child(pace_figure(pace, &pace.figure, 13., s))
            })
    }
}

impl Render for MeterView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        #[cfg(feature = "probes")]
        {
            self.renders += 1;
        }
        let pointer = self.pointer_is_over(window);
        if pointer != self.pointer_over {
            self.pointer_moved(pointer, cx);
        }
        self.arm_menu_close(cx);

        let model_entity = self.model.clone();
        let model = model_entity.read(cx);
        let s = Scale(model.settings.scale() * self.preview_zoom.unwrap_or(1.0));
        let fits_window = self.preview_zoom.is_none();
        let capsule = model.settings.layout == Layout::Capsule;
        let shown = match model.active_board() {
            Board::Session => model.session().map(Shown::Session).or(model.live().map(Shown::Target)),
            Board::Target => model.live().map(Shown::Target).or(model.session().map(Shown::Session)),
        };
        if shown.is_none() {
            self.motion.clear();
            self.boss_hp = None;
            self.shown_board = None;
        }

        let summary = model.summary();
        let collapsed = capsule && !self.expanded && model.unavailable().is_none();
        if summary.is_some() || collapsed {
            self.close_menu();
        }
        let card: AnyElement = if let Some(summary) = summary {
            self.summary_card(model, summary, s, cx).into_any_element()
        } else if collapsed {
            self.capsule(model, s).into_any_element()
        } else {
            let card = match shown {
                Some(shown) => self.board_card(model, shown, s, window, cx),
                None => self.waiting_card(model, s, cx),
            };
            if capsule {
                card.with_animation(
                    ElementId::NamedInteger("expand".into(), u64::from(self.expansions)),
                    Animation::new(EXPAND_FADE),
                    |card, t| card.opacity(t),
                )
                .into_any_element()
            } else {
                card.into_any_element()
            }
        };
        let menu = self.menu_open.then(|| self.menu(model, s, cx));
        // While the menu is open the whole window is one dismiss surface: a
        // press anywhere but the menu closes it and reaches nothing below,
        // the "…" button included (so a second click on it closes too).
        let scrim = self.menu_open.then(|| {
            div()
                .id("menu-scrim")
                .absolute()
                .inset_0()
                .occlude()
                .on_any_mouse_down(cx.listener(|this, _, _, cx| {
                    this.close_menu();
                    cx.stop_propagation();
                    cx.notify();
                }))
        });

        div()
            .size_full()
            .relative()
            .flex()
            .flex_col()
            .items_start()
            .font_family(OVERLAY_FAMILY)
            .font_features(self.figures.clone())
            .line_height(relative(1.))
            .text_size(s.px(13.))
            .text_color(rgba(TEXT))
            .on_children_prepainted(move |bounds, window, cx| {
                if !fits_window {
                    return;
                }
                // The card, and the open menu where it hangs below the card;
                // the scrim only covers whatever the window already is.
                let card = bounds.first();
                let menu = bounds.get(2);
                let wanted = card.into_iter().chain(menu).fold(size(px(0.), px(0.)), |wanted, child| {
                    size(
                        wanted.width.max(child.origin.x + child.size.width),
                        wanted.height.max(child.origin.y + child.size.height),
                    )
                });
                let current = window.viewport_size();
                if (wanted.width - current.width).abs() >= px(0.5)
                    || (wanted.height - current.height).abs() >= px(0.5)
                {
                    overlay::fit_client_size(window, wanted, cx);
                }
            })
            .child(card)
            .children(scrim)
            .children(menu)
            .children(fits_window.then(|| self.drag.listener()))
    }
}

/// The card's width at 100% content scale.
pub fn board_width(settings: &OverlaySettings) -> f32 {
    if settings.show_damage {
        CARD_WIDTH_WITH_DAMAGE
    } else {
        CARD_WIDTH
    }
}

fn card_fill(alpha: u8) -> Hsla {
    rgba((CARD_RGB << 8) | u32::from(alpha)).into()
}

fn card(alpha: u8, width: f32, s: Scale) -> Div {
    div()
        .relative()
        .flex_none()
        .w(s.px(width))
        .flex()
        .flex_col()
        .pt(s.px(8.))
        .px(s.px(8.))
        .pb(s.px(6.))
        .rounded(s.px(14.))
        .border_1()
        .border_color(rgba(CARD_BORDER))
        .bg(card_fill(alpha))
        .overflow_hidden()
}

fn capsule_badge(rank: SharedString, colour: Option<u32>, s: Scale) -> Div {
    let fill: Hsla = match colour {
        Some(colour) => rgb(colour).into(),
        None => rgba(YOU_BADGE).into(),
    };
    div()
        .size(s.px(22.))
        .flex_none()
        .flex()
        .items_center()
        .justify_center()
        .rounded(s.px(5.))
        .bg(fill)
        .text_color(rgb(colour.map_or(INK, shades::ink_on)))
        .text_size(s.px(13.))
        .font_weight(FontWeight::BOLD)
        .child(rank)
}

fn unidentified_note(s: Scale) -> Div {
    div()
        .h(s.px(20.))
        .mt(s.px(4.))
        .px(s.px(4.))
        .flex()
        .items_center()
        .gap(s.px(5.))
        .overflow_hidden()
        .text_size(s.px(11.5))
        .text_color(rgba(MUTED))
        .child(icon(Icon::Map, 12., rgba(FAINT), s))
        .child(fl!("meter-unidentified-hint"))
}

/// What the switch's tooltip says for the side under the pointer; nothing
/// for the active side while the choice is automatic.
fn switch_tip(side: Board, active: Board, pinned: bool) -> Option<(SharedString, SharedString)> {
    match (side == active, side, pinned) {
        (false, Board::Target, _) => {
            Some((fl!("meter-switch-tip-target").into(), fl!("meter-switch-tip-target-line").into()))
        }
        (false, Board::Session, _) => {
            Some((fl!("meter-switch-tip-session").into(), fl!("meter-switch-tip-session-line").into()))
        }
        (true, Board::Session, true) => {
            Some((fl!("meter-switch-tip-pinned-session").into(), fl!("meter-switch-tip-unpin").into()))
        }
        (true, Board::Target, true) => {
            Some((fl!("meter-switch-tip-pinned-target").into(), fl!("meter-switch-tip-unpin").into()))
        }
        (true, _, false) => None,
    }
}

/// Drawn inside the card above the footer; with a single row there is no
/// room above, so it sits beside the switch.
fn tooltip(title: SharedString, line: SharedString, beside: bool, s: Scale) -> impl IntoElement {
    let tip = div()
        .absolute()
        .max_w(s.px(if beside { 180. } else { 230. }))
        .pt(s.px(6.))
        .pb(s.px(7.))
        .px(s.px(9.))
        .rounded(s.px(8.))
        .border_1()
        .border_color(rgba(POPOVER_BORDER))
        .bg(rgba(POPOVER))
        .text_size(s.px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .line_height(relative(1.25))
        .whitespace_normal()
        .child(title)
        .child(
            div()
                .mt(s.px(2.))
                .text_size(s.px(11.5))
                .font_weight(FontWeight::MEDIUM)
                .text_color(rgba(MUTED))
                .child(line),
        );
    let tip = if beside {
        tip.bottom(s.px(4.)).right(s.px(116.))
    } else {
        tip.bottom(s.px(30.)).right(s.px(8.))
    };
    tip.with_animation("switch-tip", Animation::new(CHIP_FADE), |tip, t| tip.opacity(t))
}

fn ping(model: &MeterModel, s: Scale) -> Option<Div> {
    let (ping, quality) = model.system().ping.as_ref()?;
    let color = rgb(match quality {
        PingQuality::Good => GOOD,
        PingQuality::Fair => WARN,
        PingQuality::Poor => BAD,
    });
    Some(
        div()
            .flex_none()
            .flex()
            .items_center()
            .gap(s.px(3.))
            .text_color(color)
            .child(icon(Icon::Signal, 12., color, s))
            .child(ping.clone()),
    )
}


fn footer(s: Scale) -> Div {
    div()
        .h(s.px(20.))
        .px(s.px(4.))
        .flex()
        .items_center()
        .gap(s.px(8.))
        .text_size(s.px(11.5))
        .text_color(rgba(MUTED))
}

fn label(text: impl Into<SharedString>, s: Scale) -> Div {
    div()
        .flex_none()
        .text_size(s.px(11.))
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgba(FAINT))
        .child(text.into())
}

fn value(text: SharedString, s: Scale) -> Div {
    div()
        .flex_none()
        .text_size(s.px(12.5))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(rgba(TEXT))
        .child(text)
}

fn icon(icon: Icon, size: f32, color: impl Into<Hsla>, s: Scale) -> gpui::Svg {
    svg()
        .path(icon.path())
        .flex_none()
        .size(s.px(size))
        .text_color(color.into())
}

fn tool_button(id: &'static str, glyph: Icon, active: bool, s: Scale) -> Stateful<Div> {
    let group: SharedString = SharedString::new_static(id);
    div()
        .id(id)
        .group(group.clone())
        .flex_none()
        .size(s.px(22.))
        .flex()
        .items_center()
        .justify_center()
        .rounded(s.px(7.))
        .when(active, |button| button.bg(rgba(HOT)))
        .hover(|button| button.bg(rgba(HOT)))
        .cursor_pointer()
        .child(
            icon(glyph, 13., rgba(if active { TEXT } else { MUTED }), s)
                .group_hover(group, |svg| svg.text_color(rgba(TEXT))),
        )
}

fn gauge_bar(gauge: Option<&Gauge>, fraction: f32, s: Scale) -> Div {
    let mut line = div()
        .h(s.px(22.))
        .flex()
        .items_center()
        .gap(s.px(3.))
        .px(s.px(4.));
    for segment in 0..10 {
        let fill = (fraction * 10.0 - segment as f32).clamp(0.0, 1.0);
        line = line.child(
            div()
                .flex_1()
                .h(s.px(5.))
                .rounded(s.px(3.))
                .bg(rgba(GAUGE_EMPTY))
                .overflow_hidden()
                .when(fill > 0.0, |segment| {
                    segment.child(div().h_full().w(relative(fill)).rounded(s.px(3.)).bg(rgb(HP)))
                }),
        );
    }
    line.child(
        div()
            .flex_none()
            .w(s.px(32.))
            .ml(s.px(3.))
            .text_right()
            .text_size(s.px(12.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgba(if gauge.is_some() { MUTED } else { FAINT }))
            .child(gauge.map_or(SharedString::new_static("--"), |gauge| gauge.label.clone())),
    )
}

/// Target board: optional damage, then DPS in bold. Session board: DPS quiet
/// with "/s", then damage in bold.
fn player_row(row: &PlayerRow, board: Board, bar: f32, settings: &OverlaySettings, s: Scale) -> Div {
    let name = div()
        .flex_1()
        .min_w_0()
        .flex()
        .items_baseline()
        .gap(s.px(5.))
        .when_some(row.name.clone(), |block, name| {
            block.child(
                div()
                    .min_w_0()
                    .truncate()
                    .font_weight(if row.is_you { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
                    .child(name),
            )
        })
        .when_some(row.detail.clone(), |block, detail| {
            block.child(
                div()
                    .flex_none()
                    .text_size(s.px(11.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(rgba(SECONDARY))
                    .child(detail),
            )
        });
    let quiet = |text: SharedString| {
        div().flex_none().text_size(s.px(11.5)).text_color(rgba(MUTED)).child(text)
    };
    let bold = |text: SharedString| {
        div().flex_none().text_size(s.px(14.)).font_weight(FontWeight::SEMIBOLD).child(text)
    };
    let content = div()
        .size_full()
        .flex()
        .items_center()
        .gap(s.px(6.))
        .pl(s.px(if row.class.is_some() { 4. } else { 9. }))
        .pr(s.px(9.))
        .when_some(row.class, |content, class| {
            content.child(img(class.icon()).flex_none().size(s.px(16.)).rounded(s.px(8.)))
        })
        .child(name);
    let content = match board {
        Board::Target => content
            .when(settings.show_damage, |content| content.child(quiet(row.damage.clone())))
            .when(settings.show_dps, |content| content.child(bold(row.dps.clone()))),
        Board::Session => content
            .when(settings.show_dps, |content| {
                content.child(
                    quiet(row.dps.clone())
                        .flex()
                        .child(div().text_color(rgba(FAINT)).child("/s")),
                )
            })
            .child(bold(row.damage.clone())),
    };
    let track = track(class_fill(row.colour), bar, ROW_HEIGHT, s)
        .child(content)
        .when(row.is_you, |track| {
            track.child(
                div()
                    .absolute()
                    .inset_0()
                    .rounded(s.px(5.))
                    .border_1()
                    .border_color(rgba(YOU_RIM)),
            )
        });
    let (badge_fill, badge_text) = if row.is_you {
        (rgba(YOU_BADGE), rgb(INK))
    } else {
        (rgba(RANK_BADGE), rgba(MUTED))
    };
    div()
        .h(s.px(ROW_HEIGHT))
        .flex()
        .items_center()
        .gap(s.px(5.))
        .child(
            div()
                .flex_none()
                .size(s.px(18.))
                .mr(s.px(3.))
                .flex()
                .items_center()
                .justify_center()
                .rounded(s.px(4.))
                .bg(badge_fill)
                .text_color(badge_text)
                .text_size(s.px(11.5))
                .font_weight(FontWeight::SEMIBOLD)
                .child(row.rank.clone()),
        )
        .child(track)
        .child(
            div()
                .flex_none()
                .w(s.px(38.))
                .text_right()
                .text_size(s.px(12.))
                .text_color(rgba(MUTED))
                .child(row.pct.clone()),
        )
}

fn pace_figure(pace: &PaceLine, text: &SharedString, icon_size: f32, s: Scale) -> Div {
    let color = rgb(if pace.ahead { GOOD } else { BAD });
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(s.px(4.))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(color)
        .child(icon(if pace.ahead { Icon::TrendingUp } else { Icon::TrendingDown }, icon_size, color, s))
        .child(text.clone())
}

fn best_chip(best: &BestLine, s: Scale) -> impl IntoElement {
    let (fill, text, glyph, line, figure): (u32, u32, Icon, SharedString, Option<SharedString>) =
        match best {
            BestLine::First => (PB_NEW_FILL, PB_NEW_TEXT, Icon::Award, fl!("meter-best-first").into(), None),
            BestLine::New { pace } => (
                PB_NEW_FILL,
                PB_NEW_TEXT,
                Icon::Award,
                fl!("meter-best-new").into(),
                Some(pace.clone()),
            ),
            BestLine::Behind { best, pace } => (
                CHIP_BAD,
                BAD,
                Icon::TrendingDown,
                fl!("meter-best-behind", best = best.to_string()).into(),
                Some(pace.clone()),
            ),
        };
    let text_color: Hsla = rgb(text).into();
    div()
        .mt(s.px(9.))
        .h(s.px(24.))
        .px(s.px(10.))
        .flex()
        .items_center()
        .gap(s.px(6.))
        .rounded(s.px(12.))
        .bg(rgba(fill))
        .text_color(text_color)
        .text_size(s.px(12.))
        .font_weight(FontWeight::SEMIBOLD)
        .child(icon(glyph, 13., text_color, s))
        .child(div().flex_1().child(line))
        .children(figure)
        .with_animation("best", Animation::new(CHIP_FADE), |chip, t| chip.opacity(t))
}

fn system_figures(model: &MeterModel, s: Scale) -> Vec<Div> {
    let system = model.system();
    let mut figures = Vec::with_capacity(3);
    if let Some((ping, quality)) = &system.ping {
        let color = rgb(match quality {
            PingQuality::Good => GOOD,
            PingQuality::Fair => WARN,
            PingQuality::Poor => BAD,
        });
        figures.push(
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap(s.px(3.))
                .text_color(color)
                .child(icon(Icon::Signal, 12., color, s))
                .child(ping.clone()),
        );
    }
    figures.extend(system.cpu.clone().map(|cpu| div().flex_none().child(cpu)));
    figures.extend(system.memory.clone().map(|memory| div().flex_none().child(memory)));
    figures
}
