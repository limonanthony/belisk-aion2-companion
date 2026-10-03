use aether_core::events::{EventSink, MeterEvent};
use tauri::{AppHandle, Emitter};

use crate::plugins::aion2_focus;

pub struct TauriEventSink {
    app: AppHandle,
}

impl TauriEventSink {
    pub fn new(app: AppHandle) -> Self {
        Self { app }
    }
}

impl EventSink for TauriEventSink {
    fn emit(&self, event: MeterEvent) {
        let app = &self.app;
        match event {
            MeterEvent::DpsSnapshot(snapshot) => {
                let _ = app.emit("dps-snapshot", snapshot);
            }
            MeterEvent::DpsMemory(snapshot) => {
                let _ = app.emit("dps-memory", snapshot);
            }
            MeterEvent::DpsMeterStatus(running) => {
                let _ = app.emit("dps-meter-status", running);
            }
            MeterEvent::HistoryUpdated => {
                let _ = app.emit("history-updated", ());
            }
            MeterEvent::PersonalBestsUpdated => {
                let _ = app.emit("personal-bests-updated", ());
            }
            MeterEvent::MainActorDetected(payload) => {
                let _ = app.emit("dps-main-actor-detected", payload);
            }
            MeterEvent::PacketReplayFinished(count) => {
                let _ = app.emit("packet-replay-finished", count);
            }
            MeterEvent::PacketReplayFailed(error) => {
                let _ = app.emit("packet-replay-failed", error);
            }
            MeterEvent::DpsIdleHidden(hidden) => {
                aion2_focus::set_dps_idle_hidden_for_app(app, hidden);
            }
        }
    }
}
