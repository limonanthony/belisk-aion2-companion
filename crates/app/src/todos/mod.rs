//! Daily and weekly to-dos, and the game's resources: what resets and is lost
//! if not done, per character. `catalogue` lists the built-in rows and the
//! tickets they follow, `data` holds what is stored and every change to it,
//! `tickets` the values the game said, `resets` and `amount` do the clock
//! arithmetic (the PC's local time, daylight saving included), `list` builds
//! what a window shows, `copy` holds the words computed from counts, `store`
//! reads and writes `todos.json`, `service` is the `Todos` entity the windows
//! observe, `settings` the two fields Settings → To-dos changes, `view` the
//! to-do window and the Resources window.

mod amount;
mod catalogue;
pub mod copy;
mod data;
mod list;
mod resets;
mod service;
mod settings;
mod store;
mod tickets;
mod view;

pub use catalogue::{Section, ODYLE, SHUGO_KEYS};
pub use data::{
    Cadence, CharacterId, HiddenSummary, NewOwnTodo, OwnKind, PoolAbovePlainCap, ProgressChange, RemovedOwnTodo, RowId,
    MAX_OWN_NAME_CHARS, OWN_COUNT_RANGE,
};
pub use list::{AmountView, FigureTip, RowKind, RowView, SectionView, Source, SourceLine, TodoList};
pub use service::{follow_player, global, identified_by, install, record_tickets, Identified, Todos};
pub use settings::TodoSettings;
pub use view::{Contents, TodoView};
#[cfg(feature = "probes")]
pub use service::PlayerRef;
#[cfg(feature = "probes")]
pub use view::Typing;

/// A built-in row's name, as the windows show it.
pub fn row_label(id: &str) -> Option<&'static str> {
    catalogue::builtin(id).map(|builtin| builtin.label)
}
