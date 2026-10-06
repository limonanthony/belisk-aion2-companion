use std::sync::Arc;
use std::time::Duration;

use meter_core::engine::meter::DpsMeter;
use meter_core::history::HistoryRecord;
use gpui::{Context, Subscription, Task};

use super::list::HistoryList;
use crate::i18n::{self, fl};

const STATUS_FOR: Duration = Duration::from_secs(4);

/// The history window's state: the list, which record is open, and the line
/// confirming a delete. The records themselves are the meter's.
pub struct HistoryModel {
    meter: Option<Arc<DpsMeter>>,
    pub list: HistoryList,
    /// The record whose players are shown; one at a time.
    pub open: Option<String>,
    /// How many records the last "Delete all" removed, while its line shows.
    deleted: Option<usize>,
    status_timer: Option<Task<()>>,
    _language: Subscription,
}

impl HistoryModel {
    pub fn new(meter: Option<Arc<DpsMeter>>, cx: &mut Context<Self>) -> Self {
        let locale = i18n::locale(cx);
        let language = cx.observe(&locale, |this, _, cx| this.rebuild_words(cx));
        let mut model =
            Self { meter, list: HistoryList::default(), open: None, deleted: None, status_timer: None, _language: language };
        model.reload();
        model
    }

    pub fn status(&self) -> Option<String> {
        self.deleted.map(|count| fl!("history-deleted", count = count))
    }

    /// Reads the records again, after the meter saved or deleted one.
    pub fn refresh(&mut self, cx: &mut Context<Self>) {
        self.reload();
        cx.notify();
    }

    /// Writes the rows again after the language changed.
    pub fn rebuild_words(&mut self, cx: &mut Context<Self>) {
        self.list.rebuild_words();
        self.forget_missing_open_record();
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
        self.deleted = Some(count);
        self.status_timer = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(STATUS_FOR).await;
            let _ = this.update(cx, |this, cx| {
                this.deleted = None;
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

#[cfg(test)]
mod tests {
    use gpui::{AppContext as _, TestAppContext};

    use super::*;
    use crate::i18n::{Language, LanguageChoice};

    #[gpui::test]
    fn the_delete_line_follows_a_language_change_while_it_shows(cx: &mut TestAppContext) {
        let history = cx.new(|cx| HistoryModel::new(None, cx));
        history.update(cx, |history, cx| history.report_deleted(3, cx));
        let english = history.read_with(cx, |history, _| history.status()).expect("the line shows");

        cx.update(|cx| i18n::apply(LanguageChoice::French, cx));

        let french = history.read_with(cx, |history, _| history.status()).expect("the line still shows");
        assert_ne!(french, english);
        assert_eq!(french, fl!("history-deleted", count = 3));
        assert_eq!(i18n::language(), Language::French);
    }
}
