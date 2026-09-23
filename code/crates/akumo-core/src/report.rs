//! Reporting, evidence & export (FR-J). Reports are rendered from **ledger projections** (SPEC-D14)
//! — never a separate narrative that could drift from what happened. They come in a human-readable
//! (Markdown) and a machine-readable (JSON) form, with an **executive summary** and a **technical
//! detail** view (FR-J5), and computed paths export to **STIX 2.1 / MITRE Attack Flow** (FR-J7).
//!
//! Redaction (FR-J6): the report model holds no raw secrets — loot is represented as redacted
//! references, and enumeration/execution never place secret material in these projections.
//! Reproducibility (FR-J2, ADR-0010) is the ledger replay itself (see `projection`); encryption of
//! stored secrets (NFR-SEC1) is a packaging-phase concern.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use akumo_domain::event::{event_type, EventEnvelope};
use akumo_domain::graph::CoverageGap;

use akumo_dsl::Catalog;

use crate::engagement::{EngagementStatus, EngagementView};
use crate::planner::AttackPath;
use crate::projection::Projection;

/// A serializable summary of a computed path (the `PathComputed` event payload and a report row).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct PathSummary {
    /// The objective this path reaches.
    pub objective: String,
    /// Number of hops.
    pub steps: usize,
    /// `Proven` or `Candidate`.
    pub confidence: String,
    /// Search cost under the active ranking.
    pub cost: u64,
    /// Whether every action on the path is reversible.
    pub reversible: bool,
    /// The most impactful action.
    pub max_impact: String,
    /// The human-readable explanation.
    pub explanation: String,
}

impl PathSummary {
    /// Summarize a planner path toward `objective`.
    pub fn from_path(objective: impl Into<String>, path: &AttackPath) -> Self {
        Self {
            objective: objective.into(),
            steps: path.len(),
            confidence: format!("{:?}", path.confidence),
            cost: path.cost,
            reversible: path.reversible,
            max_impact: path.max_impact.to_string(),
            explanation: path.explain(),
        }
    }
}

/// The engagement report, folded from the ledger.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EngagementReport {
    /// The engagement id.
    pub engagement_id: String,
    /// The provider, if known.
    pub provider: Option<String>,
    /// Lifecycle status (`open`/`killed`/`closed`/`unknown`).
    pub status: String,
    /// Number of graph facts asserted during enumeration.
    pub facts_asserted: usize,
    /// Blind spots (denied/unseen).
    pub coverage_gaps: Vec<CoverageGap>,
    /// Computed paths (from `PathComputed` events).
    pub computed_paths: Vec<PathSummary>,
    /// Technique ids exercised.
    pub techniques_used: Vec<String>,
    /// Steps detonated.
    pub steps_detonated: usize,
    /// Steps reverted.
    pub steps_reverted: usize,
    /// Execution failures.
    pub failures: usize,
    /// MITRE ATT&CK coverage (resolved from the catalog, if provided).
    pub mitre_coverage: Vec<String>,
}

/// Build an engagement report by folding its event stream. If a `catalog` is supplied, MITRE
/// coverage is resolved from the techniques exercised.
pub fn build_report(events: &[EventEnvelope], catalog: Option<&Catalog>) -> EngagementReport {
    let state = EngagementView::replay(events.iter());

    let mut report = EngagementReport {
        engagement_id: state.as_ref().map(|s| s.id.to_string()).unwrap_or_default(),
        provider: state.as_ref().map(|s| s.provider.to_string()),
        status: state
            .as_ref()
            .map(|s| status_str(s.status).to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        facts_asserted: 0,
        coverage_gaps: Vec::new(),
        computed_paths: Vec::new(),
        techniques_used: Vec::new(),
        steps_detonated: 0,
        steps_reverted: 0,
        failures: 0,
        mitre_coverage: Vec::new(),
    };

    let mut technique_set: BTreeSet<String> = BTreeSet::new();
    for event in events {
        match event.event_type.as_str() {
            event_type::FACT_ASSERTED => report.facts_asserted += 1,
            event_type::ACCESS_DENIED => {
                if let Ok(gap) = serde_json::from_value::<CoverageGap>(event.payload.clone()) {
                    report.coverage_gaps.push(gap);
                }
            }
            event_type::PATH_COMPUTED => {
                if let Ok(path) = serde_json::from_value::<PathSummary>(event.payload.clone()) {
                    report.computed_paths.push(path);
                }
            }
            event_type::DRY_RUN_PERFORMED => {
                if let Some(id) = event.payload.get("technique").and_then(|v| v.as_str()) {
                    technique_set.insert(id.to_string());
                }
            }
            event_type::STEP_DETONATED => report.steps_detonated += 1,
            event_type::STEP_REVERTED => report.steps_reverted += 1,
            event_type::EXECUTION_FAILED => report.failures += 1,
            _ => {}
        }
    }

    report.techniques_used = technique_set.iter().cloned().collect();

    if let Some(catalog) = catalog {
        let mut mitre: BTreeSet<String> = BTreeSet::new();
        for id in &report.techniques_used {
            if let Some(technique) = catalog.get(id) {
                for m in &technique.metadata.mitre {
                    mitre.insert(m.clone());
                }
            }
        }
        report.mitre_coverage = mitre.into_iter().collect();
    }

    report
}

impl EngagementReport {
    /// Machine-readable JSON (FR-J4).
    pub fn to_json(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default()
    }

    /// Human-readable Markdown with an executive summary and a technical detail view (FR-J5).
    pub fn to_markdown(&self) -> String {
        let mut md = String::new();
        md.push_str(&format!("# Akumo Engagement Report — {}\n\n", self.engagement_id));

        md.push_str("## Executive summary\n\n");
        md.push_str(&format!(
            "- Provider: {}\n- Status: {}\n- Facts discovered: {}\n- Blind spots: {}\n\
             - Attack paths computed: {}\n- Techniques exercised: {}\n- Steps detonated / reverted: {} / {}\n\
             - Execution failures: {}\n- MITRE ATT&CK coverage: {}\n\n",
            self.provider.as_deref().unwrap_or("unknown"),
            self.status,
            self.facts_asserted,
            self.coverage_gaps.len(),
            self.computed_paths.len(),
            self.techniques_used.len(),
            self.steps_detonated,
            self.steps_reverted,
            self.failures,
            if self.mitre_coverage.is_empty() {
                "none".to_string()
            } else {
                self.mitre_coverage.join(", ")
            },
        ));

        md.push_str("## Technical detail\n\n");

        md.push_str("### Computed attack paths\n\n");
        if self.computed_paths.is_empty() {
            md.push_str("_None._\n\n");
        } else {
            for (i, path) in self.computed_paths.iter().enumerate() {
                md.push_str(&format!(
                    "**Path {} → {}** (confidence {}, {} steps, reversible {}, max impact {})\n\n```\n{}\n```\n\n",
                    i + 1,
                    path.objective,
                    path.confidence,
                    path.steps,
                    path.reversible,
                    path.max_impact,
                    path.explanation,
                ));
            }
        }

        md.push_str("### Techniques exercised\n\n");
        if self.techniques_used.is_empty() {
            md.push_str("_None._\n\n");
        } else {
            for id in &self.techniques_used {
                md.push_str(&format!("- {id}\n"));
            }
            md.push('\n');
        }

        md.push_str("### Blind spots (coverage gaps)\n\n");
        if self.coverage_gaps.is_empty() {
            md.push_str("_None recorded._\n\n");
        } else {
            for gap in &self.coverage_gaps {
                md.push_str(&format!("- `{}` — {}\n", gap.scope, gap.reason));
            }
            md.push('\n');
        }

        md.push_str("> Secrets and loot are redacted by default.\n");
        md
    }
}

/// Export a computed path as a STIX 2.1 / MITRE Attack Flow bundle (FR-J7). The structure is
/// Attack-Flow-aligned (a `attack-flow` plus one `attack-action` per hop, linked by `effect_refs`);
/// ids are deterministic slugs rather than random UUIDs in v1.
pub fn attack_flow(objective: &str, path: &AttackPath) -> serde_json::Value {
    use crate::planner::StepKind;

    let created = "2024-01-01T00:00:00.000Z";
    let flow_id = format!("attack-flow--akumo-{}", slug(objective));
    let action_id = |i: usize| format!("attack-action--akumo-{}-{}", slug(objective), i);

    let mut objects: Vec<serde_json::Value> = Vec::new();
    let start_ref = if path.steps.is_empty() {
        serde_json::Value::Null
    } else {
        serde_json::Value::String(action_id(0))
    };

    objects.push(serde_json::json!({
        "type": "attack-flow",
        "spec_version": "2.1",
        "id": flow_id,
        "created": created,
        "modified": created,
        "name": format!("Akumo path to {objective}"),
        "description": format!("confidence {:?}, {} steps, reversible {}", path.confidence, path.len(), path.reversible),
        "start_refs": if start_ref.is_null() { serde_json::json!([]) } else { serde_json::json!([start_ref]) },
    }));

    for (i, step) in path.steps.iter().enumerate() {
        let (name, technique_id) = match &step.kind {
            StepKind::Technique(id) => (format!("technique {id}"), Some(id.clone())),
            StepKind::Edge(_) => (step.enabling.clone(), None),
        };
        let effect_refs = if i + 1 < path.steps.len() {
            serde_json::json!([action_id(i + 1)])
        } else {
            serde_json::json!([])
        };
        objects.push(serde_json::json!({
            "type": "attack-action",
            "spec_version": "2.1",
            "id": action_id(i),
            "created": created,
            "modified": created,
            "name": name,
            "technique_id": technique_id,
            "description": format!("{} → {} ({:?})", step.principal, step.produced, step.epistemic.status),
            "effect_refs": effect_refs,
        }));
    }

    serde_json::json!({
        "type": "bundle",
        "id": format!("bundle--akumo-{}", slug(objective)),
        "objects": objects,
    })
}

fn status_str(status: EngagementStatus) -> &'static str {
    match status {
        EngagementStatus::Open => "open",
        EngagementStatus::Killed => "killed",
        EngagementStatus::Closed => "closed",
    }
}

fn slug(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use akumo_domain::epistemic::Epistemic;
    use akumo_domain::graph::{Assertion, EdgeKind, GraphEdge, GraphNode, NodeKind, Provenance};
    use akumo_domain::ids::{Actor, EngagementId, ProviderId, Timestamp};
    use akumo_domain::impact::ImpactLevel;
    use akumo_domain::ports::EventStore;
    use akumo_domain::principal::{Principal, PrincipalKind};
    use akumo_domain::scope::{Scope, ScopeSelector};
    use akumo_domain::seam::ActionResult;

    use akumo_dsl::{parse_technique, Catalog};
    use akumo_dsl::script::UnsupportedScriptHost;
    use akumo_ledger::FileEventStore;
    use akumo_provider_mock::{MockEnvironment, MockProvider};

    use crate::engagement::EngagementManager;
    use crate::execution::{ExecutionConfig, ExecutionEngine};
    use crate::graph::AttackGraph;
    use crate::planner::{Objective, Planner, PlannerConfig};

    const TECH: &str = r#"
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
      params: {}
    revert:
      service: iam
      operation: DeleteAccessKey
      params: {}
"#;

    #[tokio::test]
    async fn report_folds_ledger_and_resolves_mitre() {
        let root = std::env::temp_dir().join(format!(
            "akumo-report-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let store = FileEventStore::open(&root).unwrap();
        let provider = MockProvider::new(
            MockEnvironment::builder(
                "mock",
                Principal::new("arn:op", PrincipalKind::Role, ProviderId::new("mock")),
            )
            .action("iam", "CreateAccessKey", ActionResult { raw: serde_json::json!({}) })
            .action("iam", "DeleteAccessKey", ActionResult { raw: serde_json::json!({}) })
            .build(),
        );
        let host = UnsupportedScriptHost;

        let id = EngagementId::new("eng-report");
        let mgr = EngagementManager::new(&store);
        mgr.open(
            id.clone(),
            Scope::new(vec![ScopeSelector::new("account", "1")]),
            ProviderId::new("mock"),
            "cred",
            Actor::new("op"),
            true,
        )
        .await
        .unwrap();
        mgr.record_consent(&id, Actor::new("op"), ImpactLevel::MutatingReversible, true, None)
            .await
            .unwrap();

        let technique = parse_technique(TECH).unwrap();
        ExecutionEngine::new(&store, &provider, &host)
            .detonate(
                &id,
                Actor::new("op"),
                &technique,
                &serde_json::Map::new(),
                ExecutionConfig::default(),
            )
            .await
            .unwrap();

        let mut catalog = Catalog::new();
        catalog.insert(technique).unwrap();

        let events = store.read_stream(&id).await.unwrap();
        let report = build_report(&events, Some(&catalog));
        assert_eq!(report.status, "open");
        assert_eq!(report.provider.as_deref(), Some("mock"));
        assert!(report.techniques_used.contains(&"aws.iam.create-access-key".to_string()));
        assert_eq!(report.steps_detonated, 1);
        assert_eq!(report.mitre_coverage, vec!["T1098".to_string()]);

        let md = report.to_markdown();
        assert!(md.contains("Executive summary"));
        assert!(md.contains("Technical detail"));
        assert!(serde_json::from_str::<serde_json::Value>(&report.to_json()).is_ok());

        std::fs::remove_dir_all(&root).ok();
    }

    #[test]
    fn attack_flow_export_has_bundle_and_actions() {
        let graph = AttackGraph::from_assertions(vec![
            Assertion::Node(GraphNode {
                id: "f".into(),
                kind: NodeKind::Principal,
                attributes: serde_json::Map::new(),
                epistemic: Epistemic::proven(),
                provenance: Provenance::new("t", Timestamp::from_millis(0)),
            }),
            Assertion::Node(GraphNode {
                id: "a".into(),
                kind: NodeKind::Principal,
                attributes: {
                    let mut m = serde_json::Map::new();
                    m.insert("admin".into(), serde_json::Value::Bool(true));
                    m
                },
                epistemic: Epistemic::proven(),
                provenance: Provenance::new("t", Timestamp::from_millis(0)),
            }),
            Assertion::Edge(GraphEdge {
                from: "f".into(),
                to: "a".into(),
                kind: EdgeKind::CanAssume,
                enabling_permission: Some("sts:AssumeRole".into()),
                epistemic: Epistemic::proven(),
                provenance: Provenance::new("t", Timestamp::from_millis(0)),
            }),
        ]);
        let paths = Planner::plan(
            &graph,
            &["f".to_string()],
            &Objective::ReachAdmin,
            &[],
            &PlannerConfig::default(),
        );
        let flow = attack_flow("reach-admin", &paths[0]);
        assert_eq!(flow["type"], "bundle");
        let objects = flow["objects"].as_array().unwrap();
        assert!(objects.len() >= 2); // the flow + at least one action
        assert_eq!(objects[0]["type"], "attack-flow");
    }
}
