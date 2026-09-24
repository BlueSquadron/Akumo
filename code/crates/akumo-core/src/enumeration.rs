//! The Enumeration Service (FR-C): descriptor-driven discovery that records everything it learns as
//! ledger events, from which the attack graph is folded (G4). It degrades gracefully under partial
//! permissions — a denied call becomes a coverage gap, never a hard failure (FR-C6).
//!
//! Flow per descriptor: `ResourceEnumerator::execute` → `GraphMapper::map` → one `FactAsserted`
//! event per assertion. A denied call emits an `AccessDenied` (coverage gap) event and continues.
//! The run is bracketed by `EnumerationStarted` / `EnumerationCompleted`.
//!
//! v1 scope of this increment: orchestration (G6.1), partial-permission degradation (G6.5),
//! provenance carried on each assertion by the mapper (G6.4), and a per-run call cache (G6.2).
//! Bounded concurrency (G6.3) and cross-invocation caching are refinements layered on later.

use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use akumo_domain::error::Result;
use akumo_domain::event::event_type;
use akumo_domain::graph::CoverageGap;
use akumo_domain::ids::{Actor, EngagementId, Timestamp};
use akumo_domain::ports::{EventStore, Provider};
use akumo_domain::seam::{EnumerationDescriptor, RawResponse};

use crate::engagement::EngagementManager;
use crate::journal::{now_millis, Journal};

/// A summary of one enumeration run (also the `EnumerationCompleted` payload).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct EnumerationSummary {
    /// How many descriptors were processed.
    pub descriptors_run: usize,
    /// How many graph facts were asserted.
    pub facts_asserted: usize,
    /// How many coverage gaps (denied calls) were recorded.
    pub coverage_gaps: usize,
}

/// Orchestrates enumeration for an engagement over a provider.
pub struct EnumerationService<'a> {
    store: &'a dyn EventStore,
    provider: &'a dyn Provider,
}

impl<'a> EnumerationService<'a> {
    /// Create an enumeration service.
    pub fn new(store: &'a dyn EventStore, provider: &'a dyn Provider) -> Self {
        Self { store, provider }
    }

    /// Run a batch of enumeration descriptors within an active engagement, recording facts and gaps
    /// as events. Returns a summary.
    pub async fn enumerate(
        &self,
        engagement: &EngagementId,
        actor: Actor,
        descriptors: &[EnumerationDescriptor],
    ) -> Result<EnumerationSummary> {
        // The engagement must exist and be active (not killed/closed).
        let ctx = EngagementManager::new(self.store)
            .context(engagement)
            .await?;
        ctx.ensure_active()?;

        let journal = Journal::new(self.store);
        journal
            .record(
                engagement,
                actor.clone(),
                event_type::ENUMERATION_STARTED,
                serde_json::json!({ "descriptor_count": descriptors.len() }),
            )
            .await?;

        // Per-run cache: avoid re-issuing an identical provider call (FR-C2, NFR-PERF2).
        let mut cache: HashMap<String, RawResponse> = HashMap::new();
        let mut facts_asserted = 0usize;
        let mut coverage_gaps = 0usize;

        for descriptor in descriptors {
            let cache_key = format!(
                "{}::{}",
                descriptor.key(),
                serde_json::to_string(&descriptor.params).unwrap_or_default()
            );

            let raw = match cache.get(&cache_key) {
                Some(cached) => cached.clone(),
                None => match self.provider.enumerator().execute(descriptor).await {
                    Ok(raw) => {
                        cache.insert(cache_key, raw.clone());
                        raw
                    }
                    // Denied → a coverage gap, not a failure (FR-C6). Continue with the rest.
                    Err(e) if e.is_access_gap() => {
                        let gap = CoverageGap {
                            scope: descriptor.key(),
                            reason: e.to_string(),
                            observed_at: Timestamp::from_millis(now_millis()),
                        };
                        journal
                            .record(
                                engagement,
                                actor.clone(),
                                event_type::ACCESS_DENIED,
                                serde_json::to_value(&gap)?,
                            )
                            .await?;
                        coverage_gaps += 1;
                        continue;
                    }
                    Err(e) => return Err(e),
                },
            };

            for assertion in self.provider.mapper().map(&raw)? {
                journal
                    .record(
                        engagement,
                        actor.clone(),
                        event_type::FACT_ASSERTED,
                        serde_json::to_value(&assertion)?,
                    )
                    .await?;
                facts_asserted += 1;
            }
        }

        let summary = EnumerationSummary {
            descriptors_run: descriptors.len(),
            facts_asserted,
            coverage_gaps,
        };
        journal
            .record(
                engagement,
                actor,
                event_type::ENUMERATION_COMPLETED,
                serde_json::to_value(&summary)?,
            )
            .await?;
        Ok(summary)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::GraphProjection;
    use crate::projection::Projection;

    use akumo_domain::epistemic::Epistemic;
    use akumo_domain::graph::{Assertion, EdgeKind, GraphEdge, GraphNode, NodeKind, Provenance};
    use akumo_domain::ids::ProviderId;
    use akumo_domain::ports::EventStore;
    use akumo_domain::principal::{Principal, PrincipalKind};
    use akumo_domain::scope::{Scope, ScopeSelector};

    use akumo_ledger::FileEventStore;
    use akumo_provider_mock::{MockEnvironment, MockProvider};

    fn temp_root(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let mut p = std::env::temp_dir();
        p.push(format!("akumo-enum-test-{tag}-{nanos}"));
        p
    }

    fn node(id: &str) -> Assertion {
        Assertion::Node(GraphNode {
            id: id.into(),
            kind: NodeKind::Principal,
            attributes: serde_json::Map::new(),
            epistemic: Epistemic::proven(),
            provenance: Provenance::new("iam.ListPrincipals", Timestamp::from_millis(0)),
        })
    }

    fn edge(from: &str, to: &str) -> Assertion {
        Assertion::Edge(GraphEdge {
            from: from.into(),
            to: to.into(),
            kind: EdgeKind::CanAssume,
            enabling_permission: Some("sts:AssumeRole".into()),
            epistemic: Epistemic::proven(),
            provenance: Provenance::new("iam.ListPrincipals", Timestamp::from_millis(0)),
        })
    }

    fn descriptor(service: &str, operation: &str) -> EnumerationDescriptor {
        EnumerationDescriptor {
            service: service.into(),
            operation: operation.into(),
            params: serde_json::Map::new(),
            required_permission: None,
        }
    }

    /// The M2 vertical slice: open an engagement, enumerate the Mock, and fold the resulting events
    /// into a populated attack graph — with a denied call surfaced as a coverage gap.
    #[tokio::test]
    async fn m2_open_enumerate_and_build_graph() {
        let root = temp_root("m2");
        let store = FileEventStore::open(&root).unwrap();

        // A synthetic environment with one readable call and one denied call.
        let env = MockEnvironment::builder(
            "mock",
            Principal::new("arn:foothold", PrincipalKind::Role, ProviderId::new("mock")),
        )
        .enumeration(
            "iam",
            "ListPrincipals",
            vec![
                node("arn:foothold"),
                node("arn:admin"),
                edge("arn:foothold", "arn:admin"),
            ],
        )
        .denied_enumeration("kms", "ListKeys", "kms:ListKeys")
        .build();
        let provider = MockProvider::new(env);

        // Open the engagement.
        let id = EngagementId::new("eng-m2");
        EngagementManager::new(&store)
            .open(
                id.clone(),
                Scope::new(vec![ScopeSelector::new("account", "123456789012")]),
                ProviderId::new("mock"),
                "cred",
                Actor::new("op"),
                true,
            )
            .await
            .unwrap();

        // Enumerate.
        let summary = EnumerationService::new(&store, &provider)
            .enumerate(
                &id,
                Actor::new("op"),
                &[
                    descriptor("iam", "ListPrincipals"),
                    descriptor("kms", "ListKeys"),
                ],
            )
            .await
            .unwrap();
        assert_eq!(summary.facts_asserted, 3);
        assert_eq!(summary.coverage_gaps, 1);

        // Fold the full ledger into the attack graph.
        let events = store.read_stream(&id).await.unwrap();
        let graph = GraphProjection::replay(events.iter());
        assert_eq!(graph.node_count(), 2);
        assert_eq!(graph.edge_count(), 1);
        assert!(graph.can_assume("arn:foothold", "arn:admin").is_some());
        assert_eq!(graph.coverage_gaps().len(), 1);
        assert_eq!(graph.coverage_gaps()[0].scope, "kms.ListKeys");

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn enumeration_refused_on_killed_engagement() {
        let root = temp_root("killed");
        let store = FileEventStore::open(&root).unwrap();
        let provider = MockProvider::new(
            MockEnvironment::builder(
                "mock",
                Principal::new("arn:p", PrincipalKind::Role, ProviderId::new("mock")),
            )
            .build(),
        );
        let id = EngagementId::new("eng-killed");
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
        mgr.invoke_kill_switch(&id, Actor::new("op"), "abort")
            .await
            .unwrap();

        let result = EnumerationService::new(&store, &provider)
            .enumerate(
                &id,
                Actor::new("op"),
                &[descriptor("iam", "ListPrincipals")],
            )
            .await;
        assert!(result.is_err());

        std::fs::remove_dir_all(&root).ok();
    }
}
