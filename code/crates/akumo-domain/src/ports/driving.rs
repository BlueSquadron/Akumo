//! Driving ports — the high-level operations the driving adapters (CLI, library, CI) invoke on the
//! core. This is a seam placeholder for Increment 1: the full operation set (`enumerate`,
//! `compute_paths`, `preview`, `execute_step`, `execute_chain`, `revert`, `report`, …) is defined
//! incrementally as each capability lands (G5–G15). Only the engagement-open operation is shaped
//! here, using types that already exist, so the seam is real without over-committing.

use async_trait::async_trait;

use crate::error::Result;
use crate::ids::{EngagementId, ProviderId};
use crate::scope::Scope;

/// The minimal engagement-lifecycle entry point (expanded in Increment 2 / G5).
#[async_trait]
pub trait EngagementApi: Send + Sync {
    /// Open a new, isolated engagement against a provider within an authorized scope.
    ///
    /// `authorization_affirmed` records the operator's affirmation of authorization (NFR-COMP1) and
    /// MUST be `true`; implementations reject a `false` affirmation.
    async fn open_engagement(
        &self,
        scope: Scope,
        provider: ProviderId,
        authorization_affirmed: bool,
    ) -> Result<EngagementId>;
}
