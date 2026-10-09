//! The Markdown subset the changelog files are written in, and its parser.
//!
//! ```text
//! # Title                              (optional, first line only)
//!
//! ## [1.2.3] - 2026-10-21              (one per version, newest first)
//!
//! One summary paragraph.
//!
//! - A bullet, with **bold** allowed.
//!   Continued on lines indented by two spaces.
//! ```
//!
//! Anything else is an error that names its line, so a mistake fails the
//! tests instead of rendering badly in the app.

use std::fmt;

use super::version::Version;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Date {
    pub year: u16,
    pub month: u8,
    pub day: u8,
}

impl Date {
    /// `YYYY-MM-DD`, a real calendar day.
    pub fn parse(text: &str) -> Option<Self> {
        let bytes = text.as_bytes();
        let shaped = bytes.len() == 10
            && bytes[4] == b'-'
            && bytes[7] == b'-'
            && bytes.iter().enumerate().all(|(at, byte)| matches!(at, 4 | 7) || byte.is_ascii_digit());
        if !shaped {
            return None;
        }
        let year: u16 = text[..4].parse().ok()?;
        let month: u8 = text[5..7].parse().ok()?;
        let day: u8 = text[8..].parse().ok()?;
        let is_leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let days_in_month = match month {
            1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
            4 | 6 | 9 | 11 => 30,
            2 if is_leap => 29,
            2 => 28,
            _ => return None,
        };
        (1..=days_in_month).contains(&day).then_some(Self { year, month, day })
    }
}

impl fmt::Display for Date {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:04}-{:02}-{:02}", self.year, self.month, self.day)
    }
}

/// A run of text, bold or plain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Span {
    pub text: String,
    pub bold: bool,
}

/// A summary or a bullet: its runs, in order.
pub type Text = Vec<Span>;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub version: Version,
    pub date: Date,
    pub summary: Text,
    /// May be empty: a version with no bullets is listed in the changelog but
    /// never announced by What's new.
    pub bullets: Vec<Text>,
}

/// Every release of one language's file, newest first.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Changelog {
    pub releases: Vec<Release>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ParseError {
    /// 1-based; 0 when the problem is the file as a whole.
    pub line: usize,
    pub message: String,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.line {
            0 => f.write_str(&self.message),
            line => write!(f, "line {line}: {}", self.message),
        }
    }
}

impl std::error::Error for ParseError {}

fn error<T>(line: usize, message: impl Into<String>) -> Result<T, ParseError> {
    Err(ParseError { line, message: message.into() })
}

struct Raw {
    line: usize,
    text: String,
}

impl Raw {
    fn new(line: usize, text: &str) -> Self {
        Self { line, text: text.to_owned() }
    }

    fn append(&mut self, text: &str) {
        self.text.push(' ');
        self.text.push_str(text);
    }
}

struct Draft {
    line: usize,
    version: Version,
    date: Date,
    summary: Option<Raw>,
    bullets: Vec<Raw>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Open {
    Nothing,
    Summary,
    Bullet,
}

impl Changelog {
    pub fn parse(source: &str) -> Result<Self, ParseError> {
        let mut drafts: Vec<Draft> = Vec::new();
        let mut open = Open::Nothing;

        for (index, line) in source.lines().enumerate() {
            let number = index + 1;
            let line = line.trim_end();
            if line.contains('\t') {
                return error(number, "tabs are not supported");
            }
            if line.is_empty() {
                open = Open::Nothing;
                continue;
            }
            if number == 1 && drafts.is_empty() && line.strip_prefix("# ").is_some_and(|title| !title.trim().is_empty()) {
                continue;
            }
            if let Some(heading) = line.strip_prefix("## ") {
                drafts.push(parse_heading(number, heading)?);
                open = Open::Nothing;
                continue;
            }
            let Some(draft) = drafts.last_mut() else {
                return error(number, "text before the first version heading");
            };
            if let Some(text) = line.strip_prefix("- ") {
                if text.trim().is_empty() {
                    return error(number, "an empty bullet");
                }
                draft.bullets.push(Raw::new(number, text.trim()));
                open = Open::Bullet;
            } else if line.starts_with("  ") {
                let text = line.trim_start();
                if open != Open::Bullet {
                    return error(number, "indented text must continue a bullet");
                }
                if text.starts_with("- ") {
                    return error(number, "bullets cannot be nested");
                }
                if let Some(bullet) = draft.bullets.last_mut() {
                    bullet.append(text);
                }
            } else if starts_other_block(line) {
                return error(number, "not in the subset: only version headings, a summary and \"- \" bullets");
            } else if open == Open::Bullet {
                return error(number, "a line after a bullet must be indented two spaces to continue it");
            } else if !draft.bullets.is_empty() {
                return error(number, "the summary comes before the bullets");
            } else if open == Open::Summary {
                if let Some(summary) = draft.summary.as_mut() {
                    summary.append(line);
                }
            } else if draft.summary.is_some() {
                return error(number, "only one summary paragraph per version");
            } else {
                draft.summary = Some(Raw::new(number, line));
                open = Open::Summary;
            }
        }

        let mut releases: Vec<Release> = Vec::with_capacity(drafts.len());
        for draft in drafts {
            let Some(summary) = draft.summary else {
                return error(draft.line, format!("version {} has no summary", draft.version));
            };
            if releases.last().is_some_and(|newer| newer.version <= draft.version) {
                return error(draft.line, format!("version {} must come after every newer version, once", draft.version));
            }
            releases.push(Release {
                version: draft.version,
                date: draft.date,
                summary: parse_inline(&summary)?,
                bullets: draft.bullets.iter().map(parse_inline).collect::<Result<_, _>>()?,
            });
        }
        if releases.is_empty() {
            return error(0, "no version headings");
        }
        Ok(Self { releases })
    }
}

fn parse_heading(line: usize, heading: &str) -> Result<Draft, ParseError> {
    let shape = "a version heading is \"## [x.y.z] - YYYY-MM-DD\"";
    let Some((version, date)) = heading.strip_prefix('[').and_then(|rest| rest.split_once("] - ")) else {
        return error(line, shape);
    };
    let Ok(version) = version.parse::<Version>() else {
        return error(line, format!("{version:?} is not a version like 1.2.3"));
    };
    let Some(date) = Date::parse(date) else {
        return error(line, format!("{date:?} is not a date like 2026-10-21"));
    };
    Ok(Draft { line, version, date, summary: None, bullets: Vec::new() })
}

fn starts_other_block(line: &str) -> bool {
    let first = line.as_bytes()[0];
    let numbered = line.trim_start_matches(|c: char| c.is_ascii_digit());
    matches!(first, b'#' | b'>' | b'*' | b'+' | b'-' | b'|' | b'=')
        || (numbered.len() < line.len() && (numbered.starts_with(". ") || numbered.starts_with(") ")))
}

fn parse_inline(raw: &Raw) -> Result<Text, ParseError> {
    let text = raw.text.as_str();
    if let Some(bad) = text.chars().find(|c| matches!(c, '`' | '_' | '<' | '>' | '[' | ']' | '\\')) {
        return error(raw.line, format!("{bad:?} is not supported: only **bold** is"));
    }
    if text.contains("://") {
        return error(raw.line, "links are not supported");
    }
    let parts: Vec<&str> = text.split("**").collect();
    if parts.len() % 2 == 0 {
        return error(raw.line, "a ** is never closed");
    }
    let mut spans = Text::new();
    for (index, part) in parts.into_iter().enumerate() {
        let bold = index % 2 == 1;
        if part.contains('*') {
            return error(raw.line, "a single * is not supported: only **bold** is");
        }
        if part.is_empty() {
            if bold {
                return error(raw.line, "an empty **bold**");
            }
            continue;
        }
        spans.push(Span { text: part.to_owned(), bold });
    }
    Ok(spans)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rejected(source: &str) -> ParseError {
        Changelog::parse(source).expect_err(source)
    }

    fn with_release(body: &str) -> String {
        format!("## [1.0.0] - 2026-10-09\n\nSummary.\n\n{body}")
    }

    #[test]
    fn a_file_in_the_subset_parses_into_releases_newest_first() {
        let changelog = Changelog::parse(
            "# Changelog\n\n\
             ## [1.1.0] - 2026-10-21\n\nTwo **new** things.\n\n\
             - **First**: it works.\n  It continues here.\n- Second.\n\n\
             ## [1.0.0] - 2026-10-09\n\nThe first.\nOn two lines.\n",
        )
        .unwrap();
        assert_eq!(changelog.releases.len(), 2);
        let newest = &changelog.releases[0];
        assert_eq!(newest.version, Version::new(1, 1, 0));
        assert_eq!(newest.date, Date { year: 2026, month: 10, day: 21 });
        assert_eq!(
            newest.summary,
            vec![
                Span { text: "Two ".into(), bold: false },
                Span { text: "new".into(), bold: true },
                Span { text: " things.".into(), bold: false },
            ]
        );
        assert_eq!(newest.bullets.len(), 2);
        assert_eq!(
            newest.bullets[0],
            vec![
                Span { text: "First".into(), bold: true },
                Span { text: ": it works. It continues here.".into(), bold: false },
            ]
        );
        assert_eq!(changelog.releases[1].summary, vec![Span { text: "The first. On two lines.".into(), bold: false }]);
        assert!(changelog.releases[1].bullets.is_empty());
    }

    #[test]
    fn windows_line_endings_parse_the_same() {
        let lf = "# Changelog\n\n## [1.0.0] - 2026-10-09\n\nSummary.\n\n- One.\n  More.\n";
        assert_eq!(Changelog::parse(&lf.replace('\n', "\r\n")), Changelog::parse(lf));
    }

    #[test]
    fn a_bold_span_may_cover_a_whole_bullet_or_continue_over_a_line() {
        let changelog = Changelog::parse(&with_release("- **All of it**\n- A **bold\n  phrase** here.\n")).unwrap();
        assert_eq!(changelog.releases[0].bullets[0], vec![Span { text: "All of it".into(), bold: true }]);
        assert_eq!(changelog.releases[0].bullets[1][1], Span { text: "bold phrase".into(), bold: true });
    }

    #[test]
    fn syntax_outside_the_subset_is_rejected_with_its_line() {
        let cases: [(&str, usize, &str); 22] = [
            ("Text first.\n", 1, "text before"),
            ("## [1.0] - 2026-10-09\n\nS.\n", 1, "not a version"),
            ("## [1.0.0]\n\nS.\n", 1, "version heading"),
            ("## [1.0.0] - 2026-02-30\n\nS.\n", 1, "not a date"),
            ("## [1.0.0] - 2026-10-9\n\nS.\n", 1, "not a date"),
            ("## Unreleased\n\nS.\n", 1, "version heading"),
            ("## [1.0.0] - 2026-10-09\n\nS.\n\n### Added\n", 5, "not in the subset"),
            ("## [1.0.0] - 2026-10-09\n\nS.\n\n> quote\n", 5, "not in the subset"),
            ("## [1.0.0] - 2026-10-09\n\nS.\n\n* star\n", 5, "not in the subset"),
            ("## [1.0.0] - 2026-10-09\n\nS.\n\n1. numbered\n", 5, "not in the subset"),
            ("## [1.0.0] - 2026-10-09\n\nS.\n\n| a | b |\n", 5, "not in the subset"),
            ("## [1.0.0] - 2026-10-09\n\nS.\n\n- a\n  - nested\n", 6, "nested"),
            ("## [1.0.0] - 2026-10-09\n\nS.\n\n- a\nlazy\n", 6, "indented"),
            ("## [1.0.0] - 2026-10-09\n\nS.\n\n  indented alone\n", 5, "continue a bullet"),
            ("## [1.0.0] - 2026-10-09\n\nS.\n\n- a\n\nsummary after\n", 7, "before the bullets"),
            ("## [1.0.0] - 2026-10-09\n\nS.\n\nSecond paragraph.\n", 5, "one summary"),
            ("## [1.0.0] - 2026-10-09\n\nS `code`.\n", 3, "`"),
            ("## [1.0.0] - 2026-10-09\n\nS [link](x).\n", 3, "'['"),
            ("## [1.0.0] - 2026-10-09\n\nS https://example.com.\n", 3, "links"),
            ("## [1.0.0] - 2026-10-09\n\nS *italic* and **bold**.\n", 3, "single *"),
            ("## [1.0.0] - 2026-10-09\n\nS **open.\n", 3, "never closed"),
            ("## [1.0.0] - 2026-10-09\n\n\tS.\n", 3, "tabs"),
        ];
        for (source, line, fragment) in cases {
            let problem = rejected(source);
            assert_eq!(problem.line, line, "{source:?}: {problem}");
            assert!(problem.message.contains(fragment), "{source:?}: {problem}");
        }
    }

    #[test]
    fn more_syntax_is_rejected() {
        for body in ["- a_b\n", "- <b>x</b>\n", "- x \\* y\n", "- an empty ****\n", "- \n", "-no space\n", "- a\n---\n"] {
            assert!(Changelog::parse(&with_release(body)).is_err(), "{body:?}");
        }
        assert!(Changelog::parse(&with_release("- ![img](x.png)\n")).is_err());
    }

    #[test]
    fn a_version_needs_a_summary() {
        let problem = rejected("## [1.0.0] - 2026-10-09\n\n- a\n");
        assert!(problem.message.contains("no summary"), "{problem}");
    }

    #[test]
    fn versions_run_newest_first_and_never_repeat() {
        let older_first = "## [1.0.0] - 2026-10-09\n\nS.\n\n## [1.1.0] - 2026-10-10\n\nS.\n";
        assert!(rejected(older_first).message.contains("newer"));
        let repeated = "## [1.0.0] - 2026-10-09\n\nS.\n\n## [1.0.0] - 2026-10-09\n\nS.\n";
        assert!(rejected(repeated).message.contains("newer"));
    }

    #[test]
    fn a_file_without_versions_is_rejected() {
        assert_eq!(rejected("# Changelog\n").line, 0);
        assert_eq!(rejected("").line, 0);
    }

    #[test]
    fn dates_follow_the_calendar() {
        assert!(Date::parse("2028-02-29").is_some());
        assert!(Date::parse("2026-02-29").is_none());
        assert!(Date::parse("2100-02-29").is_none());
        assert!(Date::parse("2026-13-01").is_none());
        assert!(Date::parse("2026-04-31").is_none());
        assert!(Date::parse("2026-00-10").is_none());
        assert_eq!(Date { year: 2026, month: 3, day: 7 }.to_string(), "2026-03-07");
    }
}
