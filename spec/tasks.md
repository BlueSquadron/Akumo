# Akumo — Implementation Task List

> **Status:** Build plan (implementation-ready). Derived from `Analysis.md` (WHY),
> `requirements.md` (WHAT), `specification.md` (HOW), and the 31 records in `ADR/`.
> **Date:** 2026-09-23
>
> This document turns the frozen specification into an **ordered, numbered, grouped task
> list** that an implementer (target: Claude Sonnet) can follow top-to-bottom to build
> Akumo v1. Every design fork is already resolved (see `spec/ADR/README.md`); this list is pure
> *build order*. It does not re-open decisions — where an implementation-level choice remains
> (a crate, a file format), it is called out as **`IMPL-CHOICE`** with a recommended default.

---

## How to use this list

- **Read this whole preamble first**, then execute groups **in order**. Groups are numbered
  `G0…G20`; tasks are numbered `G<n>.<m>` (e.g. `G4.3`). Sub-steps are bulleted.
- **Every task is self-contained**: it states its **Goal**, **Deliverables**, **Depends on**,
  **Tests** (how it is proven — nothing is "done" without passing tests), and **Docs** (what
  documentation it must add or update). Do not mark a task done until *all four* are satisfied.
- **Traceability:** each task cites the requirements (`FR-*`, `NFR-*`, `EXR-*`, `DSR-*`),
  decisions (`OQ-*`), and specification/ADR anchors it satisfies. If code and spec ever
  disagree, **the ADR wins**; if an ADR is silent, `specification.md` wins; if that is silent,
  `requirements.md` wins. Never invent behavior that contradicts a resolved ADR.
- **Definition of Done for v1** is `requirements.md §12` (8 capability gates) plus the
  release-blocking gates in `requirements.md §8.3` and `specification.md §7.3`. `G20` verifies
  them explicitly.
- **Mock-first rule (non-negotiable):** the core is built and proven against the **Mock
  provider** before the AWS adapter exists (ADR-0001/0030, NFR-EXT7). The first end-to-end
  vertical slice (`G10`) runs entirely on the Mock. AWS (`G16`) comes *after* the loop works.
- **Safety-first rule:** any task that adds a mutating capability is not done until its
  **tested revert** passes (NFR-SAF2, FR-F4) — this is release-blocking, enforced in CI.

### Milestone map (vertical slices)

| Milestone | Meaning | Completed by |
|---|---|---|
| **M0 — Skeleton** | Workspace builds; dep-direction lint green; CI runs | end of G1 |
| **M1 — Ledger & Mock** | Event ledger + Mock provider + engagement lifecycle work | end of G5 |
| **M2 — Graph & Enumerate** | Enumerate Mock → populate graph/facts with epistemic status | end of G7 |
| **M3 — Content** | Load/validate/run a YAML+CEL technique against the Mock | end of G9 |
| **M4 — Safe Execute** | Full lifecycle: dry-run → consent → detonate → verify → revert | end of G11 |
| **M5 — Plan & Chain** | Objective → paths → chain execution with halt-and-revert | end of G12 |
| **M6 — Purple + Report** | Sigma signatures + audit-grade report + STIX export | end of G14 |
| **M7 — AWS real loop** | The same loop runs against a live authorized AWS account | end of G16 |
| **M8 — Ship** | Signed single binary + signed content bundle + full docs | end of G20 |

### Fixed repository & workspace layout (do not deviate — the dependency-direction lint depends on it)

All paths in this document are **repo-root-relative**. The repository root groups artifacts by
phase; the Rust **Cargo workspace root is `code/`** (its `Cargo.toml` lives there, not at the repo
root). **`akumo-domain` and `akumo-core` MUST NOT depend on any adapter crate or any cloud SDK
crate** (ADR-0001, NFR-EXT1 — CI-enforced by `G1.4`).

```
akumo/                               # repo root
├─ README.md · LICENSE · .gitignore
├─ assets/                           # brand assets (assets/akumo-logo.png)
├─ spec/                             # SPECIFICATION PHASE (this task list lives here too)
│  ├─ Analysis.md · requirements.md · specification.md
│  ├─ tasks.md  (this file) · REFERENCES.md · V2_BACKLOG.md
│  └─ ADR/                           # ADR-0001…0031 + ADR/README.md
├─ docs/                            # PRODUCT DOCUMENTATION
│  ├─ internal/                      # how Akumo works (architecture → components → testing)
│  └─ external/                      # how to write scenarios & attacks (beginner → advanced)
├─ tests/                           # cross-cutting / end-to-end / scenario tests + fixtures
│  └─ fixtures/
├─ scripts/                         # project automation (e.g. create-v2-issues.sh)
└─ code/                            # RUST WORKSPACE ROOT (cargo lives here)
   ├─ Cargo.toml                     # workspace manifest (members below)
   ├─ rust-toolchain.toml            # pinned toolchain
   ├─ deny.toml                      # cargo-deny: licenses, advisories, dep bans
   ├─ crates/
   │  ├─ akumo-domain/               # pure types + PORT TRAITS + error enums. Zero I/O, zero SDK.
   │  ├─ akumo-core/                 # domain SERVICES over ports: engagement, enumeration,
   │  │                              #   graph, fact views, planner, execution engine,
   │  │                              #   telemetry model, reporting. Depends only on akumo-domain.
   │  ├─ akumo-dsl/                  # technique schema + YAML loader + CEL eval + validator +
   │  │                              #   Tier-2 (Starlark) / Tier-3 (WASM) host. Depends on domain.
   │  ├─ akumo-ledger/               # Persistence Port impl: event-sourced, hash-chained store
   │  │                              #   (driven adapter). Depends on domain.
   │  ├─ akumo-provider-mock/        # Mock Provider Port adapter (first-class, always shipped).
   │  ├─ akumo-provider-aws/         # AWS Provider Port adapter (aws-sdk-* lives ONLY here).
   │  ├─ akumo-content/              # content bundle: load/verify/sign default technique catalog.
   │  └─ akumo-cli/                  # driving adapters: command-per-invocation CLI + optional shell.
   ├─ bin/
   │  └─ akumo/                      # thin binary entrypoint wiring adapters → core.
   ├─ content/                       # the default technique catalog (YAML), by provider/tactic.
   │  └─ aws/…
   └─ xtask/                         # dev automation: dep-lint, docgen, leak-detector, bundle-sign.
```

> CI workflows live at the repo root under `.github/workflows/` (GitHub requires that location).
> Per-crate unit/integration tests live inside each crate under `code/`; the top-level `tests/`
> holds only cross-cutting, end-to-end, and scenario tests. Where a task below names a bare path
> like `crates/…`, `bin/…`, `content/…`, or `xtask/…`, read it as `code/…`; docs paths
> (`docs/…`), spec paths (`spec/…`), and `tests/…` are already repo-root-relative.

### Conventions

- **Rust edition 2021+**, `#![forbid(unsafe_code)]` in `akumo-domain`/`akumo-core` (unsafe only
  allowed, if ever, in adapters with justification). `clippy -D warnings` and `rustfmt` enforced.
- **Errors:** typed, explicit; a first-class `AccessDenied`/partial-permission outcome
  (`spec §1.7`, FR-C6) — never a panic on a denied provider call.
- **All cloud calls are host-brokered** through the Provider Port (NFR-SEC5). Content never
  holds an SDK client.
- **Everything is an event.** No component mutates persistent state except by appending a
  ledger event (ADR-0014). Projections are pure folds.
- **`IMPL-CHOICE` crates (recommended defaults, confirm at first use):** embedded store
  `redb` (or `sled`); CEL `cel-interpreter`; Starlark `starlark`; WASM `wasmtime`; graph
  `petgraph` (in-memory projection) over the ledger; hashing `blake3`/`sha2`; signing
  `ed25519-dalek` + `minisign`-style detached sigs; AWS `aws-sdk-*` + `aws-config`; CLI `clap`;
  shell `reedline`/`rustyline`; serialization `serde` + `serde_yaml` + `serde_json`; async
  `tokio`; STIX/Attack Flow = serde-JSON to the published schema.

---

## G0 — Repository bootstrap & governance

**Goal:** a navigable repo with the README note, license clarity, and contributor scaffolding.
No Rust code yet.

### G0.1 — README with logo, etymology, and scope note
- **Goal:** make the repo self-explaining and legally scoped.
- **Deliverables:** rewrite `README.md` to include, at minimum:
  - The **logo**: reference the already-present `assets/akumo-logo.png` at the top (e.g.
    `<img src="assets/akumo-logo.png">`). *(The logo is provided in `assets/` — do not generate
    one.)* *(Already done — the root `README.md` was written during the repo reorganization.)*
  - The **name origin** note, verbatim in spirit: *"**Akumo** fuses **Akuma** (悪魔, "devil" in
    Japanese) and **Kumo** (雲, "cloud") — offense-minded security for the cloud."*
  - One-paragraph thesis (from `Analysis.md §9`): multi-cloud-ready, attack-graph-native,
    safe-by-construction offensive framework; v1 = AWS only, provider-neutral core.
  - **Authorized-use-only** banner (NFR-COMP1/2): Akumo is for owned accounts, contracted
    engagements, and lab/CTF use.
  - Badges placeholders (CI, license), a "Documentation" section linking `docs/internal/` and
    `docs/external/`, and a quickstart pointer (filled in by G13/G18).
- **Depends on:** none.
- **Tests:** markdown link-check in CI (add to `G1.5`); image path resolves.
- **Docs:** this *is* a docs task.

### G0.2 — Contributor & project scaffolding
- **Goal:** lower the contribution barrier the analysis prizes (G6 gap).
- **Deliverables:** `CONTRIBUTING.md` (build, test tiers, how to add a technique → points to
  `docs/external/`), `CODE_OF_CONDUCT.md`, `SECURITY.md` (responsible disclosure; reflects
  NFR-COMP4), `.editorconfig`, issue/PR templates, and a `CHANGELOG.md` (Keep-a-Changelog).
- **Depends on:** G0.1.
- **Tests:** none (docs); link-check in CI.
- **Docs:** self.

### G0.3 — License & ethics headers
- **Goal:** confirm licensing and embed the ethics stance.
- **Deliverables:** confirm `LICENSE` (already present) is referenced from README; add a short
  `NOTICE`/ethics statement aligned to `requirements.md §3` and `Analysis.md §11`.
- **Depends on:** G0.1.
- **Tests:** none.
- **Docs:** self.

---

## G1 — Workspace skeleton, ports, CI (Milestone M0)

**Goal:** compiling empty hexagon with the release-blocking dependency lint live.
Realizes `spec §1.1–1.2`, ADR-0001, NFR-EXT1/EXT5.

### G1.1 — Cargo workspace + toolchain
- **Goal:** the layout above, buildable.
- **Deliverables:** workspace `Cargo.toml` listing all `crates/*`, `bin/akumo`, `xtask`;
  `rust-toolchain.toml` (pin stable); empty lib/bin crates that compile; `deny.toml`.
- **Depends on:** G0.
- **Tests:** `cargo build --workspace` and `cargo test --workspace` succeed (no-op tests OK).
- **Docs:** `docs/internal/01-architecture.md` — start it: hexagon diagram (copy `spec §1.1`),
  crate map, dependency rules.

### G1.2 — Domain types & value objects (`akumo-domain`)
- **Goal:** the provider-neutral vocabulary everything shares.
- **Deliverables:** newtypes/enums for `EngagementId`, `Scope`, `Principal`, `Credentialref`
  (opaque handle, never raw secret), `ImpactLevel` (`read` / `mutating-reversible` /
  `mutating-irreversible` / `destructive`, FR-A3), `EpistemicStatus` (`Proven` / `Absent` /
  `Unknown` / `Inferred` + optional confidence, ADR-0017), `ProviderId`, `Region`, typed
  `AkumoError` (incl. `AccessDenied`, `OutOfScope`, `Throttled`). No behavior, no I/O.
- **Depends on:** G1.1.
- **Tests:** unit tests on serialization round-trips and enum invariants.
- **Docs:** `docs/internal/02-domain-model.md` (glossary mirror of `requirements.md §4`).

### G1.3 — Port traits (`akumo-domain::ports`)
- **Goal:** define the seams; no implementations yet.
- **Deliverables:** trait definitions only (async where needed):
  - **Provider Port** as the six capability interfaces (ADR-0012, `spec §2.5`):
    `IdentityResolver`, `ResourceEnumerator`, `ActionExecutor`, `GraphMapper`,
    `MetadataProvider`, `TelemetryCollector` (v2 stub — trait present, may return `Unsupported`).
  - **Persistence Port**: `EventStore` (append hash-chained event, read stream, snapshot cache).
  - **Telemetry Port**: thin (v1), shaped for v2.
  - **Driving ports**: the operations CLI/library/CI invoke (`open_engagement`, `enumerate`,
    `compute_paths`, `preview`, `execute_step`, `execute_chain`, `revert`, `report`, …).
- **Depends on:** G1.2.
- **Tests:** compile-time; a `trait-object safety` test where applicable.
- **Docs:** `docs/internal/03-ports.md` — one section per port, with the "what the seam carries"
  list from `spec §1.3`.

### G1.4 — Dependency-direction lint (release-blocking, NFR-EXT1)
- **Goal:** make "core is provider-blind" *mechanically enforced*, not asserted.
- **Deliverables:** an `xtask dep-lint` that parses `cargo metadata` and **fails** if
  `akumo-domain` or `akumo-core` (transitively) depend on `akumo-provider-*`, `akumo-cli`, or any
  `aws-*`/cloud SDK crate. Wire it into CI as a required check.
- **Depends on:** G1.1.
- **Tests:** a fixture test: add a forbidden dep in a scratch manifest → lint fails; remove →
  passes. Keep as an ignored/README-documented manual check plus the live CI gate.
- **Docs:** `docs/internal/01-architecture.md` §"Enforced rules".

### G1.5 — CI pipeline (fast tier scaffold)
- **Goal:** every PR runs fmt, clippy, build, test, dep-lint, markdown link-check.
- **Deliverables:** `.github/workflows/ci.yml` (fast tier): `cargo fmt --check`, `clippy -D
  warnings`, `cargo test --workspace`, `xtask dep-lint`, `cargo deny check`, doc link-check.
  Live-AWS tier is a *separate*, manually/release-triggered workflow stub (filled by G17).
- **Depends on:** G1.1–G1.4.
- **Tests:** CI green on an empty workspace.
- **Docs:** `docs/internal/14-testing.md` — start it: the two-tier model (ADR-0009), what runs
  where.

> **M0 reached:** skeleton compiles, hexagon boundaries enforced, CI green.

---

## G2 — Event-sourced ledger & persistence (ADR-0014, SPEC-D3)

**Goal:** the single source of truth. Realizes DSR-1/2/4, FR-J1, NFR-REL1/2, NFR-OBS1/2/3,
`spec §2.1, §2.7`.

### G2.1 — Event envelope & hash chain
- **Goal:** immutable, tamper-evident, ordered events.
- **Deliverables:** in `akumo-domain`: `EventEnvelope { id, engagement_id, seq, timestamp, actor,
  prev_hash, hash, type, payload }`; `hash = H(prev_hash ‖ canonical(payload))` with a
  canonical (deterministic) serialization. Define the v1 **event type enum** exactly per
  `spec §2.7` (Engagement / Enumeration / Planning / Content / Execution / Loot / Reporting
  categories).
- **Depends on:** G1.2.
- **Tests:** hash-chain integrity test (tamper a payload → verification fails); canonical
  serialization is stable across runs; `seq` monotonic per engagement.
- **Docs:** `docs/internal/04-ledger.md` — envelope, hashing, event catalog table.

### G2.2 — `EventStore` embedded implementation (`akumo-ledger`)
- **Goal:** durable append-only store, engagement-isolated.
- **Deliverables:** implement `EventStore` over an embedded KV store (**IMPL-CHOICE:** `redb`).
  Append, stream-read (by engagement, by seq range), and a snapshot/cache slot for projections.
  Enforce isolation by `engagement_id` (DSR-2). Crash-safe append (fsync boundary documented).
- **Depends on:** G1.3, G2.1.
- **Tests:** durability test (append, drop, reopen → stream intact); isolation test (two
  engagements never cross-read); property test on append/read ordering.
- **Docs:** `docs/internal/04-ledger.md` §storage; `docs/internal/14-testing.md` note on
  crash-recovery tests.

### G2.3 — Projection framework
- **Goal:** rebuild any read model by folding events; snapshots are cache-only.
- **Deliverables:** a generic `Projection<S>` fold trait + a projection registry; guarantee
  projections are **pure deterministic functions** of the stream (`spec §2.1`). Provide replay
  to an arbitrary `seq` (enables FR-J2 audit-grade replay, ADR-0010).
- **Depends on:** G2.2.
- **Tests:** replay determinism (same stream → identical projection twice); snapshot-vs-replay
  equivalence.
- **Docs:** `docs/internal/04-ledger.md` §projections.

---

## G3 — Provider seam: capabilities, descriptors & the Mock provider (ADR-0012/0013/0030)

**Goal:** freeze the descriptor format and ship the always-present Mock adapter, so the core can
be exercised end-to-end with zero cloud (NFR-EXT7 gate). `spec §2.5, §1.6, §7.2`.

### G3.1 — Descriptor format (declarative)
- **Goal:** the concrete realization of SPEC-D1=C.
- **Deliverables:** serde schemas for:
  - **Enumeration descriptor**: `{ service, operation, params(static|templated), pagination,
    required_permission, response_mapping → graph assertions with epistemic status }`.
  - **Action descriptor**: `{ service, operation, params(templated from facts/inputs),
    required_permission, impact_level, expected_telemetry_ref, result_mapping → facts,
    revert_ref }` (revert = declared inverse for the saga default, ADR-0019).
  - A descriptor **catalog** loader (per-provider). Descriptors live in adapters, never in core.
- **Depends on:** G1.3, G4.1 may co-develop (mapping target vocabulary) — start with a stub
  mapping target, finalize after G4.
- **Tests:** schema validation (missing field rejected); round-trip; a sample catalog parses.
- **Docs:** `docs/internal/05-provider-seam.md`; **reference** stub
  `docs/external/reference/descriptors.md`.

### G3.2 — Mock provider adapter (`akumo-provider-mock`)
- **Goal:** a behavioral, scriptable stub (ADR-0030) implementing all six capabilities.
- **Deliverables:** an authorable synthetic environment (principals, credentials, permissions,
  resources, trust edges) that tests can construct into arbitrary attack-graph shapes;
  deterministic responses; a builder API (`MockEnv::builder()…`); support for scripted
  `AccessDenied` to exercise partial-permission paths (FR-C6). Implements `IdentityResolver`,
  `ResourceEnumerator`, `ActionExecutor`, `GraphMapper`, `MetadataProvider`, and the
  `TelemetryCollector` stub.
- **Depends on:** G3.1.
- **Tests:** each capability has unit tests; a "second provider" smoke test that the core (once
  built) can run against it unchanged.
- **Docs:** `docs/internal/05-provider-seam.md` §Mock; `docs/internal/14-testing.md` §fast tier.

### G3.3 — Capability-passing / host-brokering harness
- **Goal:** content receives only the narrow capabilities the host grants per step (NFR-SEC5,
  `spec §1.7`).
- **Deliverables:** a `CapabilityGrant` mechanism the Execution Engine (G9) will use to hand a
  single step exactly the descriptors/capabilities it declared — no ambient cloud access.
- **Depends on:** G3.1.
- **Tests:** a step cannot invoke a capability it wasn't granted (compile-time or runtime guard
  test).
- **Docs:** `docs/internal/05-provider-seam.md` §host-brokering.

---

## G4 — Attack graph, fact model & epistemic status (ADR-0015/0016/0017)

**Goal:** the provider-neutral, layered property graph the planner reasons over and techniques
reference. `spec §2.2–2.4, §5.7`. Realizes FR-D, FR-F6.

### G4.1 — Attack-core ontology (layered)
- **Goal:** the compact, stable, attack-relevant node/edge set (ADR-0015).
- **Deliverables:** in `akumo-core::graph`, the exact vocabulary from `spec §5.7`:
  - **Nodes:** `Principal` (user/role/service-identity/group), `Credential`, `Resource` (typed),
    `PermissionSet`, `ExternalIdentity`.
  - **Edges:** `MEMBER_OF`, `HAS_PERMISSION`, `CAN_ASSUME`, `TRUSTS`, `HAS_CREDENTIAL`,
    `FEDERATED_AS`, and **derived** `CAN_ACCESS`, `CAN_ESCALATE_TO`.
  - Every edge carries **epistemic status + provenance + enabling-permission reference**
    (ADR-0017, FR-D3/E3). Provider inventory attaches as node attributes (the layered model —
    no provider vocabulary in the core, EXR-6).
- **Depends on:** G1.2.
- **Tests:** graph invariants (no AWS-only concept at core level — a schema-review test/asserted
  allowlist); attribute attach/detach.
- **Docs:** `docs/internal/06-graph-and-facts.md` — ontology tables + the layering rationale.

### G4.2 — Graph as a ledger projection
- **Goal:** the graph is a fold over `FactAsserted`/`AccessDenied` events, never a separate store
  (`spec §2.1`).
- **Deliverables:** a `GraphProjection` (in-memory, **IMPL-CHOICE:** `petgraph`) rebuilt from the
  ledger; incremental application of new events; queryable API (by node, edge type, reachability
  neighborhood).
- **Depends on:** G2.3, G4.1.
- **Tests:** build a scripted stream → assert graph shape; replay determinism.
- **Docs:** `docs/internal/06-graph-and-facts.md` §projection.

### G4.3 — Fact/predicate views (ADR-0016, SPEC-D5=C)
- **Goal:** named, typed predicates over the graph for ergonomic templating & requirement
  matching — one store, no dual-model drift.
- **Deliverables:** a fact-view layer exposing named predicates (e.g. `can_assume(p, role)`,
  `has_credential(p, cred)`, `can_access(p, resource, level)`) that **compile to graph pattern
  queries**; a stable vocabulary consumed by the DSL (G8) and the planner (G12).
- **Depends on:** G4.2.
- **Tests:** each predicate resolves correctly on scripted graphs; predicate ↔ graph-pattern
  equivalence.
- **Docs:** `docs/internal/06-graph-and-facts.md` §fact views; **reference**
  `docs/external/reference/fact-vocabulary.md`.

### G4.4 — Epistemic status & coverage map (ADR-0017)
- **Goal:** honest "know vs. couldn't-see" modeling.
- **Deliverables:** enforce `PROVEN/ABSENT/UNKNOWN/INFERRED` on every attack-relevant assertion;
  a **Coverage Map** projection recording `AccessDenied` gaps (fuels FR-C6 blind-spot reporting &
  FR-E5 confidence).
- **Depends on:** G4.2.
- **Tests:** denied enumeration → `UNKNOWN` assertions + coverage entry (never a hard failure).
- **Docs:** `docs/internal/06-graph-and-facts.md` §epistemics; external explainer added in G18.

---

## G5 — Engagement manager: authorization, scope, consent, kill-switch (Milestone M1)

**Goal:** the safety and isolation gate every operation passes through. `spec §1.7, §4.2`.
Realizes FR-A, NFR-COMP1/2, DSR-2.

### G5.1 — Engagement lifecycle
- **Goal:** named, isolated engagements with their own scope/creds/state/audit.
- **Deliverables:** `open_engagement` (emits `EngagementOpened` with scope + **authorization
  affirmation** NFR-COMP1 + provider + credential ref), `ScopeAmended`, `EngagementClosed`;
  engagement context is **ambient** to all ops (FR-A2/A6 — switching engagements needs no
  restart; state lives in the ledger).
- **Depends on:** G2, G3.2.
- **Tests:** two concurrent engagements never share state/creds (DSR-2, FR-A2 AC); switch without
  restart (FR-A6).
- **Docs:** `docs/internal/07-engagement.md`.

### G5.2 — Scope enforcement
- **Goal:** refuse out-of-scope actions, always.
- **Deliverables:** a scope check wrapping every `ResourceEnumerator`/`ActionExecutor` call;
  out-of-scope target → refused + `AccessDenied`/`OutOfScope` event logged (FR-A1, NFR-COMP2,
  C2).
- **Depends on:** G5.1, G3.3.
- **Tests:** enumerate/execute outside declared scope is refused and logged (FR-A1 AC).
- **Docs:** `docs/internal/07-engagement.md` §scope.

### G5.3 — Impact-gated consent
- **Goal:** nothing above `read` runs without recorded consent at/above its impact level.
- **Deliverables:** consent model with two modes (`spec §4.2`): interactive per-step; CI
  per-chain **impact ceiling**. Emits `ConsentRecorded`.
- **Depends on:** G5.1.
- **Tests:** a mutating step cannot execute without a consent decision ≥ its impact (FR-A3 AC);
  CI ceiling halts an over-ceiling step.
- **Docs:** `docs/internal/07-engagement.md` §consent.

### G5.4 — Kill-switch / safe-abort
- **Goal:** global halt + revert of completed reversible steps.
- **Deliverables:** `KillSwitchInvoked` event; halts new steps and triggers revert (wires to G9
  revert engine); available in every mode (FR-A4).
- **Depends on:** G5.1.
- **Tests:** invoking abort stops new actions and triggers chain revert (FR-A4 AC) — full test
  after G9, stub assertion now.
- **Docs:** `docs/internal/07-engagement.md` §kill-switch.

### G5.5 — Dry authorization check (FR-A5)
- **Goal:** validate scope + credential permissions without enumerating.
- **Deliverables:** `authcheck` operation using `IdentityResolver` + `MetadataProvider` only.
- **Depends on:** G5.1.
- **Tests:** returns resolved principal + permission summary; performs no enumeration.
- **Docs:** `docs/internal/07-engagement.md` §authcheck.

> **M1 reached:** ledger + Mock + engagement lifecycle operational.

---

## G6 — Enumeration engine (FR-C)

**Goal:** descriptor-driven discovery that caches, records provenance, resumes, and degrades
gracefully. `spec §2.6`.

### G6.1 — Descriptor-driven enumeration orchestration
- **Goal:** run enumeration descriptors via `ResourceEnumerator` → `GraphMapper` → `FactAsserted`
  events.
- **Deliverables:** the orchestrator: schedule descriptors, apply `response_mapping` to emit
  graph assertions with epistemic status; central paging/region handled in the adapter (FR-B5).
- **Depends on:** G3, G4, G5.2.
- **Tests:** enumerate a Mock env → graph populated for in-scope entities (FR-C1 AC).
- **Docs:** `docs/internal/08-enumeration.md`.

### G6.2 — Caching (FR-C2, NFR-PERF2)
- **Goal:** no redundant provider calls within an engagement.
- **Deliverables:** cache keyed by `(operation, params)` within engagement; explicit refresh.
- **Depends on:** G6.1.
- **Tests:** repeated read issues no duplicate provider call unless refreshed (FR-C2 AC);
  footprint counter asserts call reduction.
- **Docs:** `docs/internal/08-enumeration.md` §caching.

### G6.3 — Concurrency bounded by safety caps (NFR-PERF1 ∩ NFR-SAF3)
- **Goal:** fast but never a DoS.
- **Deliverables:** concurrency + rate limiter shared with execution (G9.7); global caps.
- **Depends on:** G6.1.
- **Tests:** caps hold under load (NFR-SAF3 test); throttling triggers backoff (NFR-REL3).
- **Docs:** `docs/internal/08-enumeration.md` §limits.

### G6.4 — Incremental, resumable, scoped, provenance (FR-C3/C4/C5)
- **Goal:** progress is events; resume from ledger; enumerate subsets; record provenance.
- **Deliverables:** `EnumerationStarted/Completed`; resume from last event; scope selector;
  per-fact provenance field (descriptor/permission/time).
- **Depends on:** G6.1.
- **Tests:** interrupt mid-enumeration → resume without restart (FR-C3); provenance present on
  every fact (FR-C4).
- **Docs:** `docs/internal/08-enumeration.md` §resume/provenance.

### G6.5 — Partial-permission degradation (FR-C6)
- **Goal:** denied calls become blind spots, not failures.
- **Deliverables:** `AccessDenied` gap events → Coverage Map + `UNKNOWN` assertions (G4.4).
- **Depends on:** G6.1, G4.4.
- **Tests:** limited-cred engagement yields a partial graph + a report of what couldn't be seen
  (FR-C6 AC).
- **Docs:** `docs/internal/08-enumeration.md` §partial permissions.

> **M2 reached:** enumerate Mock → populated graph with epistemic status & coverage.

---

## G7 — Technique content model: schema, YAML loader, validator (ADR-0004/0005/0018)

**Goal:** the declarative technique unit (metadata + contract + steps + revert), authored in
YAML+CEL, loaded and validated but not yet executed. `spec §3.1–3.5, §3.8`. Realizes FR-F.

### G7.1 — Technique schema (Tier-0 metadata + contract)
- **Goal:** the always-declarative parts the planner/validator read.
- **Deliverables:** serde types for a technique per `spec §3.1`:
  - **metadata**: `id, name, description, author, version, provider:aws, mitre[T…]
    (+optional owasp/cis), impact, expected_telemetry[]` (FR-F2).
  - **contract**: `preconditions[]` (fact/graph predicates + required epistemic status),
    `inputs[]` (typed params), `effects[]` (facts/edges asserted on success) — the PDDL-shaped
    action model (FR-F6).
- **Depends on:** G4.3.
- **Tests:** a technique missing any mandatory field is rejected (FR-F2 AC); contract references
  resolve to known fact-view predicates.
- **Docs:** `docs/internal/09-content-model.md`; **reference**
  `docs/external/reference/technique-schema.md`.

### G7.2 — Step model (Tier-1 DSL, structure)
- **Goal:** steps that reference descriptors, template params, bind results to facts.
- **Deliverables:** step schema per `spec §3.4`: `<descriptor ref> + params(static|CEL-templated)
  → bind result → named facts`; declarative `condition` (run-if) and bounded `for-each`
  iteration placeholders (CEL wired in G8). Each mutating step declares its **compensation**
  contribution (revert, G7.4).
- **Depends on:** G7.1, G3.1.
- **Tests:** parse a multi-step technique; unknown descriptor ref rejected.
- **Docs:** `docs/internal/09-content-model.md` §steps.

### G7.3 — YAML loader + catalog (FR-F5, NFR-EXT3/EXT6)
- **Goal:** load techniques from the content path; discoverable/queryable catalog.
- **Deliverables:** loader reading `content/**/*.yaml`; a **catalog** indexable by MITRE
  tactic/technique, provider, impact, and required preconditions (the last enables the planner to
  propose techniques for a discovered path). Additive: new file = new technique, no core change.
- **Depends on:** G7.1.
- **Tests:** add a technique file → appears in catalog with zero core edits (FR-F3/NFR-EXT3 AC);
  query by tactic returns it.
- **Docs:** `docs/internal/09-content-model.md` §catalog.

### G7.4 — Revert contract representation (ADR-0019, SPEC-D8 — data only)
- **Goal:** capture compensations declaratively so the engine (G9) can replay them.
- **Deliverables:** the schema for a step's compensation (default = descriptor's declared
  inverse; override per step); `mutating-irreversible` flag path + required simulated-variant
  reference (FR-G6).
- **Depends on:** G7.2, G3.1.
- **Tests:** an irreversible step without a simulated variant is rejected at validation.
- **Docs:** `docs/internal/09-content-model.md` §revert; external in G18.

### G7.5 — Validation & versioning (FR-F2, NFR-MNT3)
- **Goal:** reject bad content at load; bind content to engine/content-model versions.
- **Deliverables:** schema+contract validation at load; escape-hatch capability declarations
  checked (NFR-SEC5); technique version + compatibility range; emits `TechniqueValidated`.
- **Depends on:** G7.1–G7.4.
- **Tests:** invalid technique not loaded; incompatible version refused with actionable error
  (NFR-USE4).
- **Docs:** `docs/internal/09-content-model.md` §validation/versioning.

---

## G8 — DSL execution: CEL engine + escape hatches (ADR-0018/0020)

**Goal:** make techniques *runnable* declaratively (CEL) with per-step Starlark/WASM escapes and
the **always-declarative-contract invariant**. `spec §3.5, §3.7`.

### G8.1 — CEL expression layer (ADR-0018, SPEC-D7=A)
- **Goal:** safe, typed expressions for predicates/conditions/templating.
- **Deliverables:** integrate CEL (**IMPL-CHOICE:** `cel-interpreter`); define the **CEL
  variable/function surface** Akumo exposes (current principal, facts, inputs, step results, safe
  helpers) — this is the frozen authoring API. Compile precondition/effect predicates and step
  templating through CEL.
- **Depends on:** G7, G4.3.
- **Tests:** CEL evaluates predicates over scripted graphs; templating substitutes fact values;
  sandbox: no I/O, no unbounded loops.
- **Docs:** `docs/internal/09-content-model.md` §CEL; **reference**
  `docs/external/reference/cel-surface.md`.

### G8.2 — Conditions & bounded iteration
- **Goal:** run-if and for-each over enumerated results, expressiveness bounded by CEL.
- **Deliverables:** evaluate step `condition`; bounded `for-each` with an explicit cap; anything
  beyond CEL escapes to Tier-2 (defined below).
- **Depends on:** G8.1.
- **Tests:** conditional step skipped when predicate false; iteration respects the cap.
- **Docs:** `docs/internal/09-content-model.md` §control flow.

### G8.3 — Tier-2 Starlark escape (per-step, ADR-0020)
- **Goal:** computed values / branching CEL can't express, without losing planner visibility.
- **Deliverables:** a per-step `script:` executor (**IMPL-CHOICE:** `starlark` crate),
  sandboxed, receiving only granted capabilities (G3.3); returns values bound into facts. The
  **contract stays Tier-0 declarative** — the planner never reads the script.
- **Depends on:** G8.1, G3.3.
- **Tests:** a Starlark step produces a fact; it cannot perform I/O or reach ungranted
  capabilities; planner still reads the technique's contract unchanged.
- **Docs:** `docs/internal/09-content-model.md` §Tier-2; external advanced tutorial in G18.

### G8.4 — Tier-3 WASM escape (per-step, ADR-0020)
- **Goal:** heavy computation / libraries (crypto, encoding, payload gen).
- **Deliverables:** a per-step `wasm:` executor on **`wasmtime`**, strict sandbox (no ambient
  WASI capabilities beyond what's granted), fact-in/fact-out ABI.
- **Depends on:** G8.3.
- **Tests:** a WASM step computes a value bound into facts; sandbox denies filesystem/network;
  determinism where required.
- **Docs:** `docs/internal/09-content-model.md` §Tier-3; external advanced tutorial in G18.

> **M3 reached:** load + validate + evaluate a YAML+CEL technique (with escapes) against the
> Mock — not yet through the safe lifecycle.

---

## G9 — Safe execution engine (Milestone M4, ADR-0019/0021/0022/0023)

**Goal:** the safety-critical core: per-step lifecycle, dry-run+blast-radius, saga revert, one
teardown path. `spec §4.1–4.8`. Realizes FR-G. **Every mutating path needs a passing revert
test before "done."**

### G9.1 — Per-step lifecycle state machine (`spec §4.1`)
- **Goal:** `PLANNED → ANALYZED → CONSENTED → DETONATED → VERIFIED` (+ `FAILED → REVERTING →
  REVERTED`), each transition a ledger event.
- **Deliverables:** the state machine; emits `DryRunPerformed, BlastRadiusEstimated,
  ConsentRecorded, StepDetonated, StepVerified, StepReverted, ExecutionFailed`. Idempotent &
  resumable from the ledger (FR-G5, NFR-REL2).
- **Depends on:** G5, G7, G8, G3.3.
- **Tests:** interrupt after `DETONATED` → resume continues from last transition; no effect
  asserted before `VERIFIED`.
- **Docs:** `docs/internal/10-execution.md` — lifecycle diagram + ordering rationale.

### G9.2 — No warm-up; setup = compensated steps (ADR-0021, SPEC-D10=A)
- **Goal:** one teardown mechanism only.
- **Deliverables:** ensure setup is ordinary steps with saga compensations — **no** separate
  warm-up/cleanup phase in v1.
- **Depends on:** G9.1.
- **Tests:** a technique needing prerequisites creates them as compensated steps; teardown uses
  the single revert path.
- **Docs:** `docs/internal/10-execution.md` §no-warmup (note v2 lab-mode door, V2_BACKLOG #6).

### G9.3 — Dry-run + blast-radius: static default (ADR-0022, SPEC-D11)
- **Goal:** preview intended actions + estimated blast radius with **zero** provider calls.
- **Deliverables:** the `ANALYZED` phase computes preview from descriptors + contract effects +
  the graph; present created/modified/affected set (FR-G2/G3). No state-changing calls in
  dry-run (FR-G2 AC).
- **Depends on:** G9.1.
- **Tests:** dry-run of a mutating technique performs no mutating provider call; blast-radius set
  matches declared effects.
- **Docs:** `docs/internal/10-execution.md` §dry-run.

### G9.4 — Opt-in provider-assisted validation (ADR-0022)
- **Goal:** higher confidence + promote `INFERRED`/`UNKNOWN` toward `PROVEN`.
- **Deliverables:** opt-in path using provider dry-run / IAM policy simulation / read-only probes
  (capability on the adapter; Mock scripts them); upgrades epistemic status (feeds FR-E5).
- **Depends on:** G9.3, G4.4.
- **Tests:** enabling it on the Mock promotes a scripted `INFERRED` edge to `PROVEN`; disabled by
  default (zero footprint).
- **Docs:** `docs/internal/10-execution.md` §provider-assisted.

### G9.5 — Detonation + verification ordering (`spec §4.5`)
- **Goal:** fail-safe ordering: **detonate → record compensation → verify → assert effects**.
- **Deliverables:** `ActionExecutor` invocation (host-brokered); immediately write the concrete
  **compensation** to the ledger (ADR-0019); then verify via CEL/descriptor success check; only
  then assert effects into the graph.
- **Depends on:** G9.1, G7.4.
- **Tests:** a step that mutates but fails verification is still revertible (compensation exists);
  effects never asserted on failed verification (NFR-SAF4).
- **Docs:** `docs/internal/10-execution.md` §verify ordering.

### G9.6 — Revert engine: saga compensations (ADR-0019, release-blocking)
- **Goal:** undo only what happened, in reverse, restart-safe.
- **Deliverables:** replay recorded compensations in reverse; triggers = step failure,
  on-demand, kill-switch; irreversible steps refused unless consented (prefer simulated variant);
  revert-failure recorded and surfaced prominently (FR-G4, NFR-SAF4).
- **Depends on:** G9.5, G5.4.
- **Tests (release-blocking, NFR-SAF2):** detonate→revert restores pre-state on the Mock; crash
  between detonate and revert → reopen and revert succeeds (DSR-4); partial chain reverts only
  completed steps; unrevertable step is reported, not silently dropped.
- **Docs:** `docs/internal/10-execution.md` §revert; external in G18.

### G9.7 — Rate/concurrency/footprint controls (`spec §4.8`)
- **Goal:** shared safety governor across enumerate + execute.
- **Deliverables:** global rate + concurrency caps (NFR-SAF3); footprint reporter from descriptor
  metadata (NFR-PERF4); throttle backoff within caps (NFR-REL3).
- **Depends on:** G9.1, G6.3.
- **Tests:** caps never exceeded under parallel load; reported footprint matches issued calls.
- **Docs:** `docs/internal/10-execution.md` §governor.

> **M4 reached:** a single technique runs through dry-run → consent → detonate → verify → revert
> on the Mock, provably reversible.

---

## G10 — First end-to-end vertical slice on the Mock

**Goal:** prove the loop before AWS exists (NFR-EXT7, requirements §12 item 8). This is the
extensibility gate rehearsal.

### G10.1 — Wire the driving path (library-level)
- **Goal:** `open → enumerate → (single technique) execute → verify → revert → close`, all on the
  Mock, via the library API (no CLI yet).
- **Deliverables:** an integration test in `tests/` performing the full single-technique loop; a
  couple of hand-authored Mock techniques (one read-only, one mutating-reversible) in a test
  content dir.
- **Depends on:** G6, G8, G9.
- **Tests:** the end-to-end test passes with zero real cloud; the mutating technique reverts
  cleanly; ledger replay reconstructs the whole run.
- **Docs:** `docs/internal/11-vertical-slice.md` — annotated walkthrough of the loop.

### G10.2 — Extensibility gate assertion (NFR-EXT1/EXT7, release-blocking)
- **Goal:** codify "adding a provider is additive" using the Mock as the stand-in second
  provider.
- **Deliverables:** a CI test asserting the full loop runs against the Mock with **zero core
  changes**; the dep-lint (G1.4) remains green.
- **Depends on:** G10.1.
- **Tests:** the gate test is required in CI.
- **Docs:** `docs/internal/14-testing.md` §extensibility gate.

---

## G11 — Chaining & objective execution (FR-H, ADR-0023)

**Goal:** compose techniques into chains with CI-safe halt-and-auto-revert. `spec §4.7`.

### G11.1 — Chain executor
- **Goal:** run an ordered path of techniques step-by-step.
- **Deliverables:** chain execution over the G9 lifecycle; per-step consent honored; `PathComputed`
  consumed (from G12) or a hand-built chain (now).
- **Depends on:** G9.
- **Tests:** a 3-step chain on the Mock completes; effects of step N visible to step N+1.
- **Docs:** `docs/internal/12-chaining.md`.

### G11.2 — Failure policy: halt + auto-revert completed (ADR-0023, SPEC-D12=C)
- **Goal:** safe default for unattended/CI; opt-in interactive halt-and-hold.
- **Deliverables:** default halt + auto-revert of completed steps on failure; interactive
  `--hold` to inspect/fix/resume; kill-switch + on-demand revert always available (FR-H2, FR-G4).
- **Depends on:** G11.1, G9.6.
- **Tests:** injected mid-chain failure → completed steps auto-reverted (target clean); hold mode
  leaves state and resumes after fix.
- **Docs:** `docs/internal/12-chaining.md` §failure policy.

### G11.3 — Inter-step re-enumeration (FR-H4)
- **Goal:** later steps reflect new access created by earlier steps.
- **Deliverables:** refresh relevant graph state between steps when a step's effects change the
  environment.
- **Depends on:** G11.1, G6.
- **Tests:** a step that grants access → next step's preconditions see it.
- **Docs:** `docs/internal/12-chaining.md` §re-enumeration.

---

## G12 — Path-finding & planner (Milestone M5, ADR-0007/0011/0024/0025/0026/0027)

**Goal:** the attack-graph-native differentiator: objective → ranked, explainable paths.
`spec §5`. Realizes FR-E. v1 = deterministic only (ADR-0007; AI planner is V2_BACKLOG #3).

### G12.1 — Objective model (ADR-0027, OQ-3)
- **Goal:** goal predicates over the graph/fact views.
- **Deliverables:** two v1 objective classes: **reach-admin** (principal satisfies an admin
  predicate) and **reach-resource** (named access to a specified resource). Objectives are
  predicates, so new classes are additive.
- **Depends on:** G4.3.
- **Tests:** each objective predicate evaluates on scripted graphs.
- **Docs:** `docs/internal/13-planner.md` §objectives; external in G18/G19.

### G12.2 — Hybrid planner (ADR-0024, SPEC-D13=C)
- **Goal:** graph reachability over static edges **+** technique-as-action symbolic expansion for
  state-creating steps.
- **Deliverables:** fast frontier over existing `CAN_ASSUME/TRUSTS/HAS_PERMISSION/…` edges;
  symbolic expansion applying a technique whose **preconditions** hold to generate its **effects**
  (new edges/facts), extending the frontier — the mechanism that expresses multi-step chains
  (FR-E1/E2).
- **Depends on:** G12.1, G7.1 (contracts), G4.
- **Tests:** finds a purely-existing-edge path; finds a state-creating path (create-key → assume);
  reports "no path" honestly when none exists.
- **Docs:** `docs/internal/13-planner.md` §algorithm.

### G12.3 — UNKNOWN-edge handling: dual-mode honest default (ADR-0025, SPEC-D14=C)
- **Goal:** never hide real paths, never present unproven as certain.
- **Deliverables:** default surfaces **PROVEN paths prominently + candidate paths labeled
  "needs validation"**; modes `strict-proven` and `full-optimistic`; provider-assisted validation
  (G9.4) promotes candidates.
- **Depends on:** G12.2, G4.4.
- **Tests:** a graph with `UNKNOWN` edges yields labeled candidates in default mode, none in
  strict mode, all in optimistic mode.
- **Docs:** `docs/internal/13-planner.md` §uncertainty.

### G12.4 — Search strategy: tiered bounded + opt-in exhaustive (ADR-0026, SPEC-D15=C)
- **Goal:** scale on large graphs (NFR-PERF3) with a completeness escape.
- **Deliverables:** bounded **best-first** guided by the ranking score (top-K within depth/time/
  expansion budget); opt-in exhaustive mode.
- **Depends on:** G12.2, G12.5.
- **Tests:** bounded search returns top-K within budget on a large scripted graph; exhaustive
  finds all on a small one.
- **Docs:** `docs/internal/13-planner.md` §search.

### G12.5 — Ranking: composite + presets (ADR-0011, OQ-10)
- **Goal:** the score that is *both* sort key and search heuristic.
- **Deliverables:** composite **confidence → length → detectability**; presets `shortest /
  stealthiest / safest / most-reliable`; `confidence` from epistemic status, `detectability` from
  expected-telemetry (G13); **reversibility/blast-radius always surfaced** as a safety annotation
  even when not the sort key.
- **Depends on:** G12.2, G4.4.
- **Tests:** presets reorder paths as specified; safety annotation always present.
- **Docs:** `docs/internal/13-planner.md` §ranking.

### G12.6 — Explainability (FR-E3)
- **Goal:** every path is human-readable without external lookup.
- **Deliverables:** each hop states principal, action/technique, enabling fact/edge + epistemic
  status, and produced effect; emit `PathComputed`.
- **Depends on:** G12.2.
- **Tests:** rendered path contains all four fields per hop (FR-E3 AC).
- **Docs:** `docs/internal/13-planner.md` §explainability.

> **M5 reached:** declare objective → compute ranked explainable paths → hand a path to the chain
> executor (G11) → run with halt-and-revert, all on the Mock.

---

## G13 — Telemetry & detection output: Sigma-aligned (ADR-0008/0028)

**Goal:** every technique carries its expected telemetry signature + candidate detection.
`spec §6.1`. Realizes FR-I1/I3. (Live correlation FR-I2 is V2_BACKLOG #4 — leave the
`TelemetryCollector` stub only.)

### G13.1 — Expected-telemetry model (Sigma-aligned)
- **Goal:** signatures expressed in / compilable to Sigma (AWS CloudTrail taxonomy).
- **Deliverables:** the `expected_telemetry` schema (from G7.1) rendered as Sigma-compatible
  rules; a small native superset only where Sigma's taxonomy falls short (ADR-0028).
- **Depends on:** G7.1.
- **Tests:** a technique's signature parses as valid Sigma; MITRE mapping present.
- **Docs:** `docs/internal/15-telemetry.md`; **reference**
  `docs/external/reference/telemetry-signatures.md`.

### G13.2 — Candidate detection generation (FR-I3)
- **Goal:** auto-emit candidate detections per technique.
- **Deliverables:** generate candidate Sigma detections from signatures; expose in reports and
  the catalog.
- **Depends on:** G13.1.
- **Tests:** generated detection references the technique's API calls/log fields.
- **Docs:** `docs/internal/15-telemetry.md` §detections.

---

## G14 — Reporting, evidence & export (Milestone M6, ADR-0010, FR-J)

**Goal:** audit-grade, reproducible reporting rendered from ledger projections. `spec §6.2–6.3`.

### G14.1 — Report generator (FR-J3/J4/J5)
- **Goal:** reports from projections, never a drift-prone narrative.
- **Deliverables:** render discovered paths, executed chains, obtained access, per-action
  telemetry signatures, MITRE coverage; **human-readable** (Markdown/HTML) + **machine-readable**
  (JSON); **executive summary** + **technical detail** views; emit `ReportGenerated`.
- **Depends on:** G12, G13, G2.3.
- **Tests:** report content matches the ledger; both formats validate; both views present.
- **Docs:** `docs/internal/16-reporting.md`.

### G14.2 — Redaction by default (FR-J6, NFR-SEC1/2)
- **Goal:** no secret/loot leakage.
- **Deliverables:** secrets/loot redacted by default with explicit opt-in; loot stored as
  `LootRecorded` redacted references; encryption-at-rest for secret material (NFR-SEC1).
- **Depends on:** G14.1.
- **Tests:** default report contains no secret; opt-in reveals; logs never print secrets
  (NFR-SEC2).
- **Docs:** `docs/internal/16-reporting.md` §redaction; `docs/internal/17-security.md` (tool
  self-security: NFR-SEC).

### G14.3 — STIX / Attack Flow export (FR-J7)
- **Goal:** interoperate with threat-informed-defense ecosystem.
- **Deliverables:** export computed paths/chains as STIX 2.1 / MITRE Attack Flow JSON; emit
  `ChainExported`.
- **Depends on:** G12.6.
- **Tests:** export validates against the Attack Flow schema; round-trips key fields.
- **Docs:** `docs/internal/16-reporting.md` §export.

### G14.4 — Reproducibility (ADR-0010, OQ-9, FR-J2)
- **Goal:** audit-grade replay always; path re-verification where safe.
- **Deliverables:** replay any engagement from the ledger (G2.3); path re-verification scoped to
  the touched subgraph ("equivalent target" graph-structurally); no promise of byte-identical
  live re-execution.
- **Depends on:** G14.1, G12.
- **Tests:** replay reconstructs state to any `seq`; re-verification detects graph drift.
- **Docs:** `docs/internal/16-reporting.md` §reproducibility.

---

## G15 — Interfaces: CLI, shell, library, CI mode (ADR-0029, FR-K)

**Goal:** command-per-invocation CLI over the ledger + optional shell + stable machine output.
`spec §6.4`.

### G15.1 — Library/API surface (FR-K3)
- **Goal:** the domain core usable as an embeddable library.
- **Deliverables:** a clean public API in `akumo-core` for the driving operations; stability notes
  and semver policy.
- **Depends on:** G10–G14.
- **Tests:** the vertical-slice test (G10) uses this API; doc-tests on public items.
- **Docs:** `docs/internal/18-interfaces.md` §library.

### G15.2 — Command-per-invocation CLI (`akumo-cli`, ADR-0029)
- **Goal:** stateless invocations over the persistent ledger (cleanly solves FR-A6).
- **Deliverables:** `clap` commands: `engagement open/close/list/switch`, `authcheck`,
  `enumerate`, `graph query/export`, `paths` (objective + presets/modes), `preview`, `run`
  (technique/chain), `revert`, `report`, `catalog`, `bundle` (G17.3). Engagement selected by flag;
  no in-CLI mutable session state.
- **Depends on:** G15.1.
- **Tests:** CLI integration tests drive the full Mock loop; switching engagements is a flag
  (FR-A6 AC); exit codes stable.
- **Docs:** `docs/internal/18-interfaces.md` §CLI; **external quickstart** starts in G18.

### G15.3 — Structured, versioned machine output (FR-K2/K4)
- **Goal:** CI-consumable stable output.
- **Deliverables:** `--output json` with a **versioned** schema on every command; non-interactive
  mode with per-chain impact ceiling (G5.3).
- **Depends on:** G15.2.
- **Tests:** JSON schema validates; schema version bump policy tested.
- **Docs:** `docs/internal/18-interfaces.md` §machine output.

### G15.4 — Optional interactive shell (ADR-0029)
- **Goal:** exploratory ergonomics without Pacu's session-statefulness.
- **Deliverables:** a `reedline`/`rustyline` shell layering the same commands over the ledger;
  holds no mutable session state of its own.
- **Depends on:** G15.2.
- **Tests:** shell commands produce identical ledger effects as CLI; no hidden state across
  commands.
- **Docs:** `docs/internal/18-interfaces.md` §shell.

### G15.5 — Actionable errors + impact/reversibility surfacing (NFR-USE2/USE4)
- **Goal:** obvious safety, helpful failures.
- **Deliverables:** every mutating command shows impact + reversibility **before** commit;
  errors state what failed, why, and next step (incl. permission-denied).
- **Depends on:** G15.2.
- **Tests:** preview output shows impact/reversibility; denied action yields actionable message.
- **Docs:** `docs/internal/18-interfaces.md` §UX.

---

## G16 — AWS provider adapter (Milestone M7, ADR-0002, FR-B2)

**Goal:** the first *real* provider behind the seam — **added without touching core** (proves
EXR-3/EXR-5). `aws-sdk-*` lives only here.

### G16.1 — AWS credential resolution & identity (FR-B3/B4)
- **Goal:** `IdentityResolver` for AWS.
- **Deliverables:** support long-term keys, temporary/session creds, assumed roles; low-privilege
  foothold as starting principal; expose current principal. `aws-config`/`aws-sdk-sts`.
- **Depends on:** G3.1, G15 (loop exists).
- **Tests (live-AWS tier):** each credential type drives an engagement against an ephemeral
  account.
- **Docs:** `docs/internal/19-aws-adapter.md`.

### G16.2 — AWS enumeration descriptors + GraphMapper (FR-C, FR-D)
- **Goal:** enumerate IAM/STS/S3/EC2/Lambda/… into the attack-core graph.
- **Deliverables:** a descriptor catalog for the v1 service set (IAM principals/policies, trust
  relationships, key resources); `GraphMapper` mapping raw responses → canonical facts with
  epistemic status; central paging/region (FR-B5).
- **Depends on:** G16.1, G4.
- **Tests (live-AWS tier):** enumerate a seeded account → expected graph; partial-permission
  degradation observed (FR-C6).
- **Docs:** `docs/internal/19-aws-adapter.md` §enumeration; descriptor catalog reference.

### G16.3 — AWS ActionExecutor + provider-assisted validation (FR-G, ADR-0022)
- **Goal:** host-brokered action execution + AWS `DryRun`/IAM policy simulator.
- **Deliverables:** `ActionExecutor` for action descriptors; wire AWS `DryRun` / IAM policy
  simulation / read-only probes into G9.4.
- **Depends on:** G16.2, G9.
- **Tests (live-AWS tier):** a real detonate+revert restores state; provider-assisted validation
  promotes an inferred edge.
- **Docs:** `docs/internal/19-aws-adapter.md` §actions.

### G16.4 — AWS MetadataProvider (FR-B5)
- **Goal:** region/partition + service/operation + permission model catalog.
- **Deliverables:** the AWS `MetadataProvider`.
- **Depends on:** G16.1.
- **Tests:** region/partition resolution; permission lookups used by preconditions.
- **Docs:** `docs/internal/19-aws-adapter.md` §metadata.

### G16.5 — "AWS is a plugin" proof (EXR-3, NFR-EXT4)
- **Goal:** confirm adding AWS changed **zero** core lines.
- **Deliverables:** a diff/audit check (in review + a CI note) that G16 touched only
  `akumo-provider-aws` + `content/aws/**` (+ tests/docs); dep-lint stays green.
- **Depends on:** G16.1–G16.4.
- **Tests:** the extensibility gate (G10.2) still passes; core crates unchanged by G16.
- **Docs:** `docs/internal/01-architecture.md` §"AWS as first plugin — evidence".

> **M7 reached:** the same loop runs against a live authorized AWS account.

---

## G17 — Default technique catalog & signed content bundle (ADR-0031, FR-F)

**Goal:** real AWS techniques + the versioned, signed, independently-updatable bundle.
`spec §7.4`.

### G17.1 — Seed AWS technique catalog
- **Goal:** a meaningful v1 set spanning the kill chain (enumeration → privesc → lateral →
  persistence → exfil, reversible-first).
- **Deliverables:** YAML techniques in `content/aws/` by MITRE tactic; each with full metadata
  (MITRE, impact, expected telemetry), contract (preconditions/effects), steps, and **revert
  contract**. Prioritize well-known AWS IAM privesc paths (Analysis §4) as reversible techniques;
  gate any irreversible/destructive as simulated (FR-G6). *(Start with ~8–12 techniques; expand.)*
- **Depends on:** G16, G8.
- **Tests (release-blocking):** each technique has a detonate+revert test — mock tier always,
  live-AWS tier for release (NFR-SAF2/FR-F4). No passing revert test ⇒ excluded.
- **Docs:** each technique's page auto-generated (G18.6); catalog overview in
  `docs/internal/20-catalog.md`.

### G17.2 — Content bundle format + signing (ADR-0031, NFR-SEC5)
- **Goal:** a versioned, signed bundle loaded by the engine, offline-capable.
- **Deliverables:** `akumo-content`: bundle manifest (version, compat range, technique index),
  detached signature (**IMPL-CHOICE:** ed25519/minisign), verification on load; default catalog
  shipped alongside the binary but loadable from a content path (NFR-EXT6).
- **Depends on:** G7.3, G17.1.
- **Tests:** tampered bundle fails verification; unsigned/incompatible bundle refused; offline
  load works.
- **Docs:** `docs/internal/20-catalog.md` §bundle; **external**
  `docs/external/advanced/building-a-bundle.md` (G18).

### G17.3 — Bundle tooling (`xtask` + `akumo bundle`)
- **Goal:** build/sign/verify/publish bundles.
- **Deliverables:** `xtask bundle-build/sign/verify`; `akumo bundle install/verify/list`.
- **Depends on:** G17.2.
- **Tests:** round-trip build→sign→verify→install.
- **Docs:** `docs/internal/20-catalog.md` §tooling.

---

## G18 — Documentation: internal + external (progressive) — REQUIRED DELIVERABLE

**Goal:** the two-section documentation the project mandates. **Internal** = how Akumo works;
**External** = how users write scenarios and attacks, **beginner → advanced, strictly
progressive**. Auto-generation keeps docs from drifting (NFR-USE3). Docs are written *alongside*
each group above; this group **completes, orders, and cross-links** them and builds the tutorial
ladder.

> Throughout, each rung ends with a "You can now…" checklist and links to the next rung. No rung
> assumes concepts from a later rung.

### G18.1 — Internal docs: consolidation & index
- **Goal:** a coherent internal manual from the per-group `docs/internal/*` files.
- **Deliverables:** `docs/internal/README.md` index ordering: architecture → domain model →
  ports → ledger → provider seam → graph/facts → engagement → enumeration → content model →
  execution → chaining → planner → telemetry → reporting → interfaces → AWS adapter → catalog →
  testing → security. Ensure every component doc explains **what it does, why (ADR link), and how
  it's tested**. Add a top-level architecture overview and a "reading order for new contributors".
- **Depends on:** G1–G17 docs.
- **Tests:** link-check; every ADR referenced by at least one internal doc; every core component
  from `spec §1.2` documented.
- **Docs:** self.

### G18.2 — External rung 0: Getting started (beginner)
- **Goal:** install → authorize → first read-only engagement in minutes.
- **Deliverables:** `docs/external/00-getting-started.md`: install (binary/Homebrew/Docker/cargo),
  the authorized-use affirmation, `engagement open`, `authcheck`, `enumerate`, view the graph,
  read a report — **all against a lab or Mock target first**. Emphasize safety defaults.
- **Depends on:** G15, G17.
- **Tests:** commands in the doc are exercised by a doctest/CLI-transcript test.
- **Docs:** self.

### G18.3 — External rung 1: Running attacks safely (beginner→intermediate)
- **Goal:** read paths, preview, run a reversible technique, revert.
- **Deliverables:** `docs/external/01-running-attacks.md`: declare an **objective**, compute
  **paths**, read the explanation, **preview blast radius**, `run` a reversible technique with
  consent, then **revert**; explain impact levels and the kill-switch.
- **Depends on:** G18.2, G12, G9.
- **Tests:** transcript test on the Mock/lab.
- **Docs:** self.

### G18.4 — External rung 2: Your first technique (intermediate)
- **Goal:** author a minimal Tier-0 metadata + single Tier-1 DSL step technique.
- **Deliverables:** `docs/external/02-first-technique.md`: the technique anatomy; write metadata
  (id, MITRE, impact, expected telemetry), one enumeration/read step referencing a descriptor,
  and bind a result to a fact; load it into the catalog; test it against the Mock.
- **Depends on:** G7, G8, G3.2.
- **Tests:** the doc's example technique is a real file under `content/examples/` with a passing
  test.
- **Docs:** self.

### G18.5 — External rungs 3–6: Contracts, CEL, chaining, revert (intermediate→advanced)
- **Goal:** the full declarative authoring skillset, one concept per rung.
- **Deliverables:**
  - `03-preconditions-effects.md` — the fact model, preconditions/effects, epistemic status
    requirements; how the planner uses your contract (FR-F6).
  - `04-cel-expressions.md` — CEL predicates, conditions, bounded iteration, templating; the CEL
    surface reference.
  - `05-chaining-and-objectives.md` — how techniques chain via effects; authoring toward the two
    v1 objectives; ranking presets/modes.
  - `06-revert-contracts.md` — saga compensations, default inverses vs. overrides,
    `mutating-irreversible` + simulated variants; **writing a passing revert test**.
- **Depends on:** G8, G9, G11, G12.
- **Tests:** each rung's example is a tested file; the revert rung's example passes the
  release-blocking revert test.
- **Docs:** self.

### G18.6 — External rungs 7–9: Escape hatches, telemetry, bundles (advanced)
- **Goal:** the power-user surface + shipping content.
- **Deliverables:**
  - `07-starlark-escape.md` — when/how to drop a step to Starlark; capability grants; keeping the
    contract declarative.
  - `08-wasm-escape.md` — Tier-3 for heavy computation; the fact-in/fact-out ABI; sandbox limits.
  - `09-telemetry-and-detections.md` — authoring Sigma-aligned expected telemetry; generated
    candidate detections; purple-team workflow.
  - `advanced/building-a-bundle.md` — package, version, sign, and distribute a content bundle;
    compatibility ranges.
- **Depends on:** G8.3, G8.4, G13, G17.2.
- **Tests:** each advanced example is a tested file; the bundle example builds/signs/verifies.
- **Docs:** self.

### G18.7 — Auto-generated technique reference (NFR-USE3)
- **Goal:** per-technique docs generated from definitions so they never drift.
- **Deliverables:** `xtask docgen` renders a page per technique (metadata, MITRE, impact,
  contract, steps summary, expected telemetry, revert) into `docs/external/reference/techniques/`;
  a catalog index; CI checks generated docs are up to date.
- **Depends on:** G17.1, G13.
- **Tests:** docgen is deterministic; CI fails if committed docs are stale.
- **Docs:** `docs/internal/20-catalog.md` §docgen; the generated reference itself.

### G18.8 — "How to add a provider" & "How to add a technique" runbooks (NFR-MNT4)
- **Goal:** repeatable, documented extension procedures.
- **Deliverables:** `docs/internal/extending/add-a-provider.md` (implement six capabilities + a
  descriptor catalog; prove zero core changes) and `docs/external/advanced/add-a-technique.md`
  (end-to-end contributor path incl. tests + bundle).
- **Depends on:** G16, G17.
- **Tests:** link-check; the provider runbook cross-references the Mock/AWS adapters as worked
  examples.
- **Docs:** self.

---

## G19 — Testing hardening, mock fidelity & safety gates (ADR-0009/0030)

**Goal:** lock in the two-tier strategy and the release-blocking gates. `spec §7.1–7.3`.

### G19.1 — Fast tier completeness
- **Goal:** the whole loop + all technique logic covered on the Mock, every PR.
- **Deliverables:** ensure unit + integration coverage for core services (CEL eval, saga/ledger,
  planner, execution), plus every technique's mock detonate+revert test.
- **Depends on:** G1–G18.
- **Tests:** coverage thresholds enforced; fast tier < a few minutes.
- **Docs:** `docs/internal/14-testing.md` §fast tier.

### G19.2 — Live-AWS fidelity tier + leak detector (release-gated)
- **Goal:** semantic-fidelity backstop with no orphaned resources.
- **Deliverables:** a release-gated workflow that detonates+reverts against an ephemeral AWS
  account; a **mandatory orphaned-resource leak detector** run after every live run (OQ-8).
- **Depends on:** G16, G17.
- **Tests:** live suite passes; leak detector finds zero stragglers (fails the release if not).
- **Docs:** `docs/internal/14-testing.md` §live tier.

### G19.3 — Safety & extensibility gate tests (release-blocking, `spec §7.3`)
- **Goal:** encode all release blockers as CI gates.
- **Deliverables:** gate tests for: revert correctness (every mutating technique), scope refusal +
  kill-switch halt/revert, rate/concurrency caps under load, dependency-direction lint,
  extensibility gate (Mock as second provider, zero core changes), content validation.
- **Depends on:** G1.4, G5, G9, G10.2.
- **Tests:** each gate is a required CI check; a deliberately-broken fixture proves each gate
  actually fails when violated.
- **Docs:** `docs/internal/14-testing.md` §release gates.

### G19.4 — Mock authz-eval module (optional, deferred)
- **Goal:** note the deferred C-option (ADR-0030) without building it.
- **Deliverables:** a documented extension point only; **do not implement** unless the tier gap
  proves costly (kept out of v1 scope).
- **Depends on:** G3.2.
- **Tests:** n/a.
- **Docs:** `docs/internal/14-testing.md` §deferred.

---

## G20 — Packaging, distribution & v1 Definition-of-Done (Milestone M8, ADR-0003)

**Goal:** ship a signed single binary + signed content bundle, and **verify every v1 DoD gate**.
`spec §7.5`, `requirements.md §12`.

### G20.1 — Single static binary + reproducible signed builds
- **Goal:** low-friction distribution (NFR-PORT2).
- **Deliverables:** static binary for Linux/macOS (Windows where feasible); reproducible builds;
  **signed** releases; container image (NFR-PORT3); distribution via binary release + Homebrew +
  Docker (+ `cargo install`).
- **Depends on:** G15, G16, G17.
- **Tests:** binary runs on target platforms; signature verifies; container runs the loop;
  reproducible-build check.
- **Docs:** `docs/internal/21-packaging.md`; README quickstart finalized.

### G20.2 — Versioning & compatibility policy (NFR-MNT3)
- **Goal:** engine ↔ content-model compatibility is explicit.
- **Deliverables:** semver for engine + content model; a technique declares supported
  engine/content-model versions; documented compatibility policy.
- **Depends on:** G7.5, G17.2.
- **Tests:** incompatible technique/bundle refused with actionable error.
- **Docs:** `docs/internal/21-packaging.md` §versioning.

### G20.3 — v1 Definition-of-Done verification (release-blocking)
- **Goal:** prove all 8 capability gates of `requirements.md §12` end-to-end.
- **Deliverables:** a DoD checklist test/report proving, against an authorized AWS target, an
  operator can: (1) declare scope + open an isolated engagement; (2) enumerate into a
  provider-agnostic graph; (3) compute explainable paths toward an objective; (4) preview →
  execute → verify → **revert** through the safe lifecycle; (5) chain along a path with
  halt-and-revert; (6) get per-action MITRE + expected telemetry; (7) produce an audit-grade
  reproducible report; (8) do all of it **through the seam**, proven by the same core running the
  full loop against the **Mock** with zero core changes.
- **Depends on:** G1–G19.
- **Tests:** the DoD suite passes; all release-blocking gates (G19.3, G17.1 revert tests,
  G19.2 leak detector) green.
- **Docs:** `docs/internal/22-v1-done.md` — the DoD matrix mapping each gate → the test proving
  it; update `CHANGELOG.md` for the v1 release.

### G20.4 — v2 backlog handoff
- **Goal:** make sure nothing deferred is lost.
- **Deliverables:** confirm each `spec/V2_BACKLOG.md` item is filed as a `v2` GitHub issue (the
  repo already has `scripts/create-v2-issues.sh`); link the backlog from the README roadmap.
- **Depends on:** G20.3.
- **Tests:** n/a (process).
- **Docs:** README §roadmap.

> **M8 reached — v1 shippable.**

---

## Appendix A — Requirement → Group coverage (spot-check)

| Requirement area | Built in |
|---|---|
| FR-A authorization/scope/consent/kill-switch | G5 |
| FR-B provider abstraction / AWS | G3, G16 |
| FR-C enumeration | G6 |
| FR-D attack graph | G4 |
| FR-E path computation & explanation | G12 |
| FR-F technique content model | G7, G8, G17 |
| FR-G safe execution engine | G9 |
| FR-H chaining & objectives | G11, G12 |
| FR-I telemetry/detection | G13 |
| FR-J reporting/evidence/export | G14 |
| FR-K interfaces (CLI/library/CI/shell) | G15 |
| NFR-EXT extensibility (release gate) | G1.4, G3, G10.2, G16.5, G19.3 |
| NFR-SAF safety / tested revert (release gate) | G9, G17.1, G19.2/19.3 |
| NFR-SEC tool self-security | G3.3, G8.3/8.4, G14.2, G17.2 |
| NFR-OBS/REL audit, resume, tamper-evidence | G2 |
| NFR-PORT/MNT packaging, testing, versioning | G19, G20 |
| Documentation (internal + external progressive) | G18 (+ per-group docs) |

## Appendix B — ADR → Group index

D1/0012 seam → G3 · D2/0013 provider-scoped → G7/G17 · D3/0014 ledger → G2 ·
D4/0015 graph → G4 · D5/0016 facts → G4.3 · D6/0017 epistemics → G4.4 ·
D7/0018 YAML+CEL → G7/G8 · D8/0019 saga revert → G9.6 · D9/0020 escapes → G8.3/8.4 ·
D10/0021 no-warmup → G9.2 · D11/0022 dry-run → G9.3/9.4 · D12/0023 chain policy → G11.2 ·
D13/0024 hybrid planner → G12.2 · D14/0025 unknown edges → G12.3 · D15/0026 search → G12.4 ·
D16/0028 Sigma → G13 · D17/0029 CLI → G15 · D18/0030 mock → G3.2/G19 · D19/0031 bundle → G17 ·
0001 hexagonal → G1 · 0002 AWS-only → G16 · 0003 Rust single-binary → G1/G20 ·
0007 deterministic planning → G12 · 0008 expected-telemetry → G13 · 0009 CI tiering → G19 ·
0010 reproducibility → G14.4 · 0011 ranking → G12.5 · 0027 objectives → G12.1.

---

*End of task list. Build top-to-bottom; keep every ADR sacrosanct; nothing is done without its
tests and its docs.*
