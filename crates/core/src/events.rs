use crate::models::combat::CombatSnapshot;
use crate::models::diagnostics::MemorySnapshot;
use crate::identity::{IdentityDecision, KnownCharacter};
use crate::tickets::TicketReport;
use crate::storage::data_storage::MainActorDetectedPayload;

#[derive(Debug, Clone)]
pub enum MeterEvent {
    DpsSnapshot(CombatSnapshot),
    DpsMemory(MemorySnapshot),
    DpsMeterStatus(bool),
    HistoryUpdated,
    PersonalBestsUpdated,
    MainActorDetected(MainActorDetectedPayload),
    /// The game named the local player's character: the app may remember it.
    CharacterSeen(KnownCharacter),
    /// How the core decided who the local player's actor is.
    OwnIdentity(IdentityDecision),
    /// What the game said about the local player's tickets, keys and Odyle
    /// Energy, tied to the character it said it about.
    Tickets(TicketReport),
    PacketReplayFinished(u64),
    PacketReplayFailed(String),
    /// The overlay should step aside (`true`) or come back (`false`) because
    /// no fight is on the meter.
    DpsIdleHidden(bool),
}

/// Receives events on the thread that raised them, so it must not block.
pub trait EventSink: Send + Sync {
    fn emit(&self, event: MeterEvent);
}
