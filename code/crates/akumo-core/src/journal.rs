//! The journal: the one place events are written. It derives the next `(seq, prev_hash)` from the
//! event store's chain head, seals the event, and appends it — so every producer (engagement,
//! enumeration, execution) records events consistently and the hash chain stays intact.

use std::time::{SystemTime, UNIX_EPOCH};

use akumo_domain::error::Result;
use akumo_domain::event::EventEnvelope;
use akumo_domain::ids::{Actor, EngagementId, Seq, Timestamp};
use akumo_domain::ports::EventStore;

/// A thin writer over an [`EventStore`] that handles seq/hash bookkeeping.
pub struct Journal<'a> {
    store: &'a dyn EventStore,
}

impl<'a> Journal<'a> {
    /// Wrap an event store.
    pub fn new(store: &'a dyn EventStore) -> Self {
        Self { store }
    }

    /// Record an event: compute the next sequence and predecessor hash from the chain head, seal,
    /// and append. Returns the sealed event.
    pub async fn record(
        &self,
        engagement: &EngagementId,
        actor: Actor,
        event_type: &str,
        payload: serde_json::Value,
    ) -> Result<EventEnvelope> {
        let (seq, prev_hash) = match self.store.head(engagement).await? {
            Some((last_seq, last_hash)) => (last_seq.next(), Some(last_hash)),
            None => (Seq::ZERO, None),
        };
        let id = format!("{engagement}-{}", seq.0);
        let event = EventEnvelope::seal(
            id,
            engagement.clone(),
            seq,
            Timestamp::from_millis(now_millis()),
            actor,
            prev_hash,
            event_type,
            payload,
        )?;
        self.store.append(event.clone()).await?;
        Ok(event)
    }
}

/// Current wall-clock time in Unix epoch milliseconds (0 if the clock is before the epoch).
/// Timestamps are recorded in the envelope but are **not** part of the chain hash, so a wall clock
/// here does not affect determinism of the chain.
pub(crate) fn now_millis() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}
