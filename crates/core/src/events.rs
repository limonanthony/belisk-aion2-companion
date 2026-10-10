use crate::models::combat::CombatSnapshot;
use crate::models::diagnostics::MemorySnapshot;
use crate::identity::{IdentityDecision, KnownCharacter};
use crate::pets::PetReport;
use crate::tickets::TicketReport;
use crate::report::fight_report::FightReport;
use crate::storage::data_storage::MainActorDetectedPayload;

/// A boss attempt that ended without a kill: its health went back to full or
/// it left the field alive. The attempt is off the meter by the time this is
/// sent.
#[derive(Debug, Clone)]
pub struct WipedAttempt {
    pub target_id: u32,
    /// The meter as it stood at the wipe, with the boss's health set to the
    /// lowest it got in the attempt.
    pub snapshot: CombatSnapshot,
    /// The history record of the attempt; `None` when it held too little
    /// damage to be kept.
    pub record_id: Option<String>,
    /// The fight report of the attempt, as the saved record would give it.
    pub report: Option<FightReport>,
}

#[derive(Debug, Clone)]
pub enum MeterEvent {
    DpsSnapshot(CombatSnapshot),
    DpsMemory(MemorySnapshot),
    DpsMeterStatus(bool),
    /// You wiped on a boss. The next snapshot no longer holds the attempt.
    BossWiped(WipedAttempt),
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
    /// What the game said about the local player's pet collection and soul
    /// gains, tied to the character it said it about.
    Pets(PetReport),
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
