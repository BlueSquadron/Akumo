//! # akumo-domain
//!
//! The provider-neutral heart of Akumo: value objects, the shared error type, a minimal event
//! envelope, the provider-seam stub types, and the **port traits** every adapter implements.
//!
//! This crate has **zero I/O and zero cloud-SDK dependencies** by construction (ADR-0001,
//! NFR-EXT1). The dependency-direction lint (`xtask dep-lint`) fails the build if this crate — or
//! [`akumo-core`] — ever gains an adapter or cloud-SDK dependency.
//!
//! Modules map to the spec increments:
//! - [`ids`], [`impact`], [`epistemic`], [`scope`], [`principal`], [`error`] — core value objects
//!   (spec Inc. 1, task G1.2).
//! - [`event`] — the event envelope skeleton (fleshed out with the hash chain in Inc. 2 / G2.1).
//! - [`seam`] — provider-seam descriptor/result stub types (frozen in Inc. 2 / G3.1).
//! - [`ports`] — the port traits: Provider Port capabilities and the Persistence Port (G1.3).

pub mod epistemic;
pub mod error;
pub mod event;
pub mod graph;
pub mod hash;
pub mod ids;
pub mod impact;
pub mod ports;
pub mod principal;
pub mod scope;
pub mod seam;

pub use epistemic::{Epistemic, EpistemicStatus};
pub use error::{AkumoError, Result};
pub use event::EventEnvelope;
pub use graph::{Assertion, CoverageGap, EdgeKind, GraphEdge, GraphNode, NodeId, NodeKind, Provenance};
pub use ids::{Actor, EngagementId, EventHash, ProviderId, Region, Seq, TechniqueId, Timestamp};
pub use impact::ImpactLevel;
pub use principal::{Principal, PrincipalKind};
pub use scope::{Scope, ScopeSelector};
