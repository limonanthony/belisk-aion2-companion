use std::time::{Duration, Instant};

use meter_core::events::{EventSink, MeterEvent};
use meter_core::identity::KnownCharacter;
use meter_core::tickets::TicketReport;
use futures::channel::mpsc::{unbounded, UnboundedReceiver, UnboundedSender};
use futures::StreamExt;
use gpui::{App, Entity, Task};

use crate::meter::{MeterModel, ReplayState};
use crate::secondary;
use crate::settings;
use crate::todos;

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
            let characters_seen: Vec<KnownCharacter> = batch
                .iter()
                .filter_map(|event| match event {
                    MeterEvent::CharacterSeen(character) => Some(character.clone()),
                    _ => None,
                })
                .collect();
            let players: Vec<todos::Identified> = batch.iter().filter_map(todos::identified_by).collect();
            let ticket_reports: Vec<TicketReport> = batch
                .iter()
                .filter_map(|event| match event {
                    MeterEvent::Tickets(report) => Some(report.clone()),
                    _ => None,
                })
                .collect();

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
            if !characters_seen.is_empty() {
                cx.update(|cx| settings::remember_characters(cx, characters_seen));
            }
            if !players.is_empty() {
                cx.update(|cx| todos::follow_player(cx, players));
            }
            if !ticket_reports.is_empty() {
                cx.update(|cx| todos::record_tickets(cx, ticket_reports));
            }
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
