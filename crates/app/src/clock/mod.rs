//! The PC's local clock: the current time, the time zone read from Windows, and
//! the wall-clock arithmetic (daylight saving included) that reminders and
//! to-dos share.

mod local;
#[cfg(test)]
pub mod testing;
mod wall;

pub use local::{now_ms, LocalZone};
pub use wall::{clock_time, instant_of_wall_minute, wall_minute, ClockTime, Zone, DAY_MS, MINUTE_MS};
