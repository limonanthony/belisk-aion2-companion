//! What the fight report window shows, worked out from a [`FightReport`]: the
//! names as the overlay writes them (masked, `ID:<n>`, "You"), the players the
//! game never named left out when Settings hides them (as History does), and
//! one player's details.

use std::collections::HashMap;

use meter_core::report::fight_report::{
    BuffReport, DeathReport, FightOutcome, FightReport, HitTypes, PlayerReport, SkillReport,
};
use meter_core::report::ledger::KillerKind;

use crate::format;
use crate::game_data::{self, PlayerClass};
use crate::i18n::fl;
use crate::meter::{OverlaySettings, ShadeBook};

/// A column of the player table, and what it sorts by.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Column {
    Damage,
    Dps,
    Healing,
    Taken,
    Deaths,
}

impl Column {
    pub const ALL: [Self; 5] = [Self::Damage, Self::Dps, Self::Healing, Self::Taken, Self::Deaths];

    /// Healing, damage taken and deaths exist only in fights saved since the
    /// meter kept them; damage and DPS are in every fight.
    pub fn is_in_every_fight(self) -> bool {
        matches!(self, Self::Damage | Self::Dps)
    }

    pub fn key(self) -> &'static str {
        match self {
            Self::Damage => "damage",
            Self::Dps => "dps",
            Self::Healing => "healing",
            Self::Taken => "taken",
            Self::Deaths => "deaths",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Totals {
    pub damage: u64,
    pub party_dps: f64,
    pub healing: Option<u64>,
    pub taken: Option<u64>,
    pub deaths: Option<u32>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlayerRow {
    pub actor_id: u32,
    pub name: String,
    /// Short server name, empty when unknown.
    pub server: String,
    pub class: Option<PlayerClass>,
    /// The player's shade of their class colour in this fight.
    pub colour: Option<u32>,
    pub you: bool,
    pub damage: u64,
    pub dps: f64,
    pub healing: Option<u64>,
    pub taken: Option<u64>,
    pub deaths: Option<u32>,
}

impl PlayerRow {
    /// What the row has in `column`; nothing recorded counts as nothing.
    pub fn value(&self, column: Column) -> f64 {
        match column {
            Column::Damage => self.damage as f64,
            Column::Dps => self.dps.max(0.0),
            Column::Healing => self.healing.unwrap_or(0) as f64,
            Column::Taken => self.taken.unwrap_or(0) as f64,
            Column::Deaths => f64::from(self.deaths.unwrap_or(0)),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DeathRow {
    /// Seconds into the fight.
    pub at_secs: f64,
    pub actor_id: u32,
    pub name: String,
    pub class: Option<PlayerClass>,
    pub you: bool,
    /// The monster (or player) that killed.
    pub killer: String,
    pub skill: String,
    /// The hit taken right before the death.
    pub last_hit: Option<u64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ReportData {
    pub boss: String,
    /// Where (with the difficulty), when, how many players; a field boss has
    /// no place.
    pub sub_line: String,
    pub when: String,
    pub duration_secs: f64,
    pub outcome: FightOutcome,
    pub totals: Totals,
    /// Most damage first.
    pub players: Vec<PlayerRow>,
    /// In fight order. `None`: not recorded for this fight.
    pub deaths: Option<Vec<DeathRow>>,
    colours: HashMap<u32, u32>,
}

impl ReportData {
    pub fn build(report: &FightReport, settings: &OverlaySettings, hide_unknown_players: bool) -> Self {
        let colours = colours(report);
        let names = Names { settings, hide_unknown_players };
        let players: Vec<PlayerRow> = report
            .players
            .iter()
            .filter(|player| names.is_shown(player))
            .map(|player| PlayerRow {
                actor_id: player.actor_id,
                name: names.of(player),
                server: server(player),
                class: PlayerClass::parse(&player.class),
                colour: colours.get(&player.actor_id).copied(),
                you: player.is_you,
                damage: player.damage,
                dps: player.dps,
                healing: player.healing,
                taken: player.damage_taken,
                deaths: player.deaths,
            })
            .collect();
        let deaths = report.deaths.as_ref().map(|deaths| {
            deaths
                .iter()
                .filter_map(|death| {
                    let player = report.player(death.actor_id);
                    player.is_none_or(|player| names.is_shown(player)).then(|| death_row(death, player, &names))
                })
                .collect()
        });
        let when = crate::history::record_time(report.ended_at_ms);
        let place = report.mob_code.and_then(game_data::dungeon_of);
        let count = fl!("common-players-count", count = players.len());
        let sub_line = place.into_iter().chain([when.clone(), count]).collect::<Vec<_>>().join(" · ");
        Self {
            boss: boss_name(report),
            sub_line,
            when,
            duration_secs: report.duration_secs,
            outcome: report.outcome,
            totals: Totals {
                damage: report.totals.damage,
                party_dps: report.totals.party_dps,
                healing: report.totals.healing,
                taken: report.totals.damage_taken,
                deaths: report.totals.deaths,
            },
            players,
            deaths,
            colours,
        }
    }

    /// Whether healing, damage taken and deaths were kept for this fight.
    pub fn is_recorded(&self) -> bool {
        self.deaths.is_some()
    }

    /// The rows by `column`, highest first; ties go to the most damage.
    pub fn ranked(&self, column: Column) -> Vec<&PlayerRow> {
        let mut rows: Vec<&PlayerRow> = self.players.iter().collect();
        rows.sort_by(|a, b| b.value(column).total_cmp(&a.value(column)).then(b.damage.cmp(&a.damage)));
        rows
    }

    /// One player's details; `None` for a player the table does not show.
    pub fn player(&self, report: &FightReport, actor_id: u32, settings: &OverlaySettings) -> Option<PlayerDetail> {
        let row = self.players.iter().find(|row| row.actor_id == actor_id)?;
        let player = report.player(actor_id)?;
        let names = Names { settings, hide_unknown_players: false };
        let caster = |buff: &BuffReport| BuffLine::of(buff, report, &self.colours, &names);
        let class_colour = row.class.map(PlayerClass::rgb);
        Some(PlayerDetail {
            row: row.clone(),
            class_colour,
            combat_power: player.combat_power.filter(|power| *power > 0),
            hit_types: player.hit_types,
            skills: skill_lines(player),
            buffs_on_player: by_uptime(player.buffs.iter().map(caster).collect()),
            buffs_on_boss: by_uptime(report.boss_buffs.iter().map(caster).collect()),
            heals: player.heals.as_ref().map(|heals| {
                let total: u64 = heals.iter().map(|heal| heal.amount).sum();
                heals
                    .iter()
                    .map(|heal| HealLine {
                        code: heal.skill_code,
                        name: game_data::skill_name(heal.skill_code),
                        icon: game_data::skill_icon(heal.skill_code),
                        heals: heal.heals,
                        crits: heal.crits,
                        amount: heal.amount,
                        share: share(heal.amount, total),
                    })
                    .collect()
            }),
            taken: player.taken.as_ref().map(|taken| {
                let total: u64 = taken.iter().map(|skill| skill.amount).sum();
                taken
                    .iter()
                    .map(|skill| TakenLine {
                        skill: game_data::skill_name(skill.skill_code),
                        monster: monster_name(skill.monster_name.as_deref(), skill.monster_code),
                        hits: skill.hits,
                        largest: skill.largest,
                        amount: skill.amount,
                        share: share(skill.amount, total),
                    })
                    .collect()
            }),
            deaths: self
                .deaths
                .iter()
                .flatten()
                .filter(|death| death.actor_id == actor_id)
                .cloned()
                .collect(),
        })
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlayerDetail {
    pub row: PlayerRow,
    /// The plain class colour the skill bars use.
    pub class_colour: Option<u32>,
    pub combat_power: Option<u64>,
    pub hit_types: HitTypes,
    /// Most damage first.
    pub skills: Vec<SkillLine>,
    /// Longest up first.
    pub buffs_on_player: Vec<BuffLine>,
    pub buffs_on_boss: Vec<BuffLine>,
    /// `None`: not recorded.
    pub heals: Option<Vec<HealLine>>,
    /// `None`: not recorded.
    pub taken: Option<Vec<TakenLine>>,
    pub deaths: Vec<DeathRow>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SkillLine {
    pub code: u32,
    pub name: String,
    /// Asset path, when the game data has an icon.
    pub icon: Option<String>,
    pub spec: [bool; 5],
    pub hit_types: HitTypes,
    pub damage: u64,
    /// Of the player's damage.
    pub share: f64,
    pub min: u64,
    pub max: u64,
    pub average: u64,
    pub multi_damage: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BuffLine {
    pub code: u32,
    pub name: String,
    pub icon: Option<String>,
    pub caster: String,
    pub caster_class: Option<PlayerClass>,
    pub colour: Option<u32>,
    /// `None` in a fight saved before the meter kept uptime.
    pub uptime_secs: Option<f64>,
    /// Of the fight, 0 to 1.
    pub share: Option<f64>,
}

impl BuffLine {
    fn of(buff: &BuffReport, report: &FightReport, colours: &HashMap<u32, u32>, names: &Names) -> Self {
        let caster = report.player(buff.caster_id);
        let caster_class = buff.caster_class.as_deref().and_then(PlayerClass::parse);
        Self {
            code: buff.skill_code,
            name: game_data::skill_name(buff.skill_code),
            icon: game_data::skill_icon(buff.skill_code),
            caster: match (caster, buff.caster_name.as_deref()) {
                (Some(player), _) => names.of(player),
                (None, Some(name)) if !name.trim().is_empty() => names.settings.display_name(name),
                (None, _) => format::unnamed_player(buff.caster_id, false),
            },
            caster_class,
            colour: colours.get(&buff.caster_id).copied().or(caster_class.map(PlayerClass::rgb)),
            uptime_secs: buff.uptime_ms.map(|ms| ms as f64 / 1000.0),
            share: buff.share,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct HealLine {
    pub code: u32,
    pub name: String,
    pub icon: Option<String>,
    pub heals: u32,
    pub crits: u32,
    pub amount: u64,
    /// Of the player's healing.
    pub share: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TakenLine {
    pub skill: String,
    pub monster: String,
    pub hits: u32,
    pub largest: u64,
    pub amount: u64,
    /// Of the player's damage taken.
    pub share: f64,
}

/// How the report names players: as the overlay settings write them.
struct Names<'a> {
    settings: &'a OverlaySettings,
    hide_unknown_players: bool,
}

impl Names<'_> {
    fn is_shown(&self, player: &PlayerReport) -> bool {
        !self.hide_unknown_players || player.is_you || !player.name.trim().is_empty()
    }

    fn of(&self, player: &PlayerReport) -> String {
        if player.name.trim().is_empty() {
            format::unnamed_player(player.actor_id, player.is_you)
        } else {
            self.settings.display_name(&player.name)
        }
    }
}

fn server(player: &PlayerReport) -> String {
    let id = player.server_id.trim();
    if id.is_empty() {
        String::new()
    } else {
        game_data::server_name(id)
    }
}

fn boss_name(report: &FightReport) -> String {
    game_data::mob_name(report.mob_code, report.boss_name.as_deref()).unwrap_or_else(|| match report.mob_code {
        Some(code) => fl!("common-mob-numbered", code = code),
        None => fl!("common-target-numbered", id = report.target_id),
    })
}

fn monster_name(name: Option<&str>, code: Option<u32>) -> String {
    let english = name.filter(|name| !name.trim().is_empty());
    match (game_data::mob_name(code, english), code) {
        (Some(name), _) => name,
        (None, Some(code)) => fl!("common-mob-numbered", code = code),
        (None, None) => fl!("report-unknown"),
    }
}

fn death_row(death: &DeathReport, player: Option<&PlayerReport>, names: &Names) -> DeathRow {
    let killer = match death.killer.kind {
        KillerKind::Player => death
            .killer
            .name
            .as_deref()
            .filter(|name| !name.trim().is_empty())
            .map_or_else(|| fl!("report-unknown"), |name| names.settings.display_name(name)),
        KillerKind::Monster | KillerKind::Unknown => {
            let from_last_hit = death.last_hit.as_ref().filter(|hit| hit.monster_code == death.killer.monster_code);
            monster_name(
                death.killer.name.as_deref().or(from_last_hit.and_then(|hit| hit.monster_name.as_deref())),
                death.killer.monster_code,
            )
        }
    };
    let skill_code = match death.skill_code {
        0 => death.last_hit.as_ref().map_or(0, |hit| hit.skill_code),
        code => code,
    };
    DeathRow {
        at_secs: death.at_secs,
        actor_id: death.actor_id,
        name: match player {
            Some(player) => names.of(player),
            None if death.name.trim().is_empty() => format::unnamed_player(death.actor_id, false),
            None => names.settings.display_name(&death.name),
        },
        class: PlayerClass::parse(&death.class),
        you: player.is_some_and(|player| player.is_you),
        killer,
        skill: if skill_code == 0 { fl!("report-unknown") } else { game_data::skill_name(skill_code) },
        last_hit: death.last_hit.as_ref().map(|hit| hit.amount),
    }
}

fn skill_lines(player: &PlayerReport) -> Vec<SkillLine> {
    let total: u64 = player.skills.iter().map(|skill| skill.damage).sum();
    player.skills.iter().map(|skill| skill_line(skill, total)).collect()
}

fn skill_line(skill: &SkillReport, total: u64) -> SkillLine {
    SkillLine {
        code: skill.skill_code,
        name: game_data::skill_name(skill.skill_code),
        icon: game_data::skill_icon(skill.skill_code),
        spec: skill.spec,
        hit_types: skill.hit_types,
        damage: skill.damage,
        share: share(skill.damage, total),
        min: skill.min_damage,
        max: skill.max_damage,
        average: skill.average_damage(),
        multi_damage: skill.multi_hit_damage,
    }
}

fn by_uptime(mut buffs: Vec<BuffLine>) -> Vec<BuffLine> {
    buffs.sort_by(|a, b| b.uptime_secs.unwrap_or(0.0).total_cmp(&a.uptime_secs.unwrap_or(0.0)));
    buffs
}

fn share(part: u64, whole: u64) -> f64 {
    if whole == 0 {
        0.0
    } else {
        part as f64 / whole as f64
    }
}

/// Each player's shade within the fight, most damage first, as History shades
/// a saved fight.
fn colours(report: &FightReport) -> HashMap<u32, u32> {
    let mut book = ShadeBook::default();
    report
        .players
        .iter()
        .filter_map(|player| {
            let class = PlayerClass::parse(&player.class)?;
            Some((player.actor_id, book.colour(class, player.actor_id, player.name.trim())))
        })
        .collect()
}
