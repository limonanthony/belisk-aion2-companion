use std::sync::Arc;

use gpui::{Context, Entity, Subscription};
use meter_core::report::fight_report::FightReport;

use super::data::{PlayerDetail, ReportData};
use crate::i18n;
use crate::meter::{MeterModel, OverlaySettings};

/// The report window's state: the fight it shows, as it was when the report
/// was asked for (a fight that resumes does not change it), and the player
/// whose details show instead of the table.
pub struct ReportModel {
    meter_model: Entity<MeterModel>,
    report: Arc<FightReport>,
    data: ReportData,
    player: Option<PlayerDetail>,
    /// The overlay settings and "Hide unknown players" `data` was built with.
    built_with: (OverlaySettings, bool),
    _settings: Subscription,
    /// "You", "ID:<n>", the dates and the player count are words in `data`.
    _language: Subscription,
}

impl ReportModel {
    pub fn new(meter_model: Entity<MeterModel>, report: Arc<FightReport>, cx: &mut Context<Self>) -> Self {
        let settings = cx.observe(&meter_model, |this, meter_model, cx| {
            let meter = meter_model.read(cx);
            if meter.settings != this.built_with.0 || meter.hide_unknown_players() != this.built_with.1 {
                this.rebuild(cx);
            }
        });
        let locale = i18n::locale(cx);
        let language = cx.observe(&locale, |this, _, cx| this.rebuild(cx));
        let built_with = inputs(&meter_model, cx);
        let data = ReportData::build(&report, &built_with.0, built_with.1);
        Self { meter_model, report, data, player: None, built_with, _settings: settings, _language: language }
    }

    pub fn data(&self) -> &ReportData {
        &self.data
    }

    /// The player whose details show, if one is open.
    pub fn player(&self) -> Option<&PlayerDetail> {
        self.player.as_ref()
    }

    pub fn meter_model(&self) -> &Entity<MeterModel> {
        &self.meter_model
    }

    /// Shows `report` in place of the one open; the same fight asked for again
    /// is left as it is.
    pub fn show(&mut self, report: Arc<FightReport>, cx: &mut Context<Self>) {
        if *report == *self.report {
            return;
        }
        self.report = report;
        self.player = None;
        self.data = ReportData::build(&self.report, &self.built_with.0, self.built_with.1);
        cx.notify();
    }

    pub fn open_player(&mut self, actor_id: u32, cx: &mut Context<Self>) {
        self.player = self.data.player(&self.report, actor_id, &self.built_with.0);
        cx.notify();
    }

    pub fn back(&mut self, cx: &mut Context<Self>) {
        self.player = None;
        cx.notify();
    }

    fn rebuild(&mut self, cx: &mut Context<Self>) {
        self.built_with = inputs(&self.meter_model, cx);
        let (settings, hide_unknown_players) = &self.built_with;
        self.data = ReportData::build(&self.report, settings, *hide_unknown_players);
        let open = self.player.as_ref().map(|player| player.row.actor_id);
        self.player = open.and_then(|actor_id| self.data.player(&self.report, actor_id, settings));
        cx.notify();
    }
}

fn inputs(meter_model: &Entity<MeterModel>, cx: &Context<ReportModel>) -> (OverlaySettings, bool) {
    let meter = meter_model.read(cx);
    (meter.settings.clone(), meter.hide_unknown_players())
}
