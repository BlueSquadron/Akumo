# Changelog

All notable changes to Akumo are documented here. The format is based on
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [1.0.0] - Unreleased

Target release version. All workspace crates are versioned `1.0.0`.

### Added
- Repository reorganized into phase-grouped top-level folders (`spec/`, `docs/`, `code/`,
  `tests/`, `assets/`).
- Project governance: `CONTRIBUTING.md`, `SECURITY.md`, `CODE_OF_CONDUCT.md`, `NOTICE`,
  `.editorconfig`.
- **G0/G1 — skeleton:** Cargo workspace under `code/`; `akumo-domain` core value objects and
  port traits; adapter/CLI/content crate skeletons; `xtask dep-lint`; CI fast-tier workflow.
- **G2 — event ledger:** hash-chained `EventEnvelope` + SHA-256 chain (`akumo-domain`); durable,
  engagement-isolated, restart-safe `FileEventStore` (`akumo-ledger`); deterministic `Projection`
  folding framework (`akumo-core`).
- **G4 — attack graph:** layered attack-core ontology (`NodeKind`/`EdgeKind`/`Assertion`/
  `CoverageGap`, `akumo-domain::graph`); `GraphProjection` folding the ledger into an `AttackGraph`
  with fact/predicate views and a coverage map, honoring categorical epistemic status
  (`akumo-core::graph`).
- **G3 — provider seam & Mock:** typed `EnumerationDescriptor`/`ActionDescriptor` (`akumo-domain::
  seam`); `MockProvider` + scriptable `MockEnvironment` implementing all six capabilities with
  host-brokered action grants and scripted access-denied paths (`akumo-provider-mock`).
- **G5 — engagement manager:** `EngagementManager` (open/amend/consent/kill-switch/close) over a
  centralized `Journal` writer; `EngagementView` projection, `EngagementContext` scope/active
  guards, `ConsentPolicy`, and dry `authcheck` (`akumo-core::engagement`). Added `EventStore::head`.
- **G6 — enumeration + M2 slice:** `EnumerationService` drives descriptors → `FactAsserted`/
  `AccessDenied` events with a per-run cache and partial-permission degradation
  (`akumo-core::enumeration`); an end-to-end test proves **M2** (open → enumerate Mock → folded
  attack graph with a coverage gap).
- **G12 — planner:** deterministic hybrid path-finder (`akumo-core::planner`) — graph reachability +
  technique-as-action expansion, dual-mode uncertainty (strict/honest/optimistic), bounded
  best-first search with composite/preset ranking, and explainable paths. Objectives: reach-admin,
  reach-resource. Added `AttackGraph::from_assertions` and the `PathComputed` event type.
- **G7 — technique content model:** YAML technique schema (metadata + structured
  precondition/effect contract + Tier-1/2/3 step bodies + saga revert), structural validation, and a
  discoverable `Catalog` loader (`akumo-dsl`). Added the `TechniqueValidated` event type.
- **G8 — DSL execution:** a self-contained, sandboxed, CEL-aligned Tier-1 expression engine
  (conditions, iteration, predicate args, `$`/`${}` templating) and the `ScriptHost` seam for
  Tier-2/3 escapes (default `UnsupportedScriptHost`; Starlark/WASM runtimes deferred behind the
  trait) (`akumo-dsl::expr`, `::script`).
- **G9 — safe execution engine + M4:** per-step lifecycle (dry-run/blast-radius → consent →
  detonate → record compensation → verify), static-by-default blast radius, host-brokered
  capability grants, and restart-safe saga revert with auto-revert on failure
  (`akumo-core::execution`). Added the execution event types. `akumo-core` now depends on
  `akumo-dsl`. Reaches **M4** (safe detonate + revert on the Mock).
- **G11 — chaining:** `ChainExecutor` runs a sequence of techniques with a CI-safe halt-and-
  auto-revert failure policy (opt-in halt-and-hold), precise per-detonation revert scoping, and
  optional inter-step re-enumeration (`akumo-core::chain`). `ExecutionOutcome` now carries its
  `detonation_id`.
- **G13 — telemetry/detection:** Sigma-aligned candidate-detection generation from each technique's
  expected-telemetry signatures, rendered as Sigma YAML (`akumo-core::telemetry`).
- **G14 — reporting/evidence:** `build_report` folds the ledger into an `EngagementReport`
  (status, coverage, paths, techniques, MITRE coverage) rendered as Markdown (exec + technical) and
  JSON, plus STIX 2.1 / MITRE Attack Flow export of computed paths (`akumo-core::report`).
- **G15 — interfaces:** the `Akumo` library facade over the ports (`akumo-core::api`); a
  command-per-invocation CLI (`engagement`/`authcheck`/`enumerate`/`paths`/`catalog`/`technique`/
  `preview`/`run`/`revert`/`report`) with `--json` output and an optional interactive shell,
  wired to the Mock provider (`akumo-cli`); `FileEventStore::list_engagements`.
- **G10 — vertical slice + extensibility gate:** an end-to-end test drives the whole loop
  (authorize → enumerate → graph → plan → execute → revert → report → close) through the `Akumo`
  facade on the Mock — the release-blocking NFR-EXT7 gate (Mock as "second provider", zero core
  changes, enforced by `dep-lint`). `akumo-core` re-exports `akumo_dsl as dsl`.
- **G16 — AWS adapter:** the first real provider (`akumo-provider-aws`) — STS identity, IAM
  user/role enumeration with a pure offline-tested graph mapper, and the create/delete-access-key
  action pair, with the AWS SDK confined to the adapter crate (proving "AWS is a plugin, zero core
  changes"). The CLI now selects `mock`/`aws` via `Box<dyn Provider>`.
- **G17 — catalog & signed bundle:** a seed AWS technique catalog under `content/aws/` (5 reversible
  techniques across persistence / privilege-escalation / defense-evasion, MITRE-mapped and
  cross-referenced to the AWS Threat Technique Catalog); a versioned, Ed25519-signed content bundle
  with per-technique SHA-256 pinning (`akumo-content::bundle`). Added a `references` field to
  technique metadata. (Bundle CLI tooling, G17.3, is a follow-up.)
- **G18 — documentation:** the external **beginner→advanced ladder** (`docs/external/`, rungs 0–9 +
  building-a-bundle & contributing-a-technique + technique-schema/CEL references), the internal
  add-a-provider runbook, and `xtask docgen` — an auto-generated per-technique reference from the
  definitions (NFR-USE3), so docs never drift from behavior.
- **G19 — test gates & leak detector:** consolidated release-blocking safety gates that prove
  failure-when-violated (`akumo-core::gates`: scope, kill-switch, consent, revert, tamper-evidence);
  `xtask leak-detector` (unreverted-detonation scan of the ledgers); and a realistic release-gated
  live-AWS workflow (OIDC creds + leak detector). Rate/concurrency-cap gate deferred with the
  execution governor.
- **G20 — packaging & v1 Definition-of-Done:** single static binary via `Dockerfile` (musl/Alpine)
  and a `release.yml` workflow (musl/macOS targets, checksums, cosign signatures, GHCR image); the
  packaging/versioning-and-compatibility policy; the tool self-security notes; and the **v1 DoD
  matrix** mapping each `requirements.md §12` capability gate to the code and test that proves it.
  README roadmap links the v2 backlog.

### Status
All 20 implementation groups (G0–G20) are complete: the full unified loop —
authorize → enumerate → graph → plan → safely execute → chain → telemetry → report/export — runs on
both the Mock and AWS behind one seam, with a signed technique catalog, a CLI + shell, the full
documentation set (internal + progressive external), and the release-blocking safety/extensibility
gates. Deferrals are tracked in `spec/V2_BACKLOG.md`.

### Changed
- All workspace crate versions set to the **1.0.0** target.

<!-- Add a new "## [x.y.z] - YYYY-MM-DD" section on each release. -->
