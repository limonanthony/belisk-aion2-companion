//! Off-screen captures of the fight report window, one per state of
//! docs/gpui-mockups/approved/report.html that invented data can draw, and
//! the two ways it opens: the summary card's button and a History record's.
//! Run by `--capture --fixtures` after the other secondary windows.

use std::path::Path;
use std::sync::Arc;

use gpui::{AsyncApp, Entity};

use crate::capture_secondary::{close, pause, report_window, shoot_window, show_scene, window_handle};
use crate::fixtures::{self, ReportDeaths, ReportEnd, ReportFight, ReportFixture};
use crate::meter::{MeterModel, MeterView, OverlaySettings};
use crate::report::{Column, ReportModel, ReportRequest, ReportView, Tab};
use crate::sample;
use crate::secondary::{self, WindowKind};

type Result<T, E = String> = std::result::Result<T, E>;

const MIRELLE: u32 = 2;
const KORRINTH: u32 = 3;
const SAEBRIN: u32 = 4;
const QUENNA: u32 = 5;
const DOVREK: u32 = 6;
const FERELITH: u32 = 14;
const SERAPHINA: u32 = 1;
const SNIPE: u32 = 14_020_000;
const GALE_ARROW: u32 = 14_110_000;

/// How the report window is drawn for one capture.
struct Shot {
    name: &'static str,
    fixture: ReportFixture,
    show_server: bool,
    player: Option<u32>,
    sort: Column,
    tab: Tab,
    open_skill: Option<u32>,
    pointer: Option<crate::report::Pointed>,
    ring: Option<&'static str>,
    scrolled_rows: usize,
}

const fn shot(name: &'static str, fixture: ReportFixture) -> Shot {
    Shot {
        name,
        fixture,
        show_server: false,
        player: None,
        sort: Column::Damage,
        tab: Tab::Damage,
        open_skill: None,
        pointer: None,
        ring: None,
        scrolled_rows: 0,
    }
}

pub async fn from_fixtures(meter: gpui::WindowHandle<MeterView>, model: &Entity<MeterModel>, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    use crate::report::Pointed;

    let basilus = ReportFixture::of(ReportFight::Basilus);
    let several = ReportFixture { deaths: ReportDeaths::Several, ..basilus };
    let tassin = ReportFixture::of(ReportFight::Tassin);
    let superbia = ReportFixture::of(ReportFight::Superbia);
    let old = ReportFixture { old: true, ..basilus };
    let shots = [
        shot("report-01-fresh", basilus),
        Shot { sort: Column::Healing, ..shot("report-02-sort-heal", basilus) },
        Shot { sort: Column::Taken, ..shot("report-03-sort-taken", basilus) },
        Shot { pointer: Some(Pointed::Player(KORRINTH)), ..shot("report-04-hover", basilus) },
        Shot { ring: Some("row:4"), ..shot("report-05-focus", basilus) },
        shot("report-06-deaths-none", ReportFixture { deaths: ReportDeaths::Nobody, ..basilus }),
        Shot { sort: Column::Deaths, pointer: Some(Pointed::Death(2)), ..shot("report-07-deaths-several", several) },
        shot("report-08-many", tassin),
        Shot { sort: Column::Taken, scrolled_rows: 6, ..shot("report-09-taken-empty", tassin) },
        Shot { show_server: true, ..shot("report-10-long", superbia) },
        shot("report-11-ended", ReportFixture { end: ReportEnd::Ended, deaths: ReportDeaths::Nobody, ..basilus }),
        shot("report-12-old", old),
        shot("report-14-wiped", ReportFixture { end: ReportEnd::Wiped, deaths: ReportDeaths::Several, ..basilus }),
        Shot { player: Some(MIRELLE), ..shot("player-01-damage", basilus) },
        Shot { player: Some(MIRELLE), open_skill: Some(SNIPE), ..shot("player-02-open", basilus) },
        Shot { player: Some(MIRELLE), pointer: Some(Pointed::Skill(GALE_ARROW)), ..shot("player-03-hover", basilus) },
        Shot { player: Some(MIRELLE), pointer: Some(Pointed::Back), ..shot("player-04-back", basilus) },
        Shot { player: Some(MIRELLE), tab: Tab::Buffs, ..shot("player-05-buffs", basilus) },
        Shot { player: Some(DOVREK), tab: Tab::Healing, ..shot("player-06-healing", basilus) },
        Shot { player: Some(SAEBRIN), tab: Tab::Healing, ..shot("player-07-heal-none", several) },
        Shot { player: Some(SAEBRIN), ..shot("player-08-died", several) },
        Shot { player: Some(QUENNA), tab: Tab::Taken, ..shot("player-09-taken", basilus) },
        Shot { player: Some(FERELITH), tab: Tab::Taken, ..shot("player-10-taken-none", tassin) },
        Shot { player: Some(MIRELLE), tab: Tab::Buffs, ring: Some("tab:buffs"), ..shot("player-11-focus", basilus) },
        Shot { player: Some(SERAPHINA), show_server: true, ..shot("player-12-long", superbia) },
        Shot { player: Some(MIRELLE), ..shot("player-13-old", old) },
    ];

    cx.update(|cx| secondary::show_report(model, Arc::new(fixtures::fight_report(basilus)), cx));
    pause(cx, crate::capture_secondary::TICK).await;
    report_window(WindowKind::Report, cx)?;
    cx.update(|cx| secondary::show_report(model, Arc::new(fixtures::fight_report(basilus)), cx));
    let windows = cx.update(|cx| cx.windows().len());
    println!("single instance: the same report opened twice, {windows} windows now (meter + report = 2)");

    for shot in &shots {
        show(model, shot, cx)?;
        shoot_window(WindowKind::Report, dir, shot.name, cx).await?;
    }
    show(model, &self::shot("report-check", basilus), cx)?;
    crate::capture_secondary::check_title(WindowKind::Report, "report", cx).await?;
    close(WindowKind::Report, cx).await?;

    summary_button(meter, model, dir, cx).await
}

fn show(model: &Entity<MeterModel>, shot: &Shot, cx: &mut AsyncApp) -> Result<()> {
    set_show_server(model, shot.show_server, cx);
    cx.update(|cx| secondary::show_report(model, Arc::new(fixtures::fight_report(shot.fixture)), cx));
    let report = cx
        .update(|cx| secondary::model_of::<ReportModel>(WindowKind::Report, cx))
        .ok_or("no report window")?;
    cx.update(|cx| {
        report.update(cx, |report, cx| match shot.player {
            Some(actor_id) => report.open_player(actor_id, cx),
            None => report.back(cx),
        })
    });
    let handle = window_handle::<ReportView>(WindowKind::Report, cx)?;
    handle
        .update(cx, |view, _, cx| {
            view.show_state(shot.sort, shot.tab, shot.open_skill, shot.pointer, shot.ring.map(str::to_string), shot.scrolled_rows, cx)
        })
        .map_err(|error| format!("report window gone: {error:#}"))
}

fn set_show_server(model: &Entity<MeterModel>, show_server: bool, cx: &mut AsyncApp) {
    cx.update(|cx| {
        model.update(cx, |model, cx| {
            if model.settings.show_server != show_server {
                model.set_overlay_settings(OverlaySettings { show_server, ..model.settings.clone() });
                cx.notify();
            }
        })
    });
}

/// The summary card's button asks the windows layer for the report; without
/// a meter it is the fight the card shows, nothing recorded beyond it.
async fn summary_button(meter: gpui::WindowHandle<MeterView>, model: &Entity<MeterModel>, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    show_scene(model, sample::boss_fight(247.0, 40), cx);
    cx.update(|cx| {
        model.update(cx, |model, cx| {
            model.apply(meter_core::events::MeterEvent::DpsSnapshot(sample::boss_fight(248.0, 0)), std::time::Instant::now());
            cx.notify();
        })
    });
    let target_id = cx.update(|cx| model.read(cx).summary().map(|summary| summary.target_id)).ok_or("no summary card after the kill")?;
    meter
        .update(cx, |_, _, cx| cx.emit(ReportRequest::OnMeter { target_id }))
        .map_err(|error| format!("overlay window gone: {error:#}"))?;
    pause(cx, crate::capture_secondary::TICK).await;
    let opened = cx.update(|cx| secondary::window_of(WindowKind::Report, cx).is_some());
    println!("entry point: the summary card's report button: Report window open {opened}");
    if !opened {
        return Err("the summary card's report button opened no report".to_string());
    }
    shoot_window(WindowKind::Report, dir, "report-13-from-summary", cx).await?;
    close(WindowKind::Report, cx).await?;
    wiped_summary_button(meter, model, dir, cx).await
}

/// The wiped card's button opens the attempt's own report, held by the meter.
async fn wiped_summary_button(meter: gpui::WindowHandle<MeterView>, model: &Entity<MeterModel>, dir: &Path, cx: &mut AsyncApp) -> Result<()> {
    use meter_core::events::{MeterEvent, WipedAttempt};
    use meter_core::report::fight_report::{FightOutcome, FightReport};

    let snapshot = sample::boss_fight(177.0, 128);
    show_scene(model, snapshot.clone(), cx);
    let report = FightReport::from_snapshot(&snapshot, sample::BOSS_ID, None).map(|mut report| {
        report.outcome = FightOutcome::Wiped { hp_left: 128, max_hp: 1_000 };
        report
    });
    let held = cx.update(|cx| {
        model.update(cx, |model, cx| {
            let wipe = WipedAttempt { target_id: sample::BOSS_ID, snapshot, record_id: None, report };
            model.apply(MeterEvent::BossWiped(wipe), std::time::Instant::now());
            cx.notify();
            model.wiped_report()
        })
    });
    let held = held.ok_or("no held report after the wipe")?;
    meter
        .update(cx, |_, _, cx| cx.emit(ReportRequest::Held(held)))
        .map_err(|error| format!("overlay window gone: {error:#}"))?;
    pause(cx, crate::capture_secondary::TICK).await;
    let opened = cx.update(|cx| secondary::window_of(WindowKind::Report, cx).is_some());
    println!("entry point: the wiped card's report button: Report window open {opened}");
    if !opened {
        return Err("the wiped card's report button opened no report".to_string());
    }
    shoot_window(WindowKind::Report, dir, "report-15-from-wiped-summary", cx).await?;
    close(WindowKind::Report, cx).await
}
