use gpui::{Context, Entity, Subscription};

use super::data::{DetailData, DetailSource};
use crate::meter::{MeterModel, OverlaySettings};

/// The player detail window's state: what it was opened on, and what that
/// shows now. Live sources are rebuilt from each snapshot the meter takes in;
/// only a change redraws.
pub struct DetailModel {
    meter_model: Entity<MeterModel>,
    source: DetailSource,
    data: Option<DetailData>,
    /// The snapshot and settings `data` was built from. The meter notifies
    /// for more than snapshots (personal bests, system figures, the board
    /// switch), and those change nothing here.
    built_from: (u64, OverlaySettings),
    _snapshots: Subscription,
}

impl DetailModel {
    pub fn new(meter_model: Entity<MeterModel>, source: DetailSource, cx: &mut Context<Self>) -> Self {
        let snapshots = cx.observe(&meter_model, |this, _, cx| {
            if this.source.is_live() && this.inputs(cx) != this.built_from {
                this.rebuild(cx);
            }
        });
        let built_from = (0, OverlaySettings::default());
        let mut model = Self { meter_model, source, data: None, built_from, _snapshots: snapshots };
        model.data = model.build(cx);
        model
    }

    pub fn data(&self) -> Option<&DetailData> {
        self.data.as_ref()
    }

    pub fn source(&self) -> &DetailSource {
        &self.source
    }

    pub fn meter_model(&self) -> &Entity<MeterModel> {
        &self.meter_model
    }

    /// Points the window at another player or board.
    pub fn select(&mut self, source: DetailSource, cx: &mut Context<Self>) {
        self.source = source;
        self.data = self.build(cx);
        cx.notify();
    }

    fn rebuild(&mut self, cx: &mut Context<Self>) {
        let data = self.build(cx);
        if data != self.data {
            self.data = data;
            cx.notify();
        }
    }

    fn inputs(&self, cx: &Context<Self>) -> (u64, OverlaySettings) {
        let meter = self.meter_model.read(cx);
        (meter.snapshot_id(), meter.settings.clone())
    }

    fn build(&mut self, cx: &Context<Self>) -> Option<DetailData> {
        self.built_from = self.inputs(cx);
        let meter = self.meter_model.read(cx);
        DetailData::build(&self.source, meter.snapshot(), &meter.settings)
    }
}
