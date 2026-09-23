//! # akumo-provider-mock
//!
//! A first-class, always-shipped Provider Port adapter (ADR-0030): a behavioral, scriptable stub
//! backed by an authorable synthetic environment (principals, graph assertions, scripted denials
//! and action results). It serves two roles:
//!
//! 1. the **extensibility gate's "second provider"** — proving the full loop runs with zero core
//!    changes (NFR-EXT7), and
//! 2. the **fast test tier** — deterministic, cloud-free tests of enumeration, planning, execution,
//!    and revert logic (OQ-8).
//!
//! It is deliberately *not* an AWS simulator — the live-AWS tier is the semantic backstop.
//!
//! ```
//! use akumo_provider_mock::{MockEnvironment, MockProvider};
//! use akumo_provider_mock::domain::principal::{Principal, PrincipalKind};
//! use akumo_provider_mock::domain::ids::ProviderId;
//!
//! let env = MockEnvironment::builder(
//!     "mock",
//!     Principal::new("arn:foothold", PrincipalKind::Role, ProviderId::new("mock")),
//! )
//! .region("mock-region-1")
//! .build();
//! let _provider = MockProvider::new(env);
//! ```

#![forbid(unsafe_code)]

mod env;
mod provider;

pub use akumo_domain as domain;
pub use env::{EnumOutcome, MockEnvironment, MockEnvironmentBuilder};
pub use provider::MockProvider;
