//! # akumo-core
//!
//! The provider-agnostic domain services that implement Akumo's unified loop:
//! `authorize → enumerate → model → reason → plan → execute safely → verify → report`.
//!
//! Every service depends only on the **ports** in [`akumo_domain::ports`], never on a concrete
//! adapter or cloud SDK (ADR-0001 / NFR-EXT1, enforced by `xtask dep-lint`). The individual
//! services are added by later increments:
//!
//! | Service | Increment / task |
//! |---|---|
//! | Engagement manager (authz, scope, consent, kill-switch) | G5 |
//! | Enumeration orchestration | G6 |
//! | Attack graph + fact views | G4 |
//! | Execution engine (lifecycle, dry-run, saga revert, chaining) | G9, G11 |
//! | Planner (objective-directed path-finding + ranking) | G12 |
//! | Telemetry model + reporting | G13, G14 |
//!
//! For now this crate establishes the boundary and re-exports the domain surface the services and
//! driving adapters build on.

#![forbid(unsafe_code)]

pub mod api;
pub mod chain;
pub mod engagement;
#[cfg(test)]
mod gates;
pub mod enumeration;
pub mod execution;
pub mod graph;
pub mod journal;
pub mod planner;
pub mod projection;
pub mod report;
pub mod telemetry;

pub use akumo_domain as domain;
pub use akumo_dsl as dsl;

/// The version of the engine/content-model compatibility contract (NFR-MNT3). Bumped when the
/// content model or event schema changes in a breaking way.
pub const ENGINE_CONTRACT_VERSION: u32 = 0;

#[cfg(test)]
mod tests {
    #[test]
    fn contract_version_is_stable_for_now() {
        assert_eq!(super::ENGINE_CONTRACT_VERSION, 0);
    }
}
