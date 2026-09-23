//! The library facade (FR-K3): one entry point that wires the ports and exposes the unified loop as
//! high-level operations. The CLI and CI adapters (G15) drive this; so can any embedding tool. The
//! facade holds no mutable session state — everything lives in the ledger, so operations are
//! stateless over a persistent engagement (cleanly solving FR-A6).
//!
//! Stability: this is the intended public surface. It is versioned with the crate (1.0.0); breaking
//! changes bump the major version.

use akumo_domain::error::Result;
use akumo_domain::ids::{Actor, EngagementId, ProviderId};
use akumo_domain::ports::{EventStore, Provider};
use akumo_domain::scope::Scope;
use akumo_domain::seam::EnumerationDescriptor;

use akumo_dsl::schema::Technique;
use akumo_dsl::script::ScriptHost;
use akumo_dsl::Catalog;

use crate::engagement::{authcheck, AuthCheckReport, EngagementManager, EngagementState};
use crate::enumeration::{EnumerationService, EnumerationSummary};
use crate::execution::{
    ExecutionConfig, ExecutionEngine, ExecutionMode, ExecutionOutcome, RevertReport,
};
use crate::graph::{AttackGraph, GraphProjection};
use crate::planner::{AttackPath, Objective, PlannerConfig, PlanningAction, Planner};
use crate::projection::Projection;
use crate::report::{build_report, EngagementReport};

/// The Akumo facade over a persistence and provider seam.
pub struct Akumo<'a> {
    store: &'a dyn EventStore,
    provider: &'a dyn Provider,
    script_host: &'a dyn ScriptHost,
}

impl<'a> Akumo<'a> {
    /// Wire the facade to a store, provider, and script host.
    pub fn new(
        store: &'a dyn EventStore,
        provider: &'a dyn Provider,
        script_host: &'a dyn ScriptHost,
    ) -> Self {
        Self { store, provider, script_host }
    }

    /// Open an authorized engagement (FR-A).
    pub async fn open_engagement(
        &self,
        id: EngagementId,
        scope: Scope,
        provider: ProviderId,
        credential_ref: impl Into<String>,
        actor: Actor,
        authorization_affirmed: bool,
    ) -> Result<EngagementId> {
        EngagementManager::new(self.store)
            .open(id, scope, provider, credential_ref, actor, authorization_affirmed)
            .await
    }

    /// Close an engagement.
    pub async fn close_engagement(&self, engagement: &EngagementId, actor: Actor) -> Result<()> {
        EngagementManager::new(self.store).close(engagement, actor).await
    }

    /// Invoke the kill-switch on an engagement (FR-A4).
    pub async fn kill_switch(
        &self,
        engagement: &EngagementId,
        actor: Actor,
        reason: impl Into<String>,
    ) -> Result<()> {
        EngagementManager::new(self.store)
            .invoke_kill_switch(engagement, actor, reason)
            .await
    }

    /// Record a consent decision (FR-A3).
    pub async fn record_consent(
        &self,
        engagement: &EngagementId,
        actor: Actor,
        impact: akumo_domain::impact::ImpactLevel,
        granted: bool,
    ) -> Result<()> {
        EngagementManager::new(self.store)
            .record_consent(engagement, actor, impact, granted, None)
            .await
    }

    /// Load an engagement's current state.
    pub async fn engagement_state(
        &self,
        engagement: &EngagementId,
    ) -> Result<Option<EngagementState>> {
        EngagementManager::new(self.store).load(engagement).await
    }

    /// A dry authorization check (FR-A5).
    pub async fn authcheck(&self) -> Result<AuthCheckReport> {
        authcheck(self.provider).await
    }

    /// Enumerate the target within an engagement (FR-C).
    pub async fn enumerate(
        &self,
        engagement: &EngagementId,
        actor: Actor,
        descriptors: &[EnumerationDescriptor],
    ) -> Result<EnumerationSummary> {
        EnumerationService::new(self.store, self.provider)
            .enumerate(engagement, actor, descriptors)
            .await
    }

    /// Fold the current attack graph for an engagement (FR-D).
    pub async fn graph(&self, engagement: &EngagementId) -> Result<AttackGraph> {
        let events = self.store.read_stream(engagement).await?;
        Ok(GraphProjection::replay(events.iter()))
    }

    /// Compute attack paths toward an objective from the current principal (FR-E).
    pub async fn paths(
        &self,
        engagement: &EngagementId,
        objective: &Objective,
        actions: &[PlanningAction],
        config: &PlannerConfig,
    ) -> Result<Vec<AttackPath>> {
        let start = self.provider.identity().resolve_current_principal().await?;
        let graph = self.graph(engagement).await?;
        Ok(Planner::plan(&graph, &[start.id], objective, actions, config))
    }

    /// Preview a technique (dry-run + blast radius; no mutation) (FR-G2).
    pub async fn preview(
        &self,
        engagement: &EngagementId,
        actor: Actor,
        technique: &Technique,
        inputs: &serde_json::Map<String, serde_json::Value>,
    ) -> Result<ExecutionOutcome> {
        ExecutionEngine::new(self.store, self.provider, self.script_host)
            .detonate(
                engagement,
                actor,
                technique,
                inputs,
                ExecutionConfig { mode: ExecutionMode::DryRun, ..Default::default() },
            )
            .await
    }

    /// Detonate a technique through the safe lifecycle (FR-G).
    pub async fn run(
        &self,
        engagement: &EngagementId,
        actor: Actor,
        technique: &Technique,
        inputs: &serde_json::Map<String, serde_json::Value>,
        config: ExecutionConfig,
    ) -> Result<ExecutionOutcome> {
        ExecutionEngine::new(self.store, self.provider, self.script_host)
            .detonate(engagement, actor, technique, inputs, config)
            .await
    }

    /// Revert outstanding detonations for an engagement (FR-G4).
    pub async fn revert(&self, engagement: &EngagementId, actor: Actor) -> Result<RevertReport> {
        ExecutionEngine::new(self.store, self.provider, self.script_host)
            .revert(engagement, actor, None)
            .await
    }

    /// Build the engagement report (FR-J).
    pub async fn report(
        &self,
        engagement: &EngagementId,
        catalog: Option<&Catalog>,
    ) -> Result<EngagementReport> {
        let events = self.store.read_stream(engagement).await?;
        Ok(build_report(&events, catalog))
    }
}

#[cfg(test)]
mod tests {
    //! The end-to-end vertical slice (G10) — the entire unified loop driven through the [`Akumo`]
    //! facade against the **Mock** provider. This is also the **extensibility gate** (NFR-EXT7,
    //! release-blocking): the same provider-blind core runs the full loop with the Mock standing in
    //! as a "second provider" and **zero core changes** (enforced structurally by `xtask dep-lint`).

    use super::*;

    use akumo_domain::epistemic::Epistemic;
    use akumo_domain::graph::{Assertion, EdgeKind, GraphEdge, GraphNode, NodeKind, Provenance};
    use akumo_domain::ids::Timestamp;
    use akumo_domain::impact::ImpactLevel;
    use akumo_domain::principal::{Principal, PrincipalKind};
    use akumo_domain::scope::ScopeSelector;
    use akumo_domain::seam::ActionResult;

    use akumo_dsl::parse_technique;
    use akumo_dsl::script::UnsupportedScriptHost;
    use akumo_ledger::FileEventStore;
    use akumo_provider_mock::{MockEnvironment, MockProvider};

    use crate::engagement::EngagementStatus;

    fn prov() -> Provenance {
        Provenance::new("iam.ListPrincipals", Timestamp::from_millis(0))
    }

    fn node(id: &str, admin: bool) -> Assertion {
        let mut attributes = serde_json::Map::new();
        if admin {
            attributes.insert("admin".into(), serde_json::Value::Bool(true));
        }
        Assertion::Node(GraphNode {
            id: id.into(),
            kind: NodeKind::Principal,
            attributes,
            epistemic: Epistemic::proven(),
            provenance: prov(),
        })
    }

    fn edge(from: &str, to: &str) -> Assertion {
        Assertion::Edge(GraphEdge {
            from: from.into(),
            to: to.into(),
            kind: EdgeKind::CanAssume,
            enabling_permission: Some("sts:AssumeRole".into()),
            epistemic: Epistemic::proven(),
            provenance: prov(),
        })
    }

    const CREATE_KEY: &str = r#"
metadata:
  id: aws.iam.create-access-key
  name: Create IAM Access Key
  description: Creates a long-term access key for a target IAM user.
  version: "1.0.0"
  provider: aws
  mitre: ["T1098"]
  impact: mutating-reversible
  expected_telemetry:
    - source: cloudtrail
      event_name: CreateAccessKey
steps:
  - id: create-key
    body:
      type: call
      service: iam
      operation: CreateAccessKey
      params: { UserName: "$user" }
    revert:
      service: iam
      operation: DeleteAccessKey
      params: { UserName: "$user" }
"#;

    #[tokio::test]
    async fn extensibility_gate_full_loop_on_mock() {
        // A synthetic environment: a foothold that can assume an admin role, plus a mutating action
        // and its compensation.
        let env = MockEnvironment::builder(
            "mock",
            Principal::new("arn:foothold", PrincipalKind::Role, ProviderId::new("mock")),
        )
        .region("mock-region-1")
        .enumeration(
            "iam",
            "ListPrincipals",
            vec![node("arn:foothold", false), node("arn:admin", true), edge("arn:foothold", "arn:admin")],
        )
        .action("iam", "CreateAccessKey", ActionResult { raw: serde_json::json!({ "AccessKeyId": "AKIA" }) })
        .action("iam", "DeleteAccessKey", ActionResult { raw: serde_json::json!({}) })
        .build();

        let root = std::env::temp_dir().join(format!(
            "akumo-slice-{}",
            std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_nanos()
        ));
        let store = FileEventStore::open(&root).unwrap();
        let provider = MockProvider::new(env);
        let host = UnsupportedScriptHost;
        let akumo = Akumo::new(&store, &provider, &host);
        let actor = Actor::new("tester");

        // 1. authorize
        let who = akumo.authcheck().await.unwrap();
        assert_eq!(who.principal.id, "arn:foothold");

        let id = EngagementId::new("eng-slice");
        akumo
            .open_engagement(
                id.clone(),
                Scope::new(vec![ScopeSelector::new("account", "1")]),
                ProviderId::new("mock"),
                "cred",
                actor.clone(),
                true,
            )
            .await
            .unwrap();

        // 2. enumerate → 3. graph
        let summary = akumo
            .enumerate(&id, actor.clone(), &[EnumerationDescriptor {
                service: "iam".into(),
                operation: "ListPrincipals".into(),
                params: serde_json::Map::new(),
                required_permission: None,
            }])
            .await
            .unwrap();
        assert_eq!(summary.facts_asserted, 3);
        let graph = akumo.graph(&id).await.unwrap();
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);

        // 4. plan
        let paths = akumo
            .paths(&id, &Objective::ReachAdmin, &[], &PlannerConfig::default())
            .await
            .unwrap();
        assert!(!paths.is_empty(), "should find a foothold→admin path");
        assert_eq!(paths[0].len(), 1);

        // 5. consent + execute
        akumo
            .record_consent(&id, actor.clone(), ImpactLevel::MutatingReversible, true)
            .await
            .unwrap();
        let technique = parse_technique(CREATE_KEY).unwrap();
        let mut inputs = serde_json::Map::new();
        inputs.insert("user".into(), serde_json::json!("arn:target"));
        let outcome = akumo
            .run(&id, actor.clone(), &technique, &inputs, ExecutionConfig::default())
            .await
            .unwrap();
        assert_eq!(outcome.steps_detonated, 1);
        assert_eq!(outcome.steps_verified, 1);

        // 6. revert
        let report = akumo.revert(&id, actor.clone()).await.unwrap();
        assert_eq!(report.reverted, 1);
        assert!(report.failed.is_empty());

        // 7. report
        let mut catalog = akumo_dsl::Catalog::new();
        catalog.insert(technique).unwrap();
        let engagement_report = akumo.report(&id, Some(&catalog)).await.unwrap();
        assert!(engagement_report
            .techniques_used
            .contains(&"aws.iam.create-access-key".to_string()));
        assert_eq!(engagement_report.mitre_coverage, vec!["T1098".to_string()]);

        // 8. close
        akumo.close_engagement(&id, actor).await.unwrap();
        let state = akumo.engagement_state(&id).await.unwrap().unwrap();
        assert_eq!(state.status, EngagementStatus::Closed);

        std::fs::remove_dir_all(&root).ok();
    }
}
