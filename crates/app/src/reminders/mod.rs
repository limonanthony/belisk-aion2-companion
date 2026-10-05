//! Event reminders: a heads-up shortly before a recurring timed event of the
//! game starts (Shugo Festival, Dimensional Invasion, Rift portals ...).

mod clock;
mod events;
mod planner;
mod schedule;
mod service;
mod settings;
mod sound;
mod view;

pub use events::{EventId, Rule, EVENTS};
#[cfg(feature = "probes")]
pub use schedule::ClockTime;
pub use service::{global, install, ShowReminder};
pub use settings::{ReminderSettings, LEAD_MINUTES_OPTIONS};
pub use view::{expected_size, MeterLink, ReminderView, GAP};
