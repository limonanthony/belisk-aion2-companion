use gpui::{Context, Entity, Subscription};

use super::data::{DetailData, DetailSource};
use crate::i18n;
use crate::meter::{MeterModel, OverlaySettings};

/// The player detail window's state: what it was opened on, and what that
/// shows now. Live sources are rebuilt from each snapshot the meter takes in;
/// only a change redraws.
pub struct DetailModel {
    meter_model: Entity<MeterModel>,
    source: DetailSource,
    data: Option<DetailData>,
    /// The snapshot and settings `data` was built from. The meter notifies
    /// for more than snapshots (personal bests, system figures, the board
    /// switch), and those change nothing here.
    built_from: (u64, OverlaySettings),
    _snapshots: Subscription,
    /// "You" and "Target 123" are words in `data`.
    _language: Subscription,
}

impl DetailModel {
    pub fn new(meter_model: Entity<MeterModel>, source: DetailSource, cx: &mut Context<Self>) -> Self {
        let snapshots = cx.observe(&meter_model, |this, _, cx| {
            if this.source.is_live() && this.inputs(cx) != this.built_from {
                this.rebuild(cx);
            }
        });
        let locale = i18n::locale(cx);
        let language = cx.observe(&locale, |this, _, cx| this.rebuild(cx));
        let built_from = (0, OverlaySettings::default());
        let mut model = Self { meter_model, source, data: None, built_from, _snapshots: snapshots, _language: language };
        model.data = model.build(cx);
        model
    }

    pub fn data(&self) -> Option<&DetailData> {
        self.data.as_ref()
    }

    pub fn source(&self) -> &DetailSource {
        &self.source
    }

    pub fn meter_model(&self) -> &Entity<MeterModel> {
        &self.meter_model
    }

    /// Points the window at another player or board.
    pub fn select(&mut self, source: DetailSource, cx: &mut Context<Self>) {
        self.source = source;
        self.data = self.build(cx);
        cx.notify();
    }

    fn rebuild(&mut self, cx: &mut Context<Self>) {
        let data = self.build(cx);
        if data != self.data {
            self.data = data;
            cx.notify();
        }
    }

    fn inputs(&self, cx: &Context<Self>) -> (u64, OverlaySettings) {
        let meter = self.meter_model.read(cx);
        (meter.snapshot_id(), meter.settings.clone())
    }

    fn build(&mut self, cx: &Context<Self>) -> Option<DetailData> {
        self.built_from = self.inputs(cx);
        let meter = self.meter_model.read(cx);
        DetailData::build(&self.source, meter.snapshot(), &meter.settings)
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::sync::Arc;

    use meter_core::history::HistoryRecord;
    use meter_core::models::combat::{ActorInfo, CombatInfos};
    use gpui::{AppContext as _, TestAppContext};

    use super::*;
    use crate::i18n::LanguageChoice;

    const YOU: u32 = 7;

    fn record_of_your_unnamed_row() -> HistoryRecord {
        let you = ActorInfo {
            id: YOU,
            actor_name: None,
            actor_server_id: None,
            actor_class: None,
            combat_power: None,
            actor_skill_spec: HashMap::new(),
        };
        HistoryRecord {
            id: "100-1".to_string(),
            target_id: 100,
            total_damage: 0,
            target_info: None,
            combat_infos: CombatInfos {
                actor_infos: HashMap::from([(YOU, you)]),
                target_infos: HashMap::new(),
                main_actor_id: Some(YOU),
                main_actor_name: None,
                last_target_by_main_actor: None,
                last_target: None,
                time_now: 0.0,
            },
            player_skill_stats: HashMap::new(),
            player_stats: HashMap::new(),
            use_buffs_by_target: HashMap::new(),
            created_at: 0,
        }
    }

    #[gpui::test]
    fn an_open_window_renames_your_unnamed_row_in_the_new_language(cx: &mut TestAppContext) {
        let meter = cx.new(|_| MeterModel::new(OverlaySettings::default(), None));
        let source = DetailSource::History { actor_id: YOU, record: Arc::new(record_of_your_unnamed_row()) };
        let detail = cx.new(|cx| DetailModel::new(meter, source, cx));
        let name = |cx: &mut TestAppContext| detail.read_with(cx, |detail, _| detail.data().map(|data| data.identity.name.clone()));
        assert_eq!(name(cx).as_deref(), Some("You"));

        cx.update(|cx| crate::i18n::apply(LanguageChoice::French, cx));

        assert_eq!(name(cx).as_deref(), Some("Vous"));
    }
}
