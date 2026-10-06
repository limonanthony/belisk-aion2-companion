use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;

use meter_core::engine::meter::DpsMeter;
use meter_core::events::MeterEvent;
use meter_core::identity::IdentityDecision;
use meter_core::mobs::catalog::MobCatalog;
use meter_core::mobs::store::BossCorrection;
use meter_core::models::combat::CombatSnapshot;
use meter_core::models::diagnostics::MemorySnapshot;
use meter_core::storage::data_storage::MainActorDetectedPayload;
use gpui::{Context, SharedString, Task};

use super::board::{Board, BoardChoice};
use super::decisions::DecisionLog;
use super::fight::{FightTracker, TargetFacts};
use super::focus::{BossFocus, Change, Involvement, Release, Released, SWITCH_AFTER};
use crate::format;
use crate::game_data::PlayerClass;
use super::live::{self, LiveBoard, PaceLine};
use super::session_board::SessionBoard;
use super::settings::OverlaySettings;
use super::shades::ShadeBook;
use super::summary::{FightSummary, Subject};

static SNAPSHOT_IDS: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum ReplayState {
    #[default]
    NotStarted,
    Finished {
        packets: u64,
    },
    Failed(String),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PingQuality {
    Good,
    Fair,
    Poor,
}

/// The machine's load, for the footer.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SystemLine {
    pub ping: Option<(SharedString, PingQuality)>,
    pub cpu: Option<SharedString>,
    pub memory: Option<SharedString>,
}

impl SystemLine {
    fn from_memory(memory: &MemorySnapshot) -> Self {
        let ping = memory.ping_ms.filter(|ms| ms.is_finite()).map(|ms| {
            let ms = ms.round().max(0.0);
            let quality = if ms < 60.0 {
                PingQuality::Good
            } else if ms <= 120.0 {
                PingQuality::Fair
            } else {
                PingQuality::Poor
            };
            (SharedString::from(format!("{ms:.0} ms")), quality)
        });
        Self {
            ping,
            cpu: Some(format!("CPU {:.0}%", memory.system_cpu_percent).into()),
            memory: format::memory(memory.system_memory_used_mb).map(SharedString::from),
        }
    }
}

/// What the footer shows under the active board.
#[derive(Clone, Debug, PartialEq)]
pub enum Footer {
    Session { damage: SharedString, targets: u32 },
    /// Against your personal best, once the boss fight is 10 s old.
    TargetPace(PaceLine),
    /// Everyone's damage on the target, when there is no pace to show.
    TargetPartyDamage(SharedString),
}

/// The user's word on whether a mob is a boss.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossChoice {
    Automatic,
    Boss,
    NotBoss,
}

impl BossChoice {
    fn from_correction(correction: Option<BossCorrection>) -> Self {
        match correction {
            None => Self::Automatic,
            Some(BossCorrection::Boss) => Self::Boss,
            Some(BossCorrection::NotBoss) => Self::NotBoss,
        }
    }

    fn correction(self) -> Option<BossCorrection> {
        match self {
            Self::Automatic => None,
            Self::Boss => Some(BossCorrection::Boss),
            Self::NotBoss => Some(BossCorrection::NotBoss),
        }
    }
}

/// The mob the header names, as the boss-correction menu offers it.
#[derive(Clone, Debug, PartialEq)]
pub struct CorrectionTarget {
    pub name: SharedString,
    pub choice: BossChoice,
}

struct BestEntry {
    character: String,
    mob_code: u32,
    dps: Option<f64>,
}

/// The meter's latest state as the overlay draws it. Meter events are folded
/// in on the UI thread; views observe the entity that owns this.
pub struct MeterModel {
    pub settings: OverlaySettings,
    meter: Option<Arc<DpsMeter>>,
    /// Where boss corrections live: the meter's catalog.
    mobs: Option<Arc<MobCatalog>>,
    snapshot: Option<CombatSnapshot>,
    /// Changes with every snapshot taken in, across models, so a window built
    /// from the snapshot can tell a new one from other notifications.
    snapshot_id: u64,
    live: Option<LiveBoard>,
    summary: Option<FightSummary>,
    tracker: FightTracker,
    choice: BoardChoice,
    /// Each player's shade of their class colour, kept until a reset.
    shades: ShadeBook,
    focus: BossFocus,
    /// The boss that took over as the focused one died: it must not take that
    /// death's summary down.
    coexist_after_kill: Option<u32>,
    decisions: DecisionLog,
    session: Option<SessionBoard>,
    header: Option<SharedString>,
    system: SystemLine,
    best: Option<BestEntry>,
    pub replay: ReplayState,
    unavailable: Option<String>,
    main_actor: Option<MainActorDetectedPayload>,
    deadline: Option<(Instant, Task<()>)>,
}

impl MeterModel {
    pub fn new(settings: OverlaySettings, meter: Option<Arc<DpsMeter>>) -> Self {
        Self {
            tracker: FightTracker::new(settings.show_fight_summary),
            choice: BoardChoice::default(),
            shades: ShadeBook::default(),
            focus: BossFocus::default(),
            coexist_after_kill: None,
            decisions: DecisionLog::new(meter.as_ref().map(|meter| Arc::clone(meter.logger()))),
            session: None,
            header: None,
            settings,
            mobs: meter.as_ref().map(|meter| Arc::clone(meter.mobs())),
            meter,
            snapshot: None,
            snapshot_id: 0,
            live: None,
            summary: None,
            system: SystemLine::default(),
            best: None,
            replay: ReplayState::NotStarted,
            unavailable: None,
            main_actor: None,
            deadline: None,
        }
    }

    /// The meter could not start, so nothing will ever arrive. The overlay
    /// says why instead of waiting for a hit.
    pub fn set_unavailable(&mut self, reason: String) {
        self.meter = None;
        self.mobs = None;
        self.unavailable = Some(reason);
    }

    pub fn unavailable(&self) -> Option<&str> {
        self.unavailable.as_deref()
    }

    /// Switches the `[meter]` decision lines in the log on or off.
    pub fn set_decision_log(&mut self, enabled: bool) {
        self.decisions.set_enabled(enabled);
    }

    /// Takes new overlay options in and redraws what was prepared with the
    /// old ones, unless only the opacity or scale changed: those are painted
    /// from the settings and prepare nothing.
    pub fn set_overlay_settings(&mut self, settings: OverlaySettings) {
        let boards_stay = self.settings.same_boards_as(&settings);
        self.tracker.set_enabled(settings.show_fight_summary);
        self.settings = settings;
        if !boards_stay {
            self.rebuild_live();
            self.rebuild_summary();
        }
    }

    /// Words prepared in the old language ("You", the copied report) are
    /// prepared again.
    pub fn refresh_words(&mut self) {
        self.rebuild_live();
        self.rebuild_summary();
    }

    /// The Target board: the engaged boss, else the current target. `None`
    /// until someone has hit something.
    pub fn live(&self) -> Option<&LiveBoard> {
        self.live.as_ref()
    }

    /// `None` until a mob has taken damage.
    pub fn session(&self) -> Option<&SessionBoard> {
        self.session.as_ref()
    }

    pub fn active_board(&self) -> Board {
        self.choice.active()
    }

    /// Whether the switch was used to override the automatic choice.
    pub fn is_board_pinned(&self) -> bool {
        self.choice.is_pinned()
    }

    /// The switch was clicked on `side`: the other side is pinned, the
    /// pinned side returns to automatic.
    pub fn select_board(&mut self, side: Board) {
        self.choice.select(side);
        self.note_board();
    }

    /// The boss the Target board is locked onto, if one is engaged.
    #[cfg(test)]
    fn focused_boss(&self) -> Option<u32> {
        self.focus.focused()
    }

    /// The name of the Target board's mob: the engaged boss, else the current
    /// target. The view decides how loud it is.
    pub fn header_name(&self) -> Option<&SharedString> {
        self.header.as_ref()
    }

    /// The header clock of the active board.
    pub fn elapsed(&self) -> Option<&SharedString> {
        match self.choice.active() {
            Board::Session => self.session.as_ref().map(|board| &board.elapsed),
            Board::Target => self.live.as_ref().map(|board| &board.elapsed),
        }
    }

    pub fn footer(&self) -> Option<Footer> {
        match self.choice.active() {
            Board::Session => self.session.as_ref().map(|board| Footer::Session {
                damage: board.damage.clone(),
                targets: board.targets,
            }),
            Board::Target => self.live.as_ref().map(|board| match &board.pace {
                Some(pace) => Footer::TargetPace(pace.clone()),
                None => Footer::TargetPartyDamage(board.party_damage.clone()),
            }),
        }
    }

    /// Whether the game has told the meter which character is yours; until
    /// then no row can be marked.
    pub fn identified(&self) -> bool {
        self.main_actor.is_some()
            || self.snapshot.as_ref().is_some_and(live::is_identified)
    }

    pub fn summary(&self) -> Option<&FightSummary> {
        self.summary.as_ref()
    }

    pub fn system(&self) -> &SystemLine {
        &self.system
    }

    pub fn can_reset(&self) -> bool {
        self.meter.is_some()
    }

    /// The latest snapshot the meter took in: what the player detail window
    /// and the PvP lists are built from.
    pub fn snapshot(&self) -> Option<&CombatSnapshot> {
        self.snapshot.as_ref()
    }

    pub fn snapshot_id(&self) -> u64 {
        self.snapshot_id
    }

    /// The target the Target board is about: the engaged boss, else the one
    /// being hit.
    pub fn board_target_id(&self) -> Option<u32> {
        self.snapshot.as_ref().and_then(|snapshot| self.board_target(snapshot))
    }

    /// The meter itself, for the windows that ask it for history and PvP data.
    pub fn meter(&self) -> Option<&Arc<DpsMeter>> {
        self.meter.as_ref()
    }

    /// The player's colour as the meter draws it: their shade while the meter
    /// has drawn them since the last reset, else the plain class colour.
    pub fn player_colour(&self, class: PlayerClass, actor_id: u32, actor_name: &str) -> u32 {
        self.shades.known(class, actor_id, actor_name).unwrap_or_else(|| class.rgb())
    }

    /// [`Self::player_colour`] for a player known only by id, with the name
    /// and class the latest snapshot has for them.
    pub fn actor_colour(&self, actor_id: u32) -> Option<u32> {
        let snapshot = self.snapshot.as_ref()?;
        let (name, class) = match snapshot.session.players.iter().find(|player| player.actor_id == actor_id) {
            Some(player) => (player.actor_name.as_str(), player.actor_class.as_str()),
            None => {
                let actor = snapshot.combat_infos.actor_infos.get(&actor_id)?;
                (actor.actor_name.as_deref().unwrap_or(""), actor.actor_class.as_deref()?)
            }
        };
        Some(self.player_colour(PlayerClass::parse(class)?, actor_id, name))
    }

    /// Folds an event in and reports whether something a view draws changed.
    pub fn apply(&mut self, event: MeterEvent, now: Instant) -> bool {
        match event {
            MeterEvent::DpsSnapshot(snapshot) => self.accept_snapshot(snapshot, now),
            MeterEvent::DpsMemory(memory) => {
                let system = SystemLine::from_memory(&memory);
                if system == self.system {
                    return false;
                }
                self.system = system;
                // The live card only shows these under the pointer, and a
                // redraw every two seconds for a hidden line is wasted work.
                self.live.is_none()
            }
            MeterEvent::PersonalBestsUpdated => {
                self.best = None;
                self.rebuild_live();
                self.live.is_some()
            }
            MeterEvent::PacketReplayFinished(packets) => {
                self.replay = ReplayState::Finished { packets };
                false
            }
            MeterEvent::PacketReplayFailed(error) => {
                self.replay = ReplayState::Failed(error);
                false
            }
            MeterEvent::MainActorDetected(actor) => self.accept_main_actor(actor),
            MeterEvent::OwnIdentity(decision) => {
                self.decisions.event("identity", || describe_identity(&decision));
                false
            }
            MeterEvent::CharacterSeen(_)
            | MeterEvent::DpsMeterStatus(_)
            | MeterEvent::DpsIdleHidden(_)
            | MeterEvent::HistoryUpdated => false,
        }
    }

    /// Wakes the model when something next has to happen without a snapshot:
    /// a summary's minute running out, or the engaged boss going quiet for
    /// good. One timer, re-armed only when the deadline moves earlier.
    pub fn arm_deadline(&mut self, cx: &mut Context<Self>) {
        let Some(due) = [self.tracker.next_deadline(), self.focus.next_deadline()]
            .into_iter()
            .flatten()
            .min()
        else {
            self.deadline = None;
            return;
        };
        if self.deadline.as_ref().is_some_and(|(armed, _)| *armed <= due) {
            return;
        }
        let task = cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(due.saturating_duration_since(Instant::now()))
                .await;
            let _ = this.update(cx, |model, cx| {
                model.deadline = None;
                if model.advance(Instant::now()) {
                    cx.notify();
                }
                model.arm_deadline(cx);
            });
        });
        self.deadline = Some((due, task));
    }

    /// Time has passed with no snapshot. Returns whether a view draws
    /// something different now.
    pub fn advance(&mut self, now: Instant) -> bool {
        let mut changed = self.tracker.advance(now);
        let change = self.focus.advance(now);
        if change.any() {
            self.apply_focus_change(change);
            if let Some(Released { id, reason: Release::Quiet }) = change.released {
                let facts = self.snapshot.as_ref().and_then(|snapshot| facts_of(snapshot, id));
                if let Some(facts) = facts {
                    self.tracker.quiet(facts, now);
                }
            }
            self.rebuild_live();
            changed = true;
        }
        if changed {
            self.rebuild_summary();
        }
        changed
    }

    pub fn dismiss_summary(&mut self, cx: &mut Context<Self>) {
        if self.tracker.dismiss() {
            self.summary = None;
            cx.notify();
        }
        self.arm_deadline(cx);
    }

    /// Starts the meter over. A pinned board goes back to automatic and the
    /// engaged boss is let go.
    pub fn reset(&mut self) {
        self.choice.reset();
        self.shades.clear();
        let change = self.focus.release_all(Release::Reset);
        self.apply_focus_change(change);
        self.note_board();
        if let Some(meter) = &self.meter {
            meter.reset_from_user();
        }
    }

    /// What Copy puts on the clipboard: the summary when one is up, otherwise
    /// the live fight.
    pub fn report(&self) -> Option<String> {
        match &self.summary {
            Some(summary) => Some(summary.report.clone()),
            None => {
                let snapshot = self.snapshot.as_ref()?;
                live::battle_report(snapshot, self.board_target(snapshot)?)
            }
        }
    }

    /// The mob the header names, if the boss-correction menu can act on it:
    /// it needs a target with a mob code and a catalog to save into.
    pub fn correction_target(&self) -> Option<CorrectionTarget> {
        let mobs = self.mobs.as_ref()?;
        let snapshot = self.snapshot.as_ref()?;
        let target = live::target_info(snapshot, self.board_target(snapshot)?)?;
        let mob_code = target.target_mob_code?;
        Some(CorrectionTarget {
            name: self.header.clone()?,
            choice: BossChoice::from_correction(mobs.boss_correction(mob_code)),
        })
    }

    /// Saves the user's word on the mob the header names and redraws the
    /// boards at once, as the next snapshot would: marking a mob a boss
    /// engages it, and ruling the engaged boss out lets it go.
    pub fn set_boss_choice(&mut self, choice: BossChoice) -> bool {
        let now = Instant::now();
        let Some(mobs) = self.mobs.clone() else {
            return false;
        };
        let Some(snapshot) = &mut self.snapshot else {
            return false;
        };
        let id = match self.focus.focused() {
            Some(id) => id,
            None => match live::target_of(snapshot) {
                Some(target) => target.id,
                None => return false,
            },
        };
        let Some(mob_code) = live::target_info(snapshot, id).and_then(|target| target.target_mob_code) else {
            return false;
        };
        mobs.set_boss_correction(mob_code, choice.correction());
        let is_boss = mobs.is_boss(mob_code);
        if let Some(known) = snapshot.combat_infos.target_infos.get_mut(&id) {
            known.is_boss = is_boss;
        }
        if let Some(last) = snapshot.last_target_info.as_mut().filter(|target| target.id == id) {
            last.is_boss = is_boss;
        }
        self.decisions.event("boss correction", || {
            format!("mob {mob_code} set to {choice:?}; boss now: {is_boss}")
        });
        self.settle_focus(now);
        self.rebuild_live();
        true
    }

    /// Dev captures have no meter; corrections go to a catalog in memory.
    #[cfg(any(test, feature = "probes"))]
    pub fn use_memory_mobs(&mut self) {
        let config = std::sync::Arc::new(std::sync::RwLock::new(
            meter_core::config::DpsMeterConfig::default(),
        ));
        self.mobs = Some(Arc::new(MobCatalog::open(config, None)));
    }

    fn accept_main_actor(&mut self, actor: MainActorDetectedPayload) -> bool {
        let previous = self.main_actor.clone();
        if previous.as_ref().is_some_and(|known| {
            known.actor_id == actor.actor_id && known.actor_name == actor.actor_name
        }) {
            return false;
        }
        let named_in_place = previous.as_ref().is_some_and(|known| known.actor_id == actor.actor_id);
        let zone_change = previous
            .as_ref()
            .is_some_and(|known| known.actor_name == actor.actor_name && known.actor_id != actor.actor_id);
        self.decisions.event("identification", || match &previous {
            None if actor.actor_name.is_empty() => {
                format!("local player is actor {}, not named yet", actor.actor_id)
            }
            None => format!("local player is {:?} (actor {})", actor.actor_name, actor.actor_id),
            Some(known) if named_in_place => format!(
                "actor {} changed from {:?} to {:?}",
                actor.actor_id, known.actor_name, actor.actor_name
            ),
            Some(known) if zone_change => format!(
                "zone change: {:?} moved from actor {} to {}",
                actor.actor_name, known.actor_id, actor.actor_id
            ),
            Some(known) => format!(
                "character changed from {:?} (actor {}) to {:?} (actor {})",
                known.actor_name, known.actor_id, actor.actor_name, actor.actor_id
            ),
        });
        if let Some(snapshot) = &mut self.snapshot {
            snapshot.combat_infos.main_actor_id = Some(actor.actor_id);
            snapshot.combat_infos.main_actor_name = Some(actor.actor_name.clone());
        }
        self.main_actor = Some(actor);
        if previous.is_some() && !named_in_place {
            let change = self.focus.release_all(Release::ZoneChange);
            self.apply_focus_change(change);
        }
        self.best = None;
        self.rebuild_live();
        self.rebuild_summary();
        true
    }

    fn accept_snapshot(&mut self, mut snapshot: CombatSnapshot, now: Instant) -> bool {
        if !live::is_identified(&snapshot) {
            if let Some(actor) = &self.main_actor {
                snapshot.combat_infos.main_actor_id = Some(actor.actor_id);
                snapshot.combat_infos.main_actor_name = Some(actor.actor_name.clone());
            }
        }
        let emptied = snapshot.total_damage == 0 && snapshot.session.players.is_empty();
        let target = live::target_of(&snapshot);
        // A target the game has not described yet: the Tauri overlay skips
        // these too, so a half-known fight never replaces a known one.
        if target.is_some_and(|target| target.target_mob_code.is_none()) {
            return false;
        }
        if emptied {
            self.choice.reset();
            self.shades.clear();
            let change = self.focus.release_all(Release::Reset);
            self.apply_focus_change(change);
        }
        self.snapshot = Some(snapshot);
        self.snapshot_id = SNAPSHOT_IDS.fetch_add(1, Ordering::Relaxed) + 1;
        let tracked = self.settle_focus(now);
        let facts = self
            .snapshot
            .as_ref()
            .and_then(|snapshot| tracked.and_then(|id| facts_of(snapshot, id)));
        let summary_changed = self.tracker.observe(facts, now);
        if let Some(boss) = self.coexist_after_kill.take() {
            self.tracker.coexist_with(boss);
        }
        self.rebuild_live();
        if summary_changed {
            self.rebuild_summary();
        }
        true
    }

    /// Lets the focus look at the latest snapshot. Returns the target the
    /// fight tracker should follow: the engaged boss, the one just let go (its
    /// death is its summary), else the current target.
    fn settle_focus(&mut self, now: Instant) -> Option<u32> {
        let snapshot = self.snapshot.as_ref()?;
        let current = live::target_of(snapshot).map(|target| target.id);
        let bosses: Vec<u32> = snapshot
            .combat_infos
            .target_infos
            .values()
            .filter(|target| target.is_boss)
            .map(|target| target.id)
            .collect();
        let change = self.focus.observe(
            current,
            &bosses,
            |id| facts_of(snapshot, id),
            |id| involvement_of(snapshot, id),
            |id| my_hit_age(snapshot, id),
            now,
        );
        self.apply_focus_change(change);
        let snapshot = self.snapshot.as_ref()?;
        let died = change
            .released
            .filter(|released| released.reason == Release::Died)
            .map(|released| released.id);
        // A boss that took over the moment the other died must not hide the
        // death's summary.
        let tracked = died
            .or(self.focus.focused())
            .or(change.released.map(|released| released.id))
            .or(current)?;
        if let (Some(_), Some(engaged)) = (died, change.engaged) {
            self.coexist_after_kill = Some(engaged);
        }
        // A stranger's boss dying is not your kill, so no summary for it.
        let strangers = facts_of(snapshot, tracked).is_some_and(|facts| facts.is_boss)
            && involvement_of(snapshot, tracked).is_none();
        (!strangers).then_some(tracked)
    }

    fn apply_focus_change(&mut self, change: Change) {
        if change.released.is_some() {
            self.choice.boss_released();
        }
        if change.engaged.is_some() {
            self.choice.boss_engaged();
        }
        if !change.any() {
            return;
        }
        let snapshot = self.snapshot.as_ref();
        let name_of = |id: u32| {
            snapshot
                .and_then(|snapshot| live::target_info(snapshot, id))
                .map_or_else(|| format!("target {id}"), live::target_name)
        };
        if let Some(Released { id, reason }) = change.released {
            self.decisions.event("boss focus", || {
                format!("released {:?} ({id}): {}", name_of(id), reason.describe())
            });
        }
        if let Some(id) = change.engaged {
            let why = change.engaged_because.map_or("", Involvement::describe);
            let timing = change.moved_after_secs.map_or(String::new(), |secs| {
                format!(
                    " (moved there: the other boss left alone, or this one the only target, for {secs:.1} s; the switch needs {} s)",
                    SWITCH_AFTER.as_secs()
                )
            });
            self.decisions.event("boss focus", || {
                format!(
                    "engaged {:?} ({id}) because {why}{timing}; the Target board stays on it until it dies or is let go",
                    name_of(id)
                )
            });
        }
        self.note_board();
    }

    fn note_board(&mut self) {
        let (active, pinned, focused) = (self.choice.active(), self.choice.is_pinned(), self.focus.focused());
        self.decisions.on_change("board", || {
            let why = match (pinned, focused) {
                (true, _) => "pinned by the user".to_string(),
                (false, Some(id)) => format!("automatic: boss {id} engaged"),
                (false, None) => "automatic: no boss engaged".to_string(),
            };
            format!("{active:?} board ({why})")
        });
    }

    /// The mob the Target board is about: the engaged boss, else the current
    /// target.
    fn board_target(&self, snapshot: &CombatSnapshot) -> Option<u32> {
        self.focus.focused().or_else(|| live::target_of(snapshot).map(|target| target.id))
    }

    fn rebuild_live(&mut self) {
        let Some(snapshot) = &self.snapshot else {
            self.live = None;
            self.session = None;
            self.header = None;
            return;
        };
        self.session = SessionBoard::build(snapshot, &self.settings, &mut self.shades);
        let target_id = self.focus.focused().or_else(|| live::target_of(snapshot).map(|target| target.id));
        self.header = target_id
            .and_then(|id| live::target_info(snapshot, id))
            .map(|target| self.settings.display_name(&live::target_name(target)).into());
        let (best, meter) = (&mut self.best, self.meter.as_deref());
        self.live = target_id.and_then(|id| {
            LiveBoard::build(snapshot, id, &self.settings, &mut self.shades, |character, mob| {
                personal_best(best, meter, character, mob)
            })
        });
        self.note_board();
        self.note_rows(target_id);
    }

    /// Why your row is, or is not, on each board; written when it changes.
    fn note_rows(&mut self, target_id: Option<u32>) {
        if !self.decisions.enabled() {
            return;
        }
        let Some(snapshot) = &self.snapshot else {
            return;
        };
        if let Some(view) = target_id.and_then(|id| live::target_view(snapshot, id)) {
            let found = live::find_you(
                snapshot,
                view.players.iter().map(|player| (player.actor_id, player.actor_name.as_str())),
            );
            self.decisions.on_change("your row on the Target board", || {
                live::you_status(snapshot, found, view.players.len(), self.settings.players_shown)
            });
        }
        let players = &snapshot.session.players;
        let found = live::find_you(
            snapshot,
            players.iter().map(|player| (player.actor_id, player.actor_name.as_str())),
        );
        self.decisions.on_change("your row on the Session board", || {
            live::you_status(snapshot, found, players.len(), self.settings.players_shown)
        });
    }

    fn rebuild_summary(&mut self) {
        let (Some(shown), Some(snapshot)) = (self.tracker.shown(), &self.snapshot) else {
            self.summary = None;
            return;
        };
        let (best, meter) = (&mut self.best, self.meter.as_deref());
        self.summary = FightSummary::build(
            snapshot,
            shown.target_id,
            shown.kind,
            &self.settings,
            |character, mob| personal_best(best, meter, character, mob),
        );
        if let Some(summary) = &self.summary {
            self.decisions.on_change("summary", || {
                let about = match &summary.subject {
                    Subject::Player { name, by } => format!("about {name:?}, matched by {by:?}"),
                    Subject::Party { identified: false } => {
                        "about the party: the game has not named the local player".to_string()
                    }
                    Subject::Party { identified: true } => {
                        "about the party: the local player did no damage to this boss".to_string()
                    }
                };
                format!("{:?} for {:?} (target {}), {about}", shown.kind, summary.boss, shown.target_id)
            });
        } else {
            self.decisions.forget("summary");
        }
    }

    /// For dev captures, which have no meter to ask.
    #[cfg(feature = "probes")]
    pub fn seed_personal_best(&mut self, character: &str, mob_code: u32, dps: f64) {
        self.best = Some(BestEntry { character: character.to_string(), mob_code, dps: Some(dps) });
    }
}

/// Seconds since the local player's last hit on `target`, from the core's
/// per-player last-hit times (`TargetInfo::target_last_time`, summons merged
/// into their owner). `None` while unidentified or if they never hit it.
fn my_hit_age(snapshot: &CombatSnapshot, target: u32) -> Option<f64> {
    if !live::is_identified(snapshot) {
        return None;
    }
    let rows: Vec<_> = snapshot.by_target_player_stats.get(&target)?.values().collect();
    let found = live::find_you(snapshot, rows.iter().map(|row| (row.actor_id, row.actor_name.as_str())))?;
    let last = live::target_info(snapshot, target)?
        .target_last_time
        .get(&rows[found.index].actor_id)?;
    Some((snapshot.combat_infos.time_now - last).max(0.0))
}

/// Why `boss` is the local player's fight. The core records every hit as
/// `by_target_player_stats[target][attacker]`, mobs included, so "you hit it"
/// is your row under the boss and "it hit you" is the boss's id under your own
/// actor id. Before the game has named you nothing can be told.
fn involvement_of(snapshot: &CombatSnapshot, boss: u32) -> Option<Involvement> {
    if !live::is_identified(snapshot) {
        return Some(Involvement::Unidentified);
    }
    let hit_by_you = snapshot.by_target_player_stats.get(&boss).is_some_and(|rows| {
        live::find_you(snapshot, rows.values().map(|row| (row.actor_id, row.actor_name.as_str()))).is_some()
    });
    if hit_by_you {
        return Some(Involvement::YouHitIt);
    }
    let hit_you = snapshot
        .combat_infos
        .main_actor_id
        .and_then(|me| snapshot.by_target_player_stats.get(&me))
        .is_some_and(|attackers| attackers.contains_key(&boss));
    hit_you.then_some(Involvement::ItHitYou)
}

/// What the fight tracker and the focus need to know about a target.
fn facts_of(snapshot: &CombatSnapshot, id: u32) -> Option<TargetFacts> {
    let target = live::target_info(snapshot, id)?;
    Some(TargetFacts {
        id,
        is_boss: target.is_boss,
        current_hp: target.current_hp,
        max_hp: target.max_hp,
        duration_secs: live::fight_secs(target),
    })
}

fn describe_identity(decision: &IdentityDecision) -> String {
    match decision {
        IdentityDecision::SignatureBound { actor_id } => {
            format!("actor {actor_id} is you (remain-HP signature); shown as You until a name is known")
        }
        IdentityDecision::RememberedApplied { actor_id, name, class } => {
            format!("actor {actor_id} named {name:?} from the remembered characters: its skills show {class}")
        }
        IdentityDecision::RememberedRejected { actor_id, class, remembered } => format!(
            "actor {actor_id} stays You: none of the {remembered} remembered characters is {class}, the class its skills show"
        ),
        IdentityDecision::GameOverrode { actor_id, previous_actor_id, previous_name } => format!(
            "the game's name packet names actor {actor_id}, replacing actor {previous_actor_id:?} named {previous_name:?}"
        ),
        IdentityDecision::CarriedRejected { actor_id, name, carried_class, class } => format!(
            "actor {actor_id} no longer carries the name {name:?}: its skills show {class}, the carried character was {carried_class:?}"
        ),
    }
}

/// The best DPS for a character on a boss, asked of the meter once per pair.
fn personal_best(
    cache: &mut Option<BestEntry>,
    meter: Option<&DpsMeter>,
    character: &str,
    mob_code: u32,
) -> Option<f64> {
    match cache {
        Some(entry) if entry.character == character && entry.mob_code == mob_code => entry.dps,
        _ => {
            let dps = meter.and_then(|meter| meter.personal_best(character, mob_code)).map(|best| best.dps);
            *cache = Some(BestEntry { character: character.to_string(), mob_code, dps });
            dps
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use meter_core::models::combat::{CombatInfos, PlayerOverviewStat, TargetInfo};
    use meter_core::models::session::{SessionPlayerStat, SessionSummary};

    use super::*;
    use crate::game_data::PlayerClass;
    use crate::i18n::fl;
    use crate::meter::live::Lead;
    use crate::meter::fight::SummaryKind;

    fn player(id: u32, name: &str, damage: u64, dps: f64) -> PlayerOverviewStat {
        PlayerOverviewStat {
            actor_id: id,
            actor_name: name.to_string(),
            actor_server_id: "1001".to_string(),
            actor_class: "Ranger".to_string(),
            combat_power: Some(38_904),
            counts: 10,
            total_damage: damage,
            min_damage: 1,
            max_damage: 1,
            special_counts: HashMap::new(),
            dps,
            damage_share: 0.5,
            damage_contribution: 0.4,
        }
    }

    /// The per-target views the meter reads, filled in from the last target as
    /// the core does for every target it keeps. Run again after editing.
    fn indexed(mut snapshot: CombatSnapshot) -> CombatSnapshot {
        if let Some(target) = snapshot.last_target_info.clone() {
            snapshot.by_target_player_stats.insert(
                target.id,
                snapshot
                    .last_target_all_players_overview_stats
                    .iter()
                    .map(|player| (player.actor_id, player.clone()))
                    .collect(),
            );
            snapshot.combat_infos.target_infos.insert(target.id, target);
        }
        snapshot
    }

    fn snapshot(hp: u32, mob_code: Option<u32>, secs: f64) -> CombatSnapshot {
        let target = TargetInfo {
            id: 7,
            target_mob_code: mob_code,
            target_name: Some("Boss".to_string()),
            is_boss: true,
            current_hp: Some(hp),
            max_hp: Some(1_000),
            target_start_time: HashMap::from([(1, 100.0)]),
            target_last_time: HashMap::from([(1, 100.0 + secs)]),
        };
        indexed(CombatSnapshot {
            total_damage: 3_000,
            by_target_player_skill_stats: HashMap::new(),
            by_target_player_stats: HashMap::new(),
            use_buffs_by_target: HashMap::new(),
            combat_infos: CombatInfos {
                actor_infos: HashMap::new(),
                target_infos: HashMap::new(),
                main_actor_id: Some(2),
                main_actor_name: Some("You".to_string()),
                last_target_by_main_actor: Some(7),
                last_target: Some(7),
                time_now: 0.0,
            },
            last_target_info: Some(target),
            last_target_all_players_overview_stats: vec![
                player(2, "You", 1_000, 50.0),
                player(1, "Other", 2_000, 100.0),
            ],
            main_actor_received_player_overview_stats: Vec::new(),
            main_actor_dealt_player_overview_stats: Vec::new(),
            session: SessionSummary::default(),
        })
    }

    #[test]
    fn a_snapshot_ranks_players_and_marks_you() {
        let mut model = MeterModel::new(OverlaySettings::default(), None);
        assert!(model.apply(MeterEvent::DpsSnapshot(snapshot(500, Some(9), 30.0)), Instant::now()));
        let live = model.live().unwrap();
        assert_eq!(live.rows.len(), 2);
        assert_eq!(live.rows[0].name.as_deref(), Some("Other"));
        assert!(live.rows[1].is_you && live.rows[1].rank.as_ref() == "2");
        assert_eq!(live.rows[1].bar, 0.5);
        assert_eq!(live.rows[1].detail.as_deref(), Some("38,904"));
        assert!(matches!(&live.lead, Lead::You { rank, .. } if rank.as_ref() == "2"));
        assert_eq!(live.boss_hp.as_ref().map(|gauge| gauge.fraction), Some(0.5));
    }

    #[test]
    fn your_row_is_never_the_one_dropped() {
        let mut crowded = snapshot(500, Some(9), 30.0);
        crowded.last_target_all_players_overview_stats = (10..22)
            .map(|id| player(id, &format!("P{id}"), 10_000 + u64::from(id), 100.0))
            .chain(std::iter::once(player(2, "You", 10, 1.0)))
            .collect();
        let crowded = indexed(crowded);
        let live = LiveBoard::build(&crowded, 7, &OverlaySettings::default(), &mut ShadeBook::default(), |_, _| None).unwrap();
        assert_eq!(live.rows.len(), 10);
        let last = live.rows.last().unwrap();
        assert!(last.is_you);
        assert_eq!(last.rank.as_ref(), "13");
        assert_eq!(last.bar, 0.06);
    }

    #[test]
    fn pace_needs_ten_seconds_of_a_boss_fight_and_a_best() {
        let settings = OverlaySettings::default();
        let early = LiveBoard::build(&snapshot(500, Some(9), 9.0), 7, &settings, &mut ShadeBook::default(), |_, _| Some(40.0)).unwrap();
        assert!(early.pace.is_none());

        let mut asked = None;
        let live = LiveBoard::build(&snapshot(500, Some(9), 12.0), 7, &settings, &mut ShadeBook::default(), |character, mob| {
            asked = Some((character.to_string(), mob));
            Some(40.0)
        })
        .unwrap();
        assert_eq!(asked, Some(("You".to_string(), 9)));
        let pace = live.pace.unwrap();
        assert!(pace.ahead);
        assert_eq!(pace.figure.as_ref(), "+25.0%");
        assert_eq!(pace.versus_best.as_ref(), fl!("meter-pace-versus-best", pace = "+25.0%"));

        let unknown = LiveBoard::build(&snapshot(500, Some(9), 12.0), 7, &settings, &mut ShadeBook::default(), |_, _| None).unwrap();
        assert!(unknown.pace.is_none());
    }

    #[test]
    fn masking_covers_players_and_the_target() {
        let settings = OverlaySettings { mask_nicknames: true, ..OverlaySettings::default() };
        let mut model = MeterModel::new(settings, None);
        model.apply(MeterEvent::DpsSnapshot(snapshot(500, Some(9), 30.0)), Instant::now());
        assert_eq!(model.header_name().map(|name| name.as_ref()), Some("B**s"));
        assert_eq!(model.live().unwrap().rows[0].name.as_deref(), Some("O***r"));
    }

    #[test]
    fn new_overlay_settings_redraw_what_was_prepared_with_the_old_ones() {
        let mut model = MeterModel::new(OverlaySettings::default(), None);
        model.apply(MeterEvent::DpsSnapshot(snapshot(500, Some(9), 30.0)), Instant::now());
        assert_eq!(model.header_name().map(|name| name.as_ref()), Some("Boss"));

        model.set_overlay_settings(OverlaySettings { mask_nicknames: true, ..OverlaySettings::default() });

        assert_eq!(model.header_name().map(|name| name.as_ref()), Some("B**s"));
        assert_eq!(model.live().unwrap().rows[0].name.as_deref(), Some("O***r"));
    }

    #[test]
    fn switching_summaries_off_takes_the_one_that_is_up_down() {
        let mut model = MeterModel::new(OverlaySettings::default(), None);
        let now = Instant::now();
        model.apply(MeterEvent::DpsSnapshot(snapshot(500, Some(9), 30.0)), now);
        model.apply(MeterEvent::DpsSnapshot(snapshot(0, Some(9), 31.0)), now);
        assert!(model.summary().is_some());

        model.set_overlay_settings(OverlaySettings { show_fight_summary: false, ..OverlaySettings::default() });

        assert!(model.summary().is_none());
    }

    #[test]
    fn a_boss_correction_switches_the_automatic_board_at_once() {
        let mut model = MeterModel::new(OverlaySettings::default(), None);
        model.use_memory_mobs();
        let mut trash = snapshot(500, Some(9), 30.0);
        trash.last_target_info.as_mut().unwrap().is_boss = false;
        model.apply(MeterEvent::DpsSnapshot(indexed(trash)), Instant::now());
        assert_eq!(model.active_board(), Board::Session);
        assert_eq!(model.correction_target().unwrap().choice, BossChoice::Automatic);

        assert!(model.set_boss_choice(BossChoice::Boss));
        assert_eq!(model.active_board(), Board::Target);
        assert_eq!(model.correction_target().unwrap().choice, BossChoice::Boss);

        assert!(model.set_boss_choice(BossChoice::Automatic));
        assert_eq!(model.active_board(), Board::Session);
        assert_eq!(model.correction_target().unwrap().choice, BossChoice::Automatic);
    }

    #[test]
    fn without_a_catalog_or_mob_code_there_is_nothing_to_correct() {
        let mut model = MeterModel::new(OverlaySettings::default(), None);
        model.apply(MeterEvent::DpsSnapshot(snapshot(500, Some(9), 30.0)), Instant::now());
        assert!(model.correction_target().is_none());
        assert!(!model.set_boss_choice(BossChoice::Boss));
    }

    #[test]
    fn a_target_without_a_mob_code_is_skipped() {
        let mut model = MeterModel::new(OverlaySettings::default(), None);
        assert!(!model.apply(MeterEvent::DpsSnapshot(snapshot(500, None, 30.0)), Instant::now()));
        assert!(model.live().is_none());
    }

    #[test]
    fn a_kill_puts_up_the_final_summary() {
        let mut model = MeterModel::new(OverlaySettings::default(), None);
        let now = Instant::now();
        model.apply(MeterEvent::DpsSnapshot(snapshot(500, Some(9), 30.0)), now);
        model.apply(MeterEvent::DpsSnapshot(snapshot(0, Some(9), 31.0)), now);
        let summary = model.summary().unwrap();
        assert_eq!(summary.kind, SummaryKind::Final);
        assert_eq!(summary.rank.as_deref(), Some("2/2"));
        assert!(model.report().unwrap().starts_with("Boss · defeated in 0:31"));
    }

    #[test]
    fn system_figures_redraw_only_while_waiting() {
        let mut model = MeterModel::new(OverlaySettings::default(), None);
        let memory = |cpu: f32| MemorySnapshot {
            cpu_percent: 0.0,
            rss_mb: 0.0,
            vms_mb: 0.0,
            memory_percent: 0.0,
            system_cpu_percent: cpu,
            system_memory_used_mb: 11_468.8,
            system_memory_total_mb: 32_768.0,
            cap_device: None,
            cap_port: None,
            packet_sizes: HashMap::new(),
            ping_ms: Some(42.4),
            main_actor_name: None,
        };
        let now = Instant::now();
        assert!(model.apply(MeterEvent::DpsMemory(memory(18.0)), now));
        assert!(!model.apply(MeterEvent::DpsMemory(memory(18.2)), now));
        assert_eq!(model.system().ping, Some(("42 ms".into(), PingQuality::Good)));

        model.apply(MeterEvent::DpsSnapshot(snapshot(500, Some(9), 30.0)), now);
        assert!(!model.apply(MeterEvent::DpsMemory(memory(40.0)), now));
        assert_eq!(model.system().cpu.as_deref(), Some("CPU 40%"));
    }

    #[test]
    fn events_nothing_draws_do_not_ask_for_a_redraw() {
        let mut model = MeterModel::new(OverlaySettings::default(), None);
        let now = Instant::now();
        assert!(!model.apply(MeterEvent::HistoryUpdated, now));
        assert!(!model.apply(MeterEvent::PersonalBestsUpdated, now));
        assert!(!model.apply(MeterEvent::DpsMeterStatus(true), now));
        assert!(!model.apply(MeterEvent::DpsIdleHidden(true), now));
    }

    #[test]
    fn a_replay_outcome_is_kept() {
        let mut model = MeterModel::new(OverlaySettings::default(), None);
        let now = Instant::now();
        model.apply(MeterEvent::PacketReplayFinished(42), now);
        assert_eq!(model.replay, ReplayState::Finished { packets: 42 });
        model.apply(MeterEvent::PacketReplayFailed("bad".into()), now);
        assert_eq!(model.replay, ReplayState::Failed("bad".into()));
    }

    #[test]
    fn a_detected_character_redraws_and_moves_the_you_marker() {
        let mut model = MeterModel::new(OverlaySettings::default(), None);
        let now = Instant::now();
        let detected = |name: &str| {
            MeterEvent::MainActorDetected(MainActorDetectedPayload {
                actor_id: 1,
                actor_name: name.to_string(),
                sid: None,
            })
        };

        assert!(model.apply(detected("Other"), now));
        assert!(model.identified());
        assert!(!model.apply(detected("Other"), now), "the same character again changes nothing");

        model.apply(MeterEvent::DpsSnapshot(snapshot(500, Some(9), 30.0)), now);
        assert!(model.apply(detected("Renamed"), now));
        assert!(model.live().unwrap().rows.iter().all(|row| !row.is_you));
    }

    #[test]
    fn an_unavailable_meter_keeps_its_reason_and_cannot_reset() {
        let mut model = MeterModel::new(OverlaySettings::default(), None);
        assert_eq!(model.unavailable(), None);
        model.set_unavailable("no capture backend".to_string());
        assert_eq!(model.unavailable(), Some("no capture backend"));
        assert!(!model.can_reset());
    }

    fn open_world(target_id: u32, is_boss: bool) -> CombatSnapshot {
        let mut snapshot = snapshot(500, Some(9), 30.0);
        let target = snapshot.last_target_info.as_mut().unwrap();
        target.id = target_id;
        target.is_boss = is_boss;
        snapshot.combat_infos.last_target = Some(target_id);
        snapshot.session = SessionSummary {
            total_damage: 5_000,
            target_count: 3,
            combat_time_secs: 75.0,
            players: vec![
                session_player(1, "Other", 3_000, 60.0),
                session_player(2, "You", 2_000, 40.0),
            ],
        };
        snapshot.combat_infos.target_infos.clear();
        snapshot.by_target_player_stats.clear();
        indexed(snapshot)
    }

    fn session_player(id: u32, name: &str, damage: u64, dps: f64) -> SessionPlayerStat {
        SessionPlayerStat {
            actor_id: id,
            earlier_actor_ids: Vec::new(),
            actor_name: name.to_string(),
            actor_server_id: "1001".to_string(),
            actor_class: "Ranger".to_string(),
            combat_power: None,
            total_damage: damage,
            combat_time_secs: damage as f64 / dps,
            dps,
            damage_share: damage as f64 / 5_000.0,
        }
    }

    fn model_on(snapshot: CombatSnapshot) -> MeterModel {
        let mut model = MeterModel::new(OverlaySettings::default(), None);
        model.apply(MeterEvent::DpsSnapshot(snapshot), Instant::now());
        model
    }

    fn show(model: &mut MeterModel, snapshot: CombatSnapshot) {
        model.apply(MeterEvent::DpsSnapshot(snapshot), Instant::now());
    }

    #[test]
    fn the_open_world_shows_the_session_and_a_boss_shows_its_own_board() {
        let mut model = model_on(open_world(7, false));
        assert_eq!(model.active_board(), Board::Session);
        assert!(!model.is_board_pinned());

        show(&mut model, open_world(8, true));
        assert_eq!(model.active_board(), Board::Target);
    }

    #[test]
    fn the_session_board_ranks_by_damage_and_marks_you() {
        let model = model_on(open_world(7, false));

        let session = model.session().unwrap();

        assert_eq!(session.rows.len(), 2);
        assert_eq!(session.rows[0].name.as_deref(), Some("Other"));
        assert_eq!(session.rows[0].pct.as_ref(), "60.0%");
        assert_eq!(session.rows[0].dps.as_ref(), "60");
        assert!(session.rows[1].is_you && session.rows[1].rank.as_ref() == "2");
        assert_eq!(session.rows[1].bar, 2.0 / 3.0);
        assert!(matches!(&session.lead, Lead::You { rank, .. } if rank.as_ref() == "2"));
        assert_eq!(session.elapsed.as_ref(), "1:15");
    }

    #[test]
    fn players_of_one_class_keep_their_own_shade_on_both_boards() {
        let colour_of = |rows: &[live::PlayerRow], name: &str| {
            rows.iter().find(|row| row.name.as_deref() == Some(name)).and_then(|row| row.colour)
        };
        let mut model = model_on(open_world(7, false));
        let session = &model.session().unwrap().rows;
        let (other, you) = (colour_of(session, "Other"), colour_of(session, "You"));
        assert_eq!(other, Some(PlayerClass::Ranger.rgb()), "the first ranger keeps the class colour");
        assert!(you.is_some() && you != other, "the second ranger gets a shade");
        let target = &model.live().unwrap().rows;
        assert_eq!((colour_of(target, "Other"), colour_of(target, "You")), (other, you));
        assert!(matches!(model.session().unwrap().lead, Lead::You { colour, .. } if colour == you));

        let mut swapped = open_world(7, false);
        swapped.session.players.reverse();
        swapped.session.players[0].total_damage = 4_000;
        show(&mut model, swapped);
        let session = &model.session().unwrap().rows;
        assert_eq!(session[0].name.as_deref(), Some("You"), "ranks swapped");
        assert_eq!((colour_of(session, "Other"), colour_of(session, "You")), (other, you));

        let mut emptied = open_world(7, false);
        emptied.total_damage = 0;
        emptied.session = SessionSummary::default();
        emptied.last_target_all_players_overview_stats.clear();
        emptied.by_target_player_stats.clear();
        show(&mut model, emptied);
        let mut after = open_world(7, false);
        after.session.players.reverse();
        show(&mut model, after);
        let session = &model.session().unwrap().rows;
        assert_eq!(colour_of(session, "You"), other, "after a reset the first seen keeps the class colour");
    }

    #[test]
    fn the_header_names_the_current_target_on_either_board() {
        let mut model = model_on(open_world(7, false));
        assert_eq!(model.header_name().map(|name| name.as_ref()), Some("Boss"));
        assert_eq!(model.elapsed().map(|time| time.as_ref()), Some("1:15"));

        show(&mut model, open_world(8, true));
        assert_eq!(model.header_name().map(|name| name.as_ref()), Some("Boss"));
        assert_eq!(model.elapsed().map(|time| time.as_ref()), Some("0:30"));
    }

    #[test]
    fn the_session_footer_counts_damage_and_targets() {
        let model = model_on(open_world(7, false));

        assert_eq!(
            model.footer(),
            Some(Footer::Session { damage: "5000".into(), targets: 3 })
        );
    }

    #[test]
    fn the_target_footer_shows_pace_against_a_personal_best_else_party_damage() {
        let mut model = model_on(open_world(8, true));
        assert_eq!(model.footer(), Some(Footer::TargetPartyDamage("3000".into())));

        model.best = Some(BestEntry { character: "You".to_string(), mob_code: 9, dps: Some(40.0) });
        model.rebuild_live();
        let Some(Footer::TargetPace(pace)) = model.footer() else {
            panic!("expected a pace line, got {:?}", model.footer());
        };
        assert_eq!(pace.figure.as_ref(), "+25.0%");
        assert_eq!(pace.versus_best.as_ref(), fl!("meter-pace-versus-best", pace = "+25.0%"));

        let mut early = open_world(8, true);
        early.last_target_info.as_mut().unwrap().target_last_time = HashMap::from([(1, 105.0)]);
        show(&mut model, indexed(early));
        assert_eq!(model.footer(), Some(Footer::TargetPartyDamage("3000".into())));
    }

    #[test]
    fn clicking_the_other_side_pins_it_and_the_active_side_unpins() {
        let mut model = model_on(open_world(7, false));

        model.select_board(Board::Target);
        assert_eq!(model.active_board(), Board::Target);
        assert!(model.is_board_pinned());
        assert!(model.live().is_some(), "a pinned Target board follows the current target");

        model.select_board(Board::Target);
        assert_eq!(model.active_board(), Board::Session);
        assert!(!model.is_board_pinned());
    }

    #[test]
    fn a_pinned_target_board_follows_the_target_and_the_next_boss_ends_the_pin() {
        let mut model = model_on(open_world(7, false));
        model.select_board(Board::Target);

        show(&mut model, open_world(9, false));
        assert!(model.is_board_pinned());
        assert_eq!(model.active_board(), Board::Target);

        show(&mut model, open_world(10, true));
        assert!(!model.is_board_pinned());
        assert_eq!(model.active_board(), Board::Target);
    }

    #[test]
    fn a_reset_and_an_emptied_meter_both_clear_the_pin() {
        let mut model = model_on(open_world(7, false));
        model.select_board(Board::Target);
        model.reset();
        assert!(!model.is_board_pinned());

        model.select_board(Board::Target);
        let mut emptied = open_world(7, false);
        emptied.total_damage = 0;
        emptied.session = SessionSummary::default();
        show(&mut model, emptied);
        assert!(!model.is_board_pinned());
        assert_eq!(model.active_board(), Board::Session);
    }

    #[test]
    fn you_are_identified_once_the_game_names_your_character() {
        let mut model = MeterModel::new(OverlaySettings::default(), None);
        assert!(!model.identified());

        let mut unnamed = open_world(7, false);
        unnamed.combat_infos.main_actor_id = None;
        unnamed.combat_infos.main_actor_name = None;
        show(&mut model, unnamed);
        assert!(!model.identified());
        assert!(
            matches!(model.session().unwrap().lead, Lead::Party { .. }),
            "the top row is not presented as you"
        );

        model.apply(
            MeterEvent::MainActorDetected(MainActorDetectedPayload {
                actor_id: 2,
                actor_name: "You".to_string(),
                sid: None,
            }),
            Instant::now(),
        );
        assert!(model.identified());
        assert!(model.session().unwrap().rows.iter().any(|row| row.is_you));
        assert!(matches!(model.session().unwrap().lead, Lead::You { .. }));
    }

    #[test]
    fn your_row_reads_you_while_the_game_has_not_named_you() {
        let mut model = MeterModel::new(OverlaySettings::default(), None);
        let mut unnamed = open_world(7, false);
        unnamed.combat_infos.main_actor_id = Some(2);
        unnamed.combat_infos.main_actor_name = None;
        for player in &mut unnamed.session.players {
            if player.actor_id == 2 {
                player.actor_name.clear();
            }
        }
        show(&mut model, unnamed);

        assert!(model.identified());
        let session = model.session().unwrap();
        let you = session.rows.iter().find(|row| row.is_you).expect("your row is marked");
        assert_eq!(you.name.as_deref(), Some("You"));
        assert!(matches!(session.lead, Lead::You { .. }));
    }

    // ---- Boss focus, summary subject and your row ----

    const BOSS_ID: u32 = 100;
    const OTHER_BOSS_ID: u32 = 101;
    const TRASH_ID: u32 = 200;

    struct Mob {
        id: u32,
        boss: bool,
        hp: u32,
        secs: f64,
        players: Vec<PlayerOverviewStat>,
    }

    fn mob(id: u32, boss: bool, hp: u32, secs: f64, players: Vec<PlayerOverviewStat>) -> Mob {
        Mob { id, boss, hp, secs, players }
    }

    /// Several targets at once, `current` being the one hit last. Boss and
    /// trash rows carry different names so a board shows which it is.
    fn world(current: u32, mobs: &[Mob], you: Option<(u32, &str)>) -> CombatSnapshot {
        let mut snapshot = snapshot(500, Some(9), 30.0);
        snapshot.combat_infos.target_infos.clear();
        snapshot.by_target_player_stats.clear();
        snapshot.combat_infos.main_actor_id = you.map(|(id, _)| id);
        snapshot.combat_infos.main_actor_name = you.map(|(_, name)| name.to_string());
        snapshot.combat_infos.last_target = Some(current);
        for mob in mobs {
            let info = TargetInfo {
                id: mob.id,
                target_mob_code: Some(9),
                target_name: Some(if mob.boss { "Boss" } else { "Trash" }.to_string()),
                is_boss: mob.boss,
                current_hp: Some(mob.hp),
                max_hp: Some(1_000),
                target_start_time: HashMap::from([(1, 100.0)]),
                target_last_time: HashMap::from([(1, 100.0 + mob.secs)]),
            };
            snapshot.by_target_player_stats.insert(
                mob.id,
                mob.players.iter().map(|player| (player.actor_id, player.clone())).collect(),
            );
            if mob.id == current {
                snapshot.last_target_info = Some(info.clone());
                snapshot.last_target_all_players_overview_stats = mob.players.clone();
            }
            snapshot.combat_infos.target_infos.insert(mob.id, info);
        }
        snapshot
    }

    fn boss_party() -> Vec<PlayerOverviewStat> {
        vec![player(2, "You", 4_000, 80.0), player(3, "Healer", 1_000, 20.0)]
    }

    fn trash_party() -> Vec<PlayerOverviewStat> {
        vec![player(4, "Puller", 900, 30.0)]
    }

    fn at(start: Instant, secs: u64) -> Instant {
        start + std::time::Duration::from_secs(secs)
    }

    fn boss_fight_with_adds(start: Instant) -> MeterModel {
        let mut model = MeterModel::new(OverlaySettings::default(), None);
        let mobs = [
            mob(BOSS_ID, true, 800, 40.0, boss_party()),
            mob(TRASH_ID, false, 100, 5.0, trash_party()),
        ];
        model.apply(MeterEvent::DpsSnapshot(world(BOSS_ID, &mobs, Some((2, "You")))), start);
        model
    }

    #[test]
    fn naming_your_actor_in_place_keeps_the_boss_and_the_kill_summary_is_about_you() {
        let start = Instant::now();
        let detected = |name: &str| {
            MeterEvent::MainActorDetected(MainActorDetectedPayload {
                actor_id: 2,
                actor_name: name.to_string(),
                sid: None,
            })
        };
        let mut model = MeterModel::new(OverlaySettings::default(), None);
        model.apply(detected(""), start);
        let unnamed_party = vec![player(2, "", 4_000, 80.0), player(3, "Healer", 1_000, 20.0)];
        let alive = [mob(BOSS_ID, true, 800, 40.0, unnamed_party)];
        model.apply(MeterEvent::DpsSnapshot(world(BOSS_ID, &alive, Some((2, "")))), at(start, 1));
        assert_eq!(model.focused_boss(), Some(BOSS_ID));

        model.apply(detected("Mirelle"), at(start, 2));

        assert_eq!(model.focused_boss(), Some(BOSS_ID), "a name arriving for your own actor is not a zone change");
        let named_party = vec![player(2, "Mirelle", 4_000, 80.0), player(3, "Healer", 1_000, 20.0)];
        let dead = [mob(BOSS_ID, true, 0, 45.0, named_party)];
        model.apply(MeterEvent::DpsSnapshot(world(BOSS_ID, &dead, Some((2, "Mirelle")))), at(start, 45));
        let summary = model.summary().expect("final summary");
        assert_eq!(summary.kind, SummaryKind::Final);
        assert!(summary.personal);
        assert_eq!(
            summary.subject,
            Subject::Player { name: "Mirelle".to_string(), by: crate::meter::live::YouMatch::Name }
        );
    }

    #[test]
    fn hitting_adds_does_not_move_the_board_off_the_boss() {
        let start = Instant::now();
        let mut model = boss_fight_with_adds(start);
        assert_eq!(model.active_board(), Board::Target);

        for second in 1..=30u64 {
            let mobs = [
                mob(BOSS_ID, true, 800, 40.0, boss_party()),
                mob(TRASH_ID, false, 100, 5.0 + second as f64, trash_party()),
            ];
            let current = if second % 2 == 0 { BOSS_ID } else { TRASH_ID };
            model.apply(MeterEvent::DpsSnapshot(world(current, &mobs, Some((2, "You")))), at(start, second));

            assert_eq!(model.active_board(), Board::Target, "flapped at {second}s");
            assert_eq!(model.header_name().map(|name| name.as_ref()), Some("Boss"));
            let rows = &model.live().unwrap().rows;
            assert_eq!(rows[0].name.as_deref(), Some("You"), "the board left the boss at {second}s");
            assert_eq!(rows.len(), 2);
        }
    }

    #[test]
    fn sixty_seconds_of_adds_keep_the_boss_and_two_quiet_minutes_let_go() {
        let start = Instant::now();
        let mut model = boss_fight_with_adds(start);
        let mobs = [
            mob(BOSS_ID, true, 800, 40.0, boss_party()),
            mob(TRASH_ID, false, 100, 65.0, trash_party()),
        ];
        model.apply(MeterEvent::DpsSnapshot(world(TRASH_ID, &mobs, Some((2, "You")))), at(start, 60));

        assert!(!model.advance(at(start, 60)));
        assert_eq!(model.active_board(), Board::Target);
        assert!(!model.advance(at(start, 119)));

        assert!(model.advance(at(start, 120)));
        assert_eq!(model.focused_boss(), None);
        assert_eq!(model.active_board(), Board::Session);
        assert_eq!(model.summary().map(|summary| summary.kind), Some(SummaryKind::Provisional));
    }

    #[test]
    fn a_dead_boss_gives_a_final_summary_about_the_boss_and_the_board_goes_automatic() {
        let start = Instant::now();
        let mut model = boss_fight_with_adds(start);
        let mobs = [
            mob(BOSS_ID, true, 0, 45.0, boss_party()),
            mob(TRASH_ID, false, 100, 9.0, trash_party()),
        ];
        // The last thing hit was an add, not the boss that just died.
        model.apply(MeterEvent::DpsSnapshot(world(TRASH_ID, &mobs, Some((2, "You")))), at(start, 45));

        let summary = model.summary().expect("final summary");
        assert_eq!(summary.kind, SummaryKind::Final);
        assert_eq!(summary.boss.as_ref(), "Boss");
        assert_eq!(summary.rank.as_deref(), Some("1/2"));
        assert_eq!(model.focused_boss(), None);
        assert_eq!(model.active_board(), Board::Session);

        let mobs = [
            mob(BOSS_ID, true, 0, 45.0, boss_party()),
            mob(TRASH_ID, false, 100, 12.0, trash_party()),
        ];
        model.apply(MeterEvent::DpsSnapshot(world(BOSS_ID, &mobs, Some((2, "You")))), at(start, 50));
        assert_eq!(model.active_board(), Board::Session, "a corpse is not engaged again");
        assert_eq!(model.summary().map(|summary| summary.boss.as_ref()), Some("Boss"));
    }

    /// Sets "now" and how long ago you last hit each given target.
    fn with_hit_ages(mut snapshot: CombatSnapshot, ages: &[(u32, f64)]) -> CombatSnapshot {
        snapshot.combat_infos.time_now = 1_000.0;
        for &(id, age) in ages {
            if let Some(info) = snapshot.combat_infos.target_infos.get_mut(&id) {
                info.target_last_time.insert(2, 1_000.0 - age);
            }
        }
        snapshot
    }

    fn two_bosses() -> [Mob; 2] {
        [
            mob(BOSS_ID, true, 800, 40.0, boss_party()),
            mob(OTHER_BOSS_ID, true, 900, 30.0, vec![player(4, "Puller", 9_000, 90.0), player(2, "You", 500, 5.0)]),
        ]
    }

    #[test]
    fn alternating_between_two_bosses_keeps_the_first_on_the_board() {
        let start = Instant::now();
        let mut model = boss_fight_with_adds(start);

        for second in 1..=60u64 {
            let on_first = second % 2 == 0;
            let ages = [(BOSS_ID, if on_first { 0.0 } else { 3.0 }), (OTHER_BOSS_ID, if on_first { 3.0 } else { 0.0 })];
            let current = if on_first { BOSS_ID } else { OTHER_BOSS_ID };
            let snapshot = with_hit_ages(world(current, &two_bosses(), Some((2, "You"))), &ages);
            model.apply(MeterEvent::DpsSnapshot(snapshot), at(start, second));

            assert_eq!(model.focused_boss(), Some(BOSS_ID), "flipped at {second}s");
            assert_eq!(model.header_name().map(|name| name.as_ref()), Some("Boss"));
        }
    }

    #[test]
    fn twenty_seconds_on_only_the_other_boss_moves_the_board_without_a_summary() {
        let start = Instant::now();
        let mut model = boss_fight_with_adds(start);
        let ages = [(BOSS_ID, 25.0), (OTHER_BOSS_ID, 1.0)];
        let snapshot = with_hit_ages(world(OTHER_BOSS_ID, &two_bosses(), Some((2, "You"))), &ages);

        model.apply(MeterEvent::DpsSnapshot(snapshot), at(start, 25));

        assert_eq!(model.focused_boss(), Some(OTHER_BOSS_ID));
        assert_eq!(model.live().unwrap().rows[0].name.as_deref(), Some("Puller"));
        assert!(model.summary().is_none(), "walking away is not a fight that ended");
        assert!(!model.advance(at(start, 26)));
        assert!(model.summary().is_none());
    }

    #[test]
    fn the_first_boss_dying_gives_its_summary_and_the_other_takes_over_at_once() {
        let start = Instant::now();
        let mut model = boss_fight_with_adds(start);
        let mobs = [
            mob(BOSS_ID, true, 0, 45.0, boss_party()),
            mob(OTHER_BOSS_ID, true, 900, 30.0, vec![player(4, "Puller", 9_000, 90.0), player(2, "You", 500, 5.0)]),
        ];
        let snapshot = with_hit_ages(world(OTHER_BOSS_ID, &mobs, Some((2, "You"))), &[(OTHER_BOSS_ID, 2.0)]);

        model.apply(MeterEvent::DpsSnapshot(snapshot.clone()), at(start, 45));

        assert_eq!(model.focused_boss(), Some(OTHER_BOSS_ID));
        let summary = model.summary().expect("the first boss's final summary");
        assert_eq!(summary.kind, SummaryKind::Final);
        assert_eq!(summary.rank.as_deref(), Some("1/2"));

        model.apply(MeterEvent::DpsSnapshot(snapshot), at(start, 46));
        assert!(model.summary().is_some(), "the boss already being fought does not take it down");
    }

    #[test]
    fn the_second_boss_dying_while_the_first_summary_is_up_shows_its_own() {
        let start = Instant::now();
        let mut model = boss_fight_with_adds(start);
        let first_dead = [
            mob(BOSS_ID, true, 0, 45.0, boss_party()),
            mob(OTHER_BOSS_ID, true, 900, 30.0, vec![player(4, "Puller", 9_000, 90.0), player(2, "You", 500, 5.0)]),
        ];
        let snapshot = with_hit_ages(world(OTHER_BOSS_ID, &first_dead, Some((2, "You"))), &[(OTHER_BOSS_ID, 2.0)]);
        model.apply(MeterEvent::DpsSnapshot(snapshot), at(start, 45));
        assert_eq!(model.summary().map(|summary| summary.rank.clone()), Some(Some("1/2".into())));

        let both_dead = [
            mob(BOSS_ID, true, 0, 45.0, boss_party()),
            mob(OTHER_BOSS_ID, true, 0, 50.0, vec![player(4, "Puller", 9_000, 90.0), player(2, "You", 500, 5.0)]),
        ];
        let snapshot = with_hit_ages(world(OTHER_BOSS_ID, &both_dead, Some((2, "You"))), &[(OTHER_BOSS_ID, 1.0)]);
        model.apply(MeterEvent::DpsSnapshot(snapshot), at(start, 50));

        let summary = model.summary().expect("the second kill's summary");
        assert_eq!(summary.kind, SummaryKind::Final);
        assert_eq!(summary.rank.as_deref(), Some("2/2"), "the second boss's board, where you rank second");
    }

    #[test]
    fn a_second_boss_you_did_not_touch_does_not_take_the_board_over() {
        let start = Instant::now();
        let mut model = boss_fight_with_adds(start);
        let mobs = [
            mob(BOSS_ID, true, 800, 40.0, boss_party()),
            mob(OTHER_BOSS_ID, true, 1_000, 2.0, trash_party()),
        ];

        model.apply(MeterEvent::DpsSnapshot(world(OTHER_BOSS_ID, &mobs, Some((2, "You")))), at(start, 30));

        assert_eq!(model.focused_boss(), Some(BOSS_ID));
    }

    #[test]
    fn a_strangers_boss_hit_only_by_others_is_not_engaged_once_you_are_identified() {
        let mut model = MeterModel::new(OverlaySettings::default(), None);
        let mobs = [mob(BOSS_ID, true, 800, 40.0, trash_party())];

        model.apply(MeterEvent::DpsSnapshot(world(BOSS_ID, &mobs, Some((2, "You")))), Instant::now());

        assert_eq!(model.focused_boss(), None);
        assert_eq!(model.active_board(), Board::Session);
    }

    #[test]
    fn a_boss_that_hit_you_is_engaged() {
        let mut model = MeterModel::new(OverlaySettings::default(), None);
        let mobs = [mob(BOSS_ID, true, 800, 40.0, trash_party())];
        let mut snapshot = world(BOSS_ID, &mobs, Some((2, "You")));
        let boss_row = player(BOSS_ID, "", 300, 5.0);
        snapshot.by_target_player_stats.insert(2, HashMap::from([(BOSS_ID, boss_row)]));

        model.apply(MeterEvent::DpsSnapshot(snapshot), Instant::now());

        assert_eq!(model.focused_boss(), Some(BOSS_ID));
    }

    #[test]
    fn a_boss_you_hit_is_engaged() {
        let start = Instant::now();
        let model = boss_fight_with_adds(start);

        assert_eq!(model.focused_boss(), Some(BOSS_ID));
    }

    #[test]
    fn a_boss_engaged_before_identification_is_released_when_it_is_not_your_fight() {
        let start = Instant::now();
        let mut model = MeterModel::new(OverlaySettings::default(), None);
        let mobs = [mob(BOSS_ID, true, 800, 40.0, trash_party())];
        model.apply(MeterEvent::DpsSnapshot(world(BOSS_ID, &mobs, None)), start);
        assert_eq!(model.focused_boss(), Some(BOSS_ID), "unidentified: anyone's target counts");

        model.apply(MeterEvent::DpsSnapshot(world(BOSS_ID, &mobs, Some((2, "You")))), at(start, 1));

        assert_eq!(model.focused_boss(), None);
        assert_eq!(model.active_board(), Board::Session);
    }

    #[test]
    fn a_strangers_boss_dying_gives_no_summary() {
        let start = Instant::now();
        let mut model = MeterModel::new(OverlaySettings::default(), None);
        let alive = [mob(BOSS_ID, true, 800, 40.0, trash_party())];
        let dead = [mob(BOSS_ID, true, 0, 45.0, trash_party())];
        model.apply(MeterEvent::DpsSnapshot(world(BOSS_ID, &alive, Some((2, "You")))), start);

        model.apply(MeterEvent::DpsSnapshot(world(BOSS_ID, &dead, Some((2, "You")))), at(start, 45));

        assert!(model.summary().is_none());
    }

    #[test]
    fn a_reset_lets_go_of_the_boss() {
        let start = Instant::now();
        let mut model = boss_fight_with_adds(start);

        model.reset();

        assert_eq!(model.focused_boss(), None);
        assert_eq!(model.active_board(), Board::Session);
    }

    #[test]
    fn a_zone_change_lets_go_of_the_boss() {
        let start = Instant::now();
        let mut model = boss_fight_with_adds(start);
        let detected = |id: u32| {
            MeterEvent::MainActorDetected(MainActorDetectedPayload {
                actor_id: id,
                actor_name: "You".to_string(),
                sid: None,
            })
        };
        model.apply(detected(2), at(start, 1));
        assert_eq!(model.focused_boss(), Some(BOSS_ID), "the first identification is not a zone change");
        model.apply(detected(2), at(start, 2));
        assert_eq!(model.focused_boss(), Some(BOSS_ID), "a re-sent identification is not one either");

        model.apply(detected(7), at(start, 3));

        assert_eq!(model.focused_boss(), None);
    }

    #[test]
    fn the_emptied_snapshot_after_a_reset_does_not_forget_who_you_are() {
        let start = Instant::now();
        let mut model = MeterModel::new(OverlaySettings::default(), None);
        model.apply(
            MeterEvent::MainActorDetected(MainActorDetectedPayload {
                actor_id: 2,
                actor_name: "You".to_string(),
                sid: None,
            }),
            start,
        );
        let mut emptied = world(BOSS_ID, &[], None);
        emptied.total_damage = 0;
        model.apply(MeterEvent::DpsSnapshot(emptied), at(start, 1));
        let infos = &model.snapshot().unwrap().combat_infos;
        assert_eq!((infos.main_actor_id, infos.main_actor_name.as_deref()), (Some(2), Some("You")));

        let strangers_boss = [mob(BOSS_ID, true, 800, 40.0, vec![player(3, "Healer", 1_000, 20.0)])];
        model.apply(MeterEvent::DpsSnapshot(world(BOSS_ID, &strangers_boss, None)), at(start, 2));

        assert_eq!(model.focused_boss(), None, "a boss you have not fought is not engaged as if you were unknown");
    }

    #[test]
    fn a_pin_overrides_the_boss_and_the_boss_stays_engaged_behind_it() {
        let start = Instant::now();
        let mut model = boss_fight_with_adds(start);

        model.select_board(Board::Session);
        assert_eq!(model.active_board(), Board::Session);
        assert_eq!(model.focused_boss(), Some(BOSS_ID));

        model.select_board(Board::Session);
        assert_eq!(model.active_board(), Board::Target);
    }

    #[test]
    fn the_summary_is_about_you_not_the_top_player_when_you_are_not_named() {
        let start = Instant::now();
        let mut model = MeterModel::new(OverlaySettings::default(), None);
        let alive = [mob(BOSS_ID, true, 800, 40.0, boss_party())];
        let dead = [mob(BOSS_ID, true, 0, 45.0, boss_party())];
        model.apply(MeterEvent::DpsSnapshot(world(BOSS_ID, &alive, None)), start);

        model.apply(MeterEvent::DpsSnapshot(world(BOSS_ID, &dead, None)), at(start, 45));

        let summary = model.summary().expect("final summary");
        assert!(!summary.personal);
        assert_eq!(summary.rank, None);
        assert_eq!(summary.share, None);
        assert!(summary.skills.is_empty() && summary.best.is_none());
        assert_eq!(summary.damage.as_ref(), "5000", "the party's damage, not the top player's");
        assert!(!summary.report.contains("#1 of"), "{}", summary.report);
    }

    #[test]
    fn the_summary_is_about_the_party_when_you_did_no_damage_to_the_boss() {
        let start = Instant::now();
        let mut model = MeterModel::new(OverlaySettings::default(), None);
        let alive = [mob(BOSS_ID, true, 800, 40.0, boss_party())];
        let dead = [mob(BOSS_ID, true, 0, 45.0, boss_party())];
        let me = Some((9, "Absent"));
        // The boss hit you, so it is your fight, but you dealt it no damage.
        let hit_by_boss = |mut snapshot: CombatSnapshot| {
            snapshot.by_target_player_stats.insert(9, HashMap::from([(BOSS_ID, player(BOSS_ID, "", 100, 1.0))]));
            snapshot
        };
        model.apply(MeterEvent::DpsSnapshot(hit_by_boss(world(BOSS_ID, &alive, me))), start);

        model.apply(MeterEvent::DpsSnapshot(hit_by_boss(world(BOSS_ID, &dead, me))), at(start, 45));

        let summary = model.summary().expect("final summary");
        assert!(!summary.personal);
        assert_eq!(summary.subject, Subject::Party { identified: true });
    }

    #[test]
    fn a_stale_actor_id_that_now_belongs_to_someone_named_is_not_you() {
        let mut model = MeterModel::new(OverlaySettings::default(), None);
        let mobs = [mob(BOSS_ID, true, 800, 40.0, boss_party())];
        // Your id from before the zone change is the healer's id now.
        model.apply(MeterEvent::DpsSnapshot(world(BOSS_ID, &mobs, Some((3, "Absent")))), Instant::now());

        assert!(model.live().unwrap().rows.iter().all(|row| !row.is_you));
    }

    #[test]
    fn your_row_is_on_the_boss_board_even_when_an_add_was_hit_last() {
        let start = Instant::now();
        let mut model = boss_fight_with_adds(start);
        let mobs = [
            mob(BOSS_ID, true, 800, 40.0, boss_party()),
            mob(TRASH_ID, false, 100, 8.0, trash_party()),
        ];

        model.apply(MeterEvent::DpsSnapshot(world(TRASH_ID, &mobs, Some((2, "You")))), at(start, 8));

        assert!(model.live().unwrap().rows.iter().any(|row| row.is_you));
    }

    #[test]
    fn your_row_survives_the_core_cutting_the_current_targets_list() {
        // The core keeps the top ten of the current target for its own list;
        // the full overview still has you.
        let mut many: Vec<PlayerOverviewStat> =
            (10..22).map(|id| player(id, &format!("P{id}"), 10_000 + u64::from(id), 100.0)).collect();
        many.push(player(2, "You", 10, 1.0));
        let mobs = [mob(BOSS_ID, true, 800, 40.0, many.clone())];
        let mut snapshot = world(BOSS_ID, &mobs, Some((2, "You")));
        snapshot.last_target_all_players_overview_stats = many[..10].to_vec();

        let mut model = MeterModel::new(OverlaySettings::default(), None);
        model.apply(MeterEvent::DpsSnapshot(snapshot), Instant::now());

        let rows = &model.live().unwrap().rows;
        assert_eq!(rows.len(), 10);
        assert!(rows.last().unwrap().is_you);
    }
}
