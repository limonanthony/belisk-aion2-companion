//! What the live meter shows, worked out once per snapshot so drawing only
//! reads prepared strings.

use meter_core::models::combat::{CombatSnapshot, PlayerOverviewStat, TargetInfo};
use gpui::SharedString;

use crate::format;
use crate::game_data::{self, PlayerClass};
use crate::i18n::fl;
use super::settings::{OverlaySettings, PctMode};
use super::shades::ShadeBook;

/// Someone contributing 1% still gets a readable sliver of their colour.
const BAR_FLOOR: f32 = 0.06;
const PACE_MIN_FIGHT_SECS: f64 = 10.0;

pub struct LiveBoard {
    pub elapsed: SharedString,
    /// Boss health, when it is known and shown.
    pub boss_hp: Option<Gauge>,
    pub rows: Vec<PlayerRow>,
    /// The capsule's line: yours, or the party's while the meter does not
    /// know which row is you.
    pub lead: Lead,
    /// Everyone's damage on this target, for the footer when no personal best
    /// is shown.
    pub party_damage: SharedString,
    /// Against your personal best on this boss, once the fight is 10 s old.
    pub pace: Option<PaceLine>,
}

pub struct Gauge {
    /// 0–1.
    pub fraction: f32,
    pub label: SharedString,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PaceLine {
    pub ahead: bool,
    /// "+4.2%".
    pub figure: SharedString,
    /// "+4.2% vs PB".
    pub versus_best: SharedString,
}

impl PaceLine {
    fn new(pace: f64) -> Self {
        let figure = format::pace(pace);
        Self {
            ahead: pace >= 0.0,
            versus_best: fl!("meter-pace-versus-best", pace = figure.clone()).into(),
            figure: figure.into(),
        }
    }
}

pub struct PlayerRow {
    pub actor_id: u32,
    pub rank: SharedString,
    pub name: Option<SharedString>,
    /// Server and combat power, beside the name.
    pub detail: Option<SharedString>,
    pub class: Option<PlayerClass>,
    /// The player's shade of their class colour, `0xRRGGBB`.
    pub colour: Option<u32>,
    pub damage: SharedString,
    pub dps: SharedString,
    pub pct: SharedString,
    /// Bar length relative to the top damage, 0–1.
    pub bar: f32,
    pub is_you: bool,
}

/// What the capsule leads with. Before the game has named you (or when you
/// are not on the board) nobody else is presented as you.
pub enum Lead {
    You {
        rank: SharedString,
        /// Your shade of your class colour, `0xRRGGBB`.
        colour: Option<u32>,
        dps: SharedString,
        /// Total damage, for the Session capsule.
        damage: SharedString,
    },
    /// Everyone's DPS together.
    Party { dps: SharedString },
}

impl Lead {
    pub fn you(index: usize, colour: Option<u32>, dps: f64, damage: u64) -> Self {
        Self::You {
            rank: (index + 1).to_string().into(),
            colour,
            dps: format::compact(dps).into(),
            damage: format::damage(damage).into(),
        }
    }

    pub fn party(dps: impl Iterator<Item = f64>) -> Self {
        Self::Party { dps: format::compact(dps.map(|dps| dps.max(0.0)).sum()).into() }
    }
}

/// The target the snapshot is about.
pub fn target_of(snapshot: &CombatSnapshot) -> Option<&TargetInfo> {
    snapshot.last_target_info.as_ref().or_else(|| {
        let id = snapshot.combat_infos.last_target?;
        snapshot.combat_infos.target_infos.get(&id)
    })
}

/// From the first hit anyone landed on the target to the last.
pub fn fight_secs(target: &TargetInfo) -> f64 {
    let start = target.target_start_time.values().copied().fold(f64::INFINITY, f64::min);
    let end = target.target_last_time.values().copied().fold(f64::NEG_INFINITY, f64::max);
    if start.is_finite() && end.is_finite() {
        (end - start).max(0.0)
    } else {
        0.0
    }
}

pub fn boss_hp(target: &TargetInfo) -> Option<Gauge> {
    let max = target.max_hp.filter(|&max| max > 0)?;
    let fraction = (target.current_hp.unwrap_or(0) as f32 / max as f32).clamp(0.0, 1.0);
    Some(Gauge { fraction, label: format::whole_percent(f64::from(fraction)).into() })
}

pub fn target_name(target: &TargetInfo) -> String {
    let english = target.target_name.as_deref().filter(|name| !name.is_empty());
    game_data::mob_name(target.target_mob_code, english)
        .unwrap_or_else(|| fl!("common-target-numbered", id = target.id))
}

/// A target and everyone who hit it, most damage first. Built from the
/// per-target overview, which holds every player: the snapshot's own
/// `last_target_all_players_overview_stats` is cut to the core's player limit
/// before this app can keep your row in it. `players` leaves out the unnamed
/// ones when asked to, except you; the totals still count everyone, as the
/// core's session board does.
pub struct TargetView<'a> {
    pub info: &'a TargetInfo,
    pub players: Vec<&'a PlayerOverviewStat>,
    pub total_damage: u64,
    pub total_dps: f64,
}

pub fn target_info(snapshot: &CombatSnapshot, id: u32) -> Option<&TargetInfo> {
    snapshot
        .combat_infos
        .target_infos
        .get(&id)
        .or_else(|| snapshot.last_target_info.as_ref().filter(|target| target.id == id))
}

pub fn target_view(snapshot: &CombatSnapshot, id: u32, hide_unknown_players: bool) -> Option<TargetView<'_>> {
    let info = target_info(snapshot, id)?;
    let mut players: Vec<&PlayerOverviewStat> =
        snapshot.by_target_player_stats.get(&id)?.values().collect();
    players.sort_by(|a, b| b.total_damage.cmp(&a.total_damage).then(a.actor_id.cmp(&b.actor_id)));
    let total_damage = players.iter().map(|player| player.total_damage).sum();
    let total_dps = players.iter().map(|player| player.dps.max(0.0)).sum();
    if hide_unknown_players {
        let you = find_you(snapshot, players.iter().map(|player| (player.actor_id, player.actor_name.as_str())))
            .map(|found| found.index);
        players = players
            .into_iter()
            .enumerate()
            .filter(|&(index, player)| !player.actor_name.is_empty() || Some(index) == you)
            .map(|(_, player)| player)
            .collect();
    }
    Some(TargetView { info, players, total_damage, total_dps })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum YouMatch {
    /// The row carries your character's name.
    Name,
    /// The row has no name yet and your actor id.
    UnnamedId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FoundYou {
    pub index: usize,
    pub by: YouMatch,
}

/// Which of `players` (actor id, name) is you. The name wins: ids are entity
/// handles that change between zones, so a stale id can belong to someone else
/// by now, and an id is trusted only on a row nobody has named. `None` while
/// the game has not told the meter who you are, or when you are not among them.
pub fn find_you<'a>(
    snapshot: &CombatSnapshot,
    mut players: impl Iterator<Item = (u32, &'a str)> + Clone,
) -> Option<FoundYou> {
    let infos = &snapshot.combat_infos;
    if let Some(name) = infos.main_actor_name.as_deref().filter(|name| !name.is_empty()) {
        if let Some(index) = players.clone().position(|(_, row_name)| row_name == name) {
            return Some(FoundYou { index, by: YouMatch::Name });
        }
    }
    let id = infos.main_actor_id?;
    players
        .position(|(row_id, row_name)| row_id == id && row_name.is_empty())
        .map(|index| FoundYou { index, by: YouMatch::UnnamedId })
}

pub fn is_identified(snapshot: &CombatSnapshot) -> bool {
    let infos = &snapshot.combat_infos;
    infos.main_actor_id.is_some() || infos.main_actor_name.as_deref().is_some_and(|name| !name.is_empty())
}

/// Whether your row is on a board and how it was matched, for the diagnostic
/// log. Row numbers and counts are left out so the text changes only when the
/// answer does.
pub fn you_status(snapshot: &CombatSnapshot, found: Option<FoundYou>, count: usize, cap: usize) -> String {
    let infos = &snapshot.combat_infos;
    match found {
        Some(found) => format!(
            "your row is shown, matched by {}; {}",
            match found.by {
                YouMatch::Name => "name",
                YouMatch::UnnamedId => "actor id (the row has no name)",
            },
            if found.index >= cap { "ranked below the row cap, so it takes the last row" } else { "within the row cap" }
        ),
        None if !is_identified(snapshot) => {
            "not identified: the game has not named the local player, so no row is marked".to_string()
        }
        None if count == 0 => "no players on this board yet".to_string(),
        None => format!(
            "identified as {:?} (actor {:?}) but not among the players: no damage dealt here, or the actor id changed with a zone and the row carries another name",
            infos.main_actor_name.as_deref().unwrap_or(""),
            infos.main_actor_id
        ),
    }
}

pub fn party_dps(view: &TargetView) -> Option<SharedString> {
    (view.players.len() > 1 && view.total_dps > 0.0).then(|| format::compact(view.total_dps).into())
}

pub fn pace_against(best_dps: f64, dps: f64) -> Option<f64> {
    (best_dps > 0.0 && dps > 0.0).then(|| (dps - best_dps) / best_dps)
}

impl LiveBoard {
    /// `None` when nobody has hit anything yet. `personal_best` answers the
    /// best DPS for a character and boss.
    pub fn build(
        snapshot: &CombatSnapshot,
        target_id: u32,
        settings: &OverlaySettings,
        hide_unknown_players: bool,
        shades: &mut ShadeBook,
        personal_best: impl FnOnce(&str, u32) -> Option<f64>,
    ) -> Option<Self> {
        let view = target_view(snapshot, target_id, hide_unknown_players)?;
        let TargetView { info: target, players: ranked, .. } = view;
        if ranked.is_empty() {
            return None;
        }
        let you = find_you(snapshot, ranked.iter().map(|player| (player.actor_id, player.actor_name.as_str())))
            .map(|found| found.index);
        let top_damage = ranked[0].total_damage.max(1) as f32;

        let rows: Vec<PlayerRow> = visible_indexes(ranked.len(), you, settings.players_shown)
            .into_iter()
            .map(|index| {
                let facts = RowFacts::of_target_player(ranked[index], settings.pct_mode);
                player_row(&facts, index + 1, Some(index) == you, top_damage, settings, shades)
            })
            .collect();

        let lead = match you {
            Some(index) => {
                let player = ranked[index];
                let colour = rows.iter().find(|row| row.is_you).and_then(|row| row.colour);
                Lead::you(index, colour, player.dps, player.total_damage)
            }
            None => Lead::party(std::iter::once(view.total_dps)),
        };

        let pace = match you {
            Some(you)
                if settings.show_personal_best
                    && target.is_boss
                    && fight_secs(target) >= PACE_MIN_FIGHT_SECS =>
            {
                let character = snapshot.combat_infos.main_actor_name.as_deref().unwrap_or_default();
                target
                    .target_mob_code
                    .and_then(|mob| personal_best(character, mob))
                    .and_then(|best| pace_against(best, ranked[you].dps))
                    .map(PaceLine::new)
            }
            _ => None,
        };

        Some(Self {
            elapsed: format::duration(fight_secs(target)).into(),
            boss_hp: Some(target).filter(|_| settings.show_boss_hp).and_then(boss_hp),
            rows,
            lead,
            party_damage: format::damage(view.total_damage).into(),
            pace,
        })
    }
}

/// What a row draws, whichever board it is on.
pub struct RowFacts<'a> {
    pub actor_id: u32,
    pub actor_name: &'a str,
    pub actor_server_id: &'a str,
    pub actor_class: &'a str,
    pub combat_power: Option<u64>,
    pub total_damage: u64,
    pub dps: f64,
    /// 0–1, already chosen for the board.
    pub pct: f64,
}

impl<'a> RowFacts<'a> {
    fn of_target_player(player: &'a PlayerOverviewStat, pct_mode: PctMode) -> Self {
        Self {
            actor_id: player.actor_id,
            actor_name: &player.actor_name,
            actor_server_id: &player.actor_server_id,
            actor_class: &player.actor_class,
            combat_power: player.combat_power,
            total_damage: player.total_damage,
            dps: player.dps,
            pct: match pct_mode {
                PctMode::Contribution => player.damage_contribution,
                PctMode::Share => player.damage_share,
            },
        }
    }
}

/// Which of `count` ranked players get a row: the first `cap`, with your row
/// taking the last place when you rank below them.
pub fn visible_indexes(count: usize, you: Option<usize>, cap: usize) -> Vec<usize> {
    let mut shown: Vec<usize> = (0..count.min(cap)).collect();
    if let Some(you) = you.filter(|&you| you >= cap) {
        shown[cap - 1] = you;
    }
    shown
}

pub fn player_row(
    player: &RowFacts,
    rank: usize,
    is_you: bool,
    top_damage: f32,
    settings: &OverlaySettings,
    shades: &mut ShadeBook,
) -> PlayerRow {
    let raw_bar = player.total_damage as f32 / top_damage;
    let name = if player.actor_name.is_empty() {
        format::unnamed_player(player.actor_id, is_you)
    } else {
        settings.display_name(player.actor_name)
    };
    let mut detail = Vec::with_capacity(2);
    if settings.show_server && !player.actor_server_id.is_empty() {
        detail.push(game_data::server_name(player.actor_server_id));
    }
    if settings.show_combat_power {
        if let Some(power) = player.combat_power.filter(|&power| power > 0) {
            detail.push(format::grouped(power as f64));
        }
    }
    let class = PlayerClass::parse(player.actor_class);
    PlayerRow {
        actor_id: player.actor_id,
        rank: rank.to_string().into(),
        name: settings.show_player_name.then(|| name.into()),
        detail: (!detail.is_empty()).then(|| detail.join(" · ").into()),
        class,
        colour: class.map(|class| shades.colour(class, player.actor_id, player.actor_name)),
        damage: format::damage(player.total_damage).into(),
        dps: format::compact(player.dps).into(),
        pct: format::percent(player.pct).into(),
        bar: if raw_bar > 0.0 { raw_bar.max(BAR_FLOOR) } else { 0.0 },
        is_you,
    }
}

/// The text Copy puts on the clipboard while the live meter is up.
pub fn battle_report(snapshot: &CombatSnapshot, target_id: u32, hide_unknown_players: bool) -> Option<String> {
    let view = target_view(snapshot, target_id, hide_unknown_players)?;
    let ranked = view.players;
    if ranked.is_empty() {
        return None;
    }
    let target = Some(view.info);
    let total = view.total_damage;
    let secs = target.map_or(0.0, fight_secs);
    let party = if secs > 0.0 { total as f64 / secs } else { 0.0 };
    let name = target
        .and_then(|target| game_data::mob_name(target.target_mob_code, target.target_name.as_deref()))
        .map(|name| format!("{name} · "))
        .unwrap_or_default();
    let mut report = format!(
        "{name}{}\n",
        fl!(
            "meter-report-header",
            duration = format::report_duration(secs),
            damage = format::damage(total),
            dps = format::grouped(party)
        )
    );
    for player in &ranked {
        let who = if player.actor_name.is_empty() {
            format::unnamed_player(player.actor_id, snapshot.combat_infos.main_actor_id == Some(player.actor_id))
        } else {
            player.actor_name.clone()
        };
        report.push_str(&fl!(
            "meter-copy-player",
            name = who,
            damage = format::damage(player.total_damage),
            dps = format::grouped(player.dps)
        ));
        report.push('\n');
    }
    report.push('\n');
    report.push_str(&fl!("meter-copy-sign-off"));
    Some(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn your_row_takes_the_last_place_when_you_rank_below_the_cap() {
        assert_eq!(visible_indexes(15, Some(13), 12), vec![0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 13]);
        assert_eq!(visible_indexes(15, Some(3), 12), (0..12).collect::<Vec<_>>());
        assert_eq!(visible_indexes(4, Some(2), 12), vec![0, 1, 2, 3]);
    }

    #[test]
    fn the_cap_is_the_players_shown_setting() {
        assert_eq!(visible_indexes(20, None, 5).len(), 5);
        assert_eq!(visible_indexes(20, None, 20).len(), 20);
    }
}
