//! Daily and weekly to-dos: what resets and is lost if not done, per character.
//! `catalogue` lists the built-in rows, `data` holds what is stored and every
//! change to it, `resets` and `amount` do the clock arithmetic (the PC's local
//! time, daylight saving included), `list` builds what a window shows, `copy`
//! holds the words computed from counts, `store` reads and writes `todos.json`,
//! `service` is the `Todos` entity the windows observe, `settings` the two
//! fields Settings → To-dos changes, `view` the to-do window.

mod amount;
mod catalogue;
pub mod copy;
mod data;
mod list;
mod resets;
mod service;
mod settings;
mod store;
mod view;

pub use catalogue::Section;
pub use data::{
    Cadence, CharacterId, HiddenSummary, NewOwnTodo, OwnKind, ProgressChange, RemovedOwnTodo, RowId, MAX_OWN_NAME_CHARS,
    OWN_COUNT_RANGE,
};
pub use list::{AmountView, GroupView, RowKind, RowView, SectionView, TodoList};
pub use service::{follow_player, global, identified_by, install, Identified, Todos};
pub use settings::TodoSettings;
pub use view::TodoView;
#[cfg(feature = "probes")]
pub use service::PlayerRef;
#[cfg(feature = "probes")]
pub use view::HiddenIn;
