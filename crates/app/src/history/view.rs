//! The fight history as approved in docs/gpui-mockups/approved/history.html:
//! newest records first, a dropdown for each filter, one record open at a
//! time, deleting confirmed in the button itself, and boss records marked
//! with a crown and a button for their fight report.

use std::collections::HashMap;
use std::time::Duration;

use gpui::{
    div, list, prelude::*, px, rgb, rgba, Animation, AnimationExt as _, AnyElement, App, Context, Entity, EventEmitter,
    FocusHandle, FontWeight, IntoElement, KeyDownEvent, Pixels, Render, Role, ScrollHandle, SharedString, Subscription,
    Task, Window,
};

use super::list::{unknown_player, FilterOption, PlayerLine, RecordRow};
use super::model::HistoryModel;
use crate::assets::Icon;
use crate::detail::DetailSource;
use crate::format;
use crate::i18n::fl;
use crate::meter::{MeterModel, OverlaySettings};
use crate::report::ReportRequest;
use crate::overlay::WindowDrag;
use crate::secondary::chrome::{self, Anchor, ButtonKind, Look, Rows, Scale, Tone, FAINT, MUTED, SOFT, TEXT};
use crate::secondary;
use crate::style::{class_fill, track};

const WIDTH: f32 = 370.0;
/// About seven closed records; the list scrolls past it.
const LIST_MAX: f32 = 318.0;
const RECORD_HEIGHT: f32 = 42.0;
const RECORD_GAP: f32 = 4.0;
const MENU_WIDTH: f32 = 240.0;
const MENU_ROWS: usize = 8;
/// A delete that is not confirmed goes back to the plain button.
const CONFIRM_FOR: Duration = Duration::from_secs(4);
const MENU_CLOSE_AFTER: Duration = Duration::from_millis(400);
const MENU_FADE: Duration = Duration::from_millis(100);
const NOTE_FADE: Duration = Duration::from_millis(150);
const HP: u32 = 0xe8735f;
const FOCUS_RING: u32 = 0xf3ebe1e6;
const TIP_FILL: u32 = 0x1e1a16fa;
const TIP_BORDER: u32 = 0xffecd624;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Filter {
    Target,
    Player,
}

#[derive(Clone, PartialEq, Eq)]
enum Confirm {
    One(String),
    All,
}

/// What the records are drawn with in this frame.
#[derive(Clone, Copy)]
struct RecordsFrame {
    s: Scale,
    overflows: bool,
}

pub struct HistoryView {
    model: Entity<HistoryModel>,
    meter_model: Entity<MeterModel>,
    menu: Option<Filter>,
    confirm: Option<Confirm>,
    confirm_timer: Option<Task<()>>,
    menu_timer: Option<Task<()>>,
    /// The records, drawn by GPUI's `list` as they come into view.
    rows: Rows,
    records_frame: Option<RecordsFrame>,
    /// The record whose report button the keyboard just reached, scrolled
    /// into view at the next frame.
    reveal: Option<String>,
    menu_scroll: ScrollHandle,
    target_anchor: Anchor,
    player_anchor: Anchor,
    drag: WindowDrag,
    /// The record whose report button the pointer is on, from the button's
    /// enter and leave.
    report_hover: Option<String>,
    /// The report buttons' keyboard focus, by record id.
    report_focus: HashMap<String, FocusHandle>,
    root_focus: FocusHandle,
    /// A capture's window is off screen, so no pointer is ever over it.
    #[cfg(feature = "probes")]
    pointer_held_over: bool,
    /// Dev captures draw a focus ring without a keyboard.
    #[cfg(feature = "probes")]
    forced_ring: Option<String>,
    /// The records the list built in the last frame, for the scrollbar check.
    #[cfg(feature = "probes")]
    built: std::cell::RefCell<Vec<String>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<ReportRequest> for HistoryView {}

impl HistoryView {
    pub fn new(model: Entity<HistoryModel>, meter_model: Entity<MeterModel>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscriptions = vec![
            cx.observe(&model, |this, model, cx| {
                // A record that went away takes its pending delete with it.
                if let Some(Confirm::One(id)) = &this.confirm {
                    if model.read(cx).list.record(id).is_none() {
                        this.confirm = None;
                    }
                }
                cx.notify();
            }),
            // A focused button whose record went away leaves the focus with
            // the window, so Tab keeps working.
            cx.on_focus_lost(window, |this, window, cx| window.focus(&this.root_focus, cx)),
        ];
        let root_focus = cx.focus_handle();
        window.focus(&root_focus, cx);
        Self {
            model,
            meter_model,
            menu: None,
            confirm: None,
            confirm_timer: None,
            menu_timer: None,
            rows: Rows::new(px(LIST_MAX)),
            records_frame: None,
            reveal: None,
            menu_scroll: ScrollHandle::new(),
            target_anchor: Anchor::default(),
            player_anchor: Anchor::default(),
            drag: WindowDrag::default(),
            report_hover: None,
            report_focus: HashMap::new(),
            root_focus,
            #[cfg(feature = "probes")]
            pointer_held_over: false,
            #[cfg(feature = "probes")]
            forced_ring: None,
            #[cfg(feature = "probes")]
            built: std::cell::RefCell::default(),
            _subscriptions: subscriptions,
        }
    }

    fn report_button_focus(&mut self, id: &str, cx: &App) -> FocusHandle {
        if let Some(handle) = self.report_focus.get(id) {
            return handle.clone();
        }
        let handle = cx.focus_handle().tab_stop(true);
        self.report_focus.insert(id.to_string(), handle.clone());
        handle
    }

    /// Tab moves between the report buttons, in the records' order and
    /// round, also to buttons scrolled out of view, which the list then
    /// brings in; the window never takes the keyboard from the game, so this
    /// serves accessibility tools.
    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let modifiers = event.keystroke.modifiers;
        if event.keystroke.key != "tab" || modifiers.control || modifiers.alt || modifiers.platform {
            return;
        }
        cx.stop_propagation();
        let bosses: Vec<String> = self.model.read(cx).list.shown().filter(|row| row.is_boss).map(|row| row.id.clone()).collect();
        let Some(last) = bosses.len().checked_sub(1) else {
            return;
        };
        let at = bosses.iter().position(|id| self.report_focus.get(id).is_some_and(|handle| handle.is_focused(window)));
        let next = match (at, modifiers.shift) {
            (None, false) => 0,
            (None, true) => last,
            (Some(at), false) => if at == last { 0 } else { at + 1 },
            (Some(at), true) => if at == 0 { last } else { at - 1 },
        };
        let handle = self.report_button_focus(&bosses[next], cx);
        window.focus(&handle, cx);
        self.reveal = Some(bosses[next].clone());
        cx.notify();
    }

    fn point_at_report(&mut self, id: &str, hovered: bool, cx: &mut Context<Self>) {
        let now = if hovered {
            Some(id.to_string())
        } else if self.report_hover.as_deref() == Some(id) {
            None
        } else {
            return;
        };
        if self.report_hover != now {
            self.report_hover = now;
            cx.notify();
        }
    }

    fn ask(&mut self, confirm: Confirm, cx: &mut Context<Self>) {
        self.confirm = Some(confirm);
        self.confirm_timer = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(CONFIRM_FOR).await;
            let _ = this.update(cx, |this, cx| {
                this.confirm = None;
                cx.notify();
            });
        }));
        cx.notify();
    }

    fn settle(&mut self, cx: &mut Context<Self>) {
        self.confirm = None;
        self.confirm_timer = None;
        cx.notify();
    }

    fn toggle_menu(&mut self, filter: Filter, cx: &mut Context<Self>) {
        self.menu = if self.menu == Some(filter) { None } else { Some(filter) };
        self.menu_timer = None;
        self.menu_scroll = ScrollHandle::new();
        cx.notify();
    }

    fn close_menu(&mut self, cx: &mut Context<Self>) {
        self.menu = None;
        self.menu_timer = None;
        cx.notify();
    }

    fn pointer_is_over(&self, window: &Window) -> bool {
        #[cfg(feature = "probes")]
        if self.pointer_held_over {
            return true;
        }
        window.is_window_hovered()
    }

    /// The dropdown closes a moment after the pointer has left the window;
    /// GPUI redraws the window when the pointer enters or leaves it.
    fn watch_pointer(&mut self, window: &Window, cx: &mut Context<Self>) {
        if self.menu.is_none() || self.pointer_is_over(window) {
            self.menu_timer = None;
            return;
        }
        if self.menu_timer.is_some() {
            return;
        }
        self.menu_timer = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(MENU_CLOSE_AFTER).await;
            let _ = this.update_in(cx, |this, window, cx| {
                this.menu_timer = None;
                if !this.pointer_is_over(window) {
                    this.close_menu(cx);
                }
            });
        }));
    }
}

impl Render for HistoryView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.watch_pointer(window, cx);
        let settings = self.meter_model.read(cx).settings.clone();
        let look = Look::of(&settings);
        let s = look.s;
        let model = self.model.read(cx);
        let empty = model.list.total() == 0;
        if empty {
            self.menu = None;
        }

        let header = chrome::header(
            chrome::title(Icon::History, fl!("history-title"), Some(model.list.count_label().into()), s),
            Vec::new(),
            &self.drag,
            s,
        );
        let records = model.list.visible();
        let body: AnyElement = if empty {
            chrome::empty(Some(fl!("history-empty-title").into()), fl!("history-empty-line"), s).into_any_element()
        } else if records.is_empty() {
            chrome::empty(None, fl!("history-no-match"), s).into_any_element()
        } else {
            // An opened record adds its players to the room, so it opens
            // without pushing the others out of sight.
            let open = model.open.as_deref();
            let opened = open
                .filter(|id| records.iter().any(|row| row.id == *id))
                .map_or(0., |id| players_height(model.players(id).len()));
            let mut items: Vec<(Pixels, Option<FocusHandle>)> = Vec::with_capacity(records.len());
            for (ix, row) in records.iter().enumerate() {
                let gap = if ix > 0 { RECORD_GAP } else { 0. };
                let players = if open == Some(row.id.as_str()) { opened } else { 0. };
                let focus = row.is_boss.then(|| self.report_button_focus(&row.id, cx));
                items.push((s.px(RECORD_HEIGHT + gap + players), focus));
            }
            let max = LIST_MAX + opened;
            self.rows.set(&items);
            if let Some(ix) = self.reveal.take().and_then(|id| records.iter().position(|row| row.id == id)) {
                self.rows.reveal(ix, s.px(3.));
            }
            #[cfg(feature = "probes")]
            self.built.borrow_mut().clear();
            self.records_frame = Some(RecordsFrame { s, overflows: self.rows.overflows(max, s) });
            let records = list(self.rows.state.clone(), cx.processor(|this: &mut Self, ix: usize, window, cx| this.record_line(ix, window, cx)));
            chrome::scroll_rows("records", &self.rows, records, max, s).into_any_element()
        };
        let status = model.status().map(|status| {
            chrome::note(Icon::Check, status.clone(), true, s)
                .mt(s.px(8.))
                .with_animation(SharedString::from(status), Animation::new(NOTE_FADE), |note, t| note.opacity(t))
        });
        let card = chrome::card(WIDTH, look)
            .child(header)
            .child(self.filters(&settings, empty, s, cx))
            .child(body)
            .children(status)
            .child(self.footer(empty, s, cx));

        let menu = self.menu.and_then(|filter| self.menu_popover(filter, &settings, s, cx));
        let scrim = menu.is_some().then(|| {
            chrome::scrim("menu-scrim").on_any_mouse_down(cx.listener(|this, _, _, cx| {
                cx.stop_propagation();
                this.close_menu(cx);
            }))
        });
        chrome::root(look)
            .track_focus(&self.root_focus)
            .on_key_down(cx.listener(Self::on_key))
            .child(card)
            .children(scrim)
            .children(menu)
    }
}

impl HistoryView {
    fn filters(&self, settings: &OverlaySettings, disabled: bool, s: Scale, cx: &Context<Self>) -> impl IntoElement {
        let list = &self.model.read(cx).list;
        let target = list.target_filter().map_or_else(|| fl!("history-all-targets").into(), |target| target.to_string().into());
        let player = list
            .player_filter()
            .map_or_else(|| fl!("history-all-players").into(), |player| settings.display_name(player).into());
        let select = |filter: Filter, label: String, value: SharedString, anchor: &Anchor| {
            let id = match filter {
                Filter::Target => "target-filter",
                Filter::Player => "player-filter",
            };
            chrome::select(id, label, value, self.menu == Some(filter), s)
                .relative()
                .child(anchor.marker())
                .when(!disabled, |select| {
                    select.cursor_pointer().on_click(cx.listener(move |this, _, _, cx| this.toggle_menu(filter, cx)))
                })
        };
        div()
            .flex_none()
            .flex()
            .gap(s.px(8.))
            .mt(s.px(2.))
            .mb(s.px(8.))
            .when(disabled, |filters| filters.opacity(0.45))
            .child(select(Filter::Target, fl!("common-target"), target, &self.target_anchor))
            .child(select(Filter::Player, fl!("common-player"), player, &self.player_anchor))
    }

    fn menu_popover(&self, filter: Filter, settings: &OverlaySettings, s: Scale, cx: &Context<Self>) -> Option<AnyElement> {
        let anchor = match filter {
            Filter::Target => &self.target_anchor,
            Filter::Player => &self.player_anchor,
        };
        let bounds = anchor.bounds()?;
        let list = &self.model.read(cx).list;
        let (all, options, selected): (String, Vec<FilterOption>, Option<&str>) = match filter {
            Filter::Target => (fl!("history-all-targets"), list.target_options(), list.target_filter()),
            Filter::Player => (fl!("history-all-players"), list.player_options(), list.player_filter()),
        };
        let total = list.total();
        let unknown = unknown_player();
        let label = |value: &str| match filter {
            Filter::Player if value != unknown => settings.display_name(value),
            _ => value.to_string(),
        };
        let item = |index: usize, text: String, count: usize, value: Option<String>| {
            let hot = selected == value.as_deref();
            chrome::menu_item(("filter-option", index), hot, s)
                .on_click(cx.listener(move |this, _, _, cx| {
                    let value = value.clone();
                    this.model.update(cx, |model, cx| {
                        match filter {
                            Filter::Target => model.list.filter_target(value),
                            Filter::Player => model.list.filter_player(value),
                        }
                        cx.notify();
                    });
                    this.close_menu(cx);
                }))
                .child(div().flex_1().min_w_0().truncate().font_weight(FontWeight::MEDIUM).child(text))
                .child(div().flex_none().text_size(s.px(11.)).text_color(rgba(FAINT)).child(count.to_string()))
        };
        let rows = options.len() + 1;
        let items = div()
            .flex()
            .flex_col()
            .child(item(0, all, total, None))
            .children(options.iter().enumerate().map(|(index, option)| {
                item(index + 1, label(&option.value), option.count, Some(option.value.clone()))
            }));
        let height = MENU_ROWS as f32 * 26.;
        let left = match filter {
            Filter::Target => bounds.left(),
            Filter::Player => bounds.right() - s.px(MENU_WIDTH),
        };
        Some(
            chrome::popover(s)
                .top(bounds.bottom() + s.px(6.))
                .left(left)
                .w(s.px(MENU_WIDTH))
                .child(chrome::scroll_list("filter-options", &self.menu_scroll, height, rows > MENU_ROWS, items, s))
                .with_animation("filter-menu", Animation::new(MENU_FADE), |menu, t| menu.opacity(t))
                .into_any_element(),
        )
    }

    /// Record `ix` of those shown, as the list asks for it, with the gap
    /// above it and the thumb's gutter while the list scrolls.
    fn record_line(&mut self, ix: usize, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(frame) = self.records_frame else {
            return div().into_any_element();
        };
        let Some(row) = self.model.read(cx).list.shown().nth(ix).cloned() else {
            return div().into_any_element();
        };
        let settings = self.meter_model.read(cx).settings.clone();
        #[cfg(feature = "probes")]
        self.built.borrow_mut().push(row.id.clone());
        let s = frame.s;
        div()
            .w_full()
            .flex()
            .flex_col()
            .when(ix > 0, |item| item.pt(s.px(RECORD_GAP)))
            .when(frame.overflows, |item| item.pr(s.px(chrome::GUTTER)))
            .child(self.record(&row, &settings, s, window, cx))
            .into_any_element()
    }

    fn record(&mut self, row: &RecordRow, settings: &OverlaySettings, s: Scale, window: &Window, cx: &Context<Self>) -> impl IntoElement {
        let model = self.model.read(cx);
        let open = model.open.as_deref() == Some(row.id.as_str());
        let marked = self.confirm == Some(Confirm::One(row.id.clone()));
        let id = row.id.clone();
        let delete: AnyElement = if marked {
            let confirmed = row.id.clone();
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap(s.px(4.))
                .child(
                    chrome::text_button("delete-confirm", Some(Icon::Trash), fl!("history-delete-one"), ButtonKind::Danger, 20., s).on_click(
                        cx.listener(move |this, _, _, cx| {
                            cx.stop_propagation();
                            this.settle(cx);
                            this.model.update(cx, |model, cx| model.delete(&confirmed, cx));
                        }),
                    ),
                )
                .child(chrome::text_button("delete-cancel", None, fl!("common-cancel"), ButtonKind::Quiet, 20., s).on_click(
                    cx.listener(|this, _, _, cx| {
                        cx.stop_propagation();
                        this.settle(cx);
                    }),
                ))
                .into_any_element()
        } else {
            let asked = row.id.clone();
            chrome::row_button(SharedString::from(format!("delete-{}", row.id)), Icon::Trash, fl!("history-delete-record"), s)
                .on_click(cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    this.ask(Confirm::One(asked.clone()), cx);
                }))
                .into_any_element()
        };
        let crown = div()
            .flex_none()
            .size(s.px(12.))
            .mr(s.px(-1.))
            .when(row.is_boss, |slot| slot.child(chrome::icon(Icon::Crown, 12., rgb(HP), s)));
        let report = if row.is_boss {
            self.report_button(row, s, window, cx).into_any_element()
        } else {
            div().flex_none().size(s.px(20.)).into_any_element()
        };
        let hp_left = row.hp_left.map(|hp| format::percent(f64::from(hp)));
        let wiped = row.wiped.then(|| {
            let selector = format!("wiped-{}", row.id);
            chrome::chip(Some(Icon::CircleX), fl!("meter-summary-wiped"), Tone::Bad, s)
                .debug_selector(move || selector.clone())
                .children(hp_left.clone().map(|hp| chrome::chip_note(hp, s)))
        });
        let top = div()
            .h(s.px(24.))
            .pl(s.px(10.))
            .pr(s.px(4.))
            .flex()
            .items_center()
            .gap(s.px(6.))
            .child(crown)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(s.px(6.))
                    .child(div().min_w_0().truncate().font_weight(FontWeight::SEMIBOLD).child(row.target.clone()))
                    .children(wiped),
            )
            .child(div().flex_none().text_size(s.px(11.5)).text_color(rgba(MUTED)).child(row.when.clone()))
            .child(report)
            .child(delete);
        let main = row.main_player.as_deref().map_or_else(unknown_player, |name| settings.display_name(name));
        let meta = div()
            .h(s.px(18.))
            .px(s.px(10.))
            .pb(s.px(4.))
            .flex()
            .items_center()
            .gap(s.px(10.))
            .text_size(s.px(11.5))
            .text_color(rgba(MUTED))
            .child(
                div()
                    .min_w_0()
                    .flex()
                    .items_center()
                    .gap(s.px(4.))
                    .text_color(rgba(TEXT))
                    .child(chrome::icon(Icon::User, 11., rgba(MUTED), s))
                    .child(div().min_w_0().truncate().child(main)),
            )
            .child(div().flex_none().child(fl!("common-players-count", count = row.player_count)))
            .child(
                div()
                    .flex_none()
                    .flex()
                    .gap(s.px(3.))
                    .child(div().text_color(rgba(TEXT)).font_weight(FontWeight::SEMIBOLD).child(format::compact(row.damage as f64)))
                    .child(fl!("history-damage")),
            )
            .child(div().flex_none().child(if row.duration_secs > 0.0 {
                format::duration(row.duration_secs)
            } else {
                "--".to_string()
            }));
        let players = open.then(|| self.players(row, settings, s, cx));
        let tip = (self.report_hover.as_deref() == Some(row.id.as_str())).then(|| report_tip(s));
        let name = match (row.is_boss, row.wiped, hp_left) {
            (_, true, Some(hp)) => fl!("history-wiped-record-hp", name = row.target.clone(), hp = hp),
            (_, true, None) => fl!("history-wiped-record", name = row.target.clone()),
            (true, false, _) => fl!("history-boss-record", name = row.target.clone()),
            (false, false, _) => row.target.clone(),
        };
        div()
            .id(SharedString::from(format!("record-{}", row.id)))
            .role(Role::Button)
            .aria_label(name)
            .flex_none()
            .relative()
            .flex()
            .flex_col()
            .rounded(s.px(7.))
            .bg(rgba(if open { 0x00000066 } else { crate::style::TRACK }))
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| this.model.update(cx, |model, cx| model.toggle_open(&id, cx))))
            .child(top)
            .child(meta)
            .children(players)
            .when(open || marked, |record| {
                record.child(
                    div()
                        .absolute()
                        .inset_0()
                        .rounded(s.px(7.))
                        .border_1()
                        .border_color(rgba(if marked { 0xf7a8a88c } else { 0xfff6ec2e })),
                )
            })
            .children(tip)
    }

    /// The record's fight report button: focusable, so Enter opens the report
    /// as a click does.
    fn report_button(&mut self, row: &RecordRow, s: Scale, window: &Window, cx: &Context<Self>) -> impl IntoElement {
        let focus = self.report_button_focus(&row.id, cx);
        #[cfg(feature = "probes")]
        let forced = self.forced_ring.as_deref() == Some(row.id.as_str());
        #[cfg(not(feature = "probes"))]
        let forced = false;
        let ring = forced || (focus.is_focused(window) && window.last_input_was_keyboard());
        let hot = self.report_hover.as_deref() == Some(row.id.as_str());
        let (hovered_id, clicked_id, selector) = (row.id.clone(), row.id.clone(), format!("report-{}", row.id));
        div()
            .id(SharedString::from(selector.clone()))
            .debug_selector(move || selector.clone())
            .role(Role::Button)
            .aria_label(fl!("history-report"))
            .track_focus(&focus)
            .relative()
            .flex_none()
            .size(s.px(20.))
            .flex()
            .items_center()
            .justify_center()
            .rounded(s.px(7.))
            .cursor_pointer()
            .when(hot, |button| button.bg(rgba(SOFT)))
            .child(chrome::icon(Icon::FileText, 12., rgba(if hot { TEXT } else { MUTED }), s))
            .when(ring, |button| {
                button.child(
                    div()
                        .debug_selector(|| "focus-ring".to_string())
                        .absolute()
                        .top(px(-3.))
                        .left(px(-3.))
                        .right(px(-3.))
                        .bottom(px(-3.))
                        .rounded(s.px(7.) + px(3.))
                        .border_2()
                        .border_color(rgba(FOCUS_RING)),
                )
            })
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| this.point_at_report(&hovered_id, *hovered, cx)))
            .on_click(cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                if let Some(record) = this.model.read(cx).list.record(&clicked_id).cloned() {
                    cx.emit(ReportRequest::Saved(record));
                }
            }))
    }

    fn players(&self, row: &RecordRow, settings: &OverlaySettings, s: Scale, cx: &Context<Self>) -> impl IntoElement {
        let model = self.model.read(cx);
        let lines = model.players(&row.id);
        let top = lines.iter().map(|player| player.dps).fold(0.0, f64::max);
        let record = model.list.record(&row.id).cloned();
        let hint = div()
            .h(s.px(20.))
            .flex()
            .items_center()
            .gap(s.px(5.))
            .text_size(s.px(11.))
            .text_color(rgba(FAINT))
            .child(chrome::icon(Icon::External, 11., rgba(FAINT), s))
            .child(fl!("history-player-hint"));
        let rows = lines.iter().enumerate().map(|(index, player)| {
            let meter_model = self.meter_model.clone();
            let record = record.clone();
            let actor_id = player.actor_id;
            player_row(index + 1, player, if top > 0.0 { (player.dps / top) as f32 } else { 0.0 }, settings, s)
                .id(SharedString::from(format!("player-{}-{}", row.id, player.actor_id)))
                .cursor_pointer()
                .on_click(move |_, _, cx| {
                    cx.stop_propagation();
                    if let Some(record) = record.clone() {
                        secondary::open_detail(&meter_model, DetailSource::History { actor_id, record }, cx);
                    }
                })
        });
        div()
            .pt(s.px(2.))
            .px(s.px(8.))
            .pb(s.px(8.))
            .flex()
            .flex_col()
            .child(hint)
            .child(div().flex().flex_col().gap(s.px(3.)).children(rows))
    }

    fn footer(&self, empty: bool, s: Scale, cx: &Context<Self>) -> impl IntoElement {
        let total = self.model.read(cx).list.total();
        let foot = chrome::footer(s)
            .child(div().flex_1().min_w_0().truncate().text_color(rgba(FAINT)).child(fl!("history-saved-when")));
        if self.confirm == Some(Confirm::All) {
            return foot
                .child(
                    chrome::text_button(
                        "delete-all-confirm",
                        Some(Icon::Trash),
                        fl!("history-delete-all-confirm", count = total),
                        ButtonKind::Danger,
                        22.,
                        s,
                    )
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.settle(cx);
                        this.model.update(cx, |model, cx| model.delete_all(cx));
                    })),
                )
                .child(
                    chrome::text_button("delete-all-cancel", None, fl!("common-cancel"), ButtonKind::Quiet, 22., s)
                        .on_click(cx.listener(|this, _, _, cx| this.settle(cx))),
                );
        }
        if empty {
            return foot.child(chrome::text_button("delete-all", Some(Icon::Trash), fl!("history-delete-all"), ButtonKind::Disabled, 22., s));
        }
        foot.child(
            chrome::text_button("delete-all", Some(Icon::Trash), fl!("history-delete-all"), ButtonKind::Quiet, 22., s)
                .on_click(cx.listener(|this, _, _, cx| this.ask(Confirm::All, cx))),
        )
    }
}

/// Hint, rows and padding under an opened record.
fn players_height(rows: usize) -> f32 {
    2. + 20. + rows as f32 * 22. + rows.saturating_sub(1) as f32 * 3. + 8.
}

/// "Fight report" under the pointer's report button, on the popover colours.
fn report_tip(s: Scale) -> impl IntoElement {
    div()
        .debug_selector(|| "report-tip".to_string())
        .absolute()
        .top(s.px(23.))
        .right(s.px(22.))
        .py(s.px(4.))
        .px(s.px(8.))
        .rounded(s.px(6.))
        .border_1()
        .border_color(rgba(TIP_BORDER))
        .bg(rgba(TIP_FILL))
        .text_size(s.px(11.5))
        .font_weight(FontWeight::MEDIUM)
        .text_color(rgba(TEXT))
        .child(fl!("history-report"))
}

fn player_row(rank: usize, player: &PlayerLine, bar: f32, settings: &OverlaySettings, s: Scale) -> gpui::Div {
    let name = if player.named { settings.display_name(&player.name) } else { player.name.clone() };
    let content = chrome::bar_content(player.class.is_none(), s)
        .when_some(player.class, |content, class| content.child(chrome::class_icon(class, 16., s)))
        .child(chrome::name(name, None, if player.you { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM }, s))
        .child(
            div()
                .flex_none()
                .text_size(s.px(11.5))
                .text_color(rgba(MUTED))
                .child(format::compact(player.damage as f64)),
        )
        .child(chrome::strong(format::compact(player.dps), s));
    let bar = track(class_fill(player.colour), bar.max(0.06), 22., s)
        .child(content)
        .when(player.you, |bar| bar.child(chrome::you_rim(s)));
    chrome::row(s)
        .child(chrome::rank_badge(rank, player.you, s))
        .child(bar)
        .child(chrome::percent(format::percent(player.share), s))
}

#[cfg(feature = "probes")]
impl HistoryView {
    /// Opens a filter's dropdown or asks to delete, as a click would; the
    /// pointer is held over the window so the dropdown stays open.
    pub fn show_state(&mut self, menu: Option<bool>, confirm_record: Option<String>, confirm_all: bool, cx: &mut Context<Self>) {
        self.pointer_held_over = menu.is_some();
        self.menu = menu.map(|target| if target { Filter::Target } else { Filter::Player });
        self.menu_scroll = ScrollHandle::new();
        match (confirm_record, confirm_all) {
            (Some(id), _) => self.ask(Confirm::One(id), cx),
            (None, true) => self.ask(Confirm::All, cx),
            (None, false) => self.settle(cx),
        }
        cx.notify();
    }

    pub fn menu_is_open(&self) -> bool {
        self.menu.is_some()
    }

    /// The pointer on a record's report button, and a focus ring on one, as
    /// the pointer's enter and the keyboard would.
    pub fn show_report_button(&mut self, hovered: Option<String>, ring: Option<String>, cx: &mut Context<Self>) {
        self.report_hover = hovered;
        self.forced_ring = ring;
        cx.notify();
    }

    /// The records' list as the scrollbar check reads it.
    pub fn list_reading(&self, cx: &App) -> crate::capture_secondary::ListReading {
        let built = self.built.borrow();
        let list = &self.model.read(cx).list;
        let built_record = |row: Option<&super::list::RecordRow>| row.is_some_and(|row| built.contains(&row.id));
        crate::capture_secondary::ListReading {
            viewport: self.rows.viewport(),
            max: f32::from(self.rows.max()),
            scrolled: f32::from(self.rows.scrolled()),
            first_built: built_record(list.shown().next()),
            last_built: built_record(list.shown().last()),
        }
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::HashMap;
    use std::rc::Rc;

    use gpui::{AppContext as _, KeyUpEvent, Keystroke, Modifiers, TestAppContext, VisualTestContext};
    use meter_core::history::HistoryRecord;
    use meter_core::models::combat::{CombatInfos, PlayerOverviewStat, TargetInfo};

    use super::*;

    fn record(id: &str, target: &str, is_boss: bool, created_at: u64) -> HistoryRecord {
        let stat = PlayerOverviewStat {
            actor_id: 1,
            actor_name: "Mirelle".to_string(),
            actor_server_id: "1001".to_string(),
            actor_class: "Ranger".to_string(),
            combat_power: None,
            counts: 10,
            total_damage: 5_000,
            min_damage: 1,
            max_damage: 9,
            special_counts: HashMap::new(),
            dps: 50.0,
            damage_share: 1.0,
            damage_contribution: 1.0,
        };
        HistoryRecord {
            id: id.to_string(),
            target_id: 7,
            total_damage: 5_000,
            target_info: Some(TargetInfo {
                id: 7,
                target_mob_code: None,
                target_name: Some(target.to_string()),
                is_boss,
                current_hp: Some(0),
                max_hp: Some(100),
                target_start_time: HashMap::from([(1, 1_000.0)]),
                target_last_time: HashMap::from([(1, 1_100.0)]),
            }),
            combat_infos: CombatInfos {
                actor_infos: HashMap::new(),
                target_infos: HashMap::new(),
                main_actor_id: Some(1),
                main_actor_name: Some("Mirelle".to_string()),
                last_target_by_main_actor: None,
                last_target: None,
                time_now: 0.0,
            },
            player_skill_stats: HashMap::new(),
            player_stats: HashMap::from([(1, stat)]),
            use_buffs_by_target: HashMap::new(),
            ledger: None,
            wiped: false,
            created_at,
        }
    }

    fn wiped(mut record: HistoryRecord, hp: u32) -> HistoryRecord {
        record.wiped = true;
        if let Some(target) = record.target_info.as_mut() {
            target.current_hp = Some(hp);
        }
        record
    }

    /// The window on a boss record, a mob record and a wiped attempt, with
    /// the records its report buttons ask for.
    fn open(cx: &mut TestAppContext) -> (&mut VisualTestContext, Rc<RefCell<Vec<String>>>) {
        let meter_model = cx.new(|_| MeterModel::new(OverlaySettings::default(), None));
        let history = cx.new(|cx| HistoryModel::new(None, &meter_model, cx));
        history.update(cx, |history, _| {
            history.load(vec![
                record("boss", "Veilwarden Orsikh", true, 2),
                record("mob", "Thornback Boar", false, 1),
                wiped(record("wipe", "Veilwarden Orsikh", true, 0), 30),
            ])
        });
        let (view, cx) = cx.add_window_view(|window, cx| HistoryView::new(history, meter_model, window, cx));
        let asked = Rc::new(RefCell::new(Vec::new()));
        let log = asked.clone();
        cx.update(|_, cx| {
            cx.subscribe(&view, move |_, request: &ReportRequest, _| {
                if let ReportRequest::Saved(record) = request {
                    log.borrow_mut().push(record.id.clone());
                }
            })
            .detach()
        });
        (cx, asked)
    }

    #[gpui::test]
    fn only_boss_records_have_a_report_button(cx: &mut TestAppContext) {
        let (cx, asked) = open(cx);
        assert!(cx.debug_bounds("report-mob").is_none());

        let button = cx.debug_bounds("report-boss").expect("the boss record's button");
        cx.simulate_click(button.center(), Modifiers::none());

        assert_eq!(*asked.borrow(), vec!["boss".to_string()]);
    }

    #[gpui::test]
    fn the_report_button_takes_the_keyboard_and_enter_opens_the_report(cx: &mut TestAppContext) {
        let (cx, asked) = open(cx);
        for key in ["tab", "enter"] {
            let keystroke = Keystroke::parse(key).expect("a key name");
            cx.simulate_event(KeyDownEvent { keystroke: keystroke.clone(), is_held: false, prefer_character_input: false });
            cx.simulate_event(KeyUpEvent { keystroke });
            if key == "tab" {
                assert!(cx.debug_bounds("focus-ring").is_some(), "the ring on the report button");
            }
        }

        assert_eq!(*asked.borrow(), vec!["boss".to_string()]);
    }

    #[gpui::test]
    fn a_wiped_attempt_is_marked_and_keeps_its_report(cx: &mut TestAppContext) {
        let (cx, asked) = open(cx);
        assert!(cx.debug_bounds("wiped-boss").is_none(), "a kill carries no wipe mark");
        assert!(cx.debug_bounds("wiped-wipe").is_some());

        let button = cx.debug_bounds("report-wipe").expect("the wiped record's button");
        cx.simulate_click(button.center(), Modifiers::none());

        assert_eq!(*asked.borrow(), vec!["wipe".to_string()]);
    }

    /// The window on 40 boss records, newest first: `r39` on top, `r0` at
    /// the bottom.
    fn open_long(cx: &mut TestAppContext) -> (Entity<HistoryView>, &mut VisualTestContext) {
        let meter_model = cx.new(|_| MeterModel::new(OverlaySettings::default(), None));
        let history = cx.new(|cx| HistoryModel::new(None, &meter_model, cx));
        history.update(cx, |history, _| {
            history.load((0..40).map(|i| record(&format!("r{i}"), "Veilwarden Orsikh", true, i as u64)).collect())
        });
        cx.add_window_view(|window, cx| HistoryView::new(history, meter_model, window, cx))
    }

    /// The list's viewport, its furthest scroll and the scroll now, after a
    /// frame drawn with them.
    fn scroll_of(view: &Entity<HistoryView>, cx: &mut VisualTestContext) -> (gpui::Bounds<Pixels>, Pixels, Pixels) {
        cx.update(|window, cx| {
            view.update(cx, |_, cx| cx.notify());
            let _ = window.draw(cx);
            let rows = &view.read(cx).rows;
            (rows.viewport(), rows.max(), rows.scrolled())
        })
    }

    #[gpui::test]
    fn the_thumb_keeps_following_the_pointer_over_the_records_and_below_the_list(cx: &mut TestAppContext) {
        let (view, cx) = open_long(cx);
        let (viewport, max, scrolled) = scroll_of(&view, cx);
        assert!(max > px(0.) && scrolled == px(0.));
        assert!(cx.debug_bounds("report-r39").is_some() && cx.debug_bounds("report-r0").is_none(), "only the top records are built");

        let height = viewport.size.height;
        let length = height * (height / (height + max));
        let travel = height - px(4.) - length;
        let thumb = gpui::point(viewport.right() - px(4.), viewport.top() + px(2.) + length / 2.);
        let over_records = viewport.center().x;
        cx.simulate_mouse_down(thumb, gpui::MouseButton::Left, Modifiers::none());
        cx.simulate_mouse_move(gpui::point(over_records, thumb.y + px(30.)), Some(gpui::MouseButton::Left), Modifiers::none());
        let expected = max * (px(30.) / travel);
        assert!((scroll_of(&view, cx).2 - expected).abs() < px(0.5), "over the records, the list follows the thumb in proportion");
        let below = gpui::point(over_records, viewport.bottom() + px(300.));
        cx.simulate_mouse_move(below, Some(gpui::MouseButton::Left), Modifiers::none());
        cx.simulate_mouse_up(below, gpui::MouseButton::Left, Modifiers::none());
        assert_eq!(scroll_of(&view, cx).2, max, "below the list, the list reaches its end");
        assert!(cx.debug_bounds("report-r0").is_some() && cx.debug_bounds("report-r39").is_none(), "the bottom records are built, the top ones not");
    }

    #[gpui::test]
    fn tab_reaches_report_buttons_scrolled_out_of_view_and_brings_their_record_in(cx: &mut TestAppContext) {
        let (view, cx) = open_long(cx);
        let press = |key: &str, cx: &mut VisualTestContext| {
            let keystroke = Keystroke::parse(key).expect("a key name");
            cx.simulate_event(KeyDownEvent { keystroke: keystroke.clone(), is_held: false, prefer_character_input: false });
            cx.simulate_event(KeyUpEvent { keystroke });
        };
        let inside = |selector: &'static str, viewport: gpui::Bounds<Pixels>, cx: &mut VisualTestContext| {
            cx.debug_bounds(selector).is_some_and(|button| button.top() >= viewport.top() && button.bottom() <= viewport.bottom())
        };
        assert!(cx.debug_bounds("report-r29").is_none(), "the 11th record is far below the view");
        for _ in 0..11 {
            press("tab", cx);
        }
        let (viewport, _, scrolled) = scroll_of(&view, cx);
        assert!(scrolled > px(0.), "the list scrolled");
        assert!(inside("report-r29", viewport, cx), "the 11th record's button, which has the keyboard, is in view");
        assert!(cx.debug_bounds("focus-ring").is_some());

        press("shift-tab", cx);
        let (viewport, _, _) = scroll_of(&view, cx);
        assert!(inside("report-r30", viewport, cx), "Shift+Tab goes back one");

        for _ in 0..10 {
            press("shift-tab", cx);
        }
        press("shift-tab", cx);
        let (viewport, max, scrolled) = scroll_of(&view, cx);
        assert_eq!(scrolled, max, "from the first, Shift+Tab goes round to the last record, at the bottom");
        assert!(inside("report-r0", viewport, cx));
    }
}
