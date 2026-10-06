use i18n_embed::unic_langid::LanguageIdentifier;
use serde::{Deserialize, Serialize};
use windows::Win32::Globalization::GetUserDefaultUILanguage;

/// A language the app is written in.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum Language {
    #[default]
    English,
    French,
}

impl Language {
    pub const ALL: [Language; 2] = [Language::English, Language::French];

    pub(super) fn identifier(self) -> LanguageIdentifier {
        let tag = match self {
            Self::English => "en",
            Self::French => "fr",
        };
        tag.parse().unwrap_or_else(|error| panic!("{tag:?} is not a language tag: {error}"))
    }

    /// The language's own name, which is never translated.
    pub fn autonym(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::French => "Fran\u{e7}ais",
        }
    }

    fn of_windows_language_id(language_id: u16) -> Self {
        const PRIMARY_LANGUAGE_MASK: u16 = 0x3FF;
        const PRIMARY_FRENCH: u16 = 0x0C;
        if language_id & PRIMARY_LANGUAGE_MASK == PRIMARY_FRENCH {
            Self::French
        } else {
            Self::English
        }
    }

    /// The language of Windows' display, for the current user.
    pub fn of_windows() -> Self {
        // SAFETY: takes no arguments and only reads the user's setting.
        Self::of_windows_language_id(unsafe { GetUserDefaultUILanguage() })
    }
}

/// What Settings → General → Language stores.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum LanguageChoice {
    /// Follows Windows' display language: French, else English.
    #[default]
    System,
    English,
    French,
}

impl LanguageChoice {
    pub const ALL: [LanguageChoice; 3] = [Self::System, Self::English, Self::French];

    pub fn resolve(self) -> Language {
        match self {
            Self::System => Language::of_windows(),
            Self::English => Language::English,
            Self::French => Language::French,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn any_french_windows_language_is_french_and_the_rest_english() {
        for french in [0x040C, 0x080C, 0x0C0C, 0x100C, 0x140C, 0x180C, 0x0C] {
            assert_eq!(Language::of_windows_language_id(french), Language::French, "{french:#06x}");
        }
        for other in [0x0409, 0x0809, 0x0407, 0x0411, 0x0000] {
            assert_eq!(Language::of_windows_language_id(other), Language::English, "{other:#06x}");
        }
    }

    #[test]
    fn an_explicit_choice_ignores_windows() {
        assert_eq!(LanguageChoice::English.resolve(), Language::English);
        assert_eq!(LanguageChoice::French.resolve(), Language::French);
    }

    #[test]
    fn the_choice_is_stored_by_name() {
        assert_eq!(serde_json::to_string(&LanguageChoice::System).unwrap(), "\"system\"");
        assert_eq!(serde_json::from_str::<LanguageChoice>("\"french\"").unwrap(), LanguageChoice::French);
        assert!(serde_json::from_str::<LanguageChoice>("\"klingon\"").is_err());
    }
}
