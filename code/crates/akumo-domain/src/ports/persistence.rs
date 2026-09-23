//! The Persistence Port: the durable, append-only, hash-chained event store that is Akumo's single
//! source of truth (ADR-0014). The substrate (an embedded KV store) is an Increment 7 detail behind
//! this trait; the core only ever sees the port.

use async_trait::async_trait;

use crate::error::Result;
use crate::event::EventEnvelope;
use crate::ids::{EngagementId, EventHash, Seq};

/// Append-only event storage, isolated per engagement (DSR-2). Appends must be durable
/// (crash-safe) so an interrupted engagement can resume from the ledger (NFR-REL1/2).
#[async_trait]
pub trait EventStore: Send + Sync {
    /// Append one event to its engagement's chain. Implementations enforce ordering and durability.
    async fn append(&self, event: EventEnvelope) -> Result<()>;

    /// Read the full ordered event stream for an engagement (used to fold projections).
    async fn read_stream(&self, engagement: &EngagementId) -> Result<Vec<EventEnvelope>>;

    /// The hash of the most recent event in an engagement's chain, or `None` if empty. Callers use
    /// this as `prev_hash` when appending the next event.
    async fn last_hash(&self, engagement: &EngagementId) -> Result<Option<EventHash>>;

    /// The chain head — the last `(seq, hash)` — or `None` for an empty engagement. Callers derive
    /// the next event's `seq` (`head.seq.next()`, else `Seq::ZERO`) and `prev_hash` from this.
    async fn head(&self, engagement: &EngagementId) -> Result<Option<(Seq, EventHash)>>;
}
