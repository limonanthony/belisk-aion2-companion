use std::sync::Arc;
use std::time::Duration;

use meter_core::engine::meter::DpsMeter;
use meter_core::models::combat::{PvpCombatStatsRow, PvpWatchInfoResponse};
use gpui::{Context, Entity, Subscription, Task};

use super::data::{self, DamageRow, StatsSort, Suggestion, WatchRow};
use crate::meter::MeterModel;

/// How often the watch list and the kill stats are read from the meter.
const POLL_EVERY: Duration = Duration::from_millis(500);

/// The PvP window's state. The watch names live here and nowhere else (they
/// are not saved yet); everything else is read from the meter.
pub struct PvpModel {
    meter_model: Entity<MeterModel>,
    meter: Option<Arc<DpsMeter>>,
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
    _poll: Task<()>,
    _snapshots: Subscription,
}

impl PvpModel {
    pub fn new(meter_model: Entity<MeterModel>, cx: &mut Context<Self>) -> Self {
        let meter = meter_model.read(cx).meter().cloned();
        let snapshots = cx.observe(&meter_model, |this, _, cx| this.read_snapshot(cx));
        let poll = cx.spawn(async move |this, cx| loop {
            cx.background_executor().timer(POLL_EVERY).await;
            if this.update(cx, |this, cx| this.poll(cx)).is_err() {
                break;
            }
        });
        let mut model = Self {
            meter_model,
            meter,
            names: Vec::new(),
            response: PvpWatchInfoResponse { watch_info: Vec::new(), known_players: Vec::new(), last_dealt_player: None },
            watch: Vec::new(),
            stats: Vec::new(),
            sort: StatsSort::default(),
            dealt: Vec::new(),
            received: Vec::new(),
            main_actor_id: None,
            main_actor_name: None,
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
            self.rebuild_watch(cx);
            self.poll(cx);
        }
        added
    }

    pub fn remove_name(&mut self, name: &str, cx: &mut Context<Self>) {
        if data::remove_name(&mut self.names, name) {
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

    /// Empties the running tally at once, as Aether did.
    pub fn clear_stats(&mut self, cx: &mut Context<Self>) {
        if let Some(meter) = &self.meter {
            meter.clear_pvp_combat_stats();
        }
        self.stats.clear();
        cx.notify();
    }

    /// Reads the watch list and the kill stats from the meter, and redraws
    /// only when something differs.
    fn poll(&mut self, cx: &mut Context<Self>) {
        let Some(meter) = &self.meter else {
            return;
        };
        let response = meter.get_pvp_watch_info(&self.names);
        let stats = meter.get_pvp_combat_stats();
        self.take(response, stats, cx);
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
