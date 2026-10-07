use std::collections::HashMap;
use std::sync::Arc;

use gpui::{point, AppContext as _, Entity, KeyDownEvent, KeyUpEvent, Keystroke, Modifiers, Point, TestAppContext, VisualTestContext};
use meter_core::history::HistoryRecord;
use meter_core::models::combat::{CombatInfos, PlayerOverviewStat, SkillStats, TargetInfo};
use meter_core::report::events::HealKind;
use meter_core::report::fight_report::FightReport;
use meter_core::report::ledger::{
    FightLedger, FightWindow, HealSkill, KillerKind, LedgerDeath, LedgerKiller, LedgerLastHit, LedgerPlayer, TakenSkill,
};

use super::data::Column;
use super::view::{Pointed, Tab};
use super::{ReportModel, ReportView};
use crate::meter::{MeterModel, OverlaySettings};

const BOSS: u32 = 900;
const MIRELLE: u32 = 1;
const SAEBRIN: u32 = 2;
const DOVREK: u32 = 3;
/// The game never named this player.
const UNNAMED: u32 = 4;

fn stat(actor_id: u32, name: &str, class: &str, damage: u64, dps: f64) -> PlayerOverviewStat {
    PlayerOverviewStat {
        actor_id,
        actor_name: name.to_string(),
        actor_server_id: "1001".to_string(),
        actor_class: class.to_string(),
        combat_power: Some(40_000),
        counts: 30,
        total_damage: damage,
        min_damage: 1,
        max_damage: 1,
        special_counts: HashMap::from([("CRITICAL".to_string(), 12)]),
        dps,
        damage_share: 0.0,
        damage_contribution: 0.0,
    }
}

fn ledger_player(actor_id: u32, name: &str, class: &str, healing: Vec<HealSkill>, damage_taken: Vec<TakenSkill>) -> LedgerPlayer {
    LedgerPlayer {
        actor_id,
        actor_name: name.to_string(),
        actor_server_id: "1001".to_string(),
        actor_class: class.to_string(),
        healing,
        damage_taken,
    }
}

fn taken(amount: u64) -> Vec<TakenSkill> {
    vec![TakenSkill {
        monster_code: Some(2_301_723),
        monster_name: Some("Basilus the False".to_string()),
        skill_code: 1_217_100,
        hits: 3,
        largest: amount / 2,
        amount,
    }]
}

fn heal(kind: HealKind, amount: u64) -> HealSkill {
    HealSkill { skill_code: 17_100_000, kind, heals: 5, crits: 2, amount }
}

/// A Basilus kill: Mirelle (you) did the most damage, Dovrek joined late and
/// healed, Saebrin died once, the templar the game never named took the most.
fn record(with_ledger: bool) -> HistoryRecord {
    let window = FightWindow { started_at: 1_000.0, ended_at: 1_100.0 };
    let ledger = FightLedger {
        window,
        players: vec![
            ledger_player(MIRELLE, "Mirelle", "Ranger", vec![heal(HealKind::Drain, 700)], taken(1_000)),
            ledger_player(SAEBRIN, "Saebrin", "Assassin", Vec::new(), taken(2_000)),
            ledger_player(DOVREK, "Dovrek", "Cleric", vec![heal(HealKind::Direct, 5_000)], Vec::new()),
            ledger_player(UNNAMED, "", "Templar", Vec::new(), taken(9_000)),
        ],
        deaths: vec![LedgerDeath {
            at_secs: 40.0,
            actor_id: SAEBRIN,
            killer: LedgerKiller { kind: KillerKind::Monster, name: Some("Basilus the False".to_string()), monster_code: Some(2_301_723) },
            skill_code: 1_217_100,
            last_hit: Some(LedgerLastHit {
                monster_code: Some(2_301_723),
                monster_name: Some("Basilus the False".to_string()),
                skill_code: 1_217_100,
                amount: 1_500,
            }),
        }],
        buffs: Vec::new(),
    };
    let skills = HashMap::from([
        (14_020_000, SkillStats { counts: 10, total_damage: 6_000, min_damage: 100, max_damage: 900, special_counts: HashMap::new() }),
        (14_110_000, SkillStats { counts: 20, total_damage: 3_000, min_damage: 50, max_damage: 300, special_counts: HashMap::new() }),
    ]);
    HistoryRecord {
        id: "fight".to_string(),
        target_id: BOSS,
        total_damage: 19_000,
        target_info: Some(TargetInfo {
            id: BOSS,
            target_mob_code: Some(2_301_723),
            target_name: Some("Basilus the False".to_string()),
            is_boss: true,
            current_hp: Some(0),
            max_hp: Some(1_000),
            target_start_time: HashMap::from([(MIRELLE, window.started_at)]),
            target_last_time: HashMap::from([(MIRELLE, window.ended_at)]),
        }),
        combat_infos: CombatInfos {
            actor_infos: HashMap::new(),
            target_infos: HashMap::new(),
            main_actor_id: Some(MIRELLE),
            main_actor_name: Some("Mirelle".to_string()),
            last_target_by_main_actor: Some(BOSS),
            last_target: Some(BOSS),
            time_now: window.ended_at,
        },
        player_skill_stats: HashMap::from([(MIRELLE, skills)]),
        player_stats: HashMap::from([
            (MIRELLE, stat(MIRELLE, "Mirelle", "Ranger", 9_000, 90.0)),
            (SAEBRIN, stat(SAEBRIN, "Saebrin", "Assassin", 6_000, 60.0)),
            (DOVREK, stat(DOVREK, "Dovrek", "Cleric", 3_000, 120.0)),
            (UNNAMED, stat(UNNAMED, "", "Templar", 1_000, 10.0)),
        ]),
        use_buffs_by_target: HashMap::new(),
        ledger: with_ledger.then_some(ledger),
        created_at: 1_100_000,
    }
}

fn report(with_ledger: bool) -> Arc<FightReport> {
    Arc::new(FightReport::from_record(&record(with_ledger)).expect("a boss fight"))
}

struct Opened<'a> {
    view: Entity<ReportView>,
    model: Entity<ReportModel>,
    meter: Entity<MeterModel>,
    cx: &'a mut VisualTestContext,
}

fn open(cx: &mut TestAppContext, report: Arc<FightReport>) -> Opened<'_> {
    let (model, meter) = cx.update(|cx| {
        let meter = cx.new(|_| MeterModel::new(OverlaySettings::default(), None));
        let model = cx.new(|cx| ReportModel::new(meter.clone(), report, cx));
        (model, meter)
    });
    let view_model = model.clone();
    let (view, cx) = cx.add_window_view(|window, cx| ReportView::new(view_model, window, cx));
    Opened { view, model, meter, cx }
}

impl Opened<'_> {
    fn click(&mut self, selector: &'static str) {
        let centre = self.cx.debug_bounds(selector).unwrap_or_else(|| panic!("{selector} is not drawn")).center();
        self.cx.simulate_click(centre, Modifiers::none());
    }

    fn is_drawn(&mut self, selector: &'static str) -> bool {
        self.cx.debug_bounds(selector).is_some()
    }

    fn move_to(&mut self, at: Point<gpui::Pixels>) {
        self.cx.simulate_mouse_move(at, None, Modifiers::none());
    }

    /// The players' rows from top to bottom.
    fn order(&mut self) -> Vec<u32> {
        let mut rows: Vec<(gpui::Pixels, u32)> = [("row:1", MIRELLE), ("row:2", SAEBRIN), ("row:3", DOVREK), ("row:4", UNNAMED)]
            .into_iter()
            .filter_map(|(selector, id)| self.cx.debug_bounds(selector).map(|bounds| (bounds.top(), id)))
            .collect();
        rows.sort_by(|a, b| a.0.partial_cmp(&b.0).expect("finite positions"));
        rows.into_iter().map(|(_, id)| id).collect()
    }

    fn open_player(&mut self) -> Option<u32> {
        let model = self.model.clone();
        self.cx.update(|_, cx| model.read(cx).player().map(|player| player.row.actor_id))
    }

    fn view<R>(&mut self, read: impl FnOnce(&ReportView) -> R) -> R {
        let view = self.view.clone();
        self.cx.update(|_, cx| read(view.read(cx)))
    }

    fn press(&mut self, key: &str) {
        let keystroke = Keystroke::parse(key).expect("a key name");
        self.cx.simulate_event(KeyDownEvent { keystroke: keystroke.clone(), is_held: false, prefer_character_input: false });
        self.cx.simulate_event(KeyUpEvent { keystroke });
    }

    fn focused(&mut self) -> Option<String> {
        let view = self.view.clone();
        self.cx.update(|window, cx| view.read(cx).focused_control(window))
    }

    fn tab_to(&mut self, key: &str) {
        for _ in 0..40 {
            self.press("tab");
            if self.focused().as_deref() == Some(key) {
                return;
            }
        }
        panic!("Tab never reached {key}");
    }
}

#[gpui::test]
fn a_click_on_a_column_header_sorts_by_it_highest_first(cx: &mut TestAppContext) {
    let mut window = open(cx, report(true));
    assert_eq!(window.order(), vec![MIRELLE, SAEBRIN, DOVREK, UNNAMED], "damage first");

    for (header, expected) in [
        ("sort:dps", vec![DOVREK, MIRELLE, SAEBRIN, UNNAMED]),
        ("sort:healing", vec![DOVREK, MIRELLE, SAEBRIN, UNNAMED]),
        ("sort:taken", vec![UNNAMED, SAEBRIN, MIRELLE, DOVREK]),
        ("sort:deaths", vec![SAEBRIN, MIRELLE, DOVREK, UNNAMED]),
        ("sort:damage", vec![MIRELLE, SAEBRIN, DOVREK, UNNAMED]),
    ] {
        window.click(header);
        assert_eq!(window.order(), expected, "{header}");
    }
}

#[gpui::test]
fn the_healing_column_leaves_out_life_drain(cx: &mut TestAppContext) {
    let window = open(cx, report(true));
    let model = window.model.clone();
    let mirelle = window.cx.update(|_, cx| model.read(cx).data().players.iter().find(|row| row.actor_id == MIRELLE).cloned());

    assert_eq!(mirelle.and_then(|row| row.healing), Some(0));
}

#[gpui::test]
fn the_deaths_list_sits_under_the_player_table(cx: &mut TestAppContext) {
    let window = open(cx, report(true));
    let last_row = window.cx.debug_bounds("row:4").expect("the last row");
    let death = window.cx.debug_bounds("death:0").expect("the death row");

    assert!(death.top() > last_row.bottom(), "{death:?} overlaps {last_row:?}");
}

#[gpui::test]
fn a_player_opens_in_the_window_and_back_returns_to_the_table_as_sorted(cx: &mut TestAppContext) {
    let mut window = open(cx, report(true));
    window.click("sort:taken");

    window.click("row:2");
    assert_eq!(window.open_player(), Some(SAEBRIN));
    assert!(window.is_drawn("report-player") && !window.is_drawn("report"));

    window.click("back");
    assert_eq!(window.open_player(), None);
    assert_eq!(window.view(ReportView::sort), Column::Taken);
    assert_eq!(window.order(), vec![UNNAMED, SAEBRIN, MIRELLE, DOVREK], "the sort is kept");
}

#[gpui::test]
fn a_death_row_opens_the_player_who_died(cx: &mut TestAppContext) {
    let mut window = open(cx, report(true));

    window.click("death:0");

    assert_eq!(window.open_player(), Some(SAEBRIN));
}

#[gpui::test]
fn the_tab_is_kept_for_the_next_player(cx: &mut TestAppContext) {
    let mut window = open(cx, report(true));
    window.click("row:1");
    window.click("tab:taken");
    window.click("back");

    window.click("row:3");

    assert_eq!(window.view(ReportView::tab), Tab::Taken);
}

#[gpui::test]
fn a_fight_saved_before_the_report_cannot_sort_what_it_did_not_record(cx: &mut TestAppContext) {
    let mut window = open(cx, report(false));
    for header in ["sort:healing", "sort:taken", "sort:deaths"] {
        assert!(!window.is_drawn(header), "{header} sorts");
    }
    assert!(window.is_drawn("sort:dps"));

    window.click("head:healing");

    assert_eq!(window.view(ReportView::sort), Column::Damage);
    assert!(!window.is_drawn("death:0"), "no deaths are listed");
    let model = window.model.clone();
    assert!(!window.cx.update(|_, cx| model.read(cx).data().is_recorded()));
}

#[gpui::test]
fn the_keyboard_moves_between_rows_opens_one_and_comes_back_to_it(cx: &mut TestAppContext) {
    let mut window = open(cx, report(true));
    window.tab_to("row:1");
    assert!(window.is_drawn("focus-ring"));

    window.press("down");
    assert_eq!(window.focused().as_deref(), Some("row:2"));
    window.press("up");
    window.press("down");

    window.press("enter");
    assert_eq!(window.open_player(), Some(SAEBRIN));
    assert_eq!(window.focused().as_deref(), Some("back"));

    window.press("enter");
    assert_eq!(window.open_player(), None);
    assert_eq!(window.focused().as_deref(), Some("row:2"), "back on the row that was opened");
}

#[gpui::test]
fn left_and_right_move_between_the_tabs(cx: &mut TestAppContext) {
    let mut window = open(cx, report(true));
    window.click("row:1");
    window.tab_to("tab:damage");

    window.press("right");
    assert_eq!(window.view(ReportView::tab), Tab::Buffs);
    assert_eq!(window.focused().as_deref(), Some("tab:buffs"));

    window.press("left");
    assert_eq!(window.view(ReportView::tab), Tab::Damage);
}

#[gpui::test]
fn the_pointer_lifts_a_row_on_enter_and_lets_go_on_leave_only(cx: &mut TestAppContext) {
    let mut window = open(cx, report(true));
    let row = window.cx.debug_bounds("row:1").expect("Mirelle's row");
    assert!(!window.is_drawn("lift"));

    window.move_to(row.center());
    assert!(window.is_drawn("lift"));
    for step in 1..6 {
        window.move_to(row.origin + point(row.size.width * (step as f32 / 6.), row.size.height / 2.));
        assert_eq!(window.view(ReportView::pointer_on), Some(Pointed::Player(MIRELLE)), "moving inside, step {step}");
        assert!(window.is_drawn("lift"));
    }

    let next = window.cx.debug_bounds("row:2").expect("Saebrin's row").center();
    window.move_to(next);
    assert_eq!(window.view(ReportView::pointer_on), Some(Pointed::Player(SAEBRIN)));

    let header = window.cx.debug_bounds("sort:damage").expect("the header").center();
    window.move_to(header);
    assert_eq!(window.view(ReportView::pointer_on), None);
    assert!(!window.is_drawn("lift"));
}

#[gpui::test]
fn an_open_report_follows_the_hide_unknown_players_switch(cx: &mut TestAppContext) {
    let mut window = open(cx, report(true));
    assert!(window.is_drawn("row:4"));

    let meter = window.meter.clone();
    window.cx.update(|_, cx| {
        meter.update(cx, |meter, cx| {
            meter.set_hide_unknown_players(true);
            cx.notify();
        })
    });

    assert!(!window.is_drawn("row:4"));
    assert_eq!(window.order(), vec![MIRELLE, SAEBRIN, DOVREK]);
}

#[gpui::test]
fn another_fight_replaces_the_report_and_the_same_one_leaves_it_as_it_is(cx: &mut TestAppContext) {
    let mut window = open(cx, report(true));
    window.click("row:2");
    let model = window.model.clone();

    window.cx.update(|_, cx| model.update(cx, |model, cx| model.show(report(true), cx)));
    assert_eq!(window.open_player(), Some(SAEBRIN), "the same fight asked for again");

    window.cx.update(|_, cx| model.update(cx, |model, cx| model.show(report(false), cx)));
    assert_eq!(window.open_player(), None, "another fight opens on its table");
    assert!(window.is_drawn("report"));
}
