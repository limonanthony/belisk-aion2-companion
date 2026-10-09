//! The words the model hands to the windows: countdowns, day stamps, plural
//! forms and the notes whose text depends on counts. The fixed labels are the
//! windows' own.

use super::data::HiddenSummary;
use crate::clock::Zone;
use crate::i18n::{day_stamp, duration_words, fl};

pub fn left(count: usize) -> String {
    fl!("todos-left", count = count)
}

pub fn all_done() -> String {
    fl!("todos-all-done")
}

pub fn done() -> String {
    fl!("todos-done")
}

pub fn weekly_on_wednesday() -> String {
    fl!("todos-weekly-on-wednesday")
}

pub fn not_tracked() -> String {
    fl!("todos-not-tracked")
}

/// "425 of 560".
pub fn value_of(value: u32, cap: u32) -> String {
    fl!("todos-value-of", value = value, cap = cap)
}

pub fn full(count: usize) -> String {
    fl!("todos-full", count = count)
}

pub fn hidden(count: usize) -> String {
    fl!("todos-hidden", count = count)
}

pub fn done_of(done: usize, total: usize) -> String {
    fl!("todos-done-of", done = done, total = total)
}

pub fn resets_in(ms: i64) -> String {
    fl!("todos-resets-in", time = duration_words(ms))
}

pub fn weekly_reset_when(zone: &dyn Zone, reset_ms: i64, now_ms: i64) -> String {
    fl!("todos-weekly-reset-when", day = day_stamp(zone, reset_ms), time = duration_words(reset_ms - now_ms))
}

pub fn amount_full_in(ms: i64) -> String {
    fl!("todos-amount-full-in", time = duration_words(ms))
}

pub fn full_being_wasted() -> String {
    fl!("todos-full-being-wasted")
}

pub fn full_spend_by(stamp: &str) -> String {
    fl!("todos-full-spend-by", stamp = stamp)
}

pub fn full_refills(stamp: &str) -> String {
    fl!("todos-full-refills", stamp = stamp)
}

/// "Everything is done on <name>." with the name as the window shows it.
pub fn everything_done(character_name: &str) -> String {
    fl!("todos-everything-done", name = character_name)
}

pub fn removed_note(name: &str) -> String {
    fl!("todos-removed-note", name = name)
}

pub fn added_note(name: &str, section_title: &str) -> String {
    fl!("todos-added-note", name = name, section = section_title)
}

pub fn to_dos_back(count: usize) -> String {
    fl!("todos-back", count = count)
}

/// Settings → To-dos → Hidden to-dos, the row's description.
pub fn hidden_description(summary: HiddenSummary) -> String {
    if summary.hidden == 0 {
        return fl!("todos-hidden-description-none");
    }
    fl!("todos-hidden-description", hidden = summary.hidden, characters = summary.characters)
}

/// The first paragraph of the "Reset the to-dos completely?" dialog; the clause
/// about your own to-dos is left out when there are none.
pub fn reset_dialog_body(own_todos: usize) -> String {
    if own_todos == 0 {
        fl!("todos-reset-dialog-body")
    } else {
        fl!("todos-reset-dialog-body-with-own", own = own_todos)
    }
}

pub fn reset_dialog_keeps() -> String {
    fl!("todos-reset-dialog-keeps")
}

// ---- A figure's tooltip: where its value comes from ----

pub fn tip_live() -> String {
    fl!("todos-tip-live")
}

/// "+3 extra from items: they never refill and are not part of the 21."
pub fn tip_extra(extra: u32, cap: u32) -> String {
    fl!("todos-tip-extra", extra = extra, cap = cap)
}

pub fn tip_extra_entries(extra: u32) -> String {
    fl!("todos-tip-extra-entries", extra = extra)
}

/// "125 at your login Sun 22:40.", or "… from the game, Sun 23:10." for a
/// value the game sent later in that session.
pub fn tip_game_value(value: u32, stamp: &str, at_login: bool) -> String {
    if at_login {
        fl!("todos-tip-at-login", value = value, stamp = stamp)
    } else {
        fl!("todos-tip-game-value", value = value, stamp = stamp)
    }
}

/// "+120 since: 8 refills of 15, estimated."
pub fn tip_refills(refills: u32, add: u32) -> String {
    let added: u32 = refills * add;
    fl!("todos-tip-refills", added = added, refills = refills, add = add)
}

pub fn tip_game_at(stamp: &str) -> String {
    fl!("todos-tip-game-at", stamp = stamp)
}

pub fn tip_restarted() -> String {
    fl!("todos-tip-restarted")
}

pub fn tip_shared(stamp: &str) -> String {
    fl!("todos-tip-shared", stamp = stamp)
}

pub fn tip_typed(game_value: u32) -> String {
    fl!("todos-tip-typed", value = game_value)
}

pub fn tip_typed_detail() -> String {
    fl!("todos-tip-typed-detail")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::clock::testing::{utc, TestZone};
    use crate::i18n::{set_language, Language};

    #[test]
    fn a_weekly_reset_reads_the_local_clock_and_the_time_left() {
        let zone = TestZone { base: 120, changes: Vec::new() };
        assert_eq!(weekly_reset_when(&zone, utc(2026, 10, 7, 7, 0), utc(2026, 10, 5, 19, 48)), "Wed 09:00 \u{b7} in 1 d 11 h");
        set_language(Language::French);
        assert_eq!(weekly_reset_when(&zone, utc(2026, 10, 7, 7, 0), utc(2026, 10, 5, 19, 48)), "mer. 09:00 \u{b7} dans 1 j 11 h");
    }

    #[test]
    fn plural_forms_follow_the_count() {
        assert_eq!(to_dos_back(1), "1 to-do back");
        assert_eq!(to_dos_back(6), "6 to-dos back");
        assert_eq!(
            hidden_description(HiddenSummary { hidden: 1, characters: 1 }),
            "1 hidden, on 1 character. Restoring the defaults keeps your own to-dos and all progress."
        );
        assert_eq!(
            hidden_description(HiddenSummary { hidden: 6, characters: 2 }),
            "6 hidden, on 2 characters. Restoring the defaults keeps your own to-dos and all progress."
        );
        assert_eq!(
            hidden_description(HiddenSummary::default()),
            "Nothing is hidden. Restoring the defaults keeps your own to-dos and all progress."
        );
    }

    #[test]
    fn french_counts_of_zero_and_one_are_singular() {
        set_language(Language::French);
        assert_eq!(to_dos_back(0), "0 t\u{e2}che restaur\u{e9}e");
        assert_eq!(to_dos_back(1), "1 t\u{e2}che restaur\u{e9}e");
        assert_eq!(to_dos_back(2), "2 t\u{e2}ches restaur\u{e9}es");
        assert_eq!(hidden(1), "1 masqu\u{e9}e");
        assert_eq!(hidden(3), "3 masqu\u{e9}es");
        assert_eq!(left(4), "4 \u{e0} faire");
    }

    #[test]
    fn the_reset_dialog_drops_the_own_to_dos_clause_when_there_are_none() {
        assert_eq!(
            reset_dialog_body(3),
            "On every character: your own to-dos (3) are removed, every hidden to-do comes back, and every count you typed and every amount typed in the Resources window is emptied. Values from the game stay."
        );
        assert_eq!(
            reset_dialog_body(0),
            "On every character: every hidden to-do comes back, and every count you typed and every amount typed in the Resources window is emptied. Values from the game stay."
        );
    }

    #[test]
    fn notes_name_the_to_do_and_the_section() {
        assert_eq!(added_note("Legion check-in", "This week"), "Added Legion check-in to This week.");
        assert_eq!(removed_note("Legion check-in"), "Removed Legion check-in.");
        assert_eq!(everything_done("Mirelle"), "Everything is done on Mirelle.");
    }

    #[test]
    fn a_tooltip_counts_the_refills() {
        assert_eq!(tip_refills(8, 15), "+120 since: 8 refills of 15, estimated.");
        assert_eq!(tip_refills(1, 3), "+3 since: one refill of 3, estimated.");
    }
}
