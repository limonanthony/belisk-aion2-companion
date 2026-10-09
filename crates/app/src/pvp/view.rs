//! The PvP window as approved in docs/gpui-mockups/approved/pvp.html: the
//! players you watch pinned on top with their health, then kill stats and
//! damage dealt and taken on two tabs.
//!
//! The window is non-activating like the meter. Only the add field takes
//! keyboard focus, from the press on "+" until Enter, Escape, the pointer
//! leaving the window, or the field losing focus; see
//! `secondary::typing`.

use std::time::Duration;

use meter_core::models::combat::PvpCombatStatsRow;
use gpui::{
    div, prelude::*, rgb, rgba, Animation, AnimationExt as _, AnyElement, Context, Div, Entity, Focusable as _,
    FontWeight, IntoElement, Render, ScrollHandle, SharedString, Subscription, Task, Window,
};
use windows::Win32::Foundation::HWND;

use super::data::{DamageRow, StatsSort, Suggestion, WatchRow};
use super::model::PvpModel;
use crate::assets::Icon;
use crate::detail::DetailSource;
use crate::format;
use crate::game_data;
use crate::i18n::{self, fl};
use crate::meter::{MeterModel, OverlaySettings};
use crate::overlay;
use crate::secondary::chrome::{self, Anchor, ButtonKind, Look, Scale, FAINT, MUTED, TEXT};
use crate::secondary::text_input::{TextInput, TextInputEvent};
use crate::secondary;
use crate::secondary::typing::{self, Edit};
use crate::style::{class_fill, hp_fill, track};

const WIDTH: f32 = 340.0;
/// How long the pointer may be away before an edit in progress ends.
const LEAVE_GRACE: Duration = Duration::from_millis(400);
const ROW: f32 = 22.0;
const ROW_GAP: f32 = 3.0;
/// Nine kill-stat rows; the list scrolls past them.
const KILLS_SHOWN: usize = 9;
const DAMAGE_MAX: f32 = 300.0;
const TAG_FILL: u32 = 0xe8735f2e;
const TAG_TEXT: u32 = 0xf6b4a6;
const MENU_FADE: Duration = Duration::from_millis(100);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    Kills,
    Damage,
}

pub struct PvpView {
    model: Entity<PvpModel>,
    input: Entity<TextInput>,
    edit: Edit,
    /// The window's handle, kept so focus can be given back even when the
    /// window is going away.
    hwnd: Option<HWND>,
    leave_timer: Option<Task<()>>,
    tab: Tab,
    show_info: bool,
    field: Anchor,
    kills_scroll: ScrollHandle,
    damage_scroll: ScrollHandle,
    drag: overlay::WindowDrag,
    /// A capture's window is off screen, so no pointer is ever over it.
    #[cfg(feature = "probes")]
    pointer_held_over: bool,
    _subscriptions: Vec<Subscription>,
}

impl PvpView {
    pub fn new(model: Entity<PvpModel>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let input = cx.new(|cx| TextInput::new(fl!("pvp-add-placeholder"), cx));
        let focus = input.read(cx).focus_handle(cx);
        let locale = i18n::locale(cx);
        let subscriptions = vec![
            cx.observe(&model, |_, _, cx| cx.notify()),
            cx.observe(&input, |_, _, cx| cx.notify()),
            cx.observe(&locale, |this, _, cx| {
                this.input.update(cx, |input, cx| input.set_placeholder(fl!("pvp-add-placeholder"), cx))
            }),
            cx.subscribe_in(&input, window, |this, _, event: &TextInputEvent, window, cx| match event {
                TextInputEvent::Submitted(typed) => this.submit(typed, window, cx),
                TextInputEvent::Cancelled => this.stop_editing(window, cx),
            }),
            cx.on_blur(&focus, window, |this, window, cx| this.stop_editing(window, cx)),
            cx.observe_window_activation(window, |this, window, cx| {
                if !window.is_window_active() {
                    this.stop_editing(window, cx);
                }
            }),
        ];
        let hwnd = overlay::hwnd(window).map_err(|error| crate::app_log::warn(format!("cannot edit the watch list: {error}"))).ok();
        // Closing the window or quitting mid-edit still gives focus back.
        cx.on_release(|this, _| this.release_focus()).detach();
        cx.on_app_quit(|this, _| {
            this.release_focus();
            async {}
        })
        .detach();
        Self {
            model,
            input,
            edit: Edit::Idle,
            hwnd,
            leave_timer: None,
            tab: Tab::Kills,
            show_info: false,
            field: Anchor::default(),
            kills_scroll: ScrollHandle::new(),
            damage_scroll: ScrollHandle::new(),
            drag: overlay::WindowDrag::default(),
            #[cfg(feature = "probes")]
            pointer_held_over: false,
            _subscriptions: subscriptions,
        }
    }

    /// Enter takes the highlighted suggestion, else the text as typed.
    fn submit(&mut self, typed: &str, window: &mut Window, cx: &mut Context<Self>) {
        let suggestions = self.model.read(cx).suggestions(typed);
        let name = suggestions
            .iter()
            .find(|suggestion| suggestion.name.eq_ignore_ascii_case(typed.trim()))
            .or(suggestions.first())
            .map_or_else(|| typed.to_string(), |suggestion| suggestion.name.clone());
        self.model.update(cx, |model, cx| model.add_name(&name, cx));
        self.stop_editing(window, cx);
    }

    /// Makes the window focusable and gives the field the keyboard. Windows
    /// answers a foreground request only soon after a click on our window, so
    /// this is called from the "+" button's handler; the window calls run on
    /// the next turn, not inside GPUI's event handling. If the edit stopped
    /// before the request was answered, focus is given back at once.
    fn start_editing(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(hwnd) = self.hwnd else {
            return;
        };
        if !self.edit.start() {
            return;
        }
        cx.spawn_in(window, async move |this, cx| {
            if !this.read_with(cx, |this, _| this.edit.wants_focus()).unwrap_or(false) {
                return;
            }
            let taken = typing::begin(hwnd);
            if let Err(error) = &taken {
                crate::app_log::warn(error.to_string());
            }
            let going_on = this
                .update_in(cx, |this, window, cx| {
                    let going_on = this.edit.began(taken.is_ok());
                    if going_on {
                        let focus = this.input.read(cx).focus_handle(cx);
                        window.focus(&focus, cx);
                        cx.notify();
                    }
                    going_on
                })
                .unwrap_or(false);
            if taken.is_ok() && !going_on {
                typing::end(hwnd);
            }
        })
        .detach();
    }

    /// Back to non-activating. Nothing is given focus: Windows picks the
    /// window under ours.
    fn stop_editing(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if !self.edit.stop() {
            return;
        }
        self.leave_timer = None;
        #[cfg(feature = "probes")]
        {
            self.pointer_held_over = false;
        }
        self.input.update(cx, |input, cx| input.clear(cx));
        if let Some(hwnd) = self.hwnd {
            cx.spawn(async move |_, _| typing::end(hwnd)).detach();
        }
        cx.notify();
    }

    /// The window is closing or the app quitting: no later turn to wait for.
    fn release_focus(&mut self) {
        if let Some(hwnd) = self.hwnd.filter(|_| self.edit.stop()) {
            typing::end(hwnd);
        }
    }

    fn pointer_is_over(&self, window: &Window) -> bool {
        #[cfg(feature = "probes")]
        if self.pointer_held_over {
            return true;
        }
        window.is_window_hovered()
    }

    /// An edit ends a moment after the pointer has left the window. GPUI
    /// redraws the window when the pointer enters or leaves it.
    fn watch_pointer(&mut self, window: &Window, cx: &mut Context<Self>) {
        if !self.edit.is_editing() || self.pointer_is_over(window) {
            self.leave_timer = None;
            return;
        }
        if self.leave_timer.is_some() {
            return;
        }
        self.leave_timer = Some(cx.spawn_in(window, async move |this, cx| {
            cx.background_executor().timer(LEAVE_GRACE).await;
            let _ = this.update_in(cx, |this, window, cx| {
                if !this.pointer_is_over(window) {
                    this.stop_editing(window, cx);
                }
                this.leave_timer = None;
            });
        }));
    }
}

impl Render for PvpView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.watch_pointer(window, cx);
        let model = self.model.read(cx);
        let meter = model.meter_model().read(cx);
        let settings = meter.settings.clone();
        let look = Look::of(&settings);
        let s = look.s;
        let nothing_yet =
            model.watch().is_empty() && model.stats().is_empty() && model.dealt().is_empty() && model.received().is_empty();
        let note_shown = nothing_yet || self.show_info;

        let info = chrome::tool_button("info", Icon::Info, fl!("pvp-about-capture"), self.show_info, s)
            .on_click(cx.listener(|this, _, _, cx| {
                this.show_info = !this.show_info;
                cx.notify();
            }))
            .into_any_element();
        let header = chrome::header(chrome::title(Icon::Crosshair, fl!("pvp-title"), None, s), vec![info], &self.drag, s);

        let watching = model.watch().len();
        let watch_head = chrome::section(s)
            .mt_0()
            .child(chrome::icon(Icon::Heart, 13., rgba(MUTED), s))
            .child(fl!("pvp-watching"))
            .when(watching > 0, |head| head.child(chrome::count(watching.to_string(), s).text_size(s.px(11.))))
            .child(div().flex_1())
            .when(!self.edit.is_editing(), |head| {
                head.child(
                    chrome::tool_button("add", Icon::Plus, fl!("pvp-add-placeholder"), false, s)
                        .on_click(cx.listener(|this, _, window, cx| this.start_editing(window, cx))),
                )
            });
        let watch_rows: AnyElement = if model.watch().is_empty() {
            chrome::empty(None, fl!("pvp-watch-empty"), s)
                .pt(s.px(4.))
                .into_any_element()
        } else {
            div()
                .flex()
                .flex_col()
                .gap(s.px(ROW_GAP))
                .children(model.watch().iter().map(|row| self.watch_row(row, &settings, s, cx)))
                .into_any_element()
        };
        let field = (self.edit.is_editing() || model.watch().is_empty()).then(|| self.add_field(s, cx));

        let tabs = chrome::tabs(s)
            .mt(s.px(10.))
            .child(
                chrome::tab("tab-kills", fl!("pvp-tab-kills"), Some(model.stats().len()), self.tab == Tab::Kills, s)
                    .on_click(cx.listener(|this, _, _, cx| this.select_tab(Tab::Kills, cx))),
            )
            .child(
                chrome::tab(
                    "tab-damage",
                    fl!("common-damage"),
                    Some(model.dealt().len() + model.received().len()),
                    self.tab == Tab::Damage,
                    s,
                )
                .on_click(cx.listener(|this, _, _, cx| this.select_tab(Tab::Damage, cx))),
            );
        let note = note_shown.then(|| chrome::note(Icon::Info, fl!("pvp-passive-note"), false, s).mb(s.px(6.)));
        let body: AnyElement = match self.tab {
            Tab::Kills => self.kills(model.stats(), model.sort(), model.main_actor_name(), &settings, s, cx).into_any_element(),
            Tab::Damage => self.damage(meter, &settings, s, cx).into_any_element(),
        };

        let card = chrome::card(WIDTH, look)
            .child(header)
            .child(watch_head)
            .child(watch_rows)
            .children(field)
            .child(tabs)
            .children(note)
            .child(body);
        let suggestions = self.suggestions(&settings, s, cx);
        chrome::root(look).child(card).children(suggestions)
    }
}

impl PvpView {
    fn select_tab(&mut self, tab: Tab, cx: &mut Context<Self>) {
        if self.tab != tab {
            self.tab = tab;
            cx.notify();
        }
    }

    fn add_field(&self, s: Scale, cx: &Context<Self>) -> impl IntoElement {
        let add = chrome::tool_button("add-field", Icon::Plus, fl!("pvp-add-placeholder"), self.edit.is_editing(), s);
        let field = chrome::input_box(self.edit.is_editing(), s).relative().child(self.field.marker());
        let field: AnyElement = if self.edit.is_editing() {
            field
                .child(self.input.clone())
                .child(add.on_click(cx.listener(|this, _, window, cx| {
                    let typed = this.input.read(cx).text().to_string();
                    this.submit(&typed, window, cx);
                })))
                .into_any_element()
        } else {
            field
                .id("add-placeholder")
                .cursor_pointer()
                .on_click(cx.listener(|this, _, window, cx| this.start_editing(window, cx)))
                .child(div().flex_1().min_w_0().truncate().text_color(rgba(FAINT)).child(fl!("pvp-add-placeholder")))
                .child(add)
                .into_any_element()
        };
        div().flex_none().mt(s.px(6.)).child(field)
    }

    fn suggestions(&self, settings: &OverlaySettings, s: Scale, cx: &Context<Self>) -> Option<AnyElement> {
        if !self.edit.is_editing() {
            return None;
        }
        let bounds = self.field.bounds()?;
        let typed = self.input.read(cx).text().to_string();
        let suggestions = self.model.read(cx).suggestions(&typed);
        if suggestions.is_empty() {
            return None;
        }
        let items = suggestions.into_iter().enumerate().map(|(index, suggestion)| {
            let name = suggestion.name.clone();
            chrome::menu_item(("suggestion", index), index == 0, s)
                .on_click(cx.listener(move |this, _, window, cx| {
                    this.model.update(cx, |model, cx| model.add_name(&name, cx));
                    this.stop_editing(window, cx);
                }))
                .child(chrome::icon(Icon::User, 13., rgba(MUTED), s))
                .child(suggestion_name(&suggestion, &typed, settings, s))
                .child(div().flex_none().text_size(s.px(11.)).text_color(rgba(FAINT)).child(fl!("pvp-seen-this-session")))
        });
        Some(
            chrome::popover(s)
                .top(bounds.bottom() + s.px(4.))
                .left(bounds.left())
                .w(bounds.size.width)
                .children(items)
                .with_animation("suggestions", Animation::new(MENU_FADE), |menu, t| menu.opacity(t))
                .into_any_element(),
        )
    }

    fn watch_row(&self, row: &WatchRow, settings: &OverlaySettings, s: Scale, cx: &Context<Self>) -> impl IntoElement {
        let name = settings.display_name(row.actor_name.as_deref().unwrap_or(&row.query_name));
        let server = (settings.show_server && !row.server.is_empty()).then(|| SharedString::from(row.server.clone()));
        let action: AnyElement = if row.removable() {
            let query = row.query_name.clone();
            chrome::row_button(SharedString::from(format!("remove-{}", row.query_name)), Icon::Close, fl!("pvp-stop-watching", name = name.as_str()), s)
                .on_click(cx.listener(move |this, _, _, cx| this.model.update(cx, |model, cx| model.remove_name(&query, cx))))
                .into_any_element()
        } else {
            div().flex_none().w(s.px(20.)).into_any_element()
        };
        if !row.in_range() {
            let content = chrome::bar_content(true, s)
                .child(chrome::name(name, Some(fl!("pvp-not-in-range").into()), FontWeight::MEDIUM, s))
                .child(chrome::figure("--", None, s));
            return chrome::row(s).opacity(0.55).child(track(class_fill(None), 0., ROW, s).child(content)).child(action);
        }
        let fraction = row.hp_fraction();
        let hp = match (row.hp, fraction) {
            (Some((current, max)), Some(_)) => {
                format!("{} / {}", format::grouped(f64::from(current)), format::grouped(f64::from(max)))
            }
            _ => "--".to_string(),
        };
        let content = chrome::bar_content(row.class.is_none(), s)
            .when_some(row.class, |content, class| content.child(chrome::class_icon(class, 16., s)))
            .child(chrome::name(name, server, FontWeight::MEDIUM, s))
            .when(row.last_attacker, |content| {
                content.child(
                    div()
                        .flex_none()
                        .px(s.px(6.))
                        .h(s.px(16.))
                        .flex()
                        .items_center()
                        .rounded_full()
                        .bg(rgba(TAG_FILL))
                        .text_color(rgb(TAG_TEXT))
                        .text_size(s.px(10.5))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(fl!("pvp-last-attacker")),
                )
            })
            .child(div().flex_none().text_size(s.px(11.5)).text_color(rgba(MUTED)).child(hp))
            .child(
                chrome::strong(fraction.map_or_else(|| "--".to_string(), format::whole_percent), s)
                    .w(s.px(34.))
                    .flex()
                    .justify_end(),
            );
        chrome::row(s).child(track(hp_fill(), fraction.unwrap_or(0.0) as f32, ROW, s).child(content)).child(action)
    }

    fn kills(
        &self,
        stats: &[PvpCombatStatsRow],
        sort: StatsSort,
        you: Option<&str>,
        settings: &OverlaySettings,
        s: Scale,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let clear = chrome::text_button("clear", Some(Icon::Trash), fl!("pvp-clear"), ButtonKind::Quiet, 22., s)
            .on_click(cx.listener(|this, _, _, cx| this.model.update(cx, |model, cx| model.clear_stats(cx))));
        let head = chrome::section(s)
            .mt_0()
            .child(fl!("common-players"))
            .child(chrome::count(fl!("pvp-this-session"), s).text_size(s.px(11.)))
            .child(div().flex_1())
            .child(clear);
        if stats.is_empty() {
            return div()
                .flex()
                .flex_col()
                .child(head)
                .child(chrome::empty(Some(fl!("pvp-kills-empty-title").into()), fl!("pvp-kills-empty-line"), s));
        }
        let overflows = stats.len() > KILLS_SHOWN;
        let sortable = |column: StatsSort, label: String, width: f32, id: &'static str| {
            let active = sort == column;
            div()
                .id(id)
                .flex_none()
                .w(s.px(width))
                .flex()
                .justify_end()
                .items_center()
                .gap(s.px(2.))
                .when(active, |cell| {
                    cell.text_color(rgba(TEXT))
                        .font_weight(FontWeight::SEMIBOLD)
                        .child(chrome::icon(Icon::ArrowDown, 10., rgba(TEXT), s))
                        .child(label.clone())
                })
                .when(!active, |cell| {
                    cell.cursor_pointer()
                        .child(div().underline().text_decoration_color(rgba(0xfff6ec40)).child(label))
                        .on_click(cx.listener(move |this, _, _, cx| this.model.update(cx, |model, cx| model.sort_by(column, cx))))
                })
        };
        let table_head = chrome::table_head(s)
            .pl(s.px(8.))
            .pr(s.px(if overflows { 17. } else { 9. }))
            .child(div().flex_1().child(fl!("common-player")))
            .child(sortable(StatsSort::Damage, fl!("common-damage"), 50., "sort-damage"))
            .child(sortable(StatsSort::Kills, fl!("pvp-kills"), 38., "sort-kills"))
            .child(chrome::figure(fl!("pvp-assists"), Some(38.), s))
            .child(chrome::figure(fl!("pvp-deaths"), Some(38.), s));
        let top = stats.iter().map(|row| row.damage).max().unwrap_or(0).max(1);
        let rows = div().flex().flex_col().gap(s.px(ROW_GAP)).children(stats.iter().map(|row| {
            let is_you = you == Some(row.actor_name.as_str());
            let server = game_data::server_name(&row.server_id);
            let content = chrome::bar_content(true, s)
                .child(chrome::name(
                    settings.display_name(&row.actor_name),
                    (!server.is_empty()).then(|| server.into()),
                    if is_you { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM },
                    s,
                ))
                .child(chrome::figure(format::compact(row.damage as f64), Some(50.), s))
                .child(chrome::figure(row.kills.to_string(), Some(38.), s).font_weight(FontWeight::SEMIBOLD))
                .child(chrome::figure(row.assists.to_string(), Some(38.), s))
                .child(chrome::figure(row.deaths.to_string(), Some(38.), s).text_color(rgba(MUTED)));
            chrome::row(s).child(
                track(class_fill(None), (row.damage as f32 / top as f32).max(0.06), ROW, s)
                    .child(content)
                    .when(is_you, |bar| bar.child(chrome::you_rim(s))),
            )
        }));
        let max = KILLS_SHOWN as f32 * ROW + (KILLS_SHOWN - 1) as f32 * ROW_GAP;
        div()
            .flex()
            .flex_col()
            .child(head)
            .child(table_head)
            .child(chrome::scroll_list("kills", &self.kills_scroll, max, overflows, rows, s))
    }

    fn damage(&self, meter: &MeterModel, settings: &OverlaySettings, s: Scale, cx: &Context<Self>) -> impl IntoElement {
        let model = self.model.read(cx);
        let main_actor_id = model.main_actor_id();
        let dealt = self.damage_section(
            fl!("pvp-damage-dealt"),
            fl!("pvp-dealt-empty"),
            "dealt",
            model.dealt(),
            meter,
            settings,
            s,
            cx,
            move |row| main_actor_id.map(|actor| (actor, Some(row.actor_id))),
        );
        let taken = self.damage_section(
            fl!("pvp-damage-taken"),
            fl!("pvp-taken-empty"),
            "taken",
            model.received(),
            meter,
            settings,
            s,
            cx,
            move |row| main_actor_id.map(|target| (row.actor_id, Some(target))),
        );
        let height = section_height(model.dealt().len()) + section_height(model.received().len());
        let content = div().flex().flex_col().child(dealt).child(taken);
        chrome::scroll_list("damage", &self.damage_scroll, DAMAGE_MAX, height > DAMAGE_MAX, content, s)
    }

    /// `detail_of` gives the (player, target) the detail window opens on.
    #[allow(clippy::too_many_arguments)]
    fn damage_section(
        &self,
        title: String,
        empty: String,
        prefix: &'static str,
        rows: &[DamageRow],
        meter: &MeterModel,
        settings: &OverlaySettings,
        s: Scale,
        cx: &Context<Self>,
        detail_of: impl Fn(&DamageRow) -> Option<(u32, Option<u32>)> + Copy + 'static,
    ) -> Div {
        let head = chrome::section(s)
            .mt(s.px(if prefix == "dealt" { 0. } else { 6. }))
            .child(title)
            .when(!rows.is_empty(), |head| {
                head.child(chrome::count(fl!("common-players-count", count = rows.len()), s).text_size(s.px(11.)))
            });
        if rows.is_empty() {
            return div().flex().flex_col().child(head).child(chrome::empty(None, empty, s).pt(s.px(4.)));
        }
        let meter_model = self.model.read(cx).meter_model().clone();
        let list = div().flex().flex_col().gap(s.px(ROW_GAP)).children(rows.iter().map(|row| {
            let colour = row.class.map(|class| meter.player_colour(class, row.actor_id, &row.name));
            let server = (settings.show_server && !row.server.is_empty()).then(|| SharedString::from(row.server.clone()));
            let name = if row.named { settings.display_name(&row.name) } else { row.name.clone() };
            let watch: AnyElement = if row.named {
                let watched = row.name.clone();
                chrome::row_button(SharedString::from(format!("watch-{prefix}-{}", row.actor_id)), Icon::Eye, fl!("pvp-watch-player", name = name.as_str()), s)
                    .on_click(cx.listener(move |this, _, _, cx| {
                        cx.stop_propagation();
                        this.model.update(cx, |model, cx| model.add_name(&watched, cx));
                    }))
                    .into_any_element()
            } else {
                div().flex_none().w(s.px(20.)).into_any_element()
            };
            let content = chrome::bar_content(row.class.is_none(), s)
                .when_some(row.class, |content, class| content.child(chrome::class_icon(class, 16., s)))
                .child(chrome::name(name, server, FontWeight::MEDIUM, s))
                .child(chrome::strong(format::compact(row.damage as f64), s));
            let player = row.clone();
            let meter_model = meter_model.clone();
            chrome::row(s)
                .id(SharedString::from(format!("{prefix}-{}", row.actor_id)))
                .cursor_pointer()
                .on_click(move |_, _, cx| {
                    if let Some((actor_id, target_id)) = detail_of(&player) {
                        secondary::open_detail(&meter_model, DetailSource::Target { actor_id, target_id }, cx);
                    }
                })
                .child(track(class_fill(colour), row.bar.max(0.06), ROW, s).child(content))
                .child(chrome::percent(format::percent(row.share), s))
                .child(watch)
        }));
        div().flex().flex_col().child(head).child(list)
    }
}

/// A damage section's label row and its rows (or its one-line empty text).
fn section_height(rows: usize) -> f32 {
    let body = if rows == 0 { 34. } else { rows as f32 * ROW + (rows - 1) as f32 * ROW_GAP };
    30. + body
}

/// The suggestion with the typed part in bold; a masked name stays plain.
fn suggestion_name(suggestion: &Suggestion, typed: &str, settings: &OverlaySettings, s: Scale) -> impl IntoElement {
    let name = suggestion.name.as_str();
    let typed = typed.trim();
    let found = (!settings.mask_nicknames && !typed.is_empty())
        .then(|| name.to_lowercase().find(&typed.to_lowercase()))
        .flatten()
        .filter(|start| name.is_char_boundary(*start) && name.is_char_boundary(start + typed.len()));
    let line = div().flex_1().min_w_0().flex().overflow_hidden().font_weight(FontWeight::MEDIUM);
    match found {
        Some(start) => {
            let end = start + typed.len();
            line.child(name[..start].to_string())
                .child(div().font_weight(FontWeight::BOLD).child(name[start..end].to_string()))
                .child(name[end..].to_string())
        }
        None => line.child(settings.display_name(name)),
    }
    .text_size(s.px(12.5))
}

#[cfg(feature = "probes")]
impl PvpView {
    /// Starts an edit without asking Windows for the foreground, which a
    /// capture must never do: the field is focused inside GPUI only.
    pub fn start_editing_without_activation(
        &mut self,
        pointer_over: bool,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.edit = Edit::Editing;
        self.pointer_held_over = pointer_over;
        let focus = self.input.read(cx).focus_handle(cx);
        window.focus(&focus, cx);
        cx.notify();
    }

    pub fn is_editing(&self) -> bool {
        self.edit.is_editing()
    }

    pub fn show_tab(&mut self, tab: Tab, cx: &mut Context<Self>) {
        self.tab = tab;
        cx.notify();
    }
}
