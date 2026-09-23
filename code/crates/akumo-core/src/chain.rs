//! Chaining & objective execution (FR-H). A chain runs an ordered sequence of techniques (realizing
//! a computed path) through the safe execution engine (G9), with a CI-safe failure policy
//! (ADR-0023 / SPEC-D12): **halt + auto-revert of completed techniques** by default, with an opt-in
//! interactive **halt-and-hold** to inspect/fix/resume. The kill-switch and on-demand revert remain
//! available throughout.
//!
//! Between successful techniques the executor can **re-enumerate** so later steps see access created
//! by earlier steps (FR-H4).

use akumo_domain::error::Result;
use akumo_domain::ids::{Actor, EngagementId};
use akumo_domain::ports::{EventStore, Provider};
use akumo_domain::seam::EnumerationDescriptor;

use akumo_dsl::schema::Technique;
use akumo_dsl::script::ScriptHost;

use crate::engagement::EngagementManager;
use crate::enumeration::EnumerationService;
use crate::execution::{ExecStatus, ExecutionConfig, ExecutionEngine, ExecutionMode, RevertReport};

/// One technique in a chain, with its inputs.
#[derive(Clone, Debug)]
pub struct ChainStep {
    /// The technique to run.
    pub technique: Technique,
    /// The inputs to bind for this technique.
    pub inputs: serde_json::Map<String, serde_json::Value>,
}

/// What happens when a technique in the chain fails or is blocked (ADR-0023 / SPEC-D12 = C).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ChainFailurePolicy {
    /// Halt and auto-revert the completed techniques (safe default; CI-friendly).
    #[default]
    HaltAndAutoRevert,
    /// Halt and leave completed techniques in place for inspection/fix/resume.
    HaltAndHold,
}

/// Chain execution configuration.
#[derive(Clone, Debug, Default)]
pub struct ChainConfig {
    /// Per-technique execution config (dry-run/execute + consent).
    pub exec: ExecutionConfig,
    /// Failure policy.
    pub failure: ChainFailurePolicy,
    /// Enumeration descriptors to re-run between successful techniques (FR-H4). Empty disables it.
    pub reenumerate: Vec<EnumerationDescriptor>,
}

/// The result of running a chain.
#[derive(Clone, Debug)]
pub struct ChainOutcome {
    /// Final status.
    pub status: ChainStatus,
    /// How many techniques completed.
    pub techniques_completed: usize,
    /// The revert report, if the chain reverted completed techniques.
    pub revert: Option<RevertReport>,
}

/// The status of a chain run.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ChainStatus {
    /// Every technique completed.
    Completed,
    /// The chain was a dry-run preview.
    DryRun,
    /// A technique was blocked by insufficient consent; the chain halted.
    HaltedConsent,
    /// A technique failed; completed techniques were reverted cleanly.
    FailedAndReverted,
    /// A technique failed; completed techniques were left in place (hold mode).
    FailedAndHeld,
    /// A technique failed and at least one compensation could not be undone.
    FailedRevertIncomplete,
}

/// Runs technique chains over the safe execution engine.
pub struct ChainExecutor<'a> {
    store: &'a dyn EventStore,
    provider: &'a dyn Provider,
    script_host: &'a dyn ScriptHost,
}

impl<'a> ChainExecutor<'a> {
    /// Create a chain executor.
    pub fn new(
        store: &'a dyn EventStore,
        provider: &'a dyn Provider,
        script_host: &'a dyn ScriptHost,
    ) -> Self {
        Self { store, provider, script_host }
    }

    /// Execute a chain step-by-step, applying the failure policy (FR-H2).
    pub async fn execute(
        &self,
        engagement: &EngagementId,
        actor: Actor,
        chain: &[ChainStep],
        config: &ChainConfig,
    ) -> Result<ChainOutcome> {
        let manager = EngagementManager::new(self.store);
        manager.context(engagement).await?.ensure_active()?;

        let engine = ExecutionEngine::new(self.store, self.provider, self.script_host);
        let enumerator = EnumerationService::new(self.store, self.provider);
        let mut completed_ids: Vec<String> = Vec::new();

        for step in chain {
            // Honor the kill-switch (and closure) before each technique.
            manager.context(engagement).await?.ensure_active()?;

            let outcome = engine
                .detonate(engagement, actor.clone(), &step.technique, &step.inputs, config.exec)
                .await?;

            match outcome.status {
                ExecStatus::Completed => {
                    if let Some(id) = outcome.detonation_id {
                        completed_ids.push(id);
                    }
                    // FR-H4: refresh the graph so later techniques see new access.
                    if !config.reenumerate.is_empty() {
                        enumerator
                            .enumerate(engagement, actor.clone(), &config.reenumerate)
                            .await?;
                    }
                }
                ExecStatus::DryRun => { /* preview only; nothing to track */ }
                ExecStatus::ConsentRequired => {
                    let revert = self
                        .apply_policy(&engine, engagement, actor.clone(), &completed_ids, config.failure)
                        .await?;
                    return Ok(ChainOutcome {
                        status: ChainStatus::HaltedConsent,
                        techniques_completed: completed_ids.len(),
                        revert,
                    });
                }
                ExecStatus::FailedAndReverted | ExecStatus::FailedRevertIncomplete => {
                    let revert = self
                        .apply_policy(&engine, engagement, actor.clone(), &completed_ids, config.failure)
                        .await?;
                    let status = chain_failure_status(config.failure, revert.as_ref());
                    return Ok(ChainOutcome {
                        status,
                        techniques_completed: completed_ids.len(),
                        revert,
                    });
                }
            }
        }

        if config.exec.mode == ExecutionMode::DryRun {
            return Ok(ChainOutcome {
                status: ChainStatus::DryRun,
                techniques_completed: 0,
                revert: None,
            });
        }

        Ok(ChainOutcome {
            status: ChainStatus::Completed,
            techniques_completed: completed_ids.len(),
            revert: None,
        })
    }

    /// Apply the failure policy: auto-revert the completed techniques (in reverse) or hold them.
    async fn apply_policy(
        &self,
        engine: &ExecutionEngine<'a>,
        engagement: &EngagementId,
        actor: Actor,
        completed_ids: &[String],
        policy: ChainFailurePolicy,
    ) -> Result<Option<RevertReport>> {
        match policy {
            ChainFailurePolicy::HaltAndHold => Ok(None),
            ChainFailurePolicy::HaltAndAutoRevert => {
                let mut report = RevertReport::default();
                for detonation_id in completed_ids.iter().rev() {
                    let partial = engine
                        .revert(engagement, actor.clone(), Some(detonation_id.as_str()))
                        .await?;
                    report.reverted += partial.reverted;
                    report.failed.extend(partial.failed);
                }
                Ok(Some(report))
            }
        }
    }
}

fn chain_failure_status(policy: ChainFailurePolicy, revert: Option<&RevertReport>) -> ChainStatus {
    match policy {
        ChainFailurePolicy::HaltAndHold => ChainStatus::FailedAndHeld,
        ChainFailurePolicy::HaltAndAutoRevert => match revert {
            Some(r) if !r.failed.is_empty() => ChainStatus::FailedRevertIncomplete,
            _ => ChainStatus::FailedAndReverted,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use akumo_domain::impact::ImpactLevel;
    use akumo_domain::ids::ProviderId;
    use akumo_domain::principal::{Principal, PrincipalKind};
    use akumo_domain::scope::{Scope, ScopeSelector};
    use akumo_domain::seam::ActionResult;

    use akumo_dsl::parse_technique;
    use akumo_dsl::script::UnsupportedScriptHost;
    use akumo_ledger::FileEventStore;
    use akumo_provider_mock::{MockEnvironment, MockProvider};

    fn temp_root(tag: &str) -> std::path::PathBuf {
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        std::env::temp_dir().join(format!("akumo-chain-test-{tag}-{nanos}"))
    }

    fn technique(id: &str, service: &str, op: &str, revert_op: &str) -> Technique {
        let yaml = format!(
            r#"
metadata:
  id: {id}
  name: {id}
  description: test technique
  version: "0.1.0"
  provider: aws
  mitre: ["T1098"]
  impact: mutating-reversible
  expected_telemetry:
    - source: cloudtrail
      event_name: {op}
steps:
  - id: s1
    body:
      type: call
      service: {service}
      operation: {op}
      params: {{}}
    revert:
      service: {service}
      operation: {revert_op}
      params: {{}}
"#
        );
        parse_technique(&yaml).unwrap()
    }

    fn empty_inputs() -> serde_json::Map<String, serde_json::Value> {
        serde_json::Map::new()
    }

    async fn setup(tag: &str, env: MockEnvironment) -> (FileEventStore, MockProvider, EngagementId) {
        let store = FileEventStore::open(temp_root(tag)).unwrap();
        let provider = MockProvider::new(env);
        let id = EngagementId::new(format!("eng-{tag}"));
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
        (store, provider, id)
    }

    fn ok(raw: serde_json::Value) -> ActionResult {
        ActionResult { raw }
    }

    #[tokio::test]
    async fn completes_a_two_technique_chain() {
        let env = MockEnvironment::builder(
            "mock",
            Principal::new("arn:op", PrincipalKind::Role, ProviderId::new("mock")),
        )
        .action("iam", "CreateAccessKey", ok(serde_json::json!({})))
        .action("iam", "DeleteAccessKey", ok(serde_json::json!({})))
        .action("iam", "TagUser", ok(serde_json::json!({})))
        .action("iam", "UntagUser", ok(serde_json::json!({})))
        .build();
        let (store, provider, id) = setup("ok", env).await;
        let host = UnsupportedScriptHost;
        let executor = ChainExecutor::new(&store, &provider, &host);

        let chain = vec![
            ChainStep {
                technique: technique("t.create", "iam", "CreateAccessKey", "DeleteAccessKey"),
                inputs: empty_inputs(),
            },
            ChainStep {
                technique: technique("t.tag", "iam", "TagUser", "UntagUser"),
                inputs: empty_inputs(),
            },
        ];

        let outcome = executor
            .execute(&id, Actor::new("op"), &chain, &ChainConfig::default())
            .await
            .unwrap();
        assert_eq!(outcome.status, ChainStatus::Completed);
        assert_eq!(outcome.techniques_completed, 2);
    }

    #[tokio::test]
    async fn failure_auto_reverts_prior_techniques() {
        // Second technique's call is not scripted → it fails; the first is auto-reverted.
        let env = MockEnvironment::builder(
            "mock",
            Principal::new("arn:op", PrincipalKind::Role, ProviderId::new("mock")),
        )
        .action("iam", "CreateAccessKey", ok(serde_json::json!({})))
        .action("iam", "DeleteAccessKey", ok(serde_json::json!({})))
        .build();
        let (store, provider, id) = setup("fail", env).await;
        let host = UnsupportedScriptHost;
        let executor = ChainExecutor::new(&store, &provider, &host);

        let chain = vec![
            ChainStep {
                technique: technique("t.create", "iam", "CreateAccessKey", "DeleteAccessKey"),
                inputs: empty_inputs(),
            },
            ChainStep {
                technique: technique("t.missing", "iam", "NotScripted", "AlsoMissing"),
                inputs: empty_inputs(),
            },
        ];

        let outcome = executor
            .execute(&id, Actor::new("op"), &chain, &ChainConfig::default())
            .await
            .unwrap();
        assert_eq!(outcome.status, ChainStatus::FailedAndReverted);
        assert_eq!(outcome.techniques_completed, 1);
        let report = outcome.revert.unwrap();
        assert_eq!(report.reverted, 1); // the first technique's compensation ran
        assert!(report.failed.is_empty());
    }

    #[tokio::test]
    async fn hold_mode_leaves_completed_in_place() {
        let env = MockEnvironment::builder(
            "mock",
            Principal::new("arn:op", PrincipalKind::Role, ProviderId::new("mock")),
        )
        .action("iam", "CreateAccessKey", ok(serde_json::json!({})))
        .action("iam", "DeleteAccessKey", ok(serde_json::json!({})))
        .build();
        let (store, provider, id) = setup("hold", env).await;
        let host = UnsupportedScriptHost;
        let executor = ChainExecutor::new(&store, &provider, &host);

        let chain = vec![
            ChainStep {
                technique: technique("t.create", "iam", "CreateAccessKey", "DeleteAccessKey"),
                inputs: empty_inputs(),
            },
            ChainStep {
                technique: technique("t.missing", "iam", "NotScripted", "AlsoMissing"),
                inputs: empty_inputs(),
            },
        ];

        let outcome = executor
            .execute(
                &id,
                Actor::new("op"),
                &chain,
                &ChainConfig { failure: ChainFailurePolicy::HaltAndHold, ..ChainConfig::default() },
            )
            .await
            .unwrap();
        assert_eq!(outcome.status, ChainStatus::FailedAndHeld);
        assert_eq!(outcome.techniques_completed, 1);
        assert!(outcome.revert.is_none());
    }
}
