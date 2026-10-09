use std::sync::Arc;
use std::time::Duration;

use meter_core::engine::meter::DpsMeter;
use meter_core::models::combat::{PvpCombatStatsRow, PvpWatchInfoResponse};
use gpui::{Context, Entity, Subscription, Task};

use super::data::{self, DamageRow, StatsSort, Suggestion, WatchRow};
use crate::meter::MeterModel;

/// How often the watch list and the kill stats are read from the meter.
const POLL_EVERY: Duration = Duration::from_millis(500);

/// What the window reads from the meter; each read takes the capture lock, so
/// it runs off the UI thread.
trait PvpSource: Send + Sync {
    fn watch_info(&self, names: &[String]) -> PvpWatchInfoResponse;
    fn combat_stats(&self) -> Vec<PvpCombatStatsRow>;
    fn clear_combat_stats(&self);
}

impl PvpSource for DpsMeter {
    fn watch_info(&self, names: &[String]) -> PvpWatchInfoResponse {
        self.get_pvp_watch_info(names)
    }

    fn combat_stats(&self) -> Vec<PvpCombatStatsRow> {
        self.get_pvp_combat_stats()
    }

    fn clear_combat_stats(&self) {
        self.clear_pvp_combat_stats();
    }
}

/// The PvP window's state. The watch names live here and nowhere else (they
/// are not saved yet); everything else is read from the meter.
pub struct PvpModel {
    meter_model: Entity<MeterModel>,
    source: Option<Arc<dyn PvpSource>>,
    names: Vec<String>,
    /// The meter's last answer for `names`, which the rows are built from.
    response: PvpWatchInfoResponse,
    watch: Vec<WatchRow>,
    stats: Vec<PvpCombatStatsRow>,
    sort: StatsSort,
    dealt: Vec<DamageRow>,
    received: Vec<DamageRow>,
    main_actor_id: Option<u32>,
    main_actor_name: Option<String>,
    /// Changes whenever `names` or the tally are changed by the window, so a
    /// read begun before is recognised as out of date.
    epoch: u64,
    /// The read in flight; dropped with the window, which ends it.
    reading: Option<Task<()>>,
    /// "Clear" was pressed and the meter has not been told yet.
    clear_pending: bool,
    _poll: Task<()>,
    _snapshots: Subscription,
}

impl PvpModel {
    pub fn new(meter_model: Entity<MeterModel>, cx: &mut Context<Self>) -> Self {
        let source = meter_model.read(cx).meter().map(|meter| meter.clone() as Arc<dyn PvpSource>);
        Self::with_source(meter_model, source, cx)
    }

    fn with_source(meter_model: Entity<MeterModel>, source: Option<Arc<dyn PvpSource>>, cx: &mut Context<Self>) -> Self {
        let snapshots = cx.observe(&meter_model, |this, _, cx| this.read_snapshot(cx));
        let poll = cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(POLL_EVERY).await;
            if this.update(cx, |this, cx| this.poll(cx)).is_err() {
                break;
            }
        });
        let mut model = Self {
            meter_model,
            source,
            names: Vec::new(),
            response: PvpWatchInfoResponse { watch_info: Vec::new(), known_players: Vec::new(), last_dealt_player: None },
            watch: Vec::new(),
            stats: Vec::new(),
            sort: StatsSort::default(),
            dealt: Vec::new(),
            received: Vec::new(),
            main_actor_id: None,
            main_actor_name: None,
            epoch: 0,
            reading: None,
            clear_pending: false,
            _poll: poll,
            _snapshots: snapshots,
        };
        model.read_snapshot(cx);
        model.poll(cx);
        model
    }

    pub fn meter_model(&self) -> &Entity<MeterModel> {
        &self.meter_model
    }

    pub fn watch(&self) -> &[WatchRow] {
        &self.watch
    }

    pub fn stats(&self) -> &[PvpCombatStatsRow] {
        &self.stats
    }

    pub fn sort(&self) -> StatsSort {
        self.sort
    }

    pub fn dealt(&self) -> &[DamageRow] {
        &self.dealt
    }

    pub fn received(&self) -> &[DamageRow] {
        &self.received
    }

    pub fn main_actor_id(&self) -> Option<u32> {
        self.main_actor_id
    }

    /// Your character's name, which marks your row in the kill stats.
    pub fn main_actor_name(&self) -> Option<&str> {
        self.main_actor_name.as_deref()
    }

    pub fn suggestions(&self, typed: &str) -> Vec<Suggestion> {
        data::suggestions(&self.response.known_players, &self.names, typed)
    }

    /// The new name shows at once, "not in range" until the meter has seen
    /// the player.
    pub fn add_name(&mut self, name: &str, cx: &mut Context<Self>) -> bool {
        let added = data::add_name(&mut self.names, name);
        if added {
            self.epoch += 1;
            self.rebuild_watch(cx);
            self.poll(cx);
        }
        added
    }

    pub fn remove_name(&mut self, name: &str, cx: &mut Context<Self>) {
        if data::remove_name(&mut self.names, name) {
            self.epoch += 1;
            self.rebuild_watch(cx);
            self.poll(cx);
        }
    }

    pub fn sort_by(&mut self, sort: StatsSort, cx: &mut Context<Self>) {
        if self.sort != sort {
            self.sort = sort;
            data::sort_stats(&mut self.stats, sort);
            cx.notify();
        }
    }

    /// Empties the running tally at once.
    pub fn clear_stats(&mut self, cx: &mut Context<Self>) {
        self.epoch += 1;
        self.clear_pending = true;
        self.stats.clear();
        self.poll(cx);
        cx.notify();
    }

    /// Starts reading the watch list and the kill stats from the meter on the
    /// background executor, unless the previous read is still running. A
    /// pending clear of the tally goes first, in the same task, so the read
    /// cannot see the old tally.
    fn poll(&mut self, cx: &mut Context<Self>) {
        let Some(source) = self.source.clone() else {
            return;
        };
        if self.reading.is_some() {
            return;
        }
        let names = self.names.clone();
        let epoch = self.epoch;
        let clear_first = std::mem::take(&mut self.clear_pending);
        self.reading = Some(cx.spawn(async move |this, cx| {
            let (response, stats) = cx
                .background_executor()
                .spawn(async move {
                    if clear_first {
                        source.clear_combat_stats();
                    }
                    (source.watch_info(&names), source.combat_stats())
                })
                .await;
            let _ = this.update(cx, |this, cx| this.finish_read(epoch, response, stats, cx));
        }));
    }

    /// An answer to a read that began before the names or the tally changed
    /// is out of date: it is dropped and the meter is asked again.
    fn finish_read(&mut self, epoch: u64, response: PvpWatchInfoResponse, stats: Vec<PvpCombatStatsRow>, cx: &mut Context<Self>) {
        self.reading = None;
        if epoch == self.epoch {
            self.take(response, stats, cx);
        } else {
            self.poll(cx);
        }
    }

    fn take(&mut self, response: PvpWatchInfoResponse, mut stats: Vec<PvpCombatStatsRow>, cx: &mut Context<Self>) {
        data::sort_stats(&mut stats, self.sort);
        let watch = data::watch_rows(&self.names, &response);
        let mut changed = response.known_players.len() != self.response.known_players.len();
        self.response = response;
        if watch != self.watch {
            self.watch = watch;
            changed = true;
        }
        if stats_differ(&stats, &self.stats) {
            self.stats = stats;
            changed = true;
        }
        if changed {
            cx.notify();
        }
    }

    fn rebuild_watch(&mut self, cx: &mut Context<Self>) {
        self.watch = data::watch_rows(&self.names, &self.response);
        cx.notify();
    }

    fn read_snapshot(&mut self, cx: &mut Context<Self>) {
        let meter_model = self.meter_model.read(cx);
        let Some(snapshot) = meter_model.snapshot() else {
            return;
        };
        let dealt = data::dealt_rows(&snapshot.main_actor_dealt_player_overview_stats);
        let received = data::received_rows(&snapshot.main_actor_received_player_overview_stats);
        let main_actor_id = snapshot.combat_infos.main_actor_id;
        let main_actor_name = snapshot.combat_infos.main_actor_name.clone().filter(|name| !name.is_empty());
        if dealt != self.dealt
            || received != self.received
            || main_actor_id != self.main_actor_id
            || main_actor_name != self.main_actor_name
        {
            self.dealt = dealt;
            self.received = received;
            self.main_actor_id = main_actor_id;
            self.main_actor_name = main_actor_name;
            cx.notify();
        }
    }

    /// Feeds what the meter would have answered, for the fixtures that draw
    /// the window without one.
    #[cfg(feature = "probes")]
    pub fn show_fixture(
        &mut self,
        names: &[&str],
        response: PvpWatchInfoResponse,
        stats: Vec<PvpCombatStatsRow>,
        cx: &mut Context<Self>,
    ) {
        self.names = names.iter().map(|name| name.to_string()).collect();
        self.take(response, stats, cx);
    }
}

fn stats_differ(new: &[PvpCombatStatsRow], old: &[PvpCombatStatsRow]) -> bool {
    new.len() != old.len()
        || new.iter().zip(old).any(|(a, b)| {
            (&a.actor_name, &a.server_id, a.damage, a.kills, a.assists, a.deaths)
                != (&b.actor_name, &b.server_id, b.damage, b.kills, b.assists, b.deaths)
        })
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use gpui::{AppContext as _, TestAppContext};
    use meter_core::models::combat::PvpWatchInfo;

    use super::*;
    use crate::meter::OverlaySettings;

    #[derive(Debug, PartialEq)]
    enum Call {
        Clear,
        Read(Vec<String>),
    }

    #[derive(Default)]
    struct FakeSource {
        calls: Mutex<Vec<Call>>,
    }

    impl FakeSource {
        fn reads(&self) -> usize {
            self.calls.lock().unwrap().iter().filter(|call| matches!(call, Call::Read(_))).count()
        }
    }

    impl PvpSource for FakeSource {
        fn watch_info(&self, names: &[String]) -> PvpWatchInfoResponse {
            self.calls.lock().unwrap().push(Call::Read(names.to_vec()));
            let watch_info = names
                .iter()
                .map(|name| PvpWatchInfo {
                    query_name: name.clone(),
                    actor_id: Some(7),
                    actor_name: Some(name.clone()),
                    server_id: Some("1001".to_string()),
                    actor_class: Some("Sorcerer".to_string()),
                    current_hp: Some(50),
                    max_hp: Some(100),
                })
                .collect();
            PvpWatchInfoResponse { watch_info, known_players: Vec::new(), last_dealt_player: None }
        }

        fn combat_stats(&self) -> Vec<PvpCombatStatsRow> {
            let reads = self.reads() as u64;
            vec![PvpCombatStatsRow { actor_name: "Kael".to_string(), server_id: "1001".to_string(), damage: reads, kills: 1, assists: 0, deaths: 0 }]
        }

        fn clear_combat_stats(&self) {
            self.calls.lock().unwrap().push(Call::Clear);
        }
    }

    fn open(cx: &mut TestAppContext) -> (Entity<PvpModel>, Arc<FakeSource>) {
        let source = Arc::new(FakeSource::default());
        let meter_model = cx.new(|_| MeterModel::new(OverlaySettings::default(), None));
        let model = cx.new(|cx| PvpModel::with_source(meter_model, Some(source.clone() as Arc<dyn PvpSource>), cx));
        (model, source)
    }

    fn tick(cx: &mut TestAppContext) {
        cx.executor().advance_clock(POLL_EVERY);
        cx.run_until_parked();
    }

    #[gpui::test]
    fn the_meters_answer_reaches_the_model_off_the_ui_thread(cx: &mut TestAppContext) {
        let (model, source) = open(cx);
        assert_eq!(source.reads(), 0, "opening the window reads nothing on the UI thread");
        model.read_with(cx, |model, _| assert!(model.stats().is_empty()));

        cx.run_until_parked();

        assert_eq!(source.reads(), 1);
        model.read_with(cx, |model, _| {
            assert_eq!(model.stats().len(), 1);
            assert_eq!(model.stats()[0].actor_name, "Kael");
        });
    }

    #[gpui::test]
    fn a_tick_is_skipped_while_the_previous_read_is_running(cx: &mut TestAppContext) {
        let (model, source) = open(cx);
        model.update(cx, |model, cx| model.poll(cx));
        model.update(cx, |model, cx| model.poll(cx));
        cx.run_until_parked();
        assert_eq!(source.reads(), 1, "three polls while the first read runs make one read");

        tick(cx);
        assert_eq!(source.reads(), 2, "the next tick reads again once the read is over");
        model.read_with(cx, |model, _| assert_eq!(model.stats()[0].damage, 2));
    }

    #[gpui::test]
    fn closing_the_window_stops_the_polling(cx: &mut TestAppContext) {
        let (model, source) = open(cx);
        cx.run_until_parked();
        tick(cx);
        assert_eq!(source.reads(), 2);

        drop(model);
        tick(cx);
        tick(cx);

        assert_eq!(source.reads(), 2);
    }

    #[gpui::test]
    fn a_name_added_while_a_read_runs_is_read_again_at_once(cx: &mut TestAppContext) {
        let (model, source) = open(cx);
        model.update(cx, |model, cx| assert!(model.add_name("Vaelith", cx)));
        cx.run_until_parked();

        assert_eq!(*source.calls.lock().unwrap(), vec![Call::Read(Vec::new()), Call::Read(vec!["Vaelith".to_string()])]);
        model.read_with(cx, |model, _| assert!(model.watch().iter().any(|row| row.query_name == "Vaelith" && row.in_range())));
    }

    #[gpui::test]
    fn clearing_the_tally_reaches_the_meter_before_the_next_read(cx: &mut TestAppContext) {
        let (model, source) = open(cx);
        cx.run_until_parked();

        model.update(cx, |model, cx| model.clear_stats(cx));
        model.read_with(cx, |model, _| assert!(model.stats().is_empty(), "the rows go at once"));
        cx.run_until_parked();

        assert_eq!(*source.calls.lock().unwrap(), vec![Call::Read(Vec::new()), Call::Clear, Call::Read(Vec::new())]);
    }
}
