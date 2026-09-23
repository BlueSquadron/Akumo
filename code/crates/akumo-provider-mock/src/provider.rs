//! [`MockProvider`] — the Provider Port implemented over a [`MockEnvironment`]. It implements all
//! six capability traits and the aggregate [`Provider`] trait, so the core can run the full loop
//! against it as a stand-in "second provider" (the NFR-EXT7 extensibility gate).

use async_trait::async_trait;

use akumo_domain::error::{AkumoError, Result};
use akumo_domain::graph::Assertion;
use akumo_domain::ids::{ProviderId, Region};
use akumo_domain::ports::provider::{
    ActionExecutor, GraphMapper, IdentityResolver, MetadataProvider, Provider, ResourceEnumerator,
    TelemetryCollector,
};
use akumo_domain::principal::Principal;
use akumo_domain::seam::{
    ActionDescriptor, ActionResult, CapabilityGrant, EnumerationDescriptor, RawResponse,
};

use crate::env::{EnumOutcome, MockEnvironment};

/// A Provider Port adapter backed by a scripted [`MockEnvironment`].
pub struct MockProvider {
    env: MockEnvironment,
}

impl MockProvider {
    /// Wrap a synthetic environment as a provider.
    pub fn new(env: MockEnvironment) -> Self {
        Self { env }
    }
}

#[async_trait]
impl IdentityResolver for MockProvider {
    async fn resolve_current_principal(&self) -> Result<Principal> {
        Ok(self.env.current_principal.clone())
    }
}

#[async_trait]
impl ResourceEnumerator for MockProvider {
    async fn execute(&self, descriptor: &EnumerationDescriptor) -> Result<RawResponse> {
        match self.env.enumerations.get(&descriptor.key()) {
            Some(EnumOutcome::Assertions(assertions)) => Ok(RawResponse {
                raw: serde_json::json!({ "assertions": assertions }),
            }),
            // A denied call is a first-class access gap, not a hard failure (FR-C6).
            Some(EnumOutcome::Denied(reason)) => Err(AkumoError::AccessDenied(reason.clone())),
            // Nothing scripted for this selector — an empty (but successful) result.
            None => Ok(RawResponse {
                raw: serde_json::json!({ "assertions": [] }),
            }),
        }
    }
}

impl GraphMapper for MockProvider {
    fn map(&self, response: &RawResponse) -> Result<Vec<Assertion>> {
        // The mock env authors assertions directly, so mapping is a straight decode.
        match response.raw.get("assertions") {
            Some(value) => Ok(serde_json::from_value::<Vec<Assertion>>(value.clone())?),
            None => Ok(Vec::new()),
        }
    }
}

#[async_trait]
impl ActionExecutor for MockProvider {
    async fn execute(
        &self,
        descriptor: &ActionDescriptor,
        grant: &CapabilityGrant,
    ) -> Result<ActionResult> {
        let key = descriptor.key();
        // Host-brokering (NFR-SEC5): the step may only invoke capabilities it was granted.
        if !grant.permits(&key) {
            return Err(AkumoError::AccessDenied(format!("capability not granted: {key}")));
        }
        match self.env.actions.get(&key) {
            Some(result) => Ok(result.clone()),
            None => Err(AkumoError::NotFound(format!("no scripted action for {key}"))),
        }
    }
}

impl MetadataProvider for MockProvider {
    fn provider_id(&self) -> ProviderId {
        self.env.provider_id.clone()
    }

    fn regions(&self) -> Vec<Region> {
        self.env.regions.clone()
    }
}

// TelemetryCollector uses the default `Unsupported` impl (v1; FR-I2 is v2).
#[async_trait]
impl TelemetryCollector for MockProvider {}

impl Provider for MockProvider {
    fn id(&self) -> ProviderId {
        self.env.provider_id.clone()
    }
    fn identity(&self) -> &dyn IdentityResolver {
        self
    }
    fn enumerator(&self) -> &dyn ResourceEnumerator {
        self
    }
    fn actions(&self) -> &dyn ActionExecutor {
        self
    }
    fn mapper(&self) -> &dyn GraphMapper {
        self
    }
    fn metadata(&self) -> &dyn MetadataProvider {
        self
    }
    fn telemetry(&self) -> &dyn TelemetryCollector {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use akumo_domain::epistemic::Epistemic;
    use akumo_domain::graph::{GraphEdge, GraphNode, NodeKind, Provenance};
    use akumo_domain::ids::Timestamp;
    use akumo_domain::impact::ImpactLevel;
    use akumo_domain::principal::PrincipalKind;

    fn principal(id: &str) -> Principal {
        Principal::new(id, PrincipalKind::Role, ProviderId::new("mock"))
    }

    fn node(id: &str) -> Assertion {
        Assertion::Node(GraphNode {
            id: id.into(),
            kind: NodeKind::Principal,
            attributes: serde_json::Map::new(),
            epistemic: Epistemic::proven(),
            provenance: Provenance::new("mock", Timestamp::from_millis(0)),
        })
    }

    fn edge(from: &str, to: &str) -> Assertion {
        Assertion::Edge(GraphEdge {
            from: from.into(),
            to: to.into(),
            kind: akumo_domain::graph::EdgeKind::CanAssume,
            enabling_permission: Some("sts:AssumeRole".into()),
            epistemic: Epistemic::proven(),
            provenance: Provenance::new("mock", Timestamp::from_millis(0)),
        })
    }

    fn env() -> MockEnvironment {
        MockEnvironment::builder("mock", principal("arn:foothold"))
            .region("mock-region-1")
            .enumeration(
                "iam",
                "ListPrincipals",
                vec![node("arn:foothold"), node("arn:admin"), edge("arn:foothold", "arn:admin")],
            )
            .denied_enumeration("kms", "ListKeys", "access denied: kms:ListKeys")
            .action("iam", "CreateAccessKey", ActionResult { raw: serde_json::json!({ "ok": true }) })
            .build()
    }

    #[tokio::test]
    async fn resolves_current_principal() {
        let p = MockProvider::new(env());
        let cur = p.resolve_current_principal().await.unwrap();
        assert_eq!(cur.id, "arn:foothold");
    }

    #[tokio::test]
    async fn enumerate_then_map_yields_assertions() {
        let p = MockProvider::new(env());
        let desc = EnumerationDescriptor {
            service: "iam".into(),
            operation: "ListPrincipals".into(),
            params: serde_json::Map::new(),
            required_permission: Some("iam:ListRoles".into()),
        };
        let raw = ResourceEnumerator::execute(&p, &desc).await.unwrap();
        let assertions = p.map(&raw).unwrap();
        assert_eq!(assertions.len(), 3);
    }

    #[tokio::test]
    async fn denied_enumeration_is_an_access_gap() {
        let p = MockProvider::new(env());
        let desc = EnumerationDescriptor {
            service: "kms".into(),
            operation: "ListKeys".into(),
            params: serde_json::Map::new(),
            required_permission: None,
        };
        let err = ResourceEnumerator::execute(&p, &desc).await.unwrap_err();
        assert!(err.is_access_gap());
    }

    #[tokio::test]
    async fn action_requires_capability_grant() {
        let p = MockProvider::new(env());
        let desc = ActionDescriptor {
            service: "iam".into(),
            operation: "CreateAccessKey".into(),
            params: serde_json::Map::new(),
            impact: ImpactLevel::MutatingReversible,
            required_permission: Some("iam:CreateAccessKey".into()),
        };

        // Without a grant: refused (host-brokering).
        let ungranted = CapabilityGrant::default();
        assert!(ActionExecutor::execute(&p, &desc, &ungranted).await.is_err());

        // With the grant: executes.
        let granted = CapabilityGrant { allowed: vec!["iam.CreateAccessKey".into()] };
        let result = ActionExecutor::execute(&p, &desc, &granted).await.unwrap();
        assert_eq!(result.raw["ok"], true);
    }
}
