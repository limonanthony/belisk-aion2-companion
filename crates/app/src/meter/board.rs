//! Which board the meter shows: the whole session or the target.
//!
//! Automatic by default: the session in the open world, the Target board while
//! a boss is engaged (see `focus`). The footer switch pins one side. A pin
//! lasts until the meter is reset or the next boss is engaged, and clicking the
//! pinned side again hands the choice back. While the Target board is pinned
//! with no boss engaged, it follows the current target.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Board {
    Session,
    Target,
}

#[derive(Debug, Default)]
pub struct BoardChoice {
    pinned: Option<Board>,
    boss_engaged: bool,
}

impl BoardChoice {
    pub fn active(&self) -> Board {
        match self.pinned {
            Some(board) => board,
            None if self.boss_engaged => Board::Target,
            None => Board::Session,
        }
    }

    pub fn is_pinned(&self) -> bool {
        self.pinned.is_some()
    }

    /// The switch was clicked on `side`: the other side is pinned, and the
    /// pinned side goes back to automatic. The automatic side stays as it is.
    pub fn select(&mut self, side: Board) {
        if side != self.active() {
            self.pinned = Some(side);
        } else if self.pinned.is_some() {
            self.pinned = None;
        }
    }

    /// A boss was engaged, which ends a pin.
    pub fn boss_engaged(&mut self) {
        self.boss_engaged = true;
        self.pinned = None;
    }

    /// The engaged boss was let go. A pin stays.
    pub fn boss_released(&mut self) {
        self.boss_engaged = false;
    }

    /// Nothing is on the meter any more.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn with_no_boss_the_session_shows() {
        let choice = BoardChoice::default();

        assert_eq!(choice.active(), Board::Session);
        assert!(!choice.is_pinned());
    }

    #[test]
    fn automatic_follows_whether_a_boss_is_engaged() {
        let mut choice = BoardChoice::default();

        choice.boss_engaged();
        assert_eq!(choice.active(), Board::Target);

        choice.boss_released();
        assert_eq!(choice.active(), Board::Session);
        assert!(!choice.is_pinned());
    }

    #[test]
    fn clicking_the_inactive_side_pins_it() {
        let mut choice = BoardChoice::default();

        choice.select(Board::Target);

        assert_eq!(choice.active(), Board::Target);
        assert!(choice.is_pinned());
    }

    #[test]
    fn clicking_the_active_side_when_pinned_returns_to_automatic() {
        let mut choice = BoardChoice::default();
        choice.select(Board::Target);

        choice.select(Board::Target);

        assert_eq!(choice.active(), Board::Session);
        assert!(!choice.is_pinned());
    }

    #[test]
    fn clicking_the_active_side_in_automatic_changes_nothing() {
        let mut choice = BoardChoice::default();

        choice.select(Board::Session);

        assert_eq!(choice.active(), Board::Session);
        assert!(!choice.is_pinned());
    }

    #[test]
    fn a_pin_on_a_boss_fight_holds_and_unpinning_returns_to_the_boss() {
        let mut choice = BoardChoice::default();
        choice.boss_engaged();
        choice.select(Board::Session);
        assert_eq!(choice.active(), Board::Session);

        choice.select(Board::Session);

        assert_eq!(choice.active(), Board::Target);
    }

    #[test]
    fn engaging_a_boss_ends_a_pin() {
        let mut choice = BoardChoice::default();
        choice.select(Board::Target);

        choice.boss_engaged();

        assert!(!choice.is_pinned());
        assert_eq!(choice.active(), Board::Target);

        choice.select(Board::Session);
        choice.boss_engaged();
        assert!(!choice.is_pinned());
    }

    #[test]
    fn a_pin_survives_the_boss_being_released() {
        let mut choice = BoardChoice::default();
        choice.boss_engaged();
        choice.select(Board::Session);

        choice.boss_released();

        assert!(choice.is_pinned());
        assert_eq!(choice.active(), Board::Session);
    }

    #[test]
    fn a_reset_clears_the_pin_and_the_boss() {
        let mut choice = BoardChoice::default();
        choice.boss_engaged();
        choice.select(Board::Session);

        choice.reset();

        assert!(!choice.is_pinned());
        assert_eq!(choice.active(), Board::Session);
    }
}
