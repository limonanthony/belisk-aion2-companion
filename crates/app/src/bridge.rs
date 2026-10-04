use std::time::{Duration, Instant};

use aether_core::events::{EventSink, MeterEvent};
use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::StreamExt;
use gpui::{App, Entity, Task};

use crate::meter::{MeterModel, ReplayState};
use crate::secondary;

/// After a replay ends the pipeline still holds packets; the app may exit once
/// no event has arrived for this long.
const QUIET_AFTER_REPLAY: Duration = Duration::from_millis(1500);

/// Hands meter events to the UI thread. `emit` runs on the meter's own
/// threads, so it only queues: it never blocks and never touches a window.
pub struct ChannelSink(UnboundedSender<MeterEvent>);

impl EventSink for ChannelSink {
    fn emit(&self, event: MeterEvent) {
        let _ = self.0.unbounded_send(event);
    }
}

pub fn channel() -> (ChannelSink, UnboundedReceiver<MeterEvent>) {
    let (sender, receiver) = unbounded();
    (ChannelSink(sender), receiver)
}

/// Drains the channel on the foreground executor into `model`. The task sleeps
/// until an event arrives; events that piled up while it was busy are applied
/// together and cost one redraw.
pub fn spawn_pump(
    model: Entity<MeterModel>,
    mut events: UnboundedReceiver<MeterEvent>,
    quit_when_replay_is_quiet: bool,
    cx: &App,
) -> Task<()> {
    cx.spawn(async move |cx| {
        let mut quit_timer: Option<Task<()>> = None;
        while let Some(first) = events.next().await {
            let mut batch = vec![first];
            while let Ok(event) = events.try_recv() {
                batch.push(event);
            }
            let saved_a_fight = batch.iter().any(|event| matches!(event, MeterEvent::HistoryUpdated));

            let (changed, replay_just_ended, replay_over) = cx.update(|cx| {
                model.update(cx, |model, cx| {
                    let replay_was_over = model.replay != ReplayState::NotStarted;
                    let now = Instant::now();
                    let mut changed = false;
                    for event in batch {
                        changed |= model.apply(event, now);
                    }
                    if changed {
                        cx.notify();
                    }
                    model.arm_deadline(cx);
                    let replay_over = model.replay != ReplayState::NotStarted;
                    (changed, replay_over && !replay_was_over, replay_over)
                })
            });
            if saved_a_fight {
                cx.update(secondary::history_updated);
            }
            if quit_when_replay_is_quiet && replay_over && (changed || replay_just_ended) {
                quit_timer = Some(cx.spawn(async move |cx| {
                    cx.background_executor().timer(QUIET_AFTER_REPLAY).await;
                    cx.update(|cx| cx.quit());
                }));
            }
        }
        drop(quit_timer);
    })
}
