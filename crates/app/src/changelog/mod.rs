//! The changelog the app shows: `CHANGELOG.md` (English) and
//! `CHANGELOG.fr.md` at the repository root, embedded at build time and parsed
//! only when a window asks for them, never at startup or during play (the
//! launch check in [`whats_new`] parses only when the version changed). The
//! files are written in the small Markdown subset of [`parser`].

mod parser;
mod version;
mod whats_new;

pub use parser::{Changelog, Date, Release, Text};
#[cfg(test)]
pub use parser::Span;
pub use version::Version;
pub use whats_new::WhatsNew;

use gpui::{rgb, FontWeight, HighlightStyle, StyledText};

use crate::i18n::{self, Language};

const ENGLISH: &str = include_str!("../../../../CHANGELOG.md");
const FRENCH: &str = include_str!("../../../../CHANGELOG.fr.md");

fn source(language: Language) -> &'static str {
    match language {
        Language::English => ENGLISH,
        Language::French => FRENCH,
    }
}

/// The whole changelog in `language`, newest version first. The embedded files
/// are checked by the tests, so a parse failure is logged and shows an empty
/// changelog rather than stopping the app.
pub fn load(language: Language) -> Changelog {
    Changelog::parse(source(language)).unwrap_or_else(|problem| {
        crate::app_log::error(format!("The embedded {language:?} changelog does not parse: {problem}"));
        Changelog::default()
    })
}

/// What to announce at launch: the versions between `last_seen` (the version
/// stored in settings) and this build. Costs nothing, and parses nothing, when
/// this build's version has already been seen.
pub fn whats_new(last_seen: Option<Version>, language: Language) -> Option<WhatsNew> {
    let current = Version::current();
    if last_seen.is_some_and(|seen| seen >= current) {
        return None;
    }
    WhatsNew::select(&load(language), last_seen, current)
}

/// "21 Oct 2026" (French: "21 oct. 2026").
pub fn date_words(date: Date) -> String {
    i18n::day_month_year(date.day, date.month, date.year)
}

/// Bold runs at 600 in the paragraph's own colour (a summary).
pub fn bold() -> HighlightStyle {
    HighlightStyle { font_weight: Some(FontWeight::SEMIBOLD), ..Default::default() }
}

/// Bold runs at 600 in the text colour inside a `muted` paragraph (the
/// bullets). GPUI blends a run's colour over the paragraph's and keeps the
/// paragraph's alpha, so the translucent text colour would come out muted:
/// the run takes the text colour opaque.
pub fn bold_in_text_colour() -> HighlightStyle {
    HighlightStyle { color: Some(rgb(crate::secondary::chrome::TEXT >> 8).into()), ..bold() }
}

/// A summary or a bullet as one text, its bold runs drawn with `bold`.
pub fn styled(text: &Text, bold: HighlightStyle) -> StyledText {
    let mut whole = String::with_capacity(text.iter().map(|span| span.text.len()).sum());
    let mut highlights = Vec::new();
    for span in text {
        let start = whole.len();
        whole.push_str(&span.text);
        if span.bold {
            highlights.push((start..whole.len(), bold));
        }
    }
    StyledText::new(whole).with_highlights(highlights)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(language: Language) -> Changelog {
        Changelog::parse(source(language)).unwrap_or_else(|problem| panic!("{language:?} changelog: {problem}"))
    }

    #[test]
    fn every_embedded_changelog_is_in_the_subset() {
        for language in Language::ALL {
            let changelog = parse(language);
            assert!(!changelog.releases.is_empty(), "{language:?}");
        }
    }

    #[test]
    fn the_languages_list_the_same_versions_dates_and_bullet_counts() {
        let english = parse(Language::English);
        let french = parse(Language::French);
        let shape = |changelog: &Changelog| -> Vec<(Version, Date, usize)> {
            changelog.releases.iter().map(|r| (r.version, r.date, r.bullets.len())).collect()
        };
        assert_eq!(shape(&english), shape(&french));
    }

    #[test]
    fn the_newest_entry_is_this_build() {
        for language in Language::ALL {
            assert_eq!(parse(language).releases[0].version, Version::current(), "{language:?}");
        }
    }

    #[test]
    fn the_changelog_starts_at_one_point_zero() {
        for language in Language::ALL {
            let oldest = parse(language).releases.pop().unwrap();
            assert_eq!(oldest.version, Version::new(1, 0, 0), "{language:?}");
            assert!(!oldest.bullets.is_empty(), "{language:?}");
        }
    }

    #[test]
    fn an_older_build_sees_the_first_notes_once_and_a_current_one_nothing() {
        for language in Language::ALL {
            let shown = whats_new(None, language).expect("notes for a settings file with no remembered version");
            assert_eq!(shown.since, None);
            assert!(shown.releases.iter().all(|release| release.version <= Version::current()));
            assert_eq!(whats_new(Some(Version::current()), language), None);
        }
    }

    #[test]
    fn a_bold_run_in_a_muted_bullet_is_drawn_in_the_text_colour_at_600() {
        use gpui::{rgba, Hsla, TextStyle};
        use crate::secondary::chrome::{MUTED, TEXT};

        let bullet = TextStyle { color: rgba(MUTED).into(), font_weight: FontWeight::MEDIUM, ..Default::default() };
        let bold_run = bullet.clone().highlight(bold_in_text_colour());
        let text: Hsla = rgba(TEXT).into();

        assert_eq!(bold_run.font_weight, FontWeight::SEMIBOLD);
        assert!((bold_run.color.l - text.l).abs() < 0.01, "lightness {} against the text colour's {}", bold_run.color.l, text.l);
        assert!(bold_run.color.a >= text.a, "alpha {} under the text colour's {}", bold_run.color.a, text.a);
        assert!(bold_run.color.a > bullet.color.a, "the bold run stayed as faint as the bullet");

        let summary_bold = TextStyle { color: text, ..bullet }.highlight(bold());
        assert_eq!((summary_bold.font_weight, summary_bold.color), (FontWeight::SEMIBOLD, text));
    }
}
