//! Provider-seam stub types (ADR-0012 / SPEC-D1 = "capability + descriptor hybrid").
//!
//! The Provider Port is a small set of capability traits (see [`crate::ports::provider`]) driven by
//! **declarative descriptors** the adapter ships. The concrete descriptor schema and the
//! response/result mapping vocabulary are frozen in Increment 2 (task G3.1) once the graph/fact
//! model lands. Until then these are intentionally thin wrappers around opaque JSON so the port
//! traits can be defined and the Mock adapter can be built, without prematurely fixing the schema.

use serde::{Deserialize, Serialize};

use crate::impact::ImpactLevel;

/// A declarative enumeration step: which provider operation to call. The adapter maps the operation
/// to a concrete API call and maps the response into graph assertions.
///
/// The response-mapping language is designed with the step-DSL (G7/G8); until then the adapter's
/// `GraphMapper` performs the mapping in code. `service`/`operation` identify the call.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct EnumerationDescriptor {
    /// Provider service (e.g. `iam`, `s3`).
    pub service: String,
    /// Operation within the service (e.g. `ListUsers`).
    pub operation: String,
    /// Static or templated parameters for the call.
    #[serde(default)]
    pub params: serde_json::Map<String, serde_json::Value>,
    /// The permission this call requires, if known (provenance/authz preview).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_permission: Option<String>,
}

/// A declarative action step: which provider operation to perform, its (templated) params, and its
/// impact level. Result-mapping into facts is done by the adapter (and, later, the step-DSL).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActionDescriptor {
    /// Provider service (e.g. `iam`).
    pub service: String,
    /// Operation within the service (e.g. `CreateAccessKey`).
    pub operation: String,
    /// Static or templated parameters for the call.
    #[serde(default)]
    pub params: serde_json::Map<String, serde_json::Value>,
    /// How consequential this action is (gates consent, FR-A3).
    pub impact: ImpactLevel,
    /// The permission this action requires, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_permission: Option<String>,
}

impl EnumerationDescriptor {
    /// The `service.operation` key used to identify this call.
    pub fn key(&self) -> String {
        format!("{}.{}", self.service, self.operation)
    }
}

impl ActionDescriptor {
    /// The `service.operation` key used to identify this action.
    pub fn key(&self) -> String {
        format!("{}.{}", self.service, self.operation)
    }
}

/// A raw provider response, returned by the enumerator/telemetry capabilities before mapping.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct RawResponse {
    /// Opaque response body plus provenance (shaped in G3/G6).
    pub raw: serde_json::Value,
}

/// The structured result of an executed action, before mapping into facts.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ActionResult {
    /// Opaque result body (shaped in G4/G9).
    pub raw: serde_json::Value,
}

// NOTE: graph `Assertion`s a `GraphMapper` produces live in [`crate::graph`] (the attack-core
// ontology), not here — they are canonical domain vocabulary, not opaque seam blobs.

/// The narrow set of capabilities the host grants a single technique step (host-brokering,
/// NFR-SEC5). Content never holds a cloud SDK; it receives only what it declared. Concrete grant
/// mechanics: Increment 2 (G3.3).
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct CapabilityGrant {
    /// The descriptor/capability identifiers this step is permitted to invoke.
    #[serde(default)]
    pub allowed: Vec<String>,
}

impl CapabilityGrant {
    /// Whether the grant permits invoking the named capability/descriptor.
    pub fn permits(&self, capability: &str) -> bool {
        self.allowed.iter().any(|c| c == capability)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn grant_permits_only_declared_capabilities() {
        let grant = CapabilityGrant {
            allowed: vec!["aws.iam.ListUsers".into()],
        };
        assert!(grant.permits("aws.iam.ListUsers"));
        assert!(!grant.permits("aws.iam.CreateUser"));
        assert!(!CapabilityGrant::default().permits("anything"));
    }
}
