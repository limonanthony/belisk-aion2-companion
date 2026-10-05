//! Settings: what the player chose, stored in `settings.json`, the live
//! application of every change, and the window that changes them. `model`
//! holds the values and the entity everything observes, `effects` carries a
//! change to the overlay and the core, `runtime` and `files` feed the
//! read-only parts of the page, `view` and `widgets` draw it.

mod effects;
mod files;
mod model;
mod runtime;
mod store;
mod view;
mod widgets;

pub use effects::connect;
pub use model::{global, install, toggle_layout, AppSettings, DetailPlacement, Settings};
pub use runtime::RuntimeModel;
pub use view::{bring_forward, open_window, SettingsView};
#[cfg(feature = "probes")]
pub use files::StorageSummary;
#[cfg(feature = "probes")]
pub use runtime::{
    BestRow, CaptureCheck, CaptureStatus, MeterTile, RecordingItem, RecordingProgress, ReplayProgress, StorageState,
};
#[cfg(feature = "probes")]
pub use view::Section;
