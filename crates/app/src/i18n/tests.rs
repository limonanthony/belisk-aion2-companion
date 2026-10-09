use std::collections::BTreeMap;

use fluent_syntax::ast::{Entry, Expression, InlineExpression, Pattern, PatternElement};
use fluent_syntax::parser;

use super::loader::Translations;
use super::{apply, fl, language, locale, Language, LanguageChoice};

const FILE: &str = "belisk.ftl";

/// For every message (and attribute) the arguments its text reads.
type Signatures = BTreeMap<String, Vec<String>>;

fn source(language: &str) -> String {
    let path = format!("{language}/{FILE}");
    let file = Translations::get(&path).unwrap_or_else(|| panic!("{path} is not embedded"));
    String::from_utf8(file.data.into_owned()).unwrap_or_else(|error| panic!("{path} is not UTF-8: {error}"))
}

fn signatures(language: &str) -> Signatures {
    let text = source(language);
    let resource = parser::parse(text.as_str()).unwrap_or_else(|(_, errors)| {
        panic!("{language}/{FILE} does not parse:\n{}", errors.iter().map(|error| format!("{error:?}")).collect::<Vec<_>>().join("\n"))
    });
    let mut found = Signatures::new();
    for entry in &resource.body {
        let (id, value, attributes) = match entry {
            Entry::Message(message) => (message.id.name, message.value.as_ref(), &message.attributes),
            Entry::Term(term) => (term.id.name, Some(&term.value), &term.attributes),
            _ => continue,
        };
        let id = if matches!(entry, Entry::Term(_)) { format!("-{id}") } else { id.to_string() };
        let mut arguments = Vec::new();
        if let Some(value) = value {
            pattern_arguments(value, &mut arguments);
        }
        assert!(found.insert(id.clone(), normalised(arguments)).is_none(), "{language}: {id} is defined twice");
        for attribute in attributes {
            let mut arguments = Vec::new();
            pattern_arguments(&attribute.value, &mut arguments);
            found.insert(format!("{id}.{}", attribute.id.name), normalised(arguments));
        }
    }
    found
}

fn normalised(mut arguments: Vec<String>) -> Vec<String> {
    arguments.sort();
    arguments.dedup();
    arguments
}

fn pattern_arguments(pattern: &Pattern<&str>, out: &mut Vec<String>) {
    for element in &pattern.elements {
        if let PatternElement::Placeable { expression } = element {
            expression_arguments(expression, out);
        }
    }
}

fn expression_arguments(expression: &Expression<&str>, out: &mut Vec<String>) {
    match expression {
        Expression::Inline(inline) => inline_arguments(inline, out),
        Expression::Select { selector, variants } => {
            inline_arguments(selector, out);
            for variant in variants {
                pattern_arguments(&variant.value, out);
            }
        }
    }
}

fn inline_arguments(inline: &InlineExpression<&str>, out: &mut Vec<String>) {
    match inline {
        InlineExpression::VariableReference { id } => out.push(format!("${}", id.name)),
        InlineExpression::MessageReference { id, .. } => out.push(format!("message {}", id.name)),
        InlineExpression::TermReference { id, .. } => out.push(format!("term -{}", id.name)),
        InlineExpression::Placeable { expression } => expression_arguments(expression, out),
        _ => {}
    }
}

#[test]
fn every_translation_file_parses() {
    for language in ["en", "fr"] {
        assert!(!signatures(language).is_empty(), "{language}");
    }
}

#[test]
fn english_and_french_have_the_same_messages() {
    let (english, french) = (signatures("en"), signatures("fr"));
    let only_english: Vec<_> = english.keys().filter(|key| !french.contains_key(*key)).collect();
    let only_french: Vec<_> = french.keys().filter(|key| !english.contains_key(*key)).collect();
    assert!(only_english.is_empty(), "missing from fr: {only_english:?}");
    assert!(only_french.is_empty(), "missing from en: {only_french:?}");
}

#[test]
fn english_and_french_read_the_same_arguments() {
    let (english, french) = (signatures("en"), signatures("fr"));
    let differing: Vec<String> = english
        .iter()
        .filter(|(key, arguments)| french.get(*key).is_some_and(|other| other != *arguments))
        .map(|(key, arguments)| format!("{key}: en {arguments:?}, fr {:?}", french[key]))
        .collect();
    assert!(differing.is_empty(), "arguments differ:\n{}", differing.join("\n"));
}

#[test]
fn no_message_is_empty_in_either_language() {
    for language in ["en", "fr"] {
        let text = source(language);
        let resource = parser::parse(text.as_str()).unwrap();
        for entry in &resource.body {
            if let Entry::Message(message) = entry {
                assert!(
                    message.value.as_ref().is_some_and(|value| !value.elements.is_empty()) || !message.attributes.is_empty(),
                    "{language}: {} has no text",
                    message.id.name
                );
            }
        }
    }
}

#[test]
fn arguments_are_not_wrapped_in_isolation_marks() {
    for language in Language::ALL {
        super::set_language(language);
        let text = fl!("duration-minutes", minutes = 5);
        assert!(text.chars().all(|letter| !('\u{2066}'..='\u{2069}').contains(&letter)), "{language:?}: {text:?}");
    }
}

#[test]
fn plural_forms_follow_each_languages_rules() {
    super::set_language(Language::English);
    assert_eq!(fl!("history-records", count = 0), "0 records");
    assert_eq!(fl!("history-records", count = 1), "1 record");
    assert_eq!(fl!("history-records", count = 2), "2 records");
    super::set_language(Language::French);
    assert_eq!(fl!("history-records", count = 0), "0 enregistrement", "French 0 is singular");
    assert_eq!(fl!("history-records", count = 1), "1 enregistrement");
    assert_eq!(fl!("history-records", count = 2), "2 enregistrements");
}

#[test]
fn french_counts_in_the_windows_agree_with_their_number() {
    super::set_language(Language::French);
    assert_eq!(fl!("common-players-count", count = 1), "1 joueur");
    assert_eq!(fl!("common-players-count", count = 4), "4 joueurs");
    assert_eq!(fl!("detail-buffs-up-now", count = 0), "0 actif");
    assert_eq!(fl!("detail-buffs-up-now", count = 3), "3 actifs");
    assert_eq!(fl!("history-delete-all-confirm", count = 1), "Supprimer 1 enregistrement\u{a0}?");
    assert_eq!(fl!("history-delete-all-confirm", count = 12), "Supprimer les 12 enregistrements\u{a0}?");
}

#[gpui::test]
fn applying_a_choice_switches_the_language_and_notifies_once(cx: &mut gpui::TestAppContext) {
    let notified = std::rc::Rc::new(std::cell::Cell::new(0));
    cx.update(|cx| {
        let locale = locale(cx);
        let counter = notified.clone();
        cx.observe(&locale, move |_, _| counter.set(counter.get() + 1)).detach();
        assert_eq!(language(), Language::English);
    });

    cx.update(|cx| apply(LanguageChoice::French, cx));
    assert_eq!((language(), notified.get()), (Language::French, 1));
    assert_eq!(fl!("duration-minutes", minutes = 5), "5 min");
    assert_eq!(fl!("weekday-short-tue"), "mar.");

    cx.update(|cx| apply(LanguageChoice::French, cx));
    assert_eq!(notified.get(), 1, "the same language again changes nothing");

    cx.update(|cx| apply(LanguageChoice::English, cx));
    assert_eq!((language(), notified.get()), (Language::English, 2));
    assert_eq!(fl!("weekday-short-tue"), "Tue");
}
