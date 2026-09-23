//! Ports — the stable interfaces the provider-agnostic core depends on (hexagonal architecture,
//! ADR-0001). Adapters implement these; the core never depends on a concrete adapter.
//!
//! - [`provider`] — the **Provider Port**: the six capability traits that are the one seam to all
//!   cloud interaction (ADR-0012), plus a [`provider::Provider`] aggregate.
//! - [`persistence`] — the **Persistence Port**: the append-only event store (ADR-0014).
//! - [`driving`] — the **driving ports**: high-level operations the CLI/library/CI invoke on the
//!   core (fleshed out across later increments).

pub mod driving;
pub mod persistence;
pub mod provider;

pub use persistence::EventStore;
pub use provider::{
    ActionExecutor, GraphMapper, IdentityResolver, MetadataProvider, Provider, ResourceEnumerator,
    TelemetryCollector,
};
