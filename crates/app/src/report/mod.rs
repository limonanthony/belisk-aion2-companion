//! The end-of-fight report of a boss fight, as approved in
//! docs/gpui-mockups/approved/report.html: one table with every figure, the
//! deaths, and a player's details inside the same window.

mod data;
mod model;
mod view;
#[cfg(test)]
mod tests;

use std::sync::Arc;

use meter_core::history::HistoryRecord;

#[cfg(feature = "probes")]
pub use data::Column;
pub use model::ReportModel;
#[cfg(feature = "probes")]
pub use view::{Pointed, Tab};
pub use view::ReportView;

/// A report asked for by the summary card or a History record; the windows
/// layer opens it.
#[derive(Clone, Debug)]
pub enum ReportRequest {
    /// The boss fight the meter has on `target_id`: still on the meter, or
    /// else its newest saved record.
    OnMeter { target_id: u32 },
    Saved(Arc<HistoryRecord>),
    /// A report already built, such as a wiped attempt's: the boss entity may
    /// have been pulled again since.
    Held(Arc<meter_core::report::fight_report::FightReport>),
}
