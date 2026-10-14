//! Event reminders: a heads-up shortly before a recurring timed event of the
//! game starts (Shugo Festival, Dimensional Invasion, Rift portals ...), and
//! optionally a second card, the start alert, when it starts.

mod events;
mod planner;
mod schedule;
mod service;
mod settings;
mod sound;
mod view;

pub use events::{EventId, Rule, EVENTS};
#[cfg(any(test, feature = "probes"))]
pub use planner::CardKind;
pub use schedule::upcoming;
pub use service::{global, install, log_to, ShowReminder};
pub use settings::{ReminderSettings, LEAD_MINUTES_OPTIONS};
pub use view::{card_width, expected_size, Countdown, MeterLink, ReminderView, TimerRing, FADE, GAP};
