//! The fight summary card: how the fight went, where you placed, what did the
//! damage, and how it stands against your best. Ported from `showSummary` in
//! `overlay/meter/main.js`.

use aether_core::models::combat::{CombatSnapshot, PlayerOverviewStat, TargetInfo};
use gpui::SharedString;

use super::fight::SummaryKind;
use crate::format;
use crate::game_data::{self, PlayerClass};
use super::live::{self, pace_against, Gauge, TargetView, YouMatch};
use super::settings::OverlaySettings;

const TOP_SKILLS: usize = 3;

#[derive(Clone, Debug, PartialEq)]
pub enum Subject {
    Player { name: String, by: YouMatch },
    /// Nobody is presented as you: either the game has not named the local
    /// player, or they did no damage to this boss.
    Party { identified: bool },
}

pub struct FightSummary {
    pub kind: SummaryKind,
    pub elapsed: SharedString,
    pub boss: SharedString,
    /// "Dungeon · Difficulty", when the boss is known.
    pub place: Option<SharedString>,
    /// Boss health while the fight may still resume.
    pub boss_hp: Option<Gauge>,
    /// Whether the figures are yours. When the meter does not know which row
    /// is you, or you did no damage to this boss, the card is about the
    /// party and nobody else is presented as you.
    pub personal: bool,
    /// Who the card is about, for the diagnostic log.
    pub subject: Subject,
    /// Yours, or the party's when `personal` is false.
    pub dps: SharedString,
    pub rank: Option<SharedString>,
    pub share: Option<SharedString>,
    /// Players who hit the boss.
    pub players: SharedString,
    pub crit: Option<SharedString>,
    pub best: Option<BestLine>,
    pub skills: Vec<SkillLine>,
    pub class: Option<PlayerClass>,
    pub party_dps: Option<SharedString>,
    pub damage: SharedString,
    pub report: String,
}

pub enum BestLine {
    First,
    New { pace: SharedString },
    Behind { best: SharedString, pace: SharedString },
}

pub struct SkillLine {
    pub name: SharedString,
    pub share: SharedString,
    /// Relative to the top skill, 0–1.
    pub bar: f32,
}

struct Skill {
    name: String,
    share: f64,
}

impl FightSummary {
    /// About `target_id`, whatever the meter's current target is now. `None`
    /// when nobody has hit it. `personal_best` answers the best DPS for a
    /// character and boss.
    pub fn build(
        snapshot: &CombatSnapshot,
        target_id: u32,
        kind: SummaryKind,
        settings: &OverlaySettings,
        personal_best: impl FnOnce(&str, u32) -> Option<f64>,
    ) -> Option<Self> {
        let TargetView { info: target, players: ranked } = live::target_view(snapshot, target_id)?;
        if ranked.is_empty() {
            return None;
        }
        let found = live::find_you(
            snapshot,
            ranked.iter().map(|player| (player.actor_id, player.actor_name.as_str())),
        );
        let you = found.map(|found| found.index);
        let subject = match found {
            Some(found) => Subject::Player {
                name: ranked[found.index].actor_name.clone(),
                by: found.by,
            },
            None => Subject::Party { identified: live::is_identified(snapshot) },
        };
        let secs = live::fight_secs(target);
        let boss = live::target_name(target);
        let place = target.target_mob_code.and_then(game_data::dungeon_of);
        let party_dps: f64 = ranked.iter().map(|player| player.dps.max(0.0)).sum();
        let party_damage: u64 = ranked.iter().map(|player| player.total_damage).sum();

        let mine = you.map(|index| {
            let player = ranked[index];
            let (skills, crit) = top_skills(snapshot, target, player);
            let character = snapshot.combat_infos.main_actor_name.as_deref().unwrap_or_default();
            let best = match target.target_mob_code {
                Some(mob) if settings.show_personal_best => {
                    Some(best_line(personal_best(character, mob), player.dps))
                }
                _ => None,
            };
            (index, player, skills, crit, best)
        });

        let share = mine.as_ref().map(|(_, player, ..)| format::percent(player.damage_contribution));
        let report = copy_text(&CopyFacts {
            boss: &boss,
            place,
            kind,
            secs,
            you: mine.as_ref().zip(share.as_deref()).map(|((index, player, _, crit, best), share)| YouFacts {
                player,
                rank: index + 1,
                share,
                crit: *crit,
                best: best.as_ref(),
            }),
            party_dps,
            party_damage,
            skills: mine.as_ref().map_or(&[], |(_, _, skills, ..)| skills.as_slice()),
            ranked: &ranked,
        });

        let skill_lines = mine.as_ref().map_or_else(Vec::new, |(_, _, skills, ..)| {
            let top_share = skills.first().map_or(1.0, |skill| skill.share.max(f64::EPSILON));
            skills
                .iter()
                .map(|skill| SkillLine {
                    name: skill.name.clone().into(),
                    share: format::whole_percent(skill.share).into(),
                    bar: (skill.share / top_share) as f32,
                })
                .collect()
        });

        Some(Self {
            kind,
            elapsed: format::duration(secs).into(),
            boss: settings.display_name(&boss).into(),
            place: place.map(SharedString::from),
            boss_hp: live::boss_hp(target).filter(|_| settings.show_boss_hp),
            personal: mine.is_some(),
            subject,
            dps: match &mine {
                Some((_, player, ..)) => format::compact(player.dps).into(),
                None => format::compact(party_dps).into(),
            },
            rank: mine.as_ref().map(|(index, ..)| format!("{}/{}", index + 1, ranked.len()).into()),
            share: share.map(SharedString::from),
            players: ranked.len().to_string().into(),
            crit: mine
                .as_ref()
                .and_then(|(_, _, _, crit, _)| *crit)
                .map(|crit| format::whole_percent(crit).into()),
            class: mine.as_ref().and_then(|(_, player, ..)| PlayerClass::parse(&player.actor_class)),
            party_dps: mine.as_ref().and_then(|_| live::party_dps(&ranked)),
            damage: match &mine {
                Some((_, player, ..)) => format::damage(player.total_damage).into(),
                None => format::damage(party_damage).into(),
            },
            best: mine.and_then(|(_, _, _, _, best)| best),
            skills: skill_lines,
            report,
        })
    }
}

fn best_line(best_dps: Option<f64>, dps: f64) -> BestLine {
    match best_dps.and_then(|best| pace_against(best, dps).map(|pace| (best, pace))) {
        None => BestLine::First,
        Some((_, pace)) if pace > 0.0 => BestLine::New { pace: format::pace(pace).into() },
        Some((best, pace)) => BestLine::Behind {
            best: format::compact(best).into(),
            pace: format::pace(pace).into(),
        },
    }
}

/// The lead's three biggest skills on this target, and their crit rate.
fn top_skills(
    snapshot: &CombatSnapshot,
    target: &TargetInfo,
    lead: &PlayerOverviewStat,
) -> (Vec<Skill>, Option<f64>) {
    let Some(skills) = snapshot
        .by_target_player_skill_stats
        .get(&target.id)
        .and_then(|by_actor| by_actor.get(&lead.actor_id))
    else {
        return (Vec::new(), None);
    };
    let total: u64 = skills.values().map(|stats| stats.total_damage).sum();
    let hits: u64 = skills.values().map(|stats| u64::from(stats.counts)).sum();
    let crits: u64 = skills
        .values()
        .map(|stats| u64::from(stats.special_counts.get("CRITICAL").copied().unwrap_or(0)))
        .sum();
    let mut by_damage: Vec<(u32, u64)> =
        skills.iter().map(|(&code, stats)| (code, stats.total_damage)).collect();
    by_damage.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let top = by_damage
        .into_iter()
        .take(TOP_SKILLS)
        .map(|(code, damage)| Skill {
            name: game_data::skill_name(code),
            share: if total > 0 { damage as f64 / total as f64 } else { 0.0 },
        })
        .collect();
    (top, (hits > 0).then(|| crits as f64 / hits as f64))
}

struct CopyFacts<'a> {
    boss: &'a str,
    place: Option<&'a str>,
    kind: SummaryKind,
    secs: f64,
    /// You, when the summary is about you.
    you: Option<YouFacts<'a>>,
    party_dps: f64,
    party_damage: u64,
    skills: &'a [Skill],
    ranked: &'a [&'a PlayerOverviewStat],
}

struct YouFacts<'a> {
    player: &'a PlayerOverviewStat,
    rank: usize,
    share: &'a str,
    crit: Option<f64>,
    best: Option<&'a BestLine>,
}

/// For party chat or Discord.
fn copy_text(facts: &CopyFacts) -> String {
    let outcome = match facts.kind {
        SummaryKind::Final => "defeated",
        SummaryKind::Provisional => "fight ended",
    };
    let place = facts.place.map(|place| format!(" ({place})")).unwrap_or_default();
    let mut lines = vec![format!(
        "{}{place} · {outcome} in {}",
        facts.boss,
        format::duration(facts.secs)
    )];

    match &facts.you {
        Some(you) => {
            let name = if you.player.actor_name.is_empty() { "?" } else { &you.player.actor_name };
            let mut line = vec![
                format!("{name} {}/s", format::grouped(you.player.dps)),
                format!("#{} of {}", you.rank, facts.ranked.len()),
                you.share.to_string(),
            ];
            if let Some(crit) = you.crit {
                line.push(format!("crit {}", format::whole_percent(crit)));
            }
            match you.best {
                Some(BestLine::First) => line.push("First record".to_string()),
                Some(BestLine::New { pace }) => line.push(format!("New PB {pace}")),
                Some(BestLine::Behind { best, pace }) => line.push(format!("PB {best}/s {pace}")),
                None => {}
            }
            lines.push(line.join(" · "));
        }
        None => lines.push(format!(
            "Party {}/s · {} damage",
            format::grouped(facts.party_dps),
            format::damage(facts.party_damage)
        )),
    }

    if !facts.skills.is_empty() {
        let top: Vec<String> = facts
            .skills
            .iter()
            .map(|skill| format!("{} {}", skill.name, format::whole_percent(skill.share)))
            .collect();
        lines.push(format!("Top: {}", top.join(", ")));
    }
    if facts.ranked.len() > 1 {
        let party: Vec<String> = facts
            .ranked
            .iter()
            .enumerate()
            .map(|(index, player)| {
                let name = if player.actor_name.is_empty() {
                    format!("ID:{}", player.actor_id)
                } else {
                    player.actor_name.clone()
                };
                format!("{}. {name} {}", index + 1, format::grouped(player.dps))
            })
            .collect();
        lines.push(format!("Party: {}", party.join(" · ")));
    }
    lines.push("— AION 2 DPS Meter".to_string());
    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_best_line_follows_the_pace() {
        assert!(matches!(best_line(None, 50_000.0), BestLine::First));
        assert!(matches!(best_line(Some(40_000.0), 50_000.0), BestLine::New { .. }));
        match best_line(Some(51_740.0), 48_210.0) {
            BestLine::Behind { best, pace } => {
                assert_eq!(best.as_ref(), "51.7K");
                assert_eq!(pace.as_ref(), "\u{2212}6.8%");
            }
            _ => panic!("expected a pace behind the best"),
        }
    }
}
