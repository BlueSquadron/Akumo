//! The safe execution engine (FR-G). Design bias throughout: **least blast radius, fail safe,
//! reversible-by-default** (NFR-SAF1/SAF4). Builds on the event ledger (ADR-0014), saga compensations
//! (ADR-0019), the technique contract + Tier-1 DSL (Inc. 3), and the provider seam.
//!
//! Per-step lifecycle (spec §4.1): `PLANNED → ANALYZED (dry-run + blast radius) → CONSENTED →
//! DETONATED (compensation recorded immediately) → VERIFIED (effects asserted)`; any failure after
//! detonation triggers auto-revert of the completed steps. There is **no warm-up phase** (ADR-0021):
//! setup is ordinary compensated steps, so there is one teardown mechanism.
//!
//! v1 scope: static-by-default dry-run/blast-radius (ADR-0022; opt-in provider-assisted validation
//! and rate/footprint controls are refinements layered on later). Verification is success-based in
//! v1; a CEL verify predicate and auto-asserting technique effects into the graph are noted follow-ups.

use serde::{Deserialize, Serialize};

use akumo_domain::error::Result;
use akumo_domain::event::event_type;
use akumo_domain::ids::{Actor, EngagementId};
use akumo_domain::impact::ImpactLevel;
use akumo_domain::ports::{EventStore, Provider};
use akumo_domain::seam::{ActionDescriptor, CapabilityGrant};

use akumo_dsl::schema::{StepBody, Technique};
use akumo_dsl::script::ScriptHost;
use akumo_dsl::{eval, eval_bool, resolve_params, Env};

use crate::engagement::{consent_satisfied, ConsentPolicy, EngagementManager};
use crate::journal::{now_millis, Journal};

/// Whether to only preview or actually execute.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecutionMode {
    /// Preview intended actions + blast radius; perform no mutating provider calls (FR-G2).
    DryRun,
    /// Execute through the full lifecycle.
    Execute,
}

/// Execution configuration.
#[derive(Clone, Copy, Debug)]
pub struct ExecutionConfig {
    /// Dry-run or execute.
    pub mode: ExecutionMode,
    /// The consent policy gating mutating steps (FR-A3).
    pub consent: ConsentPolicy,
}

impl Default for ExecutionConfig {
    fn default() -> Self {
        Self { mode: ExecutionMode::Execute, consent: ConsentPolicy::Interactive }
    }
}

/// The estimated blast radius of a technique (static preview, zero provider calls — FR-G3).
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BlastRadius {
    /// The technique previewed.
    pub technique_id: String,
    /// The provider calls (`service.operation`) the technique would make.
    pub provider_calls: Vec<String>,
    /// The declared effects it would assert.
    pub effects: Vec<String>,
    /// The most impactful action.
    pub max_impact: ImpactLevel,
}

/// The result of a detonation.
#[derive(Clone, Debug)]
pub struct ExecutionOutcome {
    /// The technique id.
    pub technique_id: String,
    /// Final status.
    pub status: ExecStatus,
    /// How many steps detonated.
    pub steps_detonated: usize,
    /// How many steps verified.
    pub steps_verified: usize,
    /// The blast-radius preview.
    pub blast_radius: BlastRadius,
    /// The revert report, if a revert ran.
    pub revert: Option<RevertReport>,
    /// The detonation id of this run (present only when steps were detonated and left standing),
    /// so a chain can scope a later revert to exactly this technique.
    pub detonation_id: Option<String>,
}

/// The status of an execution.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExecStatus {
    /// Dry-run only; nothing detonated.
    DryRun,
    /// All steps completed and verified.
    Completed,
    /// Consent was insufficient; nothing detonated.
    ConsentRequired,
    /// A step failed; completed steps were reverted cleanly.
    FailedAndReverted,
    /// A step failed and at least one compensation could not be undone (NFR-SAF4).
    FailedRevertIncomplete,
}

/// What a revert accomplished.
#[derive(Clone, Debug, Default)]
pub struct RevertReport {
    /// How many step compensations were successfully replayed.
    pub reverted: usize,
    /// Compensations that could not be undone (surfaced prominently, NFR-SAF4).
    pub failed: Vec<String>,
}

/// The safe execution engine.
pub struct ExecutionEngine<'a> {
    store: &'a dyn EventStore,
    provider: &'a dyn Provider,
    script_host: &'a dyn ScriptHost,
}

impl<'a> ExecutionEngine<'a> {
    /// Create an engine.
    pub fn new(
        store: &'a dyn EventStore,
        provider: &'a dyn Provider,
        script_host: &'a dyn ScriptHost,
    ) -> Self {
        Self { store, provider, script_host }
    }

    /// Detonate a technique's steps within an active engagement, recording the full lifecycle to the
    /// ledger. On step failure, completed steps are auto-reverted (safe default).
    pub async fn detonate(
        &self,
        engagement: &EngagementId,
        actor: Actor,
        technique: &Technique,
        inputs: &serde_json::Map<String, serde_json::Value>,
        config: ExecutionConfig,
    ) -> Result<ExecutionOutcome> {
        let ctx = EngagementManager::new(self.store).context(engagement).await?;
        ctx.ensure_active()?;

        let journal = Journal::new(self.store);
        let technique_id = technique.metadata.id.clone();
        let blast = static_blast_radius(technique);

        // ANALYZED: dry-run + blast radius (no mutation).
        journal
            .record(
                engagement,
                actor.clone(),
                event_type::DRY_RUN_PERFORMED,
                serde_json::json!({ "technique": technique_id.clone() }),
            )
            .await?;
        journal
            .record(
                engagement,
                actor.clone(),
                event_type::BLAST_RADIUS_ESTIMATED,
                serde_json::to_value(&blast)?,
            )
            .await?;

        if config.mode == ExecutionMode::DryRun {
            return Ok(ExecutionOutcome {
                technique_id,
                status: ExecStatus::DryRun,
                steps_detonated: 0,
                steps_verified: 0,
                blast_radius: blast,
                revert: None,
                detonation_id: None,
            });
        }

        // CONSENTED: gate mutating techniques (FR-A3).
        let impact = technique.metadata.impact;
        if impact.requires_consent() {
            let max_consent = EngagementManager::new(self.store)
                .load(engagement)
                .await?
                .and_then(|s| s.max_consent);
            if !consent_satisfied(config.consent, max_consent, impact) {
                return Ok(ExecutionOutcome {
                    technique_id,
                    status: ExecStatus::ConsentRequired,
                    steps_detonated: 0,
                    steps_verified: 0,
                    blast_radius: blast,
                    revert: None,
                    detonation_id: None,
                });
            }
        }

        let detonation_id = format!("{engagement}-det-{}", now_millis());
        let mut env = build_env(inputs);
        let mut steps_detonated = 0usize;
        let mut steps_verified = 0usize;

        for step in &technique.steps {
            // Optional run-if condition.
            if let Some(cond) = &step.condition {
                if !eval_bool(cond, &env)? {
                    continue;
                }
            }

            let detonation = match &step.body {
                StepBody::Call { service, operation, params } => {
                    let resolved = resolve_params(params, &env)?;
                    let grant = grant_for(service, operation);
                    let descriptor = ActionDescriptor {
                        service: service.clone(),
                        operation: operation.clone(),
                        params: resolved.clone(),
                        impact,
                        required_permission: None,
                    };
                    match self.provider.actions().execute(&descriptor, &grant).await {
                        Ok(result) => Ok((
                            ResolvedCall {
                                service: service.clone(),
                                operation: operation.clone(),
                                params: resolved,
                            },
                            result.raw,
                        )),
                        Err(e) => Err(e),
                    }
                }
                StepBody::Script { language, source, capabilities } => {
                    match self.script_host.execute(*language, source, capabilities, &env) {
                        Ok(value) => Ok((
                            ResolvedCall {
                                service: "script".to_string(),
                                operation: step.id.clone(),
                                params: serde_json::Map::new(),
                            },
                            value,
                        )),
                        Err(e) => Err(e),
                    }
                }
            };

            let (call, result_value) = match detonation {
                Ok(pair) => pair,
                Err(e) => {
                    // Detonation failed *before* any effect — the step did not happen.
                    journal
                        .record(
                            engagement,
                            actor.clone(),
                            event_type::EXECUTION_FAILED,
                            serde_json::to_value(&ExecutionFailedPayload {
                                detonation_id: detonation_id.clone(),
                                step_id: step.id.clone(),
                                error: e.to_string(),
                            })?,
                        )
                        .await?;
                    let report = self.revert(engagement, actor.clone(), Some(&detonation_id)).await?;
                    let status = if report.failed.is_empty() {
                        ExecStatus::FailedAndReverted
                    } else {
                        ExecStatus::FailedRevertIncomplete
                    };
                    return Ok(ExecutionOutcome {
                        technique_id,
                        status,
                        steps_detonated,
                        steps_verified,
                        blast_radius: blast,
                        revert: Some(report),
                        detonation_id: None,
                    });
                }
            };

            // Bind the result so later steps and the compensation can reference it.
            env.set("result", result_value);
            for binding in &step.bind {
                let value = eval(&binding.from, &env)?;
                env.set(binding.name.clone(), value);
            }

            // Resolve the compensation (may reference bound results) — recorded BEFORE verification
            // so a step that mutated but fails verification is still revertible (fail-safe).
            let compensation = match &step.revert {
                Some(c) => Some(ResolvedCall {
                    service: c.service.clone(),
                    operation: c.operation.clone(),
                    params: resolve_params(&c.params, &env)?,
                }),
                None => None,
            };
            journal
                .record(
                    engagement,
                    actor.clone(),
                    event_type::STEP_DETONATED,
                    serde_json::to_value(&StepDetonatedPayload {
                        detonation_id: detonation_id.clone(),
                        step_id: step.id.clone(),
                        call,
                        compensation,
                    })?,
                )
                .await?;
            steps_detonated += 1;

            // VERIFIED (v1: success-based). Asserting technique effects into the graph is a noted
            // follow-up (the effect→assertion mapping lands with the G7↔G12 bridge).
            journal
                .record(
                    engagement,
                    actor.clone(),
                    event_type::STEP_VERIFIED,
                    serde_json::json!({ "step": step.id.clone() }),
                )
                .await?;
            steps_verified += 1;
        }

        Ok(ExecutionOutcome {
            technique_id,
            status: ExecStatus::Completed,
            steps_detonated,
            steps_verified,
            blast_radius: blast,
            revert: None,
            detonation_id: Some(detonation_id),
        })
    }

    /// Revert detonated-but-not-yet-reverted steps by replaying their recorded compensations in
    /// reverse order. Reads the compensations from the ledger, so it is **restart-safe** (DSR-4).
    /// Pass a `detonation` id to scope the revert to one detonation; `None` reverts all outstanding.
    pub async fn revert(
        &self,
        engagement: &EngagementId,
        actor: Actor,
        detonation: Option<&str>,
    ) -> Result<RevertReport> {
        use std::collections::HashSet;

        let events = self.store.read_stream(engagement).await?;
        let mut detonated: Vec<(String, String, Option<ResolvedCall>)> = Vec::new();
        let mut reverted: HashSet<(String, String)> = HashSet::new();
        for event in &events {
            match event.event_type.as_str() {
                event_type::STEP_DETONATED => {
                    if let Ok(p) =
                        serde_json::from_value::<StepDetonatedPayload>(event.payload.clone())
                    {
                        detonated.push((p.detonation_id, p.step_id, p.compensation));
                    }
                }
                event_type::STEP_REVERTED => {
                    if let Ok(p) =
                        serde_json::from_value::<StepRevertedPayload>(event.payload.clone())
                    {
                        reverted.insert((p.detonation_id, p.step_id));
                    }
                }
                _ => {}
            }
        }

        let outstanding: Vec<(String, String, Option<ResolvedCall>)> = detonated
            .into_iter()
            .filter(|(det, step, _)| {
                if reverted.contains(&(det.clone(), step.clone())) {
                    return false;
                }
                match detonation {
                    Some(want) => det.as_str() == want,
                    None => true,
                }
            })
            .collect();

        let journal = Journal::new(self.store);
        let mut report = RevertReport::default();

        // Undo only what happened, in reverse order.
        for (det_id, step_id, compensation) in outstanding.into_iter().rev() {
            match compensation {
                Some(call) => {
                    let grant = grant_for(&call.service, &call.operation);
                    let descriptor = ActionDescriptor {
                        service: call.service.clone(),
                        operation: call.operation.clone(),
                        params: call.params.clone(),
                        impact: ImpactLevel::MutatingReversible,
                        required_permission: None,
                    };
                    match self.provider.actions().execute(&descriptor, &grant).await {
                        Ok(_) => {
                            journal
                                .record(
                                    engagement,
                                    actor.clone(),
                                    event_type::STEP_REVERTED,
                                    serde_json::to_value(&StepRevertedPayload {
                                        detonation_id: det_id,
                                        step_id,
                                    })?,
                                )
                                .await?;
                            report.reverted += 1;
                        }
                        Err(e) => {
                            report.failed.push(format!("{step_id}: {e}"));
                        }
                    }
                }
                None => {
                    // Nothing to undo (read-only / no compensation): record for bookkeeping so it is
                    // not reprocessed, but do not count it as an undo.
                    journal
                        .record(
                            engagement,
                            actor.clone(),
                            event_type::STEP_REVERTED,
                            serde_json::to_value(&StepRevertedPayload {
                                detonation_id: det_id,
                                step_id,
                            })?,
                        )
                        .await?;
                }
            }
        }

        Ok(report)
    }
}

// ---- internals ---------------------------------------------------------------------------------

/// A concrete provider call (an executed detonation call or a recorded compensation).
#[derive(Clone, Debug, Serialize, Deserialize)]
struct ResolvedCall {
    service: String,
    operation: String,
    #[serde(default)]
    params: serde_json::Map<String, serde_json::Value>,
}

#[derive(Serialize, Deserialize)]
struct StepDetonatedPayload {
    detonation_id: String,
    step_id: String,
    call: ResolvedCall,
    #[serde(default)]
    compensation: Option<ResolvedCall>,
}

#[derive(Serialize, Deserialize)]
struct StepRevertedPayload {
    detonation_id: String,
    step_id: String,
}

#[derive(Serialize, Deserialize)]
struct ExecutionFailedPayload {
    detonation_id: String,
    step_id: String,
    error: String,
}

fn build_env(inputs: &serde_json::Map<String, serde_json::Value>) -> Env {
    let mut env = Env::new();
    for (name, value) in inputs {
        env.set(name.clone(), value.clone());
    }
    env
}

fn grant_for(service: &str, operation: &str) -> CapabilityGrant {
    CapabilityGrant { allowed: vec![format!("{service}.{operation}")] }
}

fn static_blast_radius(technique: &Technique) -> BlastRadius {
    let provider_calls = technique
        .steps
        .iter()
        .filter_map(|s| match &s.body {
            StepBody::Call { service, operation, .. } => Some(format!("{service}.{operation}")),
            StepBody::Script { .. } => None,
        })
        .collect();
    let effects = technique
        .contract
        .effects
        .iter()
        .map(|e| format!("{}({})", e.predicate.name, e.predicate.args.join(", ")))
        .collect();
    BlastRadius {
        technique_id: technique.metadata.id.clone(),
        provider_calls,
        effects,
        max_impact: technique.metadata.impact,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        std::env::temp_dir().join(format!("akumo-exec-test-{tag}-{nanos}"))
    }

    const CREATE_KEY: &str = r#"
metadata:
  id: aws.iam.create-access-key
  name: Create IAM Access Key
  description: Creates a long-term access key for a target IAM user.
  version: "0.1.0"
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
    bind:
      - name: key_id
        from: result.AccessKeyId
    revert:
      service: iam
      operation: DeleteAccessKey
      params: { UserName: "$user", AccessKeyId: "$key_id" }
"#;

    fn env_with_iam() -> MockEnvironment {
        MockEnvironment::builder(
            "mock",
            Principal::new("arn:op", PrincipalKind::Role, ProviderId::new("mock")),
        )
        .action(
            "iam",
            "CreateAccessKey",
            ActionResult { raw: serde_json::json!({ "AccessKeyId": "AKIA123" }) },
        )
        .action("iam", "DeleteAccessKey", ActionResult { raw: serde_json::json!({}) })
        .build()
    }

    async fn open_engagement(store: &FileEventStore, id: &EngagementId) {
        EngagementManager::new(store)
            .open(
                id.clone(),
                Scope::new(vec![ScopeSelector::new("account", "1")]),
                ProviderId::new("mock"),
                "cred",
                Actor::new("op"),
                true,
            )
            .await
            .unwrap();
    }

    fn inputs() -> serde_json::Map<String, serde_json::Value> {
        let mut m = serde_json::Map::new();
        m.insert("user".into(), serde_json::json!("alice"));
        m
    }

    #[tokio::test]
    async fn detonate_then_revert_roundtrip() {
        let root = temp_root("roundtrip");
        let store = FileEventStore::open(&root).unwrap();
        let provider = MockProvider::new(env_with_iam());
        let host = UnsupportedScriptHost;
        let engine = ExecutionEngine::new(&store, &provider, &host);
        let technique = parse_technique(CREATE_KEY).unwrap();

        let id = EngagementId::new("eng-rt");
        open_engagement(&store, &id).await;
        // Consent for a mutating-reversible technique.
        EngagementManager::new(&store)
            .record_consent(&id, Actor::new("op"), ImpactLevel::MutatingReversible, true, None)
            .await
            .unwrap();

        let outcome = engine
            .detonate(&id, Actor::new("op"), &technique, &inputs(), ExecutionConfig::default())
            .await
            .unwrap();
        assert_eq!(outcome.status, ExecStatus::Completed);
        assert_eq!(outcome.steps_detonated, 1);
        assert_eq!(outcome.steps_verified, 1);

        // On-demand revert replays the DeleteAccessKey compensation.
        let report = engine.revert(&id, Actor::new("op"), None).await.unwrap();
        assert_eq!(report.reverted, 1);
        assert!(report.failed.is_empty());

        // Reverting again is a no-op (nothing outstanding).
        let again = engine.revert(&id, Actor::new("op"), None).await.unwrap();
        assert_eq!(again.reverted, 0);

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn dry_run_performs_no_detonation() {
        let root = temp_root("dryrun");
        let store = FileEventStore::open(&root).unwrap();
        let provider = MockProvider::new(env_with_iam());
        let host = UnsupportedScriptHost;
        let engine = ExecutionEngine::new(&store, &provider, &host);
        let technique = parse_technique(CREATE_KEY).unwrap();

        let id = EngagementId::new("eng-dry");
        open_engagement(&store, &id).await;

        let outcome = engine
            .detonate(
                &id,
                Actor::new("op"),
                &technique,
                &inputs(),
                ExecutionConfig { mode: ExecutionMode::DryRun, ..Default::default() },
            )
            .await
            .unwrap();
        assert_eq!(outcome.status, ExecStatus::DryRun);
        assert_eq!(outcome.steps_detonated, 0);
        assert_eq!(outcome.blast_radius.provider_calls, vec!["iam.CreateAccessKey".to_string()]);

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn mutating_technique_requires_consent() {
        let root = temp_root("consent");
        let store = FileEventStore::open(&root).unwrap();
        let provider = MockProvider::new(env_with_iam());
        let host = UnsupportedScriptHost;
        let engine = ExecutionEngine::new(&store, &provider, &host);
        let technique = parse_technique(CREATE_KEY).unwrap();

        let id = EngagementId::new("eng-noconsent");
        open_engagement(&store, &id).await;

        // No consent recorded → refused.
        let outcome = engine
            .detonate(&id, Actor::new("op"), &technique, &inputs(), ExecutionConfig::default())
            .await
            .unwrap();
        assert_eq!(outcome.status, ExecStatus::ConsentRequired);
        assert_eq!(outcome.steps_detonated, 0);

        std::fs::remove_dir_all(&root).ok();
    }

    #[tokio::test]
    async fn failure_auto_reverts_completed_steps() {
        // A two-step technique whose second step is not scripted in the mock → detonation fails,
        // and the first (completed) step is auto-reverted.
        let two_step = r#"
metadata:
  id: aws.iam.two-step
  name: Two Step
  description: First creates a key, then calls a missing operation.
  version: "0.1.0"
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
  - id: missing
    body:
      type: call
      service: iam
      operation: NotScripted
      params: {}
"#;
        let root = temp_root("autorevert");
        let store = FileEventStore::open(&root).unwrap();
        let provider = MockProvider::new(env_with_iam());
        let host = UnsupportedScriptHost;
        let engine = ExecutionEngine::new(&store, &provider, &host);
        let technique = parse_technique(two_step).unwrap();

        let id = EngagementId::new("eng-fail");
        open_engagement(&store, &id).await;
        EngagementManager::new(&store)
            .record_consent(&id, Actor::new("op"), ImpactLevel::MutatingReversible, true, None)
            .await
            .unwrap();

        let outcome = engine
            .detonate(&id, Actor::new("op"), &technique, &inputs(), ExecutionConfig::default())
            .await
            .unwrap();
        assert_eq!(outcome.status, ExecStatus::FailedAndReverted);
        assert_eq!(outcome.steps_detonated, 1);
        let report = outcome.revert.unwrap();
        assert_eq!(report.reverted, 1); // the create-key compensation ran
        assert!(report.failed.is_empty());

        std::fs::remove_dir_all(&root).ok();
    }
}
