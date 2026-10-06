use serde::{Deserialize, Serialize};

pub const DEFAULT_RESET_HOUR: u8 = 9;

/// What Settings → To-dos changes; the to-dos themselves live in `todos.json`.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct TodoSettings {
    /// The hour (0 to 23, the PC's clock) of the daily reset; the weekly reset
    /// is Wednesday at the same hour.
    pub reset_hour: u8,
    /// Odyle Energy fills up to 840 instead of 560.
    pub membership: bool,
}

impl Default for TodoSettings {
    fn default() -> Self {
        Self { reset_hour: DEFAULT_RESET_HOUR, membership: false }
    }
}

impl TodoSettings {
    pub fn clamped(mut self) -> Self {
        self.reset_hour = self.reset_hour.min(23);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_reset_hour_is_pulled_into_the_day() {
        assert_eq!(TodoSettings { reset_hour: 31, membership: true }.clamped().reset_hour, 23);
        assert_eq!(TodoSettings::default().clamped(), TodoSettings { reset_hour: 9, membership: false });
    }
}
