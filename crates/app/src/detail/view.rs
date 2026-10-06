//! The player detail window as approved in
//! docs/gpui-mockups/approved/detail.html: who, where, the key figures and hit
//! types, the skills as the meter's soft bars, and the buff panels.
//!
//! The Session source has no mockup of its own. It reads like the live view
//! with "Session" where the target would be: the figures and skills add up
//! every mob since the last reset, and only the player's buffs show, because
//! no single target's buffs belong to a session.

use std::collections::HashMap;

use gpui::{
    div, img, prelude::*, relative, rgb, rgba, AnyElement, Context, Div, Entity, FontWeight, IntoElement, Render,
    ScrollHandle, SharedString, Subscription, Window,
};

use super::data::{BuffRow, DetailData, DetailMode, DetailSource, SkillRow, Summary};
use super::model::DetailModel;
use crate::assets::{Icon, COMBAT_POWER};
use crate::format;
use crate::history;
use crate::i18n::fl;
use crate::meter::{MeterModel, OverlaySettings};
use crate::overlay::WindowDrag;
use crate::secondary::chrome::{self, Look, Scale, Tone, CREAM, FAINT, GOOD, MUTED, SOFT};
use crate::style::{bar, class_fill, track};

const WIDTH: f32 = 520.0;
const SKILL_ROW: f32 = 24.0;
const ROW_GAP: f32 = 3.0;
const SKILLS_SHOWN: usize = 8;
/// The opened row's twelve figures, margins included.
const EXPANDED_HEIGHT: f32 = 117.0;
const BUFF_ROW: f32 = 24.0;
const BUFF_GAP: f32 = 7.0;
const BUFFS_SHOWN: usize = 6;
const SPEC_OFF: u32 = 0xfff0e129;
const EXPANDED_FILL: u32 = 0xfff0e10d;
const PANEL_FILL: u32 = 0x00000033;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Panel {
    Player,
    Target,
}

pub struct DetailView {
    model: Entity<DetailModel>,
    /// The skill whose other figures are open; one at a time.
    open_skill: Option<u32>,
    player_buffs_open: bool,
    target_buffs_open: bool,
    skills_scroll: ScrollHandle,
    player_buffs_scroll: ScrollHandle,
    target_buffs_scroll: ScrollHandle,
    drag: WindowDrag,
    _model: Subscription,
}

impl DetailView {
    pub fn new(model: Entity<DetailModel>, cx: &mut Context<Self>) -> Self {
        let subscription = cx.observe(&model, |_, _, cx| cx.notify());
        Self {
            model,
            open_skill: None,
            player_buffs_open: false,
            target_buffs_open: false,
            skills_scroll: ScrollHandle::new(),
            player_buffs_scroll: ScrollHandle::new(),
            target_buffs_scroll: ScrollHandle::new(),
            drag: WindowDrag::default(),
            _model: subscription,
        }
    }
}

/// Where the colours of the window's players come from: a saved fight shades
/// its own players; live, the meter's shades are used.
enum Colours<'a> {
    Record(HashMap<u32, u32>),
    Meter(&'a MeterModel),
}

impl Colours<'_> {
    fn of(&self, actor_id: u32, class: Option<crate::game_data::PlayerClass>) -> Option<u32> {
        let known = match self {
            Self::Record(colours) => colours.get(&actor_id).copied(),
            Self::Meter(meter) => meter.actor_colour(actor_id),
        };
        known.or(class.map(|class| class.rgb()))
    }
}

impl Render for DetailView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let model = self.model.read(cx);
        let meter = model.meter_model().read(cx);
        let look = Look::of(&meter.settings);
        let s = look.s;
        let Some(data) = model.data() else {
            let header = chrome::header(
                div().text_size(s.px(14.5)).font_weight(FontWeight::SEMIBOLD).text_color(rgba(FAINT)).child(fl!("detail-no-player")),
                Vec::new(),
                &self.drag,
                s,
            );
            let empty = chrome::empty(Some(fl!("detail-pick-title").into()), fl!("detail-pick-line"), s);
            return chrome::root(look).child(chrome::card(WIDTH, look).child(header).child(empty));
        };
        let (colours, saved_at) = match model.source() {
            DetailSource::History { record, .. } => (Colours::Record(history::record_colours(record)), Some(record.created_at)),
            _ => (Colours::Meter(meter), None),
        };
        let colour = colours.of(data.actor_id, data.identity.class);
        let none = data.skills.is_empty();

        let mut card = chrome::card(WIDTH, look)
            .child(header(data, &meter.settings, &self.drag, s))
            .child(sub_line(data, saved_at, s))
            .child(figures(&data.summary, none, s));
        card = if none {
            card.child(
                chrome::empty(Some(fl!("detail-no-skills-title").into()), no_skills_line(data.mode), s)
                    .pt(s.px(14.))
                    .pb(s.px(8.)),
            )
        } else {
            card.child(self.skills(data, colour, s, cx))
        };
        card = card.child(self.buff_panel(Panel::Player, &data.player_buffs, &colours, s, cx));
        if data.mode != DetailMode::Session {
            card = card.child(self.buff_panel(Panel::Target, &data.target_buffs, &colours, s, cx));
        }
        chrome::root(look).child(card)
    }
}

impl DetailView {
    fn skills(&self, data: &DetailData, colour: Option<u32>, s: Scale, cx: &Context<Self>) -> impl IntoElement {
        let top = data.skills.first().map_or(0, |skill| skill.total).max(1);
        let expanded = self.open_skill.filter(|code| data.skills.iter().any(|skill| skill.code == *code));
        let rows = data.skills.len();
        let extra = if expanded.is_some() { EXPANDED_HEIGHT } else { 0. };
        let max = SKILLS_SHOWN as f32 * SKILL_ROW + (SKILLS_SHOWN - 1) as f32 * ROW_GAP + extra;
        let content = rows as f32 * SKILL_ROW + rows.saturating_sub(1) as f32 * ROW_GAP + extra;
        let overflows = content > max;
        let column = |text: String, width: f32| chrome::figure(text, Some(width), s);
        let head = chrome::table_head(s)
            .mt(s.px(8.))
            .pl(s.px(30.))
            .pr(s.px(if overflows { 17. } else { 9. }))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .gap(s.px(6.))
                    .child(fl!("detail-skill"))
                    .child(div().text_color(rgba(FAINT)).child(rows.to_string())),
            )
            .child(column(fl!("detail-hits"), 40.))
            .child(column(fl!("detail-crit"), 46.))
            .child(column(fl!("detail-total"), 92.))
            .child(div().flex_none().w(s.px(14.)));
        let list = div().flex().flex_col().gap(s.px(ROW_GAP)).children(data.skills.iter().map(|skill| {
            let open = expanded == Some(skill.code);
            div()
                .flex()
                .flex_col()
                .child(self.skill_row(skill, colour, skill.total as f32 / top as f32, open, s, cx))
                .when(open, |row| row.child(more(skill, s)))
        }));
        div()
            .flex()
            .flex_col()
            .child(head)
            .child(chrome::scroll_list("skills", &self.skills_scroll, max, overflows, list, s))
    }

    fn skill_row(&self, skill: &SkillRow, colour: Option<u32>, length: f32, open: bool, s: Scale, cx: &Context<Self>) -> impl IntoElement {
        let code = skill.code;
        let content = chrome::bar_content(false, s)
            .child(skill_icon(skill.icon.as_deref(), 18., 4., s))
            .child(chrome::name(skill.name.clone(), None, FontWeight::MEDIUM, s))
            .child(chrome::figure(skill.hits.to_string(), Some(40.), s))
            .child(chrome::figure(rate(skill.critical), Some(46.), s))
            .child(
                div()
                    .flex_none()
                    .w(s.px(92.))
                    .flex()
                    .justify_end()
                    .items_baseline()
                    .gap(s.px(4.))
                    .child(chrome::strong(format::compact(skill.total as f64), s))
                    .child(
                        div()
                            .text_size(s.px(11.))
                            .font_weight(FontWeight::MEDIUM)
                            .text_color(rgba(MUTED))
                            .child(format::percent(skill.share)),
                    ),
            )
            .child(
                div()
                    .flex_none()
                    .w(s.px(14.))
                    .flex()
                    .justify_end()
                    .child(chrome::icon(if open { Icon::ChevronUp } else { Icon::ChevronDown }, 12., rgba(FAINT), s)),
            );
        track(class_fill(colour), length.max(0.06), SKILL_ROW, s)
            .id(("skill", u64::from(code)))
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| {
                this.open_skill = if this.open_skill == Some(code) { None } else { Some(code) };
                cx.notify();
            }))
            .child(content)
    }

    fn buff_panel(&self, panel: Panel, buffs: &[BuffRow], colours: &Colours, s: Scale, cx: &Context<Self>) -> Div {
        let (open, scroll, id) = match panel {
            Panel::Player => (self.player_buffs_open, &self.player_buffs_scroll, "player-buffs"),
            Panel::Target => (self.target_buffs_open, &self.target_buffs_scroll, "target-buffs"),
        };
        let title = match panel {
            Panel::Player => fl!("detail-player-buffs"),
            Panel::Target if self.model.read(cx).data().is_some_and(|data| data.target_is_boss) => fl!("detail-boss-buffs"),
            Panel::Target => fl!("detail-target-buffs"),
        };
        let up = buffs.iter().filter(|buff| buff.active).count();
        let head = div()
            .id(id)
            .h(s.px(28.))
            .pl(s.px(8.))
            .pr(s.px(10.))
            .flex()
            .items_center()
            .gap(s.px(7.))
            .text_size(s.px(12.5))
            .font_weight(FontWeight::SEMIBOLD)
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| {
                match panel {
                    Panel::Player => this.player_buffs_open = !this.player_buffs_open,
                    Panel::Target => this.target_buffs_open = !this.target_buffs_open,
                }
                cx.notify();
            }))
            .child(chrome::icon(if open { Icon::ChevronDown } else { Icon::ChevronRight }, 13., rgba(MUTED), s))
            .child(title)
            .child(chrome::count(buffs.len().to_string(), s))
            .child(div().flex_1())
            .child(chrome::count(fl!("detail-buffs-up-now", count = up), s));
        let body = (open && !buffs.is_empty()).then(|| {
            let rows = buffs.len();
            let max = BUFFS_SHOWN as f32 * BUFF_ROW + (BUFFS_SHOWN - 1) as f32 * BUFF_GAP;
            let list = div()
                .flex()
                .flex_col()
                .gap(s.px(BUFF_GAP))
                .children(buffs.iter().map(|buff| buff_row(buff, colours.of(buff.caster_id, buff.caster_class), s)));
            div().pt(s.px(2.)).pl(s.px(8.)).pr(s.px(10.)).pb(s.px(10.)).child(chrome::scroll_list(
                SharedString::from(format!("{id}-list")),
                scroll,
                max,
                rows > BUFFS_SHOWN,
                list,
                s,
            ))
        });
        div()
            .flex_none()
            .mt(s.px(6.))
            .rounded(s.px(8.))
            .bg(rgba(PANEL_FILL))
            .flex()
            .flex_col()
            .child(head)
            .children(body)
    }
}

fn header(data: &DetailData, settings: &OverlaySettings, drag: &WindowDrag, s: Scale) -> Div {
    let identity = &data.identity;
    let who = div()
        .min_w_0()
        .flex()
        .items_center()
        .gap(s.px(7.))
        .children(identity.class.map(|class| chrome::class_icon(class, 20., s)))
        .child(
            div()
                .min_w_0()
                .truncate()
                .text_size(s.px(14.5))
                .font_weight(FontWeight::SEMIBOLD)
                .child(identity.name.clone()),
        )
        .when(settings.show_server && !identity.server.is_empty(), |who| {
            who.child(div().flex_none().text_size(s.px(12.)).text_color(rgba(MUTED)).child(identity.server.clone()))
        });
    let power = identity.combat_power.map(|power| {
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
    let chip = match data.mode {
        DetailMode::History => chrome::chip(Some(Icon::History), fl!("detail-chip-history"), Tone::Warn, s),
        DetailMode::Live | DetailMode::Session => chrome::chip(None, fl!("detail-chip-live"), Tone::Good, s),
    };
    chrome::header(
        div().min_w_0().flex().items_center().gap(s.px(8.)).child(who).children(power),
        vec![chip.into_any_element()],
        drag,
        s,
    )
}

fn sub_line(data: &DetailData, saved_at: Option<u64>, s: Scale) -> Div {
    let (glyph, place, when): (Icon, String, String) = match data.mode {
        DetailMode::Live => (Icon::Target, data.target_name.clone().unwrap_or_else(|| fl!("detail-no-target")), fl!("detail-current-target")),
        DetailMode::Session => (Icon::Target, fl!("common-session"), fl!("detail-every-target")),
        DetailMode::History => (
            Icon::Clock,
            data.target_name.clone().unwrap_or_else(|| fl!("detail-unknown-target")),
            match saved_at {
                Some(at) => fl!("detail-saved-record-at", time = history::record_time(at)),
                None => fl!("detail-saved-record"),
            },
        ),
    };
    div()
        .flex_none()
        .h(s.px(18.))
        .px(s.px(4.))
        .flex()
        .items_center()
        .gap(s.px(6.))
        .text_size(s.px(11.5))
        .text_color(rgba(MUTED))
        .child(chrome::icon(glyph, 12., rgba(MUTED), s))
        .child(div().min_w_0().truncate().child(place))
        .child(div().flex_none().text_color(rgba(FAINT)).child("·"))
        .child(div().flex_none().child(when))
}

/// Damage, DPS, fight time and hits on one plate; the hit types on a smaller
/// one below. Without skill data none of them is known.
fn figures(summary: &Summary, none: bool, s: Scale) -> Div {
    let figure = |label: String, value: String, size: f32| {
        div()
            .flex_1()
            .min_w_0()
            .flex()
            .flex_col()
            .gap(s.px(4.))
            .child(div().text_size(s.px(11.)).text_color(rgba(FAINT)).child(label))
            .child(div().text_size(s.px(size)).font_weight(FontWeight::SEMIBOLD).child(value))
    };
    let plate = |top: f32, y: f32| {
        div()
            .flex_none()
            .flex()
            .gap(s.px(4.))
            .mt(s.px(top))
            .py(s.px(y))
            .px(s.px(10.))
            .rounded(s.px(8.))
            .bg(rgba(crate::style::TRACK))
    };
    let known = |value: String| if none { "--".to_string() } else { value };
    let fight = if summary.fight_secs > 0.0 && !none { format::duration(summary.fight_secs) } else { "--".to_string() };
    let key = plate(6., 7.)
        .child(figure(fl!("common-damage"), known(format::grouped(summary.damage as f64)), 16.))
        .child(figure(fl!("common-dps"), known(format::grouped(summary.dps)), 16.))
        .child(figure(fl!("detail-fight"), fight, 16.))
        .child(figure(fl!("detail-hits"), known(format::grouped(f64::from(summary.hits))), 16.));
    let rates = [
        (fl!("detail-critical"), summary.critical),
        (fl!("detail-back"), summary.back),
        (fl!("detail-front"), summary.front),
        (fl!("detail-double"), summary.double),
        (fl!("detail-perfect"), summary.perfect),
        (fl!("detail-parry"), summary.parry),
        (fl!("detail-multi"), summary.multi),
    ];
    let hits = rates
        .into_iter()
        .fold(plate(4., 6.), |plate, (label, value)| plate.child(figure(label, known(rate(value)), 13.)));
    div().flex_none().flex().flex_col().child(key).child(hits)
}

/// The twelve figures a skill row opens to, four to a line.
fn more(skill: &SkillRow, s: Scale) -> impl IntoElement {
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
    let spec = div()
        .h(s.px(12.5))
        .pt(s.px(2.))
        .flex()
        .gap(s.px(3.))
        .children(skill.spec.iter().map(|on| {
            div().size(s.px(7.)).rounded(s.px(2.)).bg(rgba(if *on { CREAM } else { SPEC_OFF }))
        }))
        .into_any_element();
    let multi_damage = if skill.multi_damage > 0 { format::grouped(skill.multi_damage as f64) } else { "--".to_string() };
    let breakdown: Vec<String> = skill
        .multi_breakdown
        .iter()
        .map(|multi| fl!("detail-multi-hits", hits = multi.hits, count = multi.count, share = format::percent(multi.share)))
        .collect();
    let multi = div()
        .id(("multi", u64::from(skill.code)))
        .text_size(s.px(12.5))
        .font_weight(FontWeight::SEMIBOLD)
        .child(multi_damage)
        .when(!breakdown.is_empty(), |cell| cell.tooltip(chrome::tooltip(breakdown.join("  ·  ").into(), s)))
        .into_any_element();
    let average = skill.average as f64;
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
        .child(cell(fl!("detail-perfect"), value(rate(skill.perfect))))
        .child(cell(fl!("detail-double"), value(rate(skill.double))))
        .child(cell(fl!("detail-front"), value(rate(skill.front))))
        .child(cell(fl!("detail-back"), value(rate(skill.back))))
        .child(cell(fl!("detail-parry"), value(rate(skill.parry))))
        .child(cell(fl!("detail-multi"), value(rate(skill.multi))))
        .child(cell(fl!("detail-multi-damage"), multi))
        .child(cell(fl!("detail-spec"), spec))
        .child(cell(fl!("detail-min"), value(format::grouped(skill.min as f64))))
        .child(cell(fl!("detail-max"), value(format::grouped(skill.max as f64))))
        .child(cell(fl!("detail-average"), value(format::grouped(average))))
        .child(cell(fl!("detail-total"), value(format::grouped(skill.total as f64))))
}

fn buff_row(buff: &BuffRow, colour: Option<u32>, s: Scale) -> impl IntoElement {
    let caster = div()
        .flex_none()
        .flex()
        .items_center()
        .gap(s.px(4.))
        .text_size(s.px(11.))
        .text_color(rgba(MUTED))
        .children(buff.caster_class.map(|class| chrome::class_icon(class, 12., s)))
        .child(buff.caster.clone());
    let state = div()
        .flex_none()
        .text_size(s.px(10.5))
        .font_weight(FontWeight::SEMIBOLD)
        .text_color(if buff.active { rgb(GOOD) } else { rgba(FAINT) })
        .child(if buff.active { fl!("detail-buff-up-now") } else { fl!("detail-buff-ended") });
    let line = div()
        .flex()
        .items_center()
        .gap(s.px(6.))
        .child(div().min_w_0().truncate().font_weight(FontWeight::MEDIUM).child(buff.name.clone()))
        .child(caster)
        .child(div().flex_1())
        .child(state);
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
                .child(div().flex().child(bar(class_fill(colour), buff.coverage as f32, 4., 2., s))),
        )
        .child(
            div()
                .flex_none()
                .w(s.px(44.))
                .flex()
                .justify_end()
                .font_weight(FontWeight::SEMIBOLD)
                .child(format::percent(buff.coverage)),
        )
}

/// A game icon, or a quiet square of its size when the game data has none.
fn skill_icon(path: Option<&str>, size: f32, radius: f32, s: Scale) -> AnyElement {
    match path {
        Some(path) => img(SharedString::from(path.to_string()))
            .flex_none()
            .size(s.px(size))
            .rounded(s.px(radius))
            .into_any_element(),
        None => div().flex_none().size(s.px(size)).rounded(s.px(radius)).bg(rgba(SOFT)).into_any_element(),
    }
}

fn no_skills_line(mode: DetailMode) -> String {
    match mode {
        DetailMode::Live => fl!("detail-no-skills-live"),
        DetailMode::Session => fl!("detail-no-skills-session"),
        DetailMode::History => fl!("detail-no-skills-history"),
    }
}

fn rate(value: Option<f64>) -> String {
    value.map_or_else(|| "--".to_string(), format::percent)
}

#[cfg(feature = "probes")]
impl DetailView {
    /// Opens a skill's figures and the buff panels, as a click would.
    pub fn show_open(&mut self, skill: Option<u32>, buffs: bool, cx: &mut Context<Self>) {
        self.open_skill = skill;
        self.player_buffs_open = buffs;
        self.target_buffs_open = buffs;
        cx.notify();
    }
}
