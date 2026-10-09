//! The fight report window as approved in docs/gpui-mockups/approved/report.html
//! (values in tokens.md, "Fight report"): the fight, its key figures, one
//! sortable row per player, the deaths, and a player's details in place of
//! the table. Plates keep their figures at the left of each cell; tables put
//! figures and headers at the right of each column.
//!
//! The window never takes the keyboard from the game, so the focus handling
//! serves accessibility tools: the sort headers, rows and death rows are
//! buttons, Up and Down move between rows, Left and Right between tabs.

use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;

use gpui::{
    div, img, prelude::*, px, relative, rgb, rgba, AnyElement, App, Context, Div, Entity, FocusHandle, FontWeight,
    IntoElement, KeyDownEvent, Render, Role, ScrollHandle, SharedString, Stateful, Subscription, Window,
};
use meter_core::report::fight_report::{FightOutcome, HitTypes};

use super::data::{BuffLine, Column, DeathRow, HealLine, PlayerDetail, PlayerRow, ReportData, SkillLine, TakenLine};
use super::model::ReportModel;
use crate::assets::{Icon, COMBAT_POWER};
use crate::format;
use crate::i18n::fl;
use crate::meter::OverlaySettings;
use crate::overlay::WindowDrag;
use crate::secondary::chrome::{self, Look, Scale, Tone, BAD, CREAM, FAINT, GOOD, MUTED, TEXT};
use crate::settings::{reveal_marker, Reveal};
use crate::style::{bar, class_fill, hp_fill, track, TRACK};

const WIDTH: f32 = 520.0;
const ROW: f32 = 22.0;
const ROW_GAP: f32 = 3.0;
/// The player table scrolls past this many rows.
const ROWS_SHOWN: usize = 8;
const SKILL_ROW: f32 = 24.0;
/// The skill, healing and damage taken tables scroll past this many rows.
const SKILLS_SHOWN: usize = 9;
/// An opened skill row counts as this many rows.
const OPENED_ROWS: usize = 4;
const BUFF_LIST: f32 = 286.0;
/// Room around the rows of a scrolling list, so a focus ring is never clipped.
const RING_ROOM: f32 = 3.0;
const LIFT: u32 = 0xfff0e112;
const DEATH_HOVER: u32 = 0x28242099;
const DEATHS_PLATE: u32 = 0xf7a8a81a;
const EXPANDED_FILL: u32 = 0xfff0e10d;
const SPEC_OFF: u32 = 0xfff0e129;
const NO_ICON: u32 = 0xfff0e11a;
const UNDERLINE: u32 = 0xfff6ec40;
const BACK_HOT: u32 = 0xfff0e121;
const FOCUS_RING: u32 = 0xf3ebe1e6;
const TABS: [Tab; 4] = [Tab::Damage, Tab::Buffs, Tab::Healing, Tab::Taken];

/// A player's details open on a tab; the tab is kept for the next player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tab {
    Damage,
    Buffs,
    Healing,
    Taken,
}

impl Tab {
    fn key(self) -> &'static str {
        match self {
            Self::Damage => "tab:damage",
            Self::Buffs => "tab:buffs",
            Self::Healing => "tab:healing",
            Self::Taken => "tab:taken",
        }
    }
}

/// What the pointer is in, from that element's enter and leave only.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Pointed {
    Player(u32),
    /// A death row, by its place in the list.
    Death(usize),
    Skill(u32),
    Back,
}

struct Focus {
    handle: FocusHandle,
    /// Focused, and the last input came from the keyboard.
    ring: bool,
    reveal: Option<Reveal>,
}

pub struct ReportView {
    model: Entity<ReportModel>,
    sort: Column,
    tab: Tab,
    open_skill: Option<u32>,
    pointer_on: Option<Pointed>,
    /// The player whose details show, to close their opened skill when
    /// another player opens.
    shown_player: Option<u32>,
    controls: HashMap<String, FocusHandle>,
    root_focus: FocusHandle,
    /// Up and Down move along these, in the order they were drawn.
    rows_drawn: Vec<String>,
    table_scroll: ScrollHandle,
    skills_scroll: ScrollHandle,
    buffs_scroll: ScrollHandle,
    reveal: Rc<Cell<bool>>,
    drag: WindowDrag,
    /// Dev captures draw a focus ring without a keyboard.
    #[cfg(feature = "probes")]
    forced_ring: Option<String>,
    _subscriptions: Vec<Subscription>,
}

impl ReportView {
    pub fn new(model: Entity<ReportModel>, window: &mut Window, cx: &mut Context<Self>) -> Self {
        let shown_player = model.read(cx).player().map(|player| player.row.actor_id);
        let subscriptions = vec![
            cx.observe(&model, |this, model, cx| {
                let shown = model.read(cx).player().map(|player| player.row.actor_id);
                if shown != this.shown_player {
                    this.shown_player = shown;
                    this.open_skill = None;
                }
                cx.notify();
            }),
            // A focused row that goes away (the report replaced) leaves the
            // focus with the window, so Tab keeps working.
            cx.on_focus_lost(window, |this, window, cx| window.focus(&this.root_focus, cx)),
        ];
        let root_focus = cx.focus_handle();
        window.focus(&root_focus, cx);
        Self {
            model,
            sort: Column::Damage,
            tab: Tab::Damage,
            open_skill: None,
            pointer_on: None,
            shown_player,
            controls: HashMap::new(),
            root_focus,
            rows_drawn: Vec::new(),
            table_scroll: ScrollHandle::new(),
            skills_scroll: ScrollHandle::new(),
            buffs_scroll: ScrollHandle::new(),
            reveal: Rc::new(Cell::new(false)),
            drag: WindowDrag::default(),
            #[cfg(feature = "probes")]
            forced_ring: None,
            _subscriptions: subscriptions,
        }
    }

    // ---- Focus ----

    fn handle(&mut self, key: &str, cx: &App) -> FocusHandle {
        if let Some(handle) = self.controls.get(key) {
            return handle.clone();
        }
        let handle = cx.focus_handle().tab_stop(true);
        self.controls.insert(key.to_string(), handle.clone());
        handle
    }

    /// `scroll`: the list the control scrolls with, which brings it into
    /// view when the keyboard moves to it.
    fn focus_of(&mut self, key: &str, scroll: Option<&ScrollHandle>, window: &Window, cx: &App) -> Focus {
        let handle = self.handle(key, cx);
        let focused = handle.is_focused(window);
        #[cfg(feature = "probes")]
        let forced = self.forced_ring.as_deref() == Some(key);
        #[cfg(not(feature = "probes"))]
        let forced = false;
        Focus {
            ring: forced || (focused && window.last_input_was_keyboard()),
            reveal: scroll
                .filter(|_| focused)
                .map(|scroll| Reveal { scroll: scroll.clone(), pending: self.reveal.clone() }),
            handle,
        }
    }

    fn focus_key(&mut self, key: &str, window: &mut Window, cx: &mut App) {
        let handle = self.handle(key, cx);
        window.focus(&handle, cx);
        self.reveal.set(true);
    }

    fn focused_key(&self, window: &Window) -> Option<&str> {
        self.controls.iter().find(|(_, handle)| handle.is_focused(window)).map(|(key, _)| key.as_str())
    }

    fn on_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let modifiers = event.keystroke.modifiers;
        if modifiers.control || modifiers.alt || modifiers.platform {
            return;
        }
        match event.keystroke.key.as_str() {
            "tab" => {
                cx.stop_propagation();
                if modifiers.shift {
                    window.focus_prev(cx);
                } else {
                    window.focus_next(cx);
                }
                self.reveal.set(true);
                cx.notify();
            }
            key @ ("up" | "down") => {
                let Some(at) = self.focused_key(window).and_then(|focused| self.rows_drawn.iter().position(|row| row == focused))
                else {
                    return;
                };
                let next = if key == "up" { at.checked_sub(1) } else { Some(at + 1).filter(|next| *next < self.rows_drawn.len()) };
                cx.stop_propagation();
                if let Some(next) = next {
                    let next = self.rows_drawn[next].clone();
                    self.focus_key(&next, window, cx);
                    cx.notify();
                }
            }
            key @ ("left" | "right") => {
                let Some(at) = self.focused_key(window).and_then(|focused| TABS.iter().position(|tab| tab.key() == focused)) else {
                    return;
                };
                cx.stop_propagation();
                let next = if key == "left" { at.saturating_sub(1) } else { (at + 1).min(TABS.len() - 1) };
                self.tab = TABS[next];
                self.focus_key(TABS[next].key(), window, cx);
                cx.notify();
            }
            _ => {}
        }
    }

    // ---- Actions ----

    fn point(&mut self, target: Pointed, hovered: bool, cx: &mut Context<Self>) {
        let now = if hovered {
            Some(target)
        } else if self.pointer_on == Some(target) {
            None
        } else {
            return;
        };
        if self.pointer_on != now {
            self.pointer_on = now;
            cx.notify();
        }
    }

    fn sort_by(&mut self, column: Column, cx: &mut Context<Self>) {
        if self.sort != column {
            self.sort = column;
            cx.notify();
        }
    }

    fn open_player(&mut self, actor_id: u32, window: &mut Window, cx: &mut Context<Self>) {
        self.pointer_on = None;
        self.model.update(cx, |model, cx| model.open_player(actor_id, cx));
        if self.model.read(cx).player().is_some() {
            self.focus_key("back", window, cx);
        }
    }

    /// Back to the table, with the keyboard on the player who was open.
    fn back(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let was_open = self.model.read(cx).player().map(|player| player.row.actor_id);
        self.pointer_on = None;
        self.model.update(cx, |model, cx| model.back(cx));
        if let Some(actor_id) = was_open {
            self.focus_key(&format!("row:{actor_id}"), window, cx);
        }
    }

    fn show_tab(&mut self, tab: Tab, cx: &mut Context<Self>) {
        if self.tab != tab {
            self.tab = tab;
            cx.notify();
        }
    }
}

impl Render for ReportView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let model = self.model.clone();
        let model = model.read(cx);
        let settings = model.meter_model().read(cx).settings.clone();
        let look = Look::of(&settings);
        self.rows_drawn.clear();
        let card = match model.player() {
            Some(player) => self.player_page(model.data(), player, look, window, cx),
            None => self.report_page(model.data(), &settings, look, window, cx),
        };
        chrome::root(look).track_focus(&self.root_focus).on_key_down(cx.listener(Self::on_key)).child(card)
    }
}

// ---- The report ----

impl ReportView {
    fn report_page(&mut self, data: &ReportData, settings: &OverlaySettings, look: Look, window: &Window, cx: &Context<Self>) -> Stateful<Div> {
        let s = look.s;
        let recorded = data.is_recorded();
        let header = chrome::header(
            chrome::title(Icon::FileText, fl!("report-title"), None, s),
            vec![outcome_chip(data.outcome, s).into_any_element()],
            &self.drag,
            s,
        );
        let boss = div()
            .flex_none()
            .pt(s.px(2.))
            .px(s.px(4.))
            .flex()
            .flex_col()
            .child(
                div()
                    .flex()
                    .items_baseline()
                    .gap(s.px(8.))
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(s.px(14.5))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(data.boss.clone()),
                    )
                    .child(
                        div()
                            .flex_none()
                            .text_size(s.px(14.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(format::duration(data.duration_secs)),
                    ),
            )
            .child(div().mt(s.px(3.)).truncate().text_size(s.px(11.5)).text_color(rgba(MUTED)).child(data.sub_line.clone()));
        let totals = &data.totals;
        let figures = plate(6., 7., s)
            .child(figure(fl!("common-damage"), Some(format::compact(totals.damage as f64)), false, s))
            .child(figure(fl!("meter-summary-party-dps"), Some(format::compact(totals.party_dps)), false, s))
            .child(figure(fl!("report-healing"), totals.healing.map(|healing| format::compact(healing as f64)), false, s))
            .child(figure(fl!("report-damage-taken"), totals.taken.map(|taken| format::compact(taken as f64)), false, s))
            .child(figure(fl!("report-deaths"), totals.deaths.map(|deaths| deaths.to_string()), false, s));
        let note = (!recorded).then(|| chrome::note(Icon::Info, fl!("report-not-recorded-note"), false, s).mt(s.px(6.)));
        let table = self.table(data, settings.show_server, s, window, cx);
        let deaths = self.deaths(data, s, window, cx);
        let footer = chrome::footer(s).child(
            div()
                .min_w_0()
                .flex()
                .items_center()
                .gap(s.px(5.))
                .text_color(rgba(FAINT))
                .child(chrome::icon(Icon::MousePointer, 12., rgba(FAINT), s))
                .child(div().min_w_0().truncate().child(fl!("report-hint"))),
        );
        chrome::card(WIDTH, look)
            .id("report")
            .debug_selector(|| "report".to_string())
            .role(Role::Dialog)
            .aria_label(fl!("report-title"))
            .child(header)
            .child(boss)
            .child(figures)
            .children(note)
            .child(table)
            .child(deaths)
            .child(footer)
    }

    fn table(&mut self, data: &ReportData, show_server: bool, s: Scale, window: &Window, cx: &Context<Self>) -> Div {
        let recorded = data.is_recorded();
        let rows = data.ranked(self.sort);
        let overflows = rows.len() > ROWS_SHOWN;
        let mut head = chrome::table_head(s)
            .mt(s.px(6.))
            .pl(s.px(52.))
            .pr(s.px(if overflows { 35. } else { 27. }))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .gap(s.px(6.))
                    .child(fl!("common-player"))
                    .child(rows.len().to_string()),
            );
        for column in Column::ALL {
            head = head.child(self.sort_header(column, recorded, s, window, cx));
        }
        let top = rows.first().map_or(0.0, |row| row.value(self.sort));
        let scroll = self.table_scroll.clone();
        let list: Vec<AnyElement> = rows
            .iter()
            .enumerate()
            .map(|(index, row)| self.player_row(row, index + 1, top, recorded, show_server, &scroll, s, window, cx))
            .collect();
        let content = div().flex().flex_col().gap(s.px(ROW_GAP)).children(list);
        let max = rows_height(ROWS_SHOWN, ROW);
        div().flex_none().flex().flex_col().child(head).child(ring_room_list("players", &scroll, max, overflows, content, s))
    }

    fn sort_header(&mut self, column: Column, recorded: bool, s: Scale, window: &Window, cx: &Context<Self>) -> AnyElement {
        let label = column_label(column);
        let cell = div().flex_none().w(s.px(column_width(column))).flex().justify_end().items_center().gap(s.px(2.));
        if !recorded && !column.is_in_every_fight() {
            return cell.debug_selector(move || format!("head:{}", column.key())).child(label).into_any_element();
        }
        let key = format!("sort:{}", column.key());
        let focus = self.focus_of(&key, None, window, cx);
        let active = column == self.sort;
        let selector = key.clone();
        let element = cell
            .id(SharedString::from(key))
            .debug_selector(move || selector.clone())
            .role(Role::Button)
            .aria_label(fl!("report-sort-by", column = label.clone()))
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| this.sort_by(column, cx)))
            .when(active, |cell| {
                cell.text_color(rgba(TEXT))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(label.clone())
                    .child(chrome::icon(Icon::ArrowDown, 10., rgba(TEXT), s))
            })
            .when(!active, |cell| cell.child(div().underline().text_decoration_color(rgba(UNDERLINE)).child(label.clone())));
        focusable(element, &focus, 4., s).into_any_element()
    }

    #[allow(clippy::too_many_arguments)]
    fn player_row(
        &mut self,
        row: &PlayerRow,
        rank: usize,
        top: f64,
        recorded: bool,
        show_server: bool,
        scroll: &ScrollHandle,
        s: Scale,
        window: &Window,
        cx: &Context<Self>,
    ) -> AnyElement {
        let key = format!("row:{}", row.actor_id);
        self.rows_drawn.push(key.clone());
        let focus = self.focus_of(&key, Some(scroll), window, cx);
        let actor_id = row.actor_id;
        let pointed = self.pointer_on == Some(Pointed::Player(actor_id));
        let note = (show_server && !row.server.is_empty()).then(|| SharedString::from(row.server.clone()));
        let weight = if row.you { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM };
        let mut content = chrome::bar_content(row.class.is_none(), s)
            .children(row.class.map(|class| chrome::class_icon(class, 16., s)))
            .child(chrome::name(row.name.clone(), note, weight, s));
        for column in Column::ALL {
            content = content.child(cell(row, column, column == self.sort, recorded, s));
        }
        let content = content.child(chevron_slot(pointed || focus.ring, s));
        let bar = track(class_fill(row.colour), bar_length(row.value(self.sort), top), ROW, s)
            .when(pointed, |bar| bar.child(lift(s)))
            .child(content)
            .when(row.you, |bar| bar.child(chrome::you_rim(s)));
        let selector = key.clone();
        let element = chrome::row(s)
            .id(SharedString::from(key))
            .debug_selector(move || selector.clone())
            .role(Role::Button)
            .aria_label(fl!("report-open-player", name = row.name.clone()))
            .cursor_pointer()
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| this.point(Pointed::Player(actor_id), *hovered, cx)))
            .on_click(cx.listener(move |this, _, window, cx| this.open_player(actor_id, window, cx)))
            .child(chrome::rank_badge(rank, row.you, s))
            .child(bar);
        focusable(element, &focus, 5., s).into_any_element()
    }

    fn deaths(&mut self, data: &ReportData, s: Scale, window: &Window, cx: &Context<Self>) -> Div {
        let label = chrome::section(s)
            .gap(s.px(7.))
            .child(chrome::icon(Icon::Skull, 12., rgba(FAINT), s))
            .child(fl!("report-deaths"))
            .children(data.deaths.as_ref().map(|deaths| small_count(deaths.len(), s)));
        let section = div().flex_none().flex().flex_col().child(label);
        let Some(deaths) = &data.deaths else {
            return section.child(quiet(None, fl!("report-deaths-not-recorded"), rgba(MUTED).into(), s));
        };
        if deaths.is_empty() {
            return section.child(quiet(Some(Icon::Check), fl!("report-nobody-died"), rgb(GOOD).into(), s));
        }
        let head = chrome::table_head(s)
            .pl(s.px(6.))
            .pr(s.px(9.))
            .child(div().flex_none().w(s.px(32.)).flex().justify_end().child(fl!("report-time")))
            .child(div().flex_none().w(s.px(128.)).pl(s.px(22.)).child(fl!("common-player")))
            .child(div().flex_1().min_w_0().truncate().child(fl!("report-killed-by")))
            .child(div().flex_none().child(fl!("report-last-hit")));
        let rows: Vec<AnyElement> = deaths
            .iter()
            .enumerate()
            .map(|(index, death)| {
                let opens = data.players.iter().any(|row| row.actor_id == death.actor_id);
                self.death_row(index, death, opens, s, window, cx)
            })
            .collect();
        section.child(head).child(div().flex().flex_col().gap(s.px(ROW_GAP)).children(rows))
    }

    fn death_row(&mut self, index: usize, death: &DeathRow, opens: bool, s: Scale, window: &Window, cx: &Context<Self>) -> AnyElement {
        let key = format!("death:{index}");
        self.rows_drawn.push(key.clone());
        let focus = self.focus_of(&key, None, window, cx);
        let pointed = self.pointer_on == Some(Pointed::Death(index));
        let time = format::duration(death.at_secs);
        let label = fl!(
            "report-death-label",
            name = death.name.clone(),
            time = time.clone(),
            killer = death.killer.clone(),
            skill = death.skill.clone()
        );
        let who = div()
            .flex_none()
            .w(s.px(128.))
            .min_w_0()
            .flex()
            .items_center()
            .gap(s.px(6.))
            .children(death.class.map(|class| chrome::class_icon(class, 16., s)))
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .font_weight(if death.you { FontWeight::SEMIBOLD } else { FontWeight::MEDIUM })
                    .child(death.name.clone()),
            );
        let by = div()
            .flex_1()
            .min_w_0()
            .overflow_hidden()
            .flex()
            .items_baseline()
            .child(div().flex_none().mr(s.px(4.)).text_size(s.px(11.5)).text_color(rgba(FAINT)).child(fl!("report-by")))
            .child(div().min_w_0().truncate().child(death.killer.clone()))
            .child(
                div()
                    .min_w_0()
                    .ml(s.px(5.))
                    .truncate()
                    .text_size(s.px(11.5))
                    .text_color(rgba(MUTED))
                    .child(death.skill.clone()),
            );
        let actor_id = death.actor_id;
        let selector = key.clone();
        let element = div()
            .id(SharedString::from(key))
            .debug_selector(move || selector.clone())
            .role(Role::Button)
            .aria_label(label)
            .flex_none()
            .h(s.px(24.))
            .pl(s.px(6.))
            .pr(s.px(9.))
            .flex()
            .items_center()
            .gap(s.px(6.))
            .rounded(s.px(5.))
            .bg(rgba(if pointed { DEATH_HOVER } else { TRACK }))
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| this.point(Pointed::Death(index), *hovered, cx)))
            .when(opens, |row| {
                row.cursor_pointer().on_click(cx.listener(move |this, _, window, cx| this.open_player(actor_id, window, cx)))
            })
            .child(
                div()
                    .flex_none()
                    .w(s.px(32.))
                    .flex()
                    .justify_end()
                    .text_size(s.px(12.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgba(MUTED))
                    .child(time),
            )
            .child(who)
            .child(by)
            .child(
                div()
                    .flex_none()
                    .text_size(s.px(12.))
                    .text_color(rgba(MUTED))
                    .child(death.last_hit.map_or_else(|| "--".to_string(), |hit| format::compact(hit as f64))),
            );
        focusable(element, &focus, 5., s).into_any_element()
    }
}

// ---- A player's details ----

impl ReportView {
    fn player_page(&mut self, data: &ReportData, player: &PlayerDetail, look: Look, window: &Window, cx: &Context<Self>) -> Stateful<Div> {
        let s = look.s;
        let recorded = data.is_recorded();
        let row = &player.row;
        let header = self.player_header(player, s, window, cx);
        let fight = div()
            .flex_none()
            .h(s.px(18.))
            .px(s.px(4.))
            .flex()
            .items_center()
            .gap(s.px(6.))
            .text_size(s.px(11.5))
            .text_color(rgba(MUTED))
            .child(chrome::icon(Icon::FileText, 12., rgba(MUTED), s))
            .child(div().min_w_0().truncate().child(data.boss.clone()))
            .child(div().flex_none().text_color(rgba(FAINT)).child("·"))
            .child(div().flex_none().child(data.when.clone()))
            .child(div().flex_none().text_color(rgba(FAINT)).child("·"))
            .child(div().flex_none().child(format::duration(data.duration_secs)));
        let counted = |value: Option<u64>| value.map(|value| if value == 0 { "0".to_string() } else { format::compact(value as f64) });
        let figures = plate(6., 7., s)
            .child(figure(fl!("common-damage"), Some(format::compact(row.damage as f64)), false, s))
            .child(figure(fl!("common-dps"), Some(format::compact(row.dps)), false, s))
            .child(figure(fl!("report-healing"), counted(row.healing), row.healing == Some(0), s))
            .child(figure(fl!("report-damage-taken"), counted(row.taken), row.taken == Some(0), s))
            .child(figure(fl!("report-deaths"), row.deaths.map(|deaths| deaths.to_string()), row.deaths == Some(0), s));
        let note = (!recorded).then(|| chrome::note(Icon::Info, fl!("report-not-recorded-note"), false, s).mt(s.px(4.)));
        let body = match self.tab {
            Tab::Damage => self.skills(player, s, window, cx),
            Tab::Buffs => self.buffs(data, player, s),
            Tab::Healing => self.heals(player, s),
            Tab::Taken => self.taken(player, s),
        };
        chrome::card(WIDTH, look)
            .id("report-player")
            .debug_selector(|| "report-player".to_string())
            .role(Role::Dialog)
            .aria_label(row.name.clone())
            .child(header)
            .child(fight)
            .child(figures)
            .child(hit_types(&player.hit_types, s))
            .children((!player.deaths.is_empty()).then(|| player_deaths(&player.deaths, s)))
            .children(note)
            .child(self.tabs(player, recorded, s, window, cx))
            .child(body)
    }

    fn player_header(&mut self, player: &PlayerDetail, s: Scale, window: &Window, cx: &Context<Self>) -> Div {
        let focus = self.focus_of("back", None, window, cx);
        let hot = self.pointer_on == Some(Pointed::Back);
        let back = div()
            .id("back")
            .debug_selector(|| "back".to_string())
            .role(Role::Button)
            .aria_label(fl!("report-back"))
            .flex_none()
            .h(s.px(22.))
            .pl(s.px(6.))
            .pr(s.px(9.))
            .flex()
            .items_center()
            .gap(s.px(5.))
            .rounded_full()
            .cursor_pointer()
            .when(hot, |back| back.bg(rgba(BACK_HOT)))
            .text_size(s.px(12.))
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(rgba(MUTED))
            .child(chrome::icon(Icon::ChevronLeft, 12., rgba(MUTED), s))
            .child(fl!("report-back"))
            .on_hover(cx.listener(|this, hovered: &bool, _, cx| this.point(Pointed::Back, *hovered, cx)))
            .on_click(cx.listener(|this, _, window, cx| this.back(window, cx)));
        let row = &player.row;
        let who = div()
            .min_w_0()
            .flex()
            .items_center()
            .gap(s.px(7.))
            .children(row.class.map(|class| chrome::class_icon(class, 20., s)))
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_size(s.px(14.5))
                    .font_weight(FontWeight::SEMIBOLD)
                    .child(row.name.clone()),
            )
            .when(!row.server.is_empty(), |who| {
                who.child(div().flex_none().text_size(s.px(12.)).text_color(rgba(MUTED)).child(row.server.clone()))
            });
        let power = player.combat_power.map(|power| {
            div()
                .flex_none()
                .flex()
                .items_center()
                .gap(s.px(4.))
                .text_size(s.px(12.))
                .text_color(rgba(MUTED))
                .child(img(COMBAT_POWER).flex_none().size(s.px(14.)))
                .child(format::grouped(power as f64))
        });
        div()
            .flex_none()
            .h(s.px(28.))
            .flex()
            .items_center()
            .gap(s.px(8.))
            .child(focusable(back, &focus, 11., s))
            .child(self.drag.handle(div().flex_1().min_w_0().h_full().flex().items_center().gap(s.px(8.))).child(who).children(power))
            .child(chrome::close_button(s))
            .child(self.drag.listener())
    }

    fn tabs(&mut self, player: &PlayerDetail, recorded: bool, s: Scale, window: &Window, cx: &Context<Self>) -> Stateful<Div> {
        let buffs = player.buffs_on_player.len() + player.buffs_on_boss.len();
        let mut tabs = chrome::tabs(s).mt(s.px(8.)).id("tabs").role(Role::TabList);
        for tab in TABS {
            let (label, count, dimmed) = match tab {
                Tab::Damage => (fl!("report-tab-damage"), Some(player.skills.len()), false),
                Tab::Buffs => (fl!("report-tab-buffs"), Some(buffs), false),
                Tab::Healing => (fl!("report-tab-healing"), player.heals.as_ref().map(Vec::len), !recorded),
                Tab::Taken => (fl!("report-tab-taken"), player.taken.as_ref().map(Vec::len), !recorded),
            };
            let key = tab.key();
            let focus = self.focus_of(key, None, window, cx);
            let on = self.tab == tab;
            let element = chrome::tab(key, label.clone(), count, on, s)
                .debug_selector(move || key.to_string())
                .role(Role::Tab)
                .aria_label(label)
                .aria_selected(on)
                .when(dimmed, |tab| tab.opacity(0.4))
                .on_click(cx.listener(move |this, _, _, cx| this.show_tab(tab, cx)));
            tabs = tabs.child(focusable(element, &focus, 6., s).flex_1());
        }
        tabs
    }

    fn skills(&mut self, player: &PlayerDetail, s: Scale, window: &Window, cx: &Context<Self>) -> Div {
        let skills = &player.skills;
        let opened = self.open_skill.filter(|code| skills.iter().any(|skill| skill.code == *code));
        let rows = skills.len() + if opened.is_some() { OPENED_ROWS } else { 0 };
        let overflows = rows > SKILLS_SHOWN;
        let head = skill_head(fl!("detail-skill"), skills.len(), 30., overflows, s)
            .child(head_figure(fl!("detail-hits"), 40., s))
            .child(head_figure(fl!("report-crits"), 40., s))
            .child(head_figure(fl!("report-crit-rate"), 46., s))
            .child(sorted_head(fl!("common-damage"), s))
            .child(div().flex_none().w(s.px(14.)));
        let top = skills.first().map_or(0, |skill| skill.damage) as f64;
        let scroll = self.skills_scroll.clone();
        let list: Vec<AnyElement> = skills
            .iter()
            .map(|skill| {
                let open = opened == Some(skill.code);
                let row = self.skill_row(skill, player.class_colour, top, open, &scroll, s, window, cx);
                div().flex().flex_col().child(row).when(open, |row| row.child(more(skill, s))).into_any_element()
            })
            .collect();
        let content = div().flex().flex_col().gap(s.px(ROW_GAP)).children(list);
        div()
            .flex_none()
            .flex()
            .flex_col()
            .child(head)
            .child(ring_room_list("skills", &scroll, rows_height(SKILLS_SHOWN, SKILL_ROW), overflows, content, s))
    }

    #[allow(clippy::too_many_arguments)]
    fn skill_row(
        &mut self,
        skill: &SkillLine,
        colour: Option<u32>,
        top: f64,
        open: bool,
        scroll: &ScrollHandle,
        s: Scale,
        window: &Window,
        cx: &Context<Self>,
    ) -> AnyElement {
        let key = format!("skill:{}", skill.code);
        self.rows_drawn.push(key.clone());
        let focus = self.focus_of(&key, Some(scroll), window, cx);
        let code = skill.code;
        let pointed = self.pointer_on == Some(Pointed::Skill(code));
        let hits = &skill.hit_types;
        let content = chrome::bar_content(false, s)
            .child(skill_icon(skill.icon.as_deref(), 18., 4., s))
            .child(chrome::name(skill.name.clone(), None, FontWeight::MEDIUM, s))
            .child(side_figure(format::grouped(f64::from(hits.hits)), 40., s))
            .child(side_figure(format::grouped(f64::from(hits.critical)), 40., s))
            .child(side_figure(rate(hits.rate(hits.critical)), 46., s))
            .child(total_figure(skill.damage, skill.share, s))
            .child(
                div()
                    .flex_none()
                    .w(s.px(14.))
                    .flex()
                    .justify_end()
                    .child(chrome::icon(if open { Icon::ChevronUp } else { Icon::ChevronDown }, 12., rgba(if pointed { TEXT } else { FAINT }), s)),
            );
        let bar = track(class_fill(colour), bar_length(skill.damage as f64, top), SKILL_ROW, s)
            .when(pointed, |bar| bar.child(lift(s)))
            .child(content);
        let selector = key.clone();
        // The track clips its fill, so the ring goes around a frame of its own.
        let element = div()
            .id(SharedString::from(key))
            .debug_selector(move || selector.clone())
            .role(Role::Button)
            .aria_label(SharedString::from(skill.name.clone()))
            .aria_expanded(open)
            .flex_none()
            .h(s.px(SKILL_ROW))
            .flex()
            .cursor_pointer()
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| this.point(Pointed::Skill(code), *hovered, cx)))
            .on_click(cx.listener(move |this, _, _, cx| {
                this.open_skill = if this.open_skill == Some(code) { None } else { Some(code) };
                cx.notify();
            }))
            .child(bar);
        focusable(element, &focus, 5., s).into_any_element()
    }

    fn buffs(&self, data: &ReportData, player: &PlayerDetail, s: Scale) -> Div {
        let head = div()
            .flex_none()
            .h(s.px(20.))
            .pl(s.px(4.))
            .pr(s.px(12.))
            .flex()
            .items_center()
            .gap(s.px(8.))
            .text_size(s.px(11.))
            .text_color(rgba(FAINT))
            .child(div().flex_1().min_w_0().truncate().child(fl!("report-buff-and-caster")))
            .child(div().flex_none().w(s.px(62.)).flex().justify_end().child(fl!("report-up-for")))
            .child(
                div()
                    .flex_none()
                    .w(s.px(40.))
                    .flex()
                    .justify_end()
                    .child(fl!("report-of-fight", time = format::duration(data.duration_secs))),
            );
        let section = |title: String, buffs: &[BuffLine], first: bool| {
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .flex_none()
                        .h(s.px(24.))
                        .when(!first, |label| label.mt(s.px(4.)))
                        .px(s.px(4.))
                        .flex()
                        .items_center()
                        .gap(s.px(7.))
                        .text_size(s.px(12.))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgba(MUTED))
                        .child(div().min_w_0().truncate().child(title))
                        .child(small_count(buffs.len(), s)),
                )
                .child(
                    div()
                        .pt(s.px(2.))
                        .px(s.px(4.))
                        .pb(s.px(4.))
                        .flex()
                        .flex_col()
                        .gap(s.px(7.))
                        .children(buffs.iter().map(|buff| buff_row(buff, s))),
                )
        };
        let content = div()
            .flex()
            .flex_col()
            .child(section(fl!("report-on", name = player.row.name.clone()), &player.buffs_on_player, true))
            .child(section(fl!("report-on", name = data.boss.clone()), &player.buffs_on_boss, false));
        div()
            .flex_none()
            .flex()
            .flex_col()
            .child(head)
            .child(chrome::scroll_list("buffs", &self.buffs_scroll, BUFF_LIST, true, content, s))
    }

    fn heals(&self, player: &PlayerDetail, s: Scale) -> Div {
        let Some(heals) = &player.heals else {
            return not_recorded(fl!("report-not-recorded-healing"), s);
        };
        if heals.is_empty() {
            return chrome::empty(
                Some(fl!("report-no-healing-title").into()),
                fl!("report-no-healing-line", name = player.row.name.clone()),
                s,
            );
        }
        let overflows = heals.len() > SKILLS_SHOWN;
        let head = skill_head(fl!("detail-skill"), heals.len(), 30., overflows, s)
            .child(head_figure(fl!("report-heals"), 40., s))
            .child(head_figure(fl!("report-crits"), 40., s))
            .child(head_figure(fl!("report-crit-rate"), 46., s))
            .child(sorted_head(fl!("report-healed"), s))
            .child(div().flex_none().w(s.px(14.)));
        let top = heals.first().map_or(0, |heal| heal.amount) as f64;
        let content = div()
            .flex()
            .flex_col()
            .gap(s.px(ROW_GAP))
            .children(heals.iter().map(|heal| heal_row(heal, player.class_colour, top, s)));
        div()
            .flex_none()
            .flex()
            .flex_col()
            .child(head)
            .child(ring_room_list("heals", &self.skills_scroll, rows_height(SKILLS_SHOWN, SKILL_ROW), overflows, content, s))
    }

    fn taken(&self, player: &PlayerDetail, s: Scale) -> Div {
        let Some(taken) = &player.taken else {
            return not_recorded(fl!("report-not-recorded-taken"), s);
        };
        if taken.is_empty() {
            return chrome::empty(
                Some(fl!("report-no-taken-title").into()),
                fl!("report-no-taken-line", name = player.row.name.clone()),
                s,
            );
        }
        let overflows = taken.len() > SKILLS_SHOWN;
        let head = skill_head(fl!("report-monster-skill"), taken.len(), 13., overflows, s)
            .child(head_figure(fl!("detail-hits"), 40., s))
            .child(head_figure(fl!("report-largest"), 46., s))
            .child(sorted_head(fl!("common-damage"), s))
            .child(div().flex_none().w(s.px(14.)));
        let top = taken.first().map_or(0, |line| line.amount) as f64;
        let content = div().flex().flex_col().gap(s.px(ROW_GAP)).children(taken.iter().map(|line| taken_row(line, top, s)));
        div()
            .flex_none()
            .flex()
            .flex_col()
            .child(head)
            .child(ring_room_list("taken", &self.skills_scroll, rows_height(SKILLS_SHOWN, SKILL_ROW), overflows, content, s))
    }
}

// ---- Pieces ----

/// Makes `element` the focus target of `focus`, with its ring: 2 px, 1 px
/// outside the control.
fn focusable(element: Stateful<Div>, focus: &Focus, radius: f32, s: Scale) -> Stateful<Div> {
    element
        .relative()
        .track_focus(&focus.handle)
        .when(focus.ring, |element| {
            element.child(
                div()
                    .debug_selector(|| "focus-ring".to_string())
                    .absolute()
                    .top(px(-3.))
                    .left(px(-3.))
                    .right(px(-3.))
                    .bottom(px(-3.))
                    .rounded(s.px(radius) + px(3.))
                    .border_2()
                    .border_color(rgba(FOCUS_RING)),
            )
        })
        .when_some(focus.reveal.clone(), |element, reveal| element.child(reveal_marker(reveal)))
}

/// A list that scrolls past `max_height` with 3 px of room around its rows,
/// so a focus ring is never clipped; the sides give it back to the card.
/// GPUI collapses a list given negative vertical margins, so the room above
/// and below stays.
fn ring_room_list(id: &'static str, handle: &ScrollHandle, max_height: f32, overflows: bool, content: Div, s: Scale) -> Div {
    let room = s.px(RING_ROOM);
    div().flex_none().flex().flex_col().mx(-room).child(chrome::scroll_list(
        id,
        handle,
        max_height + 2. * RING_ROOM,
        overflows,
        div().p(room).child(content),
        s,
    ))
}

fn rows_height(rows: usize, row: f32) -> f32 {
    rows as f32 * row + rows.saturating_sub(1) as f32 * ROW_GAP
}

/// Value over the column's top value, floored at 6%; nothing has no bar.
fn bar_length(value: f64, top: f64) -> f32 {
    if value > 0.0 && top > 0.0 {
        (value / top).max(0.06) as f32
    } else {
        0.0
    }
}

fn outcome_chip(outcome: FightOutcome, s: Scale) -> Div {
    match outcome {
        FightOutcome::Defeated => chrome::chip(Some(Icon::Check), fl!("meter-summary-defeated"), Tone::Good, s),
        FightOutcome::Ended => chrome::chip(Some(Icon::Hourglass), fl!("meter-summary-ended"), Tone::Warn, s)
            .child(chrome::chip_note(fl!("meter-summary-may-resume"), s)),
        FightOutcome::Wiped { hp_left, max_hp } => {
            let left = (max_hp > 0).then(|| format::percent(f64::from(hp_left) / f64::from(max_hp)));
            chrome::chip(Some(Icon::CircleX), fl!("meter-summary-wiped"), Tone::Bad, s)
                .children(left.map(|hp| chrome::chip_note(fl!("meter-summary-hp-left", hp = hp), s)))
        }
    }
}

/// The dark plate under key figures, label above value in each cell.
fn plate(top: f32, y: f32, s: Scale) -> Div {
    div()
        .flex_none()
        .flex()
        .gap(s.px(4.))
        .mt(s.px(top))
        .py(s.px(y))
        .px(s.px(10.))
        .rounded(s.px(8.))
        .bg(rgba(TRACK))
}

/// `None` is not recorded ("--"); `quiet` draws a zero in `faint`.
fn figure(label: String, value: Option<String>, quiet: bool, s: Scale) -> Div {
    let faint = quiet || value.is_none();
    div()
        .flex_1()
        .min_w_0()
        .flex()
        .flex_col()
        .gap(s.px(4.))
        .child(div().truncate().text_size(s.px(11.)).text_color(rgba(FAINT)).child(label))
        .child(
            div()
                .text_size(s.px(16.))
                .font_weight(FontWeight::SEMIBOLD)
                .when(faint, |value| value.text_color(rgba(FAINT)))
                .child(value.unwrap_or_else(|| "--".to_string())),
        )
}

fn hit_types(hits: &HitTypes, s: Scale) -> Div {
    let kinds = [
        (fl!("detail-critical"), hits.critical),
        (fl!("detail-back"), hits.back),
        (fl!("detail-front"), hits.front),
        (fl!("detail-double"), hits.double),
        (fl!("detail-perfect"), hits.perfect),
        (fl!("detail-parry"), hits.parry),
        (fl!("report-multi-hit"), hits.multi_hit),
    ];
    kinds.into_iter().fold(plate(4., 6., s), |plate, (label, count)| {
        plate.child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(s.px(4.))
                .child(div().truncate().text_size(s.px(11.)).text_color(rgba(FAINT)).child(label))
                .child(count_and_rate(count, hits.rate(count), 13., s)),
        )
    })
}

/// "665 41.9%": the count, then its rate quieter.
fn count_and_rate(count: u32, rate: Option<f64>, size: f32, s: Scale) -> Div {
    div()
        .flex()
        .items_baseline()
        .gap(s.px(4.))
        .child(div().text_size(s.px(size)).font_weight(FontWeight::SEMIBOLD).child(format::grouped(f64::from(count))))
        .children(rate.map(|rate| {
            div().text_size(s.px(11.)).font_weight(FontWeight::MEDIUM).text_color(rgba(MUTED)).child(format::percent(rate))
        }))
}

fn player_deaths(deaths: &[DeathRow], s: Scale) -> Div {
    div()
        .flex_none()
        .mt(s.px(4.))
        .py(s.px(6.))
        .px(s.px(10.))
        .rounded(s.px(8.))
        .bg(rgba(DEATHS_PLATE))
        .flex()
        .flex_col()
        .gap(s.px(3.))
        .children(deaths.iter().map(|death| {
            div()
                .h(s.px(18.))
                .flex()
                .items_center()
                .gap(s.px(6.))
                .text_size(s.px(12.))
                .child(chrome::icon(Icon::Skull, 12., rgb(BAD), s))
                .child(
                    div()
                        .flex_none()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(rgb(BAD))
                        .child(fl!("report-died-at", time = format::duration(death.at_secs))),
                )
                .child(div().min_w_0().truncate().child(death.killer.clone()))
                .child(div().min_w_0().truncate().text_color(rgba(MUTED)).child(format!("· {}", death.skill)))
                .child(div().flex_1())
                .children(death.last_hit.map(|hit| {
                    div()
                        .flex_none()
                        .text_size(s.px(11.5))
                        .text_color(rgba(MUTED))
                        .child(fl!("report-last-hit-amount", amount = format::compact(hit as f64)))
                }))
        }))
}

fn column_label(column: Column) -> String {
    match column {
        Column::Damage => fl!("common-damage"),
        Column::Dps => fl!("common-dps"),
        Column::Healing => fl!("report-healing"),
        Column::Taken => fl!("report-taken"),
        Column::Deaths => fl!("report-deaths"),
    }
}

fn column_width(column: Column) -> f32 {
    match column {
        Column::Damage => 54.,
        Column::Dps => 46.,
        Column::Healing => 52.,
        Column::Taken => 50.,
        Column::Deaths => 40.,
    }
}

/// A figure in the player table: the sorted column stands out; nothing is
/// "—", not recorded "--".
fn cell(row: &PlayerRow, column: Column, sorted: bool, recorded: bool, s: Scale) -> Div {
    let base = div().flex_none().w(s.px(column_width(column))).flex().justify_end().items_center();
    let base = if sorted {
        base.text_size(s.px(13.5)).font_weight(FontWeight::SEMIBOLD).text_color(rgba(TEXT))
    } else {
        base.text_size(s.px(12.5)).font_weight(FontWeight::MEDIUM).text_color(rgba(MUTED))
    };
    if !recorded && !column.is_in_every_fight() {
        return base.text_color(rgba(FAINT)).child("--");
    }
    if column == Column::Deaths {
        return match row.deaths.unwrap_or(0) {
            0 => base.font_weight(FontWeight::MEDIUM).text_color(rgba(FAINT)).child("0"),
            deaths => base
                .gap(s.px(3.))
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(rgb(BAD))
                .child(chrome::icon(Icon::Skull, 11., rgb(BAD), s))
                .child(deaths.to_string()),
        };
    }
    let value = row.value(column);
    if value > 0.0 {
        base.child(format::compact(value))
    } else {
        base.text_color(rgba(FAINT)).child("—")
    }
}

/// The 12 px slot every row keeps at its end, with the chevron while the
/// pointer or the keyboard is on the row.
fn chevron_slot(shown: bool, s: Scale) -> Div {
    div()
        .flex_none()
        .w(s.px(12.))
        .flex()
        .children(shown.then(|| chrome::icon(Icon::ChevronRight, 12., rgba(MUTED), s)))
}

/// The layer that lifts a bar under the pointer, fill included.
fn lift(s: Scale) -> Div {
    div().debug_selector(|| "lift".to_string()).absolute().inset_0().rounded(s.px(5.)).bg(rgba(LIFT))
}

fn small_count(count: usize, s: Scale) -> Div {
    div().flex_none().text_size(s.px(11.)).font_weight(FontWeight::MEDIUM).text_color(rgba(FAINT)).child(count.to_string())
}

/// One quiet 24 px line in place of a list.
fn quiet(glyph: Option<Icon>, text: String, colour: gpui::Hsla, s: Scale) -> Div {
    div()
        .flex_none()
        .h(s.px(24.))
        .px(s.px(6.))
        .flex()
        .items_center()
        .gap(s.px(6.))
        .text_size(s.px(12.))
        .text_color(colour)
        .children(glyph.map(|glyph| chrome::icon(glyph, 12., colour, s)))
        .child(text)
}

fn not_recorded(line: String, s: Scale) -> Div {
    chrome::empty(Some(fl!("report-not-recorded-title").into()), line, s)
}

/// A per-skill table's header: the name column with its count, starting
/// `left` px in; the right edge leaves the chevron slot and, while the list
/// scrolls, the thumb's gutter.
fn skill_head(title: String, count: usize, left: f32, overflows: bool, s: Scale) -> Div {
    chrome::table_head(s)
        .pl(s.px(left))
        .pr(s.px(if overflows { 17. } else { 9. }))
        .child(div().flex_1().min_w_0().flex().gap(s.px(6.)).child(title).child(count.to_string()))
}

fn head_figure(label: String, width: f32, s: Scale) -> Div {
    div().flex_none().w(s.px(width)).flex().justify_end().child(label)
}

/// The column a per-skill table is sorted by: `text` / 600 with the arrow.
fn sorted_head(label: String, s: Scale) -> Div {
    head_figure(label, 92., s)
        .items_center()
        .gap(s.px(2.))
        .text_color(rgba(TEXT))
        .font_weight(FontWeight::SEMIBOLD)
        .child(chrome::icon(Icon::ArrowDown, 10., rgba(TEXT), s))
}

fn side_figure(text: String, width: f32, s: Scale) -> Div {
    div().flex_none().w(s.px(width)).flex().justify_end().text_color(rgba(MUTED)).child(text)
}

/// The amount 13.5 / 600 with its share after it.
fn total_figure(amount: u64, share: f64, s: Scale) -> Div {
    div()
        .flex_none()
        .w(s.px(92.))
        .flex()
        .justify_end()
        .items_baseline()
        .gap(s.px(4.))
        .child(chrome::strong(format::compact(amount as f64), s))
        .child(div().text_size(s.px(11.)).font_weight(FontWeight::MEDIUM).text_color(rgba(MUTED)).child(format::percent(share)))
}

fn rate(value: Option<f64>) -> String {
    value.map_or_else(|| "--".to_string(), format::percent)
}

/// A game icon, or a quiet square of its size when the game data has none.
fn skill_icon(path: Option<&str>, size: f32, radius: f32, s: Scale) -> AnyElement {
    match path {
        Some(path) => img(SharedString::from(path.to_string())).flex_none().size(s.px(size)).rounded(s.px(radius)).into_any_element(),
        None => div().flex_none().size(s.px(size)).rounded(s.px(radius)).bg(rgba(NO_ICON)).into_any_element(),
    }
}

/// The twelve figures a skill row opens to, four to a line.
fn more(skill: &SkillLine, s: Scale) -> Div {
    let cell = |label: String, value: AnyElement| {
        div()
            .w(relative(0.25))
            .flex()
            .flex_col()
            .gap(s.px(3.))
            .child(div().text_size(s.px(11.)).text_color(rgba(FAINT)).child(label))
            .child(value)
    };
    let value = |text: String| div().text_size(s.px(12.5)).font_weight(FontWeight::SEMIBOLD).child(text).into_any_element();
    let hits = &skill.hit_types;
    let counted = |count: u32| count_and_rate(count, hits.rate(count), 12.5, s).into_any_element();
    let spec = div()
        .h(s.px(12.5))
        .pt(s.px(2.))
        .flex()
        .gap(s.px(3.))
        .children(skill.spec.iter().map(|on| div().size(s.px(7.)).rounded(s.px(2.)).bg(rgba(if *on { CREAM } else { SPEC_OFF }))))
        .into_any_element();
    let multi_damage = if skill.multi_damage > 0 { format::grouped(skill.multi_damage as f64) } else { "--".to_string() };
    div()
        .mt(s.px(3.))
        .mb(s.px(2.))
        .py(s.px(8.))
        .px(s.px(10.))
        .rounded(s.px(6.))
        .bg(rgba(EXPANDED_FILL))
        .flex()
        .flex_wrap()
        .gap_y(s.px(8.))
        .child(cell(fl!("detail-perfect"), counted(hits.perfect)))
        .child(cell(fl!("detail-double"), counted(hits.double)))
        .child(cell(fl!("detail-front"), counted(hits.front)))
        .child(cell(fl!("detail-back"), counted(hits.back)))
        .child(cell(fl!("detail-parry"), counted(hits.parry)))
        .child(cell(fl!("report-multi-hit"), counted(hits.multi_hit)))
        .child(cell(fl!("report-multi-hit-damage"), value(multi_damage)))
        .child(cell(fl!("detail-spec"), spec))
        .child(cell(fl!("detail-min"), value(format::grouped(skill.min as f64))))
        .child(cell(fl!("detail-max"), value(format::grouped(skill.max as f64))))
        .child(cell(fl!("detail-average"), value(format::grouped(skill.average as f64))))
        .child(cell(fl!("detail-total"), value(format::grouped(skill.damage as f64))))
}

fn buff_row(buff: &BuffLine, s: Scale) -> Div {
    let caster = div()
        .flex_none()
        .flex()
        .items_center()
        .gap(s.px(4.))
        .text_size(s.px(11.))
        .text_color(rgba(MUTED))
        .children(buff.caster_class.map(|class| chrome::class_icon(class, 12., s)))
        .child(buff.caster.clone());
    let line = div()
        .flex()
        .items_center()
        .gap(s.px(6.))
        .child(div().min_w_0().truncate().font_weight(FontWeight::MEDIUM).child(buff.name.clone()))
        .child(caster);
    div()
        .flex_none()
        .flex()
        .items_center()
        .gap(s.px(8.))
        .child(skill_icon(buff.icon.as_deref(), 24., 5., s))
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .gap(s.px(4.))
                .child(line)
                .child(div().flex().child(bar(class_fill(buff.colour), buff.share.unwrap_or(0.0) as f32, 4., 2., s))),
        )
        .child(
            div()
                .flex_none()
                .w(s.px(62.))
                .flex()
                .justify_end()
                .text_size(s.px(13.5))
                .font_weight(FontWeight::SEMIBOLD)
                .child(buff.uptime_secs.map_or_else(|| "--".to_string(), format::duration)),
        )
        .child(
            div()
                .flex_none()
                .w(s.px(40.))
                .flex()
                .justify_end()
                .text_size(s.px(12.))
                .text_color(rgba(MUTED))
                .child(buff.share.map_or_else(|| "--".to_string(), format::whole_percent)),
        )
}

fn heal_row(heal: &HealLine, colour: Option<u32>, top: f64, s: Scale) -> Div {
    let rate = if heal.heals > 0 { Some(f64::from(heal.crits) / f64::from(heal.heals)) } else { None };
    let content = chrome::bar_content(false, s)
        .child(skill_icon(heal.icon.as_deref(), 18., 4., s))
        .child(chrome::name(heal.name.clone(), None, FontWeight::MEDIUM, s))
        .child(side_figure(format::grouped(f64::from(heal.heals)), 40., s))
        .child(side_figure(format::grouped(f64::from(heal.crits)), 40., s))
        .child(side_figure(self::rate(rate), 46., s))
        .child(total_figure(heal.amount, heal.share, s))
        .child(div().flex_none().w(s.px(14.)));
    track(class_fill(colour), bar_length(heal.amount as f64, top), SKILL_ROW, s).child(content)
}

fn taken_row(line: &TakenLine, top: f64, s: Scale) -> Div {
    let content = chrome::bar_content(true, s)
        .child(chrome::name(line.skill.clone(), Some(line.monster.clone().into()), FontWeight::MEDIUM, s))
        .child(side_figure(format::grouped(f64::from(line.hits)), 40., s))
        .child(side_figure(format::compact(line.largest as f64), 46., s))
        .child(total_figure(line.amount, line.share, s))
        .child(div().flex_none().w(s.px(14.)));
    track(hp_fill(), bar_length(line.amount as f64, top), SKILL_ROW, s).child(content)
}

#[cfg(feature = "probes")]
impl ReportView {
    /// What a capture shows: a sort, a tab, an opened skill, the pointer in
    /// an element (as its enter would), a focus ring, and the player table
    /// scrolled down by `scrolled_rows`.
    pub fn show_state(
        &mut self,
        sort: Column,
        tab: Tab,
        open_skill: Option<u32>,
        pointer: Option<Pointed>,
        ring: Option<String>,
        scrolled_rows: usize,
        cx: &mut Context<Self>,
    ) {
        self.sort = sort;
        self.tab = tab;
        self.open_skill = open_skill;
        self.pointer_on = pointer;
        self.forced_ring = ring;
        let offset = rows_height(scrolled_rows, ROW) + if scrolled_rows > 0 { ROW_GAP } else { 0. };
        let scale = self.model.read(cx).meter_model().read(cx).settings.scale();
        self.table_scroll.set_offset(gpui::point(px(0.), -px(offset * scale)));
        cx.notify();
    }
}

#[cfg(test)]
impl ReportView {
    pub fn sort(&self) -> Column {
        self.sort
    }

    pub fn tab(&self) -> Tab {
        self.tab
    }

    pub fn pointer_on(&self) -> Option<Pointed> {
        self.pointer_on
    }

    pub fn focused_control(&self, window: &Window) -> Option<String> {
        self.focused_key(window).map(str::to_string)
    }
}
