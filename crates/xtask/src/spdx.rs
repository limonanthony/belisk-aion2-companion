const GPL3_COMPATIBLE: &[&str] = &[
    "0BSD",
    "bzip2-1.0.6",
    "Apache-2.0",
    "BSD-1-Clause",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "BSL-1.0",
    "CC0-1.0",
    "CDLA-Permissive-2.0",
    "GPL-3.0",
    "GPL-3.0-only",
    "GPL-3.0-or-later",
    "ISC",
    "LGPL-3.0",
    "LGPL-3.0-only",
    "LGPL-3.0-or-later",
    "MIT",
    "MIT-0",
    "MPL-2.0",
    "NCSA",
    "OFL-1.1",
    "Unicode-3.0",
    "Unicode-DFS-2016",
    "Unlicense",
    "Zlib",
];

/// True when the licence expression can be satisfied with licences that may be combined
/// into a GPL-3.0-only work. A WITH exception only narrows its licence, so it is ignored.
/// `/` is the legacy spelling of OR.
pub fn is_gpl3_compatible(expression: &str) -> bool {
    let normalised = expression.replace('/', " OR ").replace('(', " ( ").replace(')', " ) ");
    let mut tokens = normalised.split_whitespace().peekable();
    let verdict = parse_or(&mut tokens);
    verdict && tokens.next().is_none()
}

type Tokens<'a> = std::iter::Peekable<std::str::SplitWhitespace<'a>>;

fn parse_or(tokens: &mut Tokens) -> bool {
    let mut verdict = parse_and(tokens);
    while tokens.next_if_eq(&"OR").is_some() {
        let alternative = parse_and(tokens);
        verdict = verdict || alternative;
    }
    verdict
}

fn parse_and(tokens: &mut Tokens) -> bool {
    let mut verdict = parse_term(tokens);
    while tokens.next_if_eq(&"AND").is_some() {
        let other = parse_term(tokens);
        verdict = verdict && other;
    }
    verdict
}

fn parse_term(tokens: &mut Tokens) -> bool {
    match tokens.next() {
        Some("(") => {
            let verdict = parse_or(tokens);
            tokens.next_if_eq(&")");
            verdict
        }
        Some(identifier) => {
            if tokens.next_if_eq(&"WITH").is_some() {
                tokens.next();
            }
            GPL3_COMPATIBLE.contains(&identifier)
        }
        None => false,
    }
}

#[cfg(test)]
mod tests {
    use super::is_gpl3_compatible;

    #[test]
    fn accepts_permissive_alternatives() {
        assert!(is_gpl3_compatible("MIT OR Apache-2.0"));
        assert!(is_gpl3_compatible("MIT/Apache-2.0"));
        assert!(is_gpl3_compatible("(MIT OR Apache-2.0) AND Unicode-3.0"));
        assert!(is_gpl3_compatible("Apache-2.0 WITH LLVM-exception"));
    }

    #[test]
    fn one_compatible_alternative_is_enough_but_every_and_term_is_needed() {
        assert!(is_gpl3_compatible("GPL-2.0-only OR MIT"));
        assert!(!is_gpl3_compatible("GPL-2.0-only AND MIT"));
    }

    #[test]
    fn rejects_incompatible_and_unknown_licences() {
        assert!(!is_gpl3_compatible("GPL-2.0-only"));
        assert!(!is_gpl3_compatible("AGPL-3.0-only"));
        assert!(!is_gpl3_compatible(""));
    }
}
