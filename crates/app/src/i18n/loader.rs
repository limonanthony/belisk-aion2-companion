use i18n_embed::fluent::{fluent_language_loader, FluentLanguageLoader};
use i18n_embed::LanguageLoader as _;
use rust_embed::RustEmbed;

use super::language::Language;

#[derive(RustEmbed)]
#[folder = "i18n/"]
pub(super) struct Translations;

fn build() -> FluentLanguageLoader {
    let loader = fluent_language_loader!();
    select(&loader, Language::English);
    loader
}

fn select(loader: &FluentLanguageLoader, language: Language) {
    loader
        .load_languages(&Translations, &[language.identifier()])
        .unwrap_or_else(|error| panic!("the bundled {language:?} translations do not load: {error}"));
    // Isolation marks around arguments show as boxes in GPUI text.
    loader.set_use_isolating(false);
}

#[cfg(not(test))]
mod global {
    use std::sync::{Arc, LazyLock};

    use super::{build, FluentLanguageLoader};

    static LOADER: LazyLock<Arc<FluentLanguageLoader>> = LazyLock::new(|| Arc::new(build()));

    pub fn get() -> Arc<FluentLanguageLoader> {
        Arc::clone(&LOADER)
    }
}

/// Tests run in parallel threads: each gets its own loader, starting in
/// English, so one test's language switch cannot reach another.
#[cfg(test)]
mod global {
    use std::sync::Arc;

    use super::{build, FluentLanguageLoader};

    thread_local! {
        static LOADER: Arc<FluentLanguageLoader> = Arc::new(build());
    }

    pub fn get() -> Arc<FluentLanguageLoader> {
        LOADER.with(Arc::clone)
    }
}

/// The loader `fl!` reads. Not meant to be called from views.
pub fn loader() -> std::sync::Arc<FluentLanguageLoader> {
    global::get()
}

pub fn set_language(language: Language) {
    select(&loader(), language);
}

pub fn language() -> Language {
    Language::ALL
        .into_iter()
        .find(|language| language.identifier() == loader().current_language())
        .unwrap_or_default()
}
