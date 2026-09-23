//! The event envelope skeleton. Akumo's engagement state is an **append-only, hash-chained event
//! ledger** (ADR-0014 / SPEC-D3): the ledger *is* the source of truth and every projection (graph,
//! execution status, reports) is a deterministic fold over it.
//!
//! This module defines the envelope shape the [`crate::ports::persistence::EventStore`] moves. The
//! canonical hashing scheme (`hash = H(prev_hash ‖ canonical(payload))`) and the concrete event
//! **type** enum (spec §2.7) are implemented in Increment 2 (task G2.1); here the payload is an
//! opaque JSON value so the port can be defined now.

use serde::{Deserialize, Serialize};

use crate::error::{AkumoError, Result};
use crate::hash;
use crate::ids::{Actor, EngagementId, EventHash, Seq, Timestamp};

/// Canonical event-type discriminators (spec §2.7). Producers and projections share these constants
/// so they never drift. The v1 `event_type` field is a string; these are the agreed values. As more
/// producers land (G5/G6/G9/G14) this set grows; a typed enum may replace it later.
pub mod event_type {
    // Engagement lifecycle (G5).
    /// A new engagement was opened (carries scope + authorization affirmation).
    pub const ENGAGEMENT_OPENED: &str = "EngagementOpened";
    /// The authorized scope was amended.
    pub const SCOPE_AMENDED: &str = "ScopeAmended";
    /// The global kill-switch was invoked.
    pub const KILL_SWITCH_INVOKED: &str = "KillSwitchInvoked";
    /// The engagement was closed.
    pub const ENGAGEMENT_CLOSED: &str = "EngagementClosed";

    // Content (G7).
    /// A technique passed schema/contract validation and was loaded.
    pub const TECHNIQUE_VALIDATED: &str = "TechniqueValidated";

    // Execution / consent (G5, G9).
    /// A consent decision was recorded (gates mutating steps, FR-A3).
    pub const CONSENT_RECORDED: &str = "ConsentRecorded";
    /// A dry-run preview was performed (no mutation).
    pub const DRY_RUN_PERFORMED: &str = "DryRunPerformed";
    /// A blast-radius estimate was produced.
    pub const BLAST_RADIUS_ESTIMATED: &str = "BlastRadiusEstimated";
    /// A step was detonated; payload carries the recorded compensation (saga, SPEC-D8).
    pub const STEP_DETONATED: &str = "StepDetonated";
    /// A step was verified and its effects asserted.
    pub const STEP_VERIFIED: &str = "StepVerified";
    /// A step's compensation was replayed (revert).
    pub const STEP_REVERTED: &str = "StepReverted";
    /// Execution failed at a step.
    pub const EXECUTION_FAILED: &str = "ExecutionFailed";

    // Enumeration (G6).
    /// Enumeration began.
    pub const ENUMERATION_STARTED: &str = "EnumerationStarted";
    /// A graph fact (node/edge) was asserted; payload is an `Assertion`.
    pub const FACT_ASSERTED: &str = "FactAsserted";
    /// A blind spot was recorded; payload is a `CoverageGap`.
    pub const ACCESS_DENIED: &str = "AccessDenied";
    /// Enumeration completed.
    pub const ENUMERATION_COMPLETED: &str = "EnumerationCompleted";

    // Planning (G12).
    /// An attack path was computed toward an objective.
    pub const PATH_COMPUTED: &str = "PathComputed";
}

/// One immutable, ordered record in an engagement's ledger.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EventEnvelope {
    /// Globally unique event id.
    pub id: String,
    /// The engagement this event belongs to (enforces isolation, DSR-2).
    pub engagement_id: EngagementId,
    /// Monotonic per-engagement sequence number.
    pub seq: Seq,
    /// When the event was recorded.
    pub timestamp: Timestamp,
    /// Who caused the event.
    pub actor: Actor,
    /// Hash of the previous event in this engagement's chain (`None` for the first).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prev_hash: Option<EventHash>,
    /// This event's hash: `H(prev_hash ‖ canonical(payload))` (scheme finalized in G2.1).
    pub hash: EventHash,
    /// The event type discriminator (becomes a typed enum in G2.1).
    pub event_type: String,
    /// The event payload. Opaque here; typed per category in Increment 2.
    pub payload: serde_json::Value,
}

impl EventEnvelope {
    /// Build and **seal** an event, computing its hash from `prev_hash` and the canonical payload
    /// (`hash = H(prev_hash ‖ canonical(payload))`, spec §2.7). The event store validates the
    /// chain on append.
    #[allow(clippy::too_many_arguments)]
    pub fn seal(
        id: impl Into<String>,
        engagement_id: EngagementId,
        seq: Seq,
        timestamp: Timestamp,
        actor: Actor,
        prev_hash: Option<EventHash>,
        event_type: impl Into<String>,
        payload: serde_json::Value,
    ) -> Result<Self> {
        let canonical = serde_json::to_vec(&payload)?;
        let hash = hash::hash_event(prev_hash.as_ref(), &canonical);
        Ok(Self {
            id: id.into(),
            engagement_id,
            seq,
            timestamp,
            actor,
            prev_hash,
            hash,
            event_type: event_type.into(),
            payload,
        })
    }

    /// Recompute this event's hash from its stored `prev_hash` and payload.
    pub fn recompute_hash(&self) -> Result<EventHash> {
        let canonical = serde_json::to_vec(&self.payload)?;
        Ok(hash::hash_event(self.prev_hash.as_ref(), &canonical))
    }

    /// Verify the stored hash matches the recomputed one (tamper-evidence, NFR-OBS3).
    pub fn verify_hash(&self) -> Result<()> {
        if self.recompute_hash()? == self.hash {
            Ok(())
        } else {
            Err(AkumoError::Integrity(format!(
                "event {} (seq {}) hash mismatch — payload or chain tampered",
                self.id, self.seq.0
            )))
        }
    }

    /// Whether this is the first event in a chain (no predecessor hash).
    pub fn is_genesis(&self) -> bool {
        self.prev_hash.is_none()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(prev: Option<EventHash>, seq: u64) -> EventEnvelope {
        EventEnvelope::seal(
            format!("evt-{seq}"),
            EngagementId::new("eng-1"),
            Seq(seq),
            Timestamp::from_millis(1),
            Actor::new("tester"),
            prev,
            "EngagementOpened",
            serde_json::json!({ "scope": ["acct:1"], "n": seq }),
        )
        .unwrap()
    }

    #[test]
    fn sealed_event_verifies() {
        let e = sample(None, 0);
        assert!(e.is_genesis());
        e.verify_hash().unwrap();
    }

    #[test]
    fn tampering_payload_breaks_verification() {
        let mut e = sample(None, 0);
        e.payload = serde_json::json!({ "scope": ["acct:evil"], "n": 0 });
        assert!(e.verify_hash().is_err());
    }

    #[test]
    fn chain_links_through_prev_hash() {
        let e0 = sample(None, 0);
        let e1 = sample(Some(e0.hash.clone()), 1);
        assert_eq!(e1.prev_hash.as_ref(), Some(&e0.hash));
        e1.verify_hash().unwrap();
    }
}
