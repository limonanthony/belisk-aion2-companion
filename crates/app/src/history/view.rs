//! The fight history as approved in docs/gpui-mockups/approved/history.html:
//! newest records first, a dropdown for each filter, one record open at a
//! time, and deleting confirmed in the button itself.

use std::time::Duration;

use gpui::{
    div, prelude::*, rgba, Animation, AnimationExt as _, AnyElement, Context, Entity, FontWeight, IntoElement,
    Render, ScrollHandle, SharedString, Subscription, Task, Window,
};

use super::list::{unknown_player, FilterOption, PlayerLine, RecordRow};
use super::model::HistoryModel;
use crate::assets::Icon;
use crate::detail::DetailSource;
use crate::format;
use crate::i18n::fl;
use crate::meter::{MeterModel, OverlaySettings};
use crate::overlay::WindowDrag;
use crate::secondary::chrome::{self, Anchor, ButtonKind, Look, Scale, FAINT, MUTED, TEXT};
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

pub struct HistoryView {
    model: Entity<HistoryModel>,
    meter_model: Entity<MeterModel>,
    menu: Option<Filter>,
    confirm: Option<Confirm>,
    confirm_timer: Option<Task<()>>,
    menu_timer: Option<Task<()>>,
    list_scroll: ScrollHandle,
    menu_scroll: ScrollHandle,
    target_anchor: Anchor,
    player_anchor: Anchor,
    drag: WindowDrag,
    /// A capture's window is off screen, so no pointer is ever over it.
    #[cfg(feature = "probes")]
    pointer_held_over: bool,
    _model: Subscription,
}

impl HistoryView {
    pub fn new(model: Entity<HistoryModel>, meter_model: Entity<MeterModel>, _: &mut Window, cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe(&model, |this, model, cx| {
            // A record that went away takes its pending delete with it.
            if let Some(Confirm::One(id)) = &this.confirm {
                if model.read(cx).list.record(id).is_none() {
                    this.confirm = None;
                }
            }
            cx.notify();
        });
        Self {
            model,
            meter_model,
            menu: None,
            confirm: None,
            confirm_timer: None,
            menu_timer: None,
            list_scroll: ScrollHandle::new(),
            menu_scroll: ScrollHandle::new(),
            target_anchor: Anchor::default(),
            player_anchor: Anchor::default(),
            drag: WindowDrag::default(),
            #[cfg(feature = "probes")]
            pointer_held_over: false,
            _model: subscription,
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
            let opened = model
                .open
                .as_deref()
                .filter(|id| records.iter().any(|row| row.id == *id))
                .map_or(0., |id| players_height(model.list.players(id).len()));
            let content_height = RECORD_HEIGHT * records.len() as f32 + RECORD_GAP * (records.len() - 1) as f32 + opened;
            let max = LIST_MAX + opened;
            let list = div()
                .flex()
                .flex_col()
                .gap(s.px(RECORD_GAP))
                .children(records.iter().map(|row| self.record(row, &settings, s, cx)));
            chrome::scroll_list("records", &self.list_scroll, max, content_height > max, list, s).into_any_element()
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
        chrome::root(look).child(card).children(scrim).children(menu)
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

    fn record(&self, row: &RecordRow, settings: &OverlaySettings, s: Scale, cx: &Context<Self>) -> impl IntoElement {
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
            chrome::row_button(SharedString::from(format!("delete-{}", row.id)), Icon::Trash, s)
                .on_click(cx.listener(move |this, _, _, cx| {
                    cx.stop_propagation();
                    this.ask(Confirm::One(asked.clone()), cx);
                }))
                .into_any_element()
        };
        let top = div()
            .h(s.px(24.))
            .pl(s.px(10.))
            .pr(s.px(4.))
            .flex()
            .items_center()
            .gap(s.px(6.))
            .child(div().flex_1().min_w_0().truncate().font_weight(FontWeight::SEMIBOLD).child(row.target.clone()))
            .child(div().flex_none().text_size(s.px(11.5)).text_color(rgba(MUTED)).child(row.when.clone()))
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
        div()
            .id(SharedString::from(format!("record-{}", row.id)))
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
    }

    fn players(&self, row: &RecordRow, settings: &OverlaySettings, s: Scale, cx: &Context<Self>) -> impl IntoElement {
        let model = self.model.read(cx);
        let lines = model.list.players(&row.id);
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
}
