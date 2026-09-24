//! Release-blocking safety & extensibility **gates** (task G19.3, spec §7.3), gathered so each is
//! proven to *fail when violated* (a deliberately-broken input), not merely to pass. These run in the
//! fast tier on every PR alongside the per-module tests they consolidate:
//!
//! - **Revert correctness** (NFR-SAF2) — `execution`/`chain` tests (detonate → clean revert).
//! - **Extensibility gate** (NFR-EXT7) — `api::tests::extensibility_gate_full_loop_on_mock`.
//! - **Dependency direction** (NFR-EXT1) — `xtask dep-lint` (CI).
//! - **Content validation** (FR-F2) — `akumo-dsl` validate tests.
//!
//! The negative paths for scope, kill-switch, consent, and tamper-evidence are asserted here.

#![cfg(test)]

use crate::api::Akumo;
use crate::engagement::EngagementManager;
use crate::execution::ExecStatus;

use akumo_domain::event::EventEnvelope;
use akumo_domain::ids::{Actor, EngagementId, ProviderId, Seq, Timestamp};
use akumo_domain::impact::ImpactLevel;
use akumo_domain::scope::{Scope, ScopeSelector};

use akumo_dsl::parse_technique;
use akumo_dsl::script::UnsupportedScriptHost;
use akumo_ledger::FileEventStore;
use akumo_provider_mock::{MockEnvironment, MockProvider};

fn temp_root(tag: &str) -> std::path::PathBuf {
    let nanos = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    std::env::temp_dir().join(format!("akumo-gate-{tag}-{nanos}"))
}

fn mock() -> MockProvider {
    MockProvider::new(
        MockEnvironment::builder(
            "mock",
            akumo_domain::principal::Principal::new(
                "arn:op",
                akumo_domain::principal::PrincipalKind::Role,
                ProviderId::new("mock"),
            ),
        )
        .action(
            "iam",
            "CreateAccessKey",
            akumo_domain::seam::ActionResult {
                raw: serde_json::json!({ "AccessKeyId": "AKIA" }),
            },
        )
        .action(
            "iam",
            "DeleteAccessKey",
            akumo_domain::seam::ActionResult {
                raw: serde_json::json!({}),
            },
        )
        .build(),
    )
}

const CREATE_KEY: &str = r#"
metadata:
  id: aws.iam.create-access-key
  name: Create IAM Access Key
  description: Creates a long-term access key.
  version: "1.0.0"
  provider: aws
  mitre: ["T1098"]
  impact: mutating-reversible
  expected_telemetry:
    - source: cloudtrail
      event_name: CreateAccessKey
steps:
  - id: create
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

async fn open(store: &FileEventStore, id: &EngagementId) {
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

/// GATE: out-of-scope targets are refused (FR-A1, NFR-COMP2).
#[tokio::test]
async fn gate_scope_refuses_out_of_scope() {
    let store = FileEventStore::open(temp_root("scope")).unwrap();
    let id = EngagementId::new("eng");
    open(&store, &id).await;
    let ctx = EngagementManager::new(&store).context(&id).await.unwrap();
    ctx.ensure_in_scope("account", "1").unwrap();
    assert!(
        ctx.ensure_in_scope("account", "999").is_err(),
        "out-of-scope must be refused"
    );
}

/// GATE: after the kill-switch, execution is halted (FR-A4).
#[tokio::test]
async fn gate_kill_switch_halts_execution() {
    let store = FileEventStore::open(temp_root("kill")).unwrap();
    let provider = mock();
    let host = UnsupportedScriptHost;
    let akumo = Akumo::new(&store, &provider, &host);
    let id = EngagementId::new("eng");
    open(&store, &id).await;

    akumo
        .kill_switch(&id, Actor::new("op"), "abort")
        .await
        .unwrap();

    let technique = parse_technique(CREATE_KEY).unwrap();
    let result = akumo
        .run(
            &id,
            Actor::new("op"),
            &technique,
            &serde_json::Map::new(),
            Default::default(),
        )
        .await;
    assert!(result.is_err(), "a killed engagement must refuse execution");
}

/// GATE: a mutating technique without consent does not detonate (FR-A3).
#[tokio::test]
async fn gate_consent_required_before_mutation() {
    let store = FileEventStore::open(temp_root("consent")).unwrap();
    let provider = mock();
    let host = UnsupportedScriptHost;
    let akumo = Akumo::new(&store, &provider, &host);
    let id = EngagementId::new("eng");
    open(&store, &id).await;

    let technique = parse_technique(CREATE_KEY).unwrap();
    let outcome = akumo
        .run(
            &id,
            Actor::new("op"),
            &technique,
            &serde_json::Map::new(),
            Default::default(),
        )
        .await
        .unwrap();
    assert_eq!(outcome.status, ExecStatus::ConsentRequired);
    assert_eq!(
        outcome.steps_detonated, 0,
        "nothing may detonate without consent"
    );
}

/// GATE: revert restores after a mutating detonation (NFR-SAF2).
#[tokio::test]
async fn gate_revert_restores() {
    let store = FileEventStore::open(temp_root("revert")).unwrap();
    let provider = mock();
    let host = UnsupportedScriptHost;
    let akumo = Akumo::new(&store, &provider, &host);
    let id = EngagementId::new("eng");
    open(&store, &id).await;
    akumo
        .record_consent(&id, Actor::new("op"), ImpactLevel::MutatingReversible, true)
        .await
        .unwrap();

    let technique = parse_technique(CREATE_KEY).unwrap();
    let outcome = akumo
        .run(
            &id,
            Actor::new("op"),
            &technique,
            &serde_json::Map::new(),
            Default::default(),
        )
        .await
        .unwrap();
    assert_eq!(outcome.status, ExecStatus::Completed);
    let report = akumo.revert(&id, Actor::new("op")).await.unwrap();
    assert_eq!(report.reverted, 1);
    assert!(report.failed.is_empty());
}

/// GATE: the hash-chained ledger detects tampering (NFR-OBS3).
#[test]
fn gate_tamper_evidence() {
    let mut event = EventEnvelope::seal(
        "e0",
        EngagementId::new("eng"),
        Seq(0),
        Timestamp::from_millis(0),
        Actor::new("op"),
        None,
        "TestEvent",
        serde_json::json!({ "scope": "acct:1" }),
    )
    .unwrap();
    event.verify_hash().unwrap();
    event.payload = serde_json::json!({ "scope": "acct:evil" });
    assert!(event.verify_hash().is_err(), "tampering must be detected");
}
