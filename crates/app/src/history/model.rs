use std::sync::Arc;
use std::time::Duration;

use aether_core::engine::meter::DpsMeter;
use aether_core::history::HistoryRecord;
use gpui::{Context, Task};

use super::list::HistoryList;

const STATUS_FOR: Duration = Duration::from_secs(4);

/// The history window's state: the list, which record is open, and the line
/// confirming a delete. The records themselves are the meter's.
pub struct HistoryModel {
    meter: Option<Arc<DpsMeter>>,
    pub list: HistoryList,
    /// The record whose players are shown; one at a time.
    pub open: Option<String>,
    status: Option<String>,
    status_timer: Option<Task<()>>,
}

impl HistoryModel {
    pub fn new(meter: Option<Arc<DpsMeter>>) -> Self {
        let mut model = Self { meter, list: HistoryList::default(), open: None, status: None, status_timer: None };
        model.reload();
        model
    }

    pub fn status(&self) -> Option<&str> {
        self.status.as_deref()
    }

    /// Reads the records again, after the meter saved or deleted one.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.reload();
        cx.notify();
    }

    pub fn load(&mut self, records: Vec<HistoryRecord>) {
        self.list.set_records(records);
        self.forget_missing_open_record();
    }

    pub fn toggle_open(&mut self, id: &str, cx: &mut Context<Self>) {
        self.open = if self.open.as_deref() == Some(id) { None } else { Some(id.to_string()) };
        cx.notify();
    }

    pub fn delete(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(meter) = &self.meter {
            meter.delete_history_record(id);
        }
        self.reload();
        cx.notify();
    }

    /// Personal bests are kept: they live in their own store.
    pub fn delete_all(&mut self, cx: &mut Context<Self>) {
        let Some(count) = self.meter.as_ref().map(|meter| meter.delete_all_history()) else {
            return;
        };
        self.reload();
        self.report_deleted(count, cx);
    }

    fn report_deleted(&mut self, count: usize, cx: &mut Context<Self>) {
        let noun = if count == 1 { "record" } else { "records" };
        self.status = Some(format!("Deleted {count} {noun}. Personal bests are kept."));
        self.status_timer = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(STATUS_FOR).await;
            let _ = this.update(cx, |this, cx| {
                this.status = None;
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// What deleting every record shows, for the captures that run without a
    /// meter.
    #[cfg(feature = "probes")]
    pub fn show_deleted_all(&mut self, cx: &mut Context<Self>) {
        let count = self.list.total();
        self.load(Vec::new());
        self.report_deleted(count, cx);
    }

    fn reload(&mut self) {
        if let Some(meter) = &self.meter {
            let records = meter.get_history();
            self.load(records);
        }
    }

    fn forget_missing_open_record(&mut self) {
        if self.open.as_deref().is_some_and(|id| self.list.record(id).is_none()) {
            self.open = None;
        }
    }
}
