//! The attack-core ontology (ADR-0015 / SPEC-D4 = layered). A **compact, provider-neutral** set of
//! node/edge types the planner reasons over; provider-specific inventory hangs off nodes as
//! `attributes` (the layered model — no provider vocabulary leaks into the core, EXR-6).
//!
//! These types are the shared seam vocabulary: a provider adapter's `GraphMapper` produces
//! [`Assertion`]s, enumeration records them as events, and the graph projection
//! ([`akumo_core::graph`]) folds them into a queryable graph. Every attack-relevant assertion
//! carries an [`Epistemic`] status (ADR-0017) and [`Provenance`] (FR-C4); every edge records the
//! permission/trust that enables it (FR-D3).

use serde::{Deserialize, Serialize};

use crate::epistemic::Epistemic;
use crate::ids::Timestamp;

/// A provider-native stable node identifier (e.g. an AWS ARN). Opaque to the core.
pub type NodeId = String;

/// The attack-core node kinds (SPEC-D4). Small and attack-relevant by design so the set stays
/// stable across future providers.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NodeKind {
    /// A user / role / service-identity / group.
    Principal,
    /// A credential (key, token, session).
    Credential,
    /// A typed resource (bucket, function, database, …).
    Resource,
    /// A set of permissions (policy/grant).
    PermissionSet,
    /// A federated/external identity (IdP).
    ExternalIdentity,
}

/// The attack-core edge kinds (SPEC-D4, spec §5.7). The last two are **derived** privesc/lateral
/// edges the planner both consumes and — via technique effects — produces.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum EdgeKind {
    /// Principal is a member of a group.
    MemberOf,
    /// Principal holds a permission set.
    HasPermission,
    /// Principal can assume a role.
    CanAssume,
    /// A trust relationship (role trust policy, federation).
    Trusts,
    /// Principal holds a credential.
    HasCredential,
    /// External identity is federated as a principal.
    FederatedAs,
    /// Derived: principal can access a resource (privilege).
    CanAccess,
    /// Derived: principal can escalate to another principal.
    CanEscalateTo,
}

/// Where a fact came from (FR-C4): the operation/descriptor that produced it, the permission it
/// required, and when it was observed.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Provenance {
    /// The descriptor/operation that produced this fact.
    pub source: String,
    /// The permission the producing call required, if known.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub permission: Option<String>,
    /// When the fact was observed.
    pub observed_at: Timestamp,
}

impl Provenance {
    /// Convenience constructor.
    pub fn new(source: impl Into<String>, observed_at: Timestamp) -> Self {
        Self {
            source: source.into(),
            permission: None,
            observed_at,
        }
    }
}

/// A node in the attack graph, plus its provider inventory (`attributes`), epistemic status, and
/// provenance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphNode {
    /// Provider-native stable id.
    pub id: NodeId,
    /// The attack-core kind.
    pub kind: NodeKind,
    /// Provider inventory layer — arbitrary provider-supplied attributes.
    #[serde(default)]
    pub attributes: serde_json::Map<String, serde_json::Value>,
    /// What we know about this node's existence.
    pub epistemic: Epistemic,
    /// Where this node came from.
    pub provenance: Provenance,
}

/// An edge in the attack graph. Carries the enabling permission/trust (FR-D3), its epistemic
/// status, and provenance.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct GraphEdge {
    /// Source node id.
    pub from: NodeId,
    /// Destination node id.
    pub to: NodeId,
    /// The edge kind.
    pub kind: EdgeKind,
    /// The permission/trust that makes this edge exist (explainability, FR-E3).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub enabling_permission: Option<String>,
    /// What we know about this edge.
    pub epistemic: Epistemic,
    /// Where this edge came from.
    pub provenance: Provenance,
}

/// A single graph assertion emitted by a `GraphMapper` and recorded as a `FactAsserted` event.
/// Internally tagged so it round-trips cleanly through the event payload.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "assertion", rename_all = "snake_case")]
pub enum Assertion {
    /// Assert (or refine) a node.
    Node(GraphNode),
    /// Assert (or refine) an edge.
    Edge(GraphEdge),
}

/// A blind spot: something enumeration could not observe (denied/throttled/unseen). Recorded as an
/// `AccessDenied` event and surfaced in the coverage map (FR-C6). "Unknown ≠ absent."
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct CoverageGap {
    /// What we tried to observe (operation/target).
    pub scope: String,
    /// Why it could not be observed.
    pub reason: String,
    /// When the gap was recorded.
    pub observed_at: Timestamp,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edge_kind_serializes_screaming_snake() {
        let json = serde_json::to_string(&EdgeKind::CanEscalateTo).unwrap();
        assert_eq!(json, "\"CAN_ESCALATE_TO\"");
    }

    #[test]
    fn assertion_roundtrips_through_json() {
        let node = GraphNode {
            id: "arn:p".into(),
            kind: NodeKind::Principal,
            attributes: serde_json::Map::new(),
            epistemic: Epistemic::proven(),
            provenance: Provenance::new("iam:GetUser", Timestamp::from_millis(1)),
        };
        let a = Assertion::Node(node);
        let value = serde_json::to_value(&a).unwrap();
        assert_eq!(value["assertion"], "node");
        let back: Assertion = serde_json::from_value(value).unwrap();
        assert_eq!(back, a);
    }
}
