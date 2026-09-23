//! The Provider Port: the single seam to all cloud interaction (NFR-EXT1, ADR-0012). It is a small
//! set of **capability traits**; provider-specific knowledge lives in declarative descriptors the
//! adapter ships (see [`crate::seam`]). Adding a provider = implement these capabilities + ship a
//! descriptor catalog, with **zero** core changes (the NFR-EXT release gate).
//!
//! Async capabilities use `async_trait` for object safety so the core can hold
//! `&dyn`/`Box<dyn ...>` providers.

use async_trait::async_trait;

use crate::error::Result;
use crate::graph::Assertion;
use crate::ids::{ProviderId, Region};
use crate::principal::Principal;
use crate::seam::{
    ActionDescriptor, ActionResult, CapabilityGrant, EnumerationDescriptor, RawResponse,
};

/// Resolve supplied credentials to the current principal and enumerate credential-derived
/// identities. Supports a low-privilege starting foothold (FR-B3/B4).
#[async_trait]
pub trait IdentityResolver: Send + Sync {
    /// Resolve the identity the supplied credentials currently act as.
    async fn resolve_current_principal(&self) -> Result<Principal>;
}

/// Execute enumeration descriptors, returning raw responses (paging/throttling/region handled
/// centrally by the adapter). Serves FR-C / FR-B5.
#[async_trait]
pub trait ResourceEnumerator: Send + Sync {
    /// Execute one enumeration descriptor and return its raw response.
    async fn execute(&self, descriptor: &EnumerationDescriptor) -> Result<RawResponse>;
}

/// Execute an action descriptor with host-brokered params and a per-step capability grant
/// (NFR-SEC5). Serves FR-G.
#[async_trait]
pub trait ActionExecutor: Send + Sync {
    /// Execute one action descriptor under the given capability grant.
    async fn execute(
        &self,
        descriptor: &ActionDescriptor,
        grant: &CapabilityGrant,
    ) -> Result<ActionResult>;
}

/// Map raw provider responses into canonical attack-graph assertions (FR-D). Synchronous: pure
/// data transformation, no I/O.
pub trait GraphMapper: Send + Sync {
    /// Map a raw response into zero or more graph assertions.
    fn map(&self, response: &RawResponse) -> Result<Vec<Assertion>>;
}

/// Region/partition, service/operation, and permission-model catalog (FR-B5).
pub trait MetadataProvider: Send + Sync {
    /// The provider this metadata describes.
    fn provider_id(&self) -> ProviderId;

    /// The regions/partitions available for this provider.
    fn regions(&self) -> Vec<Region>;
}

/// Retrieve the telemetry a detonation produced (FR-I2). **v2 capability** — the v1 default returns
/// [`crate::error::AkumoError::Unsupported`] so the seam is shaped now but unimplemented (ADR-0008).
#[async_trait]
pub trait TelemetryCollector: Send + Sync {
    /// Fetch events correlated to a detonation. Defaults to unsupported in v1.
    async fn collect(&self, _correlation_id: &str) -> Result<RawResponse> {
        Err(crate::error::AkumoError::Unsupported(
            "telemetry collection is a v2 capability (FR-I2)".to_string(),
        ))
    }
}

/// The aggregate Provider Port: a provider exposes exactly the capabilities above. The core reaches
/// the cloud only through this trait, keeping it provider-blind (ADR-0001).
pub trait Provider: Send + Sync {
    /// The provider's identifier (e.g. `aws`, `mock`).
    fn id(&self) -> ProviderId;

    /// Credential/identity resolution.
    fn identity(&self) -> &dyn IdentityResolver;

    /// Enumeration primitive.
    fn enumerator(&self) -> &dyn ResourceEnumerator;

    /// Action execution primitive.
    fn actions(&self) -> &dyn ActionExecutor;

    /// Response → graph mapping.
    fn mapper(&self) -> &dyn GraphMapper;

    /// Region/partition/permission metadata.
    fn metadata(&self) -> &dyn MetadataProvider;

    /// Telemetry access (v2 stub in v1).
    fn telemetry(&self) -> &dyn TelemetryCollector;
}
