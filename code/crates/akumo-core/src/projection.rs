//! Projections (ADR-0014 / SPEC-D3, spec §2.1). The event ledger is the single source of truth;
//! every read model — the attack graph, the coverage map, execution status, reports — is a
//! **deterministic fold** over the event stream. A projection is never a competing source of truth;
//! snapshots of one are a cache/optimization only.
//!
//! This module provides the folding contract. Concrete projections (graph, coverage, execution
//! ledger) are added by their owning increments (G4, G6, G9).

use akumo_domain::event::EventEnvelope;
use akumo_domain::ids::Seq;

/// A deterministic fold from an engagement's event stream to a read model.
///
/// Implementors provide only [`Projection::apply`]; the replay helpers are derived from it. Because
/// `apply` is a pure function of the current state and the next event, replaying the same stream
/// always yields the same state (the determinism the audit/replay guarantees depend on, FR-J2).
pub trait Projection {
    /// The read model this projection builds. Must have an empty initial value.
    type State: Default;

    /// Fold one event into the state.
    fn apply(state: &mut Self::State, event: &EventEnvelope);

    /// Replay a full event stream into a fresh state.
    fn replay<'a, I>(events: I) -> Self::State
    where
        I: IntoIterator<Item = &'a EventEnvelope>,
    {
        let mut state = Self::State::default();
        for event in events {
            Self::apply(&mut state, event);
        }
        state
    }

    /// Replay a stream up to and including `up_to` (inclusive), for audit-grade point-in-time
    /// replay (ADR-0010). Assumes the stream is ordered by `seq`.
    fn replay_to<'a, I>(events: I, up_to: Seq) -> Self::State
    where
        I: IntoIterator<Item = &'a EventEnvelope>,
    {
        let mut state = Self::State::default();
        for event in events {
            if event.seq > up_to {
                break;
            }
            Self::apply(&mut state, event);
        }
        state
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use akumo_domain::ids::{Actor, EngagementId, Timestamp};

    /// A trivial projection that counts events — enough to prove determinism and `replay_to`.
    struct EventCount;

    impl Projection for EventCount {
        type State = usize;

        fn apply(state: &mut usize, _event: &EventEnvelope) {
            *state += 1;
        }
    }

    fn stream(n: u64) -> Vec<EventEnvelope> {
        let eng = EngagementId::new("eng-proj");
        let mut prev = None;
        let mut out = Vec::new();
        for seq in 0..n {
            let e = EventEnvelope::seal(
                format!("evt-{seq}"),
                eng.clone(),
                Seq(seq),
                Timestamp::from_millis(seq),
                Actor::new("tester"),
                prev.clone(),
                "TestEvent",
                serde_json::json!({ "n": seq }),
            )
            .unwrap();
            prev = Some(e.hash.clone());
            out.push(e);
        }
        out
    }

    #[test]
    fn replay_is_deterministic() {
        let events = stream(5);
        let a = EventCount::replay(events.iter());
        let b = EventCount::replay(events.iter());
        assert_eq!(a, 5);
        assert_eq!(a, b);
    }

    #[test]
    fn replay_to_is_inclusive_and_point_in_time() {
        let events = stream(5);
        assert_eq!(EventCount::replay_to(events.iter(), Seq(2)), 3); // seq 0,1,2
        assert_eq!(EventCount::replay_to(events.iter(), Seq(0)), 1);
    }
}
