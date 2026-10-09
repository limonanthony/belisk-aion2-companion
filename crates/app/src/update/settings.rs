use serde::{Deserialize, Serialize};

/// Settings › About › Updates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct UpdateSettings {
    /// A newer release is downloaded, verified and staged without a click
    /// (throttled, and paused while fighting); the player only restarts. Off:
    /// the player clicks Update. The hourly check itself has no switch.
    pub download_automatically: bool,
}

impl Default for UpdateSettings {
    fn default() -> Self {
        Self { download_automatically: true }
    }
}

#[cfg(test)]
mod tests {
    use crate::settings::AppSettings;

    #[test]
    fn automatic_download_is_on_by_default() {
        assert!(AppSettings::default().updates.download_automatically);
    }

    #[test]
    fn a_settings_file_from_before_the_key_existed_reads_as_on() {
        let old = r#"{"general":{"language":"english"},"overlay":{"showDps":false}}"#;

        let settings: AppSettings = serde_json::from_str(old).unwrap();

        assert!(settings.updates.download_automatically);
        assert!(!settings.overlay.show_dps, "the rest of the old file is still read");
    }

    #[test]
    fn a_stored_off_stays_off_and_is_written_camel_case() {
        let mut settings = AppSettings::default();
        settings.updates.download_automatically = false;

        let json = serde_json::to_string(&settings).unwrap();
        let back: AppSettings = serde_json::from_str(&json).unwrap();

        assert!(json.contains(r#""updates":{"downloadAutomatically":false}"#), "{json}");
        assert!(!back.updates.download_automatically);
    }

    #[test]
    fn an_updates_block_without_the_key_reads_as_on() {
        let settings: AppSettings = serde_json::from_str(r#"{"updates":{}}"#).unwrap();
        assert!(settings.updates.download_automatically);
    }
}
