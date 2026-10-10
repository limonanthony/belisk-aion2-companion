use std::path::Path;

use crate::Result;

const ENGLISH_FILE: &str = "CHANGELOG.md";
const FRENCH_FILE: &str = "CHANGELOG.fr.md";
const DEFAULT_OUT: &str = "target/release-assets/notes.md";

pub fn run(root: &Path, version: &str, out: Option<&Path>) -> Result<()> {
    let read = |name: &str| {
        std::fs::read_to_string(root.join(name)).map_err(|error| format!("could not read {name}: {error}"))
    };
    let notes = build(version, &read(ENGLISH_FILE)?, &read(FRENCH_FILE)?)?;
    let out = out.map_or_else(|| root.join(DEFAULT_OUT), Path::to_path_buf);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    }
    std::fs::write(&out, &notes).map_err(|error| format!("could not write {}: {error}", out.display()))?;
    print!("{notes}");
    Ok(())
}

fn build(version: &str, english: &str, french: &str) -> Result<String> {
    let english = section_for(ENGLISH_FILE, version, english)?;
    let french = section_for(FRENCH_FILE, version, french)?;
    Ok(format!("## English\n\n{english}\n\n## Français\n\n{french}\n"))
}

fn section_for(file: &str, version: &str, changelog: &str) -> Result<String> {
    let text = changelog.replace('\r', "");
    if text.lines().any(|line| line.to_ascii_lowercase().starts_with("## [unreleased]")) {
        return Err(format!("{file} still has an [Unreleased] section: rename it to the version being released.").into());
    }
    let heading = text.lines().find(|line| line.starts_with("## ")).unwrap_or("");
    if !heading.starts_with(&format!("## [{version}] - ")) {
        return Err(format!("The top section of {file} must be '## [{version}] - <date>' (it is '{heading}').").into());
    }
    let section = top_section_body(&text);
    if section.is_empty() {
        return Err(format!("The {version} section of {file} is empty.").into());
    }
    Ok(section)
}

/// The lines under the first `## ` heading up to the next one, without the blank lines at either end.
fn top_section_body(text: &str) -> String {
    let mut lines = text.lines().skip_while(|line| !line.starts_with("## ")).skip(1);
    let mut body: Vec<&str> = lines.by_ref().take_while(|line| !line.starts_with("## ")).collect();
    let is_blank = |line: &&str| line.trim_matches([' ', '\t']).is_empty();
    while body.last().is_some_and(is_blank) {
        body.pop();
    }
    let first_content = body.iter().position(|line| !is_blank(line)).unwrap_or(body.len());
    body[first_content..].join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    const ENGLISH: &str = "# Changelog\n\n## [1.0.2] - 2026-10-20\n\nA fix.\n\n- **Meter**: one.\n- Two\n  continued.\n\n## [1.0.1] - 2026-10-10\n\nOlder.\n\n- Old.\n";
    const FRENCH: &str = "# Journal des modifications\n\n## [1.0.2] - 2026-10-20\n\nUne correction.\n\n- **Compteur** : un.\n\n## [1.0.1] - 2026-10-10\n\nAncien.\n\n- Ancien.\n";

    fn failure(result: Result<String>) -> String {
        result.unwrap_err().to_string()
    }

    #[test]
    fn writes_both_sections_under_their_language_headings() {
        assert_eq!(
            build("1.0.2", ENGLISH, FRENCH).unwrap(),
            "## English\n\nA fix.\n\n- **Meter**: one.\n- Two\n  continued.\n\n## Français\n\nUne correction.\n\n- **Compteur** : un.\n"
        );
    }

    #[test]
    fn leaves_the_older_sections_out() {
        let notes = build("1.0.2", ENGLISH, FRENCH).unwrap();
        assert!(!notes.contains("Old") && !notes.contains("Ancien"));
    }

    #[test]
    fn crlf_input_gives_the_same_notes() {
        let crlf = |text: &str| text.replace('\n', "\r\n");
        assert_eq!(
            build("1.0.2", &crlf(ENGLISH), &crlf(FRENCH)).unwrap(),
            build("1.0.2", ENGLISH, FRENCH).unwrap()
        );
    }

    #[test]
    fn keeps_single_blank_lines_inside_and_drops_the_ones_at_the_ends() {
        let english = "## [1.0.2] - 2026-10-20\n\n\n  \nFirst.\n\n\nSecond.\n \t\n\n## [1.0.1] - 2026-10-10\n\nOld.\n";
        assert_eq!(section_for("CHANGELOG.md", "1.0.2", english).unwrap(), "First.\n\n\nSecond.");
    }

    #[test]
    fn a_heading_with_three_hashes_does_not_end_the_section() {
        let english = "## [1.0.2] - 2026-10-20\n\nSummary.\n\n### Added\n\n- One.\n";
        assert_eq!(section_for("CHANGELOG.md", "1.0.2", english).unwrap(), "Summary.\n\n### Added\n\n- One.");
    }

    #[test]
    fn refuses_an_unreleased_section_anywhere_in_either_file() {
        let english = format!("## [Unreleased]\n\nNext.\n\n{}", ENGLISH.trim_start_matches("# Changelog\n\n"));
        let message = failure(build("1.0.2", &english, FRENCH));
        assert_eq!(message, "CHANGELOG.md still has an [Unreleased] section: rename it to the version being released.");

        let french = format!("{FRENCH}\n## [unreleased]\n\nPlus tard.\n");
        assert!(failure(build("1.0.2", ENGLISH, &french)).starts_with("CHANGELOG.fr.md still has an [Unreleased]"));
    }

    #[test]
    fn refuses_a_top_section_for_another_version() {
        let message = failure(build("1.0.3", ENGLISH, FRENCH));
        assert_eq!(
            message,
            "The top section of CHANGELOG.md must be '## [1.0.3] - <date>' (it is '## [1.0.2] - 2026-10-20')."
        );
        let french_behind = FRENCH.replace("1.0.2", "1.0.1");
        assert!(failure(build("1.0.2", ENGLISH, &french_behind)).starts_with("The top section of CHANGELOG.fr.md"));
    }

    #[test]
    fn refuses_a_changelog_without_a_section() {
        let message = failure(build("1.0.2", "# Changelog\n", FRENCH));
        assert_eq!(message, "The top section of CHANGELOG.md must be '## [1.0.2] - <date>' (it is '').");
    }

    #[test]
    fn refuses_an_empty_section() {
        let english = "## [1.0.2] - 2026-10-20\n\n \n## [1.0.1] - 2026-10-10\n\nOld.\n";
        assert_eq!(failure(build("1.0.2", english, FRENCH)), "The 1.0.2 section of CHANGELOG.md is empty.");
    }

    #[test]
    fn a_version_that_is_a_prefix_of_another_is_not_a_match() {
        let english = ENGLISH.replace("1.0.2", "1.0.20");
        assert!(build("1.0.2", &english, FRENCH).is_err());
    }

    #[test]
    fn the_real_changelogs_make_notes_for_their_top_version() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).ancestors().nth(2).unwrap();
        let english = std::fs::read_to_string(root.join(ENGLISH_FILE)).unwrap();
        let french = std::fs::read_to_string(root.join(FRENCH_FILE)).unwrap();
        let heading = english.lines().find(|line| line.starts_with("## ")).unwrap();
        let version = heading.trim_start_matches("## [").split(']').next().unwrap();

        let notes = build(version, &english, &french).unwrap();

        assert!(notes.starts_with("## English\n\n"));
        assert!(notes.contains("\n\n## Français\n\n"));
        assert!(notes.ends_with("\n") && !notes.ends_with("\n\n"));
        assert!(!notes.contains('\r'));
    }
}
