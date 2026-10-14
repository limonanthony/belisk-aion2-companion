//! Settings: what the player chose, stored in `settings.json`, the live
//! application of every change, and the window that changes them. `model`
//! holds the values and the entity everything observes, `effects` carries a
//! change to the overlay and the core, `characters` keeps the characters the
//! game named as the player, `runtime` and `files` feed the read-only parts of
//! the page, `view` and `widgets` draw it.

mod characters;
mod effects;
mod files;
mod model;
mod runtime;
mod store;
mod view;
mod widgets;

pub use effects::connect;
pub use model::{
    global, install, leave_window_at, mark_version_seen, remember_characters, toggle_layout, AppSettings, DetailPlacement, ScreenPoint,
    Settings, Spot,
};
pub use runtime::RuntimeModel;
pub use view::{bring_forward, open_window, Page, Section, SettingsView};
pub use widgets::{reveal_marker, Reveal};
#[cfg(feature = "probes")]
pub use files::StorageSummary;
#[cfg(feature = "probes")]
pub use runtime::{
    BestRow, CaptureCheck, CaptureStatus, MeterTile, RecordingItem, RecordingOrigin, RecordingProgress, ReplayProgress,
    StorageState,
};
