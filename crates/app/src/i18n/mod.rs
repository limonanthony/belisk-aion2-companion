//! The app's one translation setup: Fluent files in `crates/app/i18n/<lang>/`
//! embedded in the exe, English as the fallback, the language chosen in
//! Settings → General (or Windows' own) and switched live. Every user-facing
//! string goes through [`fl!`]; game names (bosses, skills) are looked up by id
//! in the language in force, English where a name has no translation.
//!
//! The language lives in the [`Locale`] entity: every window redraws when it
//! changes, and a model that keeps text it built earlier observes it to
//! rebuild that text.

mod language;
mod loader;
#[cfg(test)]
mod tests;
mod words;

use gpui::{App, AppContext as _, Entity, Global};

pub use language::{Language, LanguageChoice};
pub use words::{day_stamp, duration_words, join_and, month_short, weekday_short};

#[cfg(test)]
pub use loader::set_language;

/// A translated message: `fl!("message-id")` or `fl!("message-id", name = value)`.
/// The id and the argument names are checked against `en/aion_2_dps_meter.ftl`
/// at compile time. Needs no import beyond `use crate::i18n::fl;`.
macro_rules! fl {
    ($message_id:literal $(, $($arguments:tt)*)?) => {
        ::i18n_embed_fl::fl!(crate::i18n::loader(), $message_id $(, $($arguments)*)?)
    };
}
pub(crate) use fl;
pub(crate) use loader::loader;

/// Notifies when the language changes.
pub struct Locale;

struct LocaleHandle(Entity<Locale>);

impl Global for LocaleHandle {}

/// The [`Locale`] entity, created on first use.
pub fn locale(cx: &mut App) -> Entity<Locale> {
    if let Some(handle) = cx.try_global::<LocaleHandle>() {
        return handle.0.clone();
    }
    let locale = cx.new(|_| Locale);
    cx.set_global(LocaleHandle(locale.clone()));
    locale
}

/// The language now in force.
pub fn language() -> Language {
    loader::language()
}

/// Puts `choice` in force at once: every [`fl!`] from now on answers in it,
/// the [`Locale`] entity notifies and every window redraws. Does nothing when
/// the language is the one in force.
pub fn apply(choice: LanguageChoice, cx: &mut App) {
    let language = choice.resolve();
    let locale = locale(cx);
    if loader::language() == language {
        return;
    }
    loader::set_language(language);
    locale.update(cx, |_, cx| cx.notify());
    cx.refresh_windows();
}
