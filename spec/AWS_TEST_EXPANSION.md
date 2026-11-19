# Akumo — AWS Test & Scenario Expansion Plan

Status: **proposed** (this document) + **Tier‑2 landed** (the catalog‑contract suite in this PR).
Owner: core team. Source research: see [References](#references). Supersedes nothing; complements
`spec/V2_BACKLOG.md` and the README **Status** section.

This plan has two independent axes that are easy to conflate:

1. **Scenarios** — the *attack techniques themselves* (the YAML content catalog under `code/content/`).
2. **Harness** — the *tests that exercise them* (how we prove a technique is correct, reversible,
   and detectable without melting a real account or the CI budget).

Growing one without the other is a trap: more techniques with no new test tier means more unproven
content; a richer harness with no new techniques means well‑tested emptiness. This document plans
both and sequences them.

---

## 1. Current state (verified, not aspirational)

| Thing | Reality today |
|---|---|
| Techniques shipped | **5** YAMLs across **3** tactics (persistence ×3, privilege‑escalation ×1, defense‑evasion ×1) |
| AWS provider tests | **unit only** — `arn_kind_classification`, mapper assertions. No integration tests. |
| `ci-live-aws.yml` | Scaffolded and gated, runs `cargo test -p akumo-provider-aws -- --ignored`, **but no `#[ignore]` tests exist yet** — so the tier is currently a no‑op. |
| Catalog validation | `akumo_dsl::validate` checks metadata/MITRE/telemetry presence, step‑id uniqueness, script capabilities. It does **not** check that a *reversible* technique actually ships a revert for each mutating step. |
| Revert mechanism | `akumo-core` execution records compensation **only from an explicit `step.revert`** (`execution.rs`). A `mutating-reversible` step with no `revert` is silently un‑revertible. |

> **Callout — the quiet gap.** The schema comment hints at descriptor‑derived inverses ("if absent,
> the engine derives one from the descriptor's declared inverse"), but the engine does **not**
> implement that yet. Until it does, NFR‑SAF2 ("tested revert") holds **only** for steps that carry
> an explicit `revert`. That is exactly the invariant Tier‑2 below now enforces at test time.

---

## 2. The four‑tier test model

We currently have Tiers 1 and 4 (the latter empty). This plan adds Tiers 2 and 3.

| Tier | Runs | Needs AWS? | Cost | Proves |
|---|---|---|---|---|
| **1. Unit (mock)** | every PR | no | ~0 | wiring, mapper, contracts |
| **2. Catalog‑contract** ← *this PR* | every PR | no | ~0 | every shipped technique parses, validates, and — if `mutating-reversible` — pairs a revert to every mutating step; MITRE/telemetry well‑formed |
| **3. Emulator integration** ← *proposed* | CI | no (emulated) | low | detonate→revert round‑trips leave **zero residue**; SDK serialization against real‑ish IAM/CloudTrail |
| **4. Live‑AWS fidelity** ← *exists, empty* | release‑gated | yes (ephemeral) | $ + time | true AWS semantics + orphaned‑resource leak detector |

The guiding principle: **push every assertion down to the cheapest tier that can make it.** Reversibility
pairing is a Tier‑2 (free, offline) fact. Residue‑freedom is a Tier‑3 fact. Only genuine AWS
semantics (IAM evaluation order, eventual consistency, real CloudTrail field shapes) need Tier 4.

---

## 3. Tier 2 — Catalog‑contract tests (delivered here)

Location: `code/crates/akumo-content/tests/catalog_contract.rs`. Offline, no AWS, runs in the fast CI
tier. It loads the real shipped `content/` tree and asserts, per technique:

1. **Parses and validates** via `akumo_dsl::Catalog::load_dir` (duplicates and invalids fail).
2. **Reversibility pairing (NFR‑SAF2):** every `mutating-reversible` technique has, for *each*
   `Call` step, an explicit `revert` whose `service`/`operation` are non‑empty. `read` techniques
   are exempt (nothing to undo).
3. **Impact/variant coherence:** `mutating-irreversible` / `destructive` techniques declare a
   `simulated_variant` (mirrors `validate`, asserted here against on‑disk content).
4. **Detection meaning:** every `expected_telemetry` entry has a non‑empty `source` and `event_name`.
5. **Identity hygiene:** technique `id` is namespaced by its `provider` (e.g. `aws.…`), and the
   `provider` of everything under `content/aws/` is `aws`.
6. **MITRE coverage snapshot:** prints the distinct ATT&CK techniques covered and asserts the
   count only grows (a floor), so new content can't silently *drop* coverage.

> **Tradeoff — test vs. loader.** We deliberately put the reversibility‑pairing invariant in a
> **test**, not in `akumo_dsl::validate`. Promoting it into the loader would enforce it on *all*
> content everywhere — stronger, but it would also reject any future technique that legitimately
> relies on the (not‑yet‑built) descriptor‑derived inverse. Keeping it in Tier 2 for now means the
> PR is **purely additive** (zero behaviour change, zero risk to existing load paths). Promoting it
> is tracked as a follow‑up, gated on the descriptor‑inverse mechanism landing.

---

## 4. Tier 3 — Emulator integration (proposed)

Goal: prove a detonate→revert round‑trip against an AWS *emulator* leaves no residue, in CI, with no
account and no spend.

**Emulator choice (2026 reality check):**

| Option | Fit for Akumo | Callout |
|---|---|---|
| **Moto (server mode)** | **Recommended.** Point `aws-config` at a custom endpoint URL; run detonate+revert; re‑enumerate and assert the account state matches the pre‑detonation snapshot. | Python‑first but the HTTP server is SDK‑agnostic, so the Rust SDK reaches it fine. IAM evaluation is shallow — good for state round‑trips, not for permission‑boundary assertions. |
| **LocalStack** | Usable for round‑trips. | **Licensing is contested in 2026** (free‑tier changes / account requirement reported); Community IAM enforcement is only *partial*. Verify current licensing before adopting. |
| Floci / MiniStack | Emerging OSS alternatives post‑LocalStack‑archival. | Immature; watch, don't bet on yet. |

**Shape:** a `--features emulator` suite in `akumo-provider-aws` that spins Moto via
`testcontainers`, runs each reversible technique's detonate+revert, then asserts residue‑freedom.

> **Callout — fidelity ceiling.** An emulator proves *our* revert logic and SDK serialization, **not**
> AWS's real behaviour. Anything that depends on IAM evaluation order, eventual consistency, or exact
> CloudTrail field shapes must still be asserted at Tier 4. Treat Tier 3 as a fast regression net,
> not a source of truth.

---

## 5. Tier 4 — Live‑AWS fidelity (exists, needs tests)

`ci-live-aws.yml` is already wired (OIDC role, ephemeral account, leak detector) but runs zero tests.
Fill it with `#[ignore]`‑gated integration tests that, for each reversible technique: detonate →
revert → assert clean, then let the leak detector backstop orphaned resources. This is the source of
truth for AWS semantics and the gate for any `mutating-irreversible`/`destructive` content.

> **Callout — blast radius & cost.** Live tier runs only on manual dispatch / release and only when
> `AKUMO_LIVE_AWS=true`. Keep it scoped to a throwaway account, keep the governor (NFR‑SAF3, separate
> work) in front of it, and never run `destructive` techniques here without the simulated variant.

---

## 6. Scenario / chain tests (CloudGoat‑style)

Individual‑technique tests prove actions. **Scenario tests prove the planner.** Seed a deliberately
vulnerable environment (e.g. a role reachable via `iam:CreatePolicyVersion`) and assert that Akumo's
planner *discovers the known escalation path* end‑to‑end. This is the test that will validate the
technique‑aware planning + richer enumeration feature (the headline gap), not just the content.

CloudGoat's `iam_privesc_by_rollback` and `lambda_privesc` are the canonical path set to replicate.
These run at Tier 3 (emulated graph) first and Tier 4 (live) as fidelity.

---

## 7. Detection‑validation tests

Tie each technique's `expected_telemetry.event_name` to *actually captured* CloudTrail (depends on
the Live‑telemetry‑correlation feature, V2 #4): detonate, then assert the predicted event appeared.
This both closes the purple‑team loop and regression‑tests our telemetry metadata — a technique that
quietly stops emitting its claimed event becomes a red test.

---

## 8. B1 — Technique port‑list (the scenarios to add)

Best‑matched upstream is **Stratus Red Team** (Apache‑2.0, same *warm‑up → detonate → revert* model),
cross‑checked against the **Rhino 21 IAM privesc methods** and the **AWS Threat Technique Catalog**
(already referenced by shipped content). Port **reversible‑first**; the `impact` field decides which
tier may run a technique live.

Each of these becomes a tracked GitHub issue with a per‑technique impact assessment (codebase delta,
MITRE coverage delta, revert strategy, effort). Reversible‑first set:

| Tactic | Technique | Impact | Revert | MITRE |
|---|---|---|---|---|
| Persistence | `iam-backdoor-role` (add external trust) | reversible | `UpdateAssumeRolePolicy` (restore) | T1098 |
| Persistence | `lambda-backdoor-function` (resource policy) | reversible | `RemovePermission` | T1098 |
| Persistence | `sts-federation-token` | read/reversible | n/a (ephemeral) | T1098 / T1550 |
| Persistence | `rolesanywhere-create-trust-anchor` | reversible | `DeleteTrustAnchor` | T1098 |
| Persistence | `ec2-create-suspicious-keypair` | reversible | `DeleteKeyPair` | T1098 |
| Priv‑esc | `iam-create-policy-version` (+ set default) | reversible | `DeletePolicyVersion` | T1548 |
| Priv‑esc | `iam-add-user-to-group` | reversible | `RemoveUserFromGroup` | T1098 |
| Priv‑esc | `iam-put-user-policy` (inline) | reversible | `DeleteUserPolicy` | T1098 |
| Priv‑esc | `iam-pass-role-to-service` (PassRole + run) | reversible | terminate/remove created resource | T1078 |
| Defense‑evasion | `cloudtrail-event-selectors` (disable) | reversible | `PutEventSelectors` (restore) | T1562 |
| Defense‑evasion | `cloudtrail-delete` | reversible* | recreate from captured config | T1562.008 |
| Defense‑evasion | `vpc-remove-flow-logs` | reversible* | `CreateFlowLogs` (restore) | T1562 |
| Exfiltration | `ec2-share-ebs-snapshot` | reversible | reset `CreateVolumePermission` | T1537 |
| Exfiltration | `ec2-share-ami` | reversible | reset `LaunchPermission` | T1537 |
| Exfiltration | `rds-share-snapshot` | reversible | `ModifyDBSnapshotAttribute` (reset) | T1537 |
| Exfiltration | `s3-backdoor-bucket-policy` | reversible | restore prior policy | T1537 |
| Credential‑access | `secretsmanager-retrieve-secrets` | read‑only | n/a | T1552.007 |
| Credential‑access | `ssm-retrieve-securestring-parameters` | read‑only | n/a | T1552 |
| Credential‑access | `ec2-get-password-data` | read‑only | n/a | T1552 |
| Discovery | `ec2-enumerate-from-instance` | read‑only | n/a | T1580 |
| Discovery | `ses-enumerate` | read‑only | n/a | T1580 |

\* "reversible" here means *reconstructible from captured prior state*: the revert must first read and
stash the existing configuration, because the "undo" is a re‑create, not a true inverse. These carry
more revert risk than a clean inverse and should be the first to get Tier‑4 live coverage.

**Deferred to the destructive track (NOT in the reversible series):** `s3-ransomware-*`,
`ec2-deregister-ami`, `organizations-leave`. These require a `simulated_variant` and exist mainly to
*test the safety machinery* (assert Akumo gates/refuses them), not for routine runs.

---

## 9. Sequencing

1. **Tier‑2 catalog‑contract** (this PR) — free, immediate, protects everything below.
2. **First reversible ports** (persistence + IAM privesc) — highest signal, cleanest inverses.
3. **Tier‑3 emulator** harness (Moto) — residue‑freedom net for the new content.
4. **Tier‑4 `#[ignore]` tests** — fill the already‑wired live tier.
5. **Scenario/chain tests** — once technique‑aware planning lands.
6. **Detection‑validation** — once live telemetry correlation lands.

---

## References

- Stratus Red Team — AWS attack techniques: <https://stratus-red-team.cloud/attack-techniques/AWS/>
- Datadog Security Labs — Stratus cyber‑attack simulation: <https://securitylabs.datadoghq.com/articles/cyber-attack-simulation-with-stratus-red-team/>
- Rhino Security Labs — AWS IAM privilege‑escalation methods: <https://rhinosecuritylabs.com/aws/aws-privilege-escalation-methods-mitigation/>
- Rhino Security Labs — `aws-iam-privilege-escalation` repo: <https://github.com/rhinosecuritylabs/aws-iam-privilege-escalation>
- CloudGoat IAM privesc walkthroughs: <https://resources.infosecinstitute.com/topic/cloudgoat-walkthrough-series-iam-privilege-escalation-by-rollback/>
- Pacu checklist (RedFox): <https://www.redfoxsec.com/blog/pacu-checklist-for-aws-security>
- LocalStack integration‑test docs: <https://docs.localstack.cloud/contributing/integration-tests/>
- LocalStack OSS‑alternatives / Moto comparison (2026): <https://codenote.net/en/posts/localstack-archived-oss-alternatives-comparison/>
- AWS Threat Technique Catalog: <https://aws-samples.github.io/threat-technique-catalog-for-aws/matrix.html>
