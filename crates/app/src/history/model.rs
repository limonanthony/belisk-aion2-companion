use std::sync::Arc;
use std::time::Duration;

use meter_core::engine::meter::DpsMeter;
use meter_core::history::HistoryRecord;
use gpui::{Context, Entity, Subscription, Task};

use super::list::{HistoryList, PlayerLine};
use crate::i18n::{self, fl};
use crate::meter::MeterModel;

const STATUS_FOR: Duration = Duration::from_secs(4);

/// The history window's state: the list, which record is open, and the line
/// confirming a delete. The records themselves are the meter's.
pub struct HistoryModel {
    meter: Option<Arc<DpsMeter>>,
    pub list: HistoryList,
    /// Settings → Behaviour → "Hide unknown players", as the meter model has it.
    hide_unknown_players: bool,
    /// The record whose players are shown; one at a time.
    pub open: Option<String>,
    /// How many records the last "Delete all" removed, while its line shows.
    deleted: Option<usize>,
    status_timer: Option<Task<()>>,
    _language: Subscription,
    _switch: Subscription,
}

impl HistoryModel {
    pub fn new(meter: Option<Arc<DpsMeter>>, meter_model: &Entity<MeterModel>, cx: &mut Context<Self>) -> Self {
        let locale = i18n::locale(cx);
        let language = cx.observe(&locale, |this, _, cx| this.rebuild_words(cx));
        let switch = cx.observe(meter_model, |this, meter_model, cx| {
            let hide = meter_model.read(cx).hide_unknown_players();
            if hide != this.hide_unknown_players {
                this.hide_unknown_players = hide;
                cx.notify();
            }
        });
        let mut model = Self {
            meter,
            list: HistoryList::default(),
            hide_unknown_players: meter_model.read(cx).hide_unknown_players(),
            open: None,
            deleted: None,
            status_timer: None,
            _language: language,
            _switch: switch,
        };
        model.reload();
        model
    }

    /// The players of a record as the window lists them.
    pub fn players(&self, id: &str) -> Vec<PlayerLine> {
        self.list.players(id, self.hide_unknown_players)
    }

    pub fn status(&self) -> Option<String> {
        self.deleted.map(|count| fl!("history-deleted", count = count))
    }

    /// Reads the records again, after the meter saved or deleted one.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.reload();
        cx.notify();
    }

    /// Writes the rows again after the language changed.
    pub fn rebuild_words(&mut self, cx: &mut Context<Self>) {
        self.list.rebuild_words();
        self.forget_missing_open_record();
        cx.notify();
    }

    pub fn load(&mut self, records: Vec<HistoryRecord>) {
        self.list.set_records(records);
        self.forget_missing_open_record();
    }

    pub fn toggle_open(&mut self, id: &str, cx: &mut Context<Self>) {
        self.open = if self.open.as_deref() == Some(id) { None } else { Some(id.to_string()) };
        cx.notify();
    }

    pub fn delete(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(meter) = &self.meter {
            meter.delete_history_record(id);
        }
        self.reload();
        cx.notify();
    }

    /// Personal bests are kept: they live in their own store.
    pub fn delete_all(&mut self, cx: &mut Context<Self>) {
        let Some(count) = self.meter.as_ref().map(|meter| meter.delete_all_history()) else {
            return;
        };
        self.reload();
        self.report_deleted(count, cx);
    }

    fn report_deleted(&mut self, count: usize, cx: &mut Context<Self>) {
        self.deleted = Some(count);
        self.status_timer = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(STATUS_FOR).await;
            let _ = this.update(cx, |this, cx| {
                this.deleted = None;
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// What deleting every record shows, for the captures that run without a
    /// meter.
    #[cfg(feature = "probes")]
    pub fn show_deleted_all(&mut self, cx: &mut Context<Self>) {
        let count = self.list.total();
        self.load(Vec::new());
        self.report_deleted(count, cx);
    }

    fn reload(&mut self) {
        if let Some(meter) = &self.meter {
            let records = meter.get_history();
            self.load(records);
        }
    }

    fn forget_missing_open_record(&mut self) {
        if self.open.as_deref().is_some_and(|id| self.list.record(id).is_none()) {
            self.open = None;
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use gpui::{AppContext as _, TestAppContext};
    use meter_core::models::combat::{CombatInfos, PlayerOverviewStat};

    use super::*;
    use crate::i18n::{Language, LanguageChoice};
    use crate::meter::OverlaySettings;

    fn meter_model(cx: &mut TestAppContext) -> Entity<MeterModel> {
        cx.new(|_| MeterModel::new(OverlaySettings::default(), None))
    }

    fn record_with_a_stranger() -> HistoryRecord {
        let stat = |id: u32, name: &str, damage: u64| PlayerOverviewStat {
            actor_id: id,
            actor_name: name.to_string(),
            actor_server_id: "1001".to_string(),
            actor_class: "Ranger".to_string(),
            combat_power: None,
            counts: 10,
            total_damage: damage,
            min_damage: 1,
            max_damage: 9,
            special_counts: HashMap::new(),
            dps: damage as f64 / 10.0,
            damage_share: 0.5,
            damage_contribution: 0.5,
        };
        HistoryRecord {
            id: "fight".to_string(),
            target_id: 5,
            total_damage: 800,
            target_info: None,
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
            player_stats: HashMap::from([(1, stat(1, "Mirelle", 500)), (2, stat(2, "", 300))]),
            use_buffs_by_target: HashMap::new(),
            ledger: None,
            wiped: false,
            created_at: 1,
        }
    }

    fn listed(history: &Entity<HistoryModel>, cx: &mut TestAppContext) -> Vec<String> {
        history.read_with(cx, |history, _| history.players("fight").into_iter().map(|player| player.name).collect())
    }

    #[gpui::test]
    fn an_open_history_window_follows_the_hide_unknown_players_switch(cx: &mut TestAppContext) {
        let meter_model = meter_model(cx);
        let history = cx.new(|cx| HistoryModel::new(None, &meter_model, cx));
        history.update(cx, |history, _| history.load(vec![record_with_a_stranger()]));
        assert_eq!(listed(&history, cx), vec!["Mirelle", "ID:2"]);

        meter_model.update(cx, |model, cx| {
            model.set_hide_unknown_players(true);
            cx.notify();
        });
        assert_eq!(listed(&history, cx), vec!["Mirelle"]);

        meter_model.update(cx, |model, cx| {
            model.set_hide_unknown_players(false);
            cx.notify();
        });
        assert_eq!(listed(&history, cx), vec!["Mirelle", "ID:2"]);
    }

    #[gpui::test]
    fn a_history_window_opened_while_the_switch_is_on_hides_unknown_players(cx: &mut TestAppContext) {
        let meter_model = meter_model(cx);
        meter_model.update(cx, |model, _| model.set_hide_unknown_players(true));
        let history = cx.new(|cx| HistoryModel::new(None, &meter_model, cx));
        history.update(cx, |history, _| history.load(vec![record_with_a_stranger()]));

        assert_eq!(listed(&history, cx), vec!["Mirelle"]);
    }

    #[gpui::test]
    fn the_delete_line_follows_a_language_change_while_it_shows(cx: &mut TestAppContext) {
        let meter_model = meter_model(cx);
        let history = cx.new(|cx| HistoryModel::new(None, &meter_model, cx));
        history.update(cx, |history, cx| history.report_deleted(3, cx));
        let english = history.read_with(cx, |history, _| history.status()).expect("the line shows");

        cx.update(|cx| i18n::apply(LanguageChoice::French, cx));

        let french = history.read_with(cx, |history, _| history.status()).expect("the line still shows");
        assert_ne!(french, english);
        assert_eq!(french, fl!("history-deleted", count = 3));
        assert_eq!(i18n::language(), Language::French);
    }
}
