# Akumo — Requirements Specification

> **Status:** Requirements draft (no implementation details). Derived from `Analysis.md`.
> **Date:** 2026-09-21
>
> This document specifies **what Akumo must do and how well** — not how it is built. It
> deliberately avoids language, library, and design choices; those belong to a later design
> phase. The end goal is a real, shippable tool, so every requirement is written to be
> **atomic, testable, and traceable**.

---

## 1. Purpose & Relationship to Analysis

`Analysis.md` established the field's whitespace: the field splits into tools that *find*
attack paths (read-only), tools that *safely detonate* atomic techniques against synthetic
targets, and tools that *exploit for real* but with no safety rails, no path reasoning, and
single-cloud reach. Akumo's thesis is to **unify discovery, attack-path reasoning, and safe
execution into one loop**.

This document turns that thesis into requirements. Where useful, requirements are traced
back to the analysis via `Analysis.md §8` gaps **G1–G7** and `Analysis.md §9`
differentiators **D1–D9** (see §14 Traceability).

---

## 2. Version 1 Scope Decision

**Decision (v1): Akumo targets a single cloud provider — Amazon Web Services (AWS) — only.**

**Rationale.** AWS has the deepest public offensive-security research (privesc paths, IAM
abuse), the richest API surface, and is the shared home of every flagship inspiration
(Pacu is AWS-only; Stratus, CloudFox, Grimoire all lead with AWS). Concentrating v1 on one
provider lets Akumo go *deep* on the differentiators (graph, safe execution, purple-team
output) rather than *wide* on shallow multi-cloud parity.

> **Assumption — overridable.** AWS is chosen as the v1 provider. If the intended first
> engagement environment is Azure or GCP instead, the provider choice changes but **none of
> the requirements below change** — that is the entire point of the extensibility mandate
> in §3 and §9.

**Hard constraint.** Dropping multi-cloud from v1 **must not** mean building an
AWS-shaped monolith. The core of Akumo (graph, path-finding, execution lifecycle, technique
contract, reporting) **must be provider-agnostic**, with AWS implemented purely as the first
*provider adapter* behind a stable seam. Adding Azure/GCP/Kubernetes later must be an
*additive* effort (implement a new adapter + provider-specific techniques), **never a
rewrite of the core**. This is specified normatively in §9 (Extensibility) and treated as a
v1 acceptance gate.

---

## 3. Guiding Principles (normative intent)

These principles constrain every requirement that follows:

1. **Safety before capability.** A capability that cannot be exercised safely is not shipped
   until it can. Reversibility and blast-radius control are prerequisites, not features.
2. **Authorized use only.** Akumo assumes and enforces that the operator is authorized to
   test the target. Scope, consent, and audit are built in.
3. **Provider-agnostic core, provider-specific edges.** The interesting logic is cloud-
   independent; cloud specifics live only at well-defined seams.
4. **Everything is evidence.** Every action is reproducible, explainable, and logged to an
   audit-grade trail.
5. **Offense that teaches defense.** Every offensive action carries its defensive meaning
   (technique mapping + expected telemetry).
6. **Content is data, not code-you-must-trust.** Techniques are declarative, versioned, and
   testable, with a mandatory revert contract.

---

## 4. Definitions & Glossary

| Term | Meaning |
|---|---|
| **Provider** | A cloud/identity platform Akumo can operate against (v1: AWS only). |
| **Provider adapter** | The isolated component implementing all provider-specific behavior behind the core's stable interface. |
| **Session / Engagement** | A scoped, authorized unit of work with its own credentials, state, target boundary, and audit trail. |
| **Enumeration** | Authenticated discovery of principals, resources, permissions, and trust relationships in the target. |
| **Attack graph** | A model of principals, permissions, resources, and trust edges over which paths are computed. |
| **Technique** | A single, declaratively-defined offensive action with a defined lifecycle and a mandatory revert contract. |
| **Attack path** | An ordered, reachable sequence of edges/techniques leading from a starting principal to an objective. |
| **Chain** | An executed sequence of techniques realizing a path. |
| **Detonate** | Execute a technique/chain against the authorized target. |
| **Revert** | Undo the observable effects of a detonation. |
| **Blast radius** | The set of resources/principals an action would create, modify, or affect. |
| **Loot** | Data or access obtained during an engagement (credentials, tokens, exfiltrated objects). |
| **Telemetry signature** | The provider log/event evidence an action is expected to (or observed to) generate. |
| **Objective** | An operator-declared goal state the path-finder plans toward. |

**Requirement keywords.** MUST / MUST NOT / SHOULD / MAY per RFC 2119.
**Priority tags.** `[M]` Must-have (v1), `[S]` Should-have (v1 if feasible),
`[C]` Could-have (nice-to-have), `[W]` Won't-have in v1 (planned later).

---

## 5. Actors & Personas

- **Offensive engineer / pentester (primary).** Runs authorized engagements; wants depth,
  control, and evidence for a report.
- **Purple-team / detection engineer.** Runs techniques to validate detections; wants
  MITRE mapping and telemetry output.
- **Red-team automation / CI pipeline (non-human actor).** Invokes Akumo non-interactively
  on a schedule or per-commit; wants a stable programmatic interface and machine-readable
  output.
- **Engagement lead / reviewer.** Consumes reports and audit trails; wants reproducibility
  and scope assurance.
- **Content author / contributor.** Writes and tests new techniques; wants a declarative,
  low-barrier, testable content model.
- **Blue-team / SOC (indirect consumer).** Receives detection guidance and telemetry
  signatures produced by engagements.

---

## 6. Assumptions, Constraints & Dependencies

- **A1.** The operator possesses valid, authorized credentials for the target and legal
  authorization to test it.
- **A2.** The target is a live cloud environment (v1: AWS account/organization) reachable
  over its APIs.
- **A3.** v1 assumes a single provider is active per engagement (AWS).
- **C1.** Akumo MUST NOT require deploying persistent agents into the target to function.
- **C2.** Akumo MUST operate within the permissions of the supplied credentials and MUST NOT
  attempt to exceed authorized scope (§7.1).
- **C3.** No implementation technology is fixed by this document; NFRs (§8) constrain the
  eventual choice (e.g., distribution, portability).
- **D-dep1.** Akumo MAY consume outputs of external posture tools (Prowler, ScoutSuite,
  Cartography) but MUST NOT depend on them to perform its core functions.

---

## 7. Functional Requirements

Grouped by capability domain (A–L), following the unified loop:
`authorize → enumerate → model → reason → plan → execute safely → verify → report`.

### 7.1 FR-A — Authorization, Scoping & Sessions

- **FR-A1 [M]** — The system MUST require the operator to declare an explicit **authorized
  target scope** (e.g., account IDs / boundaries) before any action against a target.
  *AC:* an engagement cannot enumerate or execute against a target outside the declared
  scope; attempts are refused and logged.
- **FR-A2 [M]** — The system MUST support named, isolated **engagements/sessions**, each
  with its own credentials, scope, state, and audit trail.
  *AC:* two concurrent engagements never share or leak state or credentials.
- **FR-A3 [M]** — The system MUST classify every technique by **impact level** (e.g.,
  read-only / mutating-reversible / mutating-irreversible / destructive) and MUST require
  explicit operator consent for anything above read-only.
  *AC:* a mutating technique cannot execute without a recorded consent decision at or above
  its impact level.
- **FR-A4 [M]** — The system MUST provide a global **kill-switch / safe-abort** that halts
  in-flight actions and initiates revert of completed reversible steps.
  *AC:* invoking abort stops new actions and triggers revert of the current chain.
- **FR-A5 [S]** — The system SHOULD support **dry authorization checks** (validate scope +
  credential permissions) without performing enumeration.
- **FR-A6 [M]** — Switching engagements MUST NOT require restarting the application.
  *(Directly addresses a known Pacu limitation.)*

### 7.2 FR-B — Provider Abstraction & Connectivity

- **FR-B1 [M]** — The system MUST access all target functionality **through a provider
  adapter**; no core component may contain provider-specific logic (see §9).
  *AC:* removing the AWS adapter leaves the core buildable and unit-testable with a mock
  provider.
- **FR-B2 [M]** — The system MUST support AWS as the v1 provider adapter, covering the
  services required by v1 enumeration and technique content.
- **FR-B3 [M]** — The provider adapter MUST support multiple **credential/identity input
  types** appropriate to the provider (e.g., long-term keys, temporary/session credentials,
  assumed roles).
  *AC:* an engagement can be driven by each supported credential type.
- **FR-B4 [S]** — The system SHOULD support operating from a **partial/low-privilege
  foothold** (a single set of discovered credentials) as the starting principal, not only
  from admin credentials.
- **FR-B5 [S]** — The adapter SHOULD expose provider **region/partition** handling centrally
  so techniques and enumeration need not repeat it. *(Generalizes a Pacu design strength.)*

### 7.3 FR-C — Enumeration & Discovery

- **FR-C1 [M]** — The system MUST perform authenticated enumeration of the target's
  **principals, permissions/policies, resources, and trust relationships** within scope.
  *AC:* after enumeration, the graph (§7.4) is populated with these entities for the
  authorized scope.
- **FR-C2 [M]** — Enumeration MUST **cache** retrieved data within an engagement to avoid
  redundant API calls and reduce the target-side log footprint.
  *AC:* repeated reads within an engagement do not re-issue identical provider calls unless
  explicitly refreshed. *(Generalizes Pacu's caching strength; supports NFR-PERF.)*
- **FR-C3 [M]** — Enumeration MUST be **incremental and resumable**; an interrupted
  enumeration can continue without restarting from zero.
- **FR-C4 [S]** — The system SHOULD record, per discovered fact, its **provenance** (which
  call/permission produced it and when).
- **FR-C5 [S]** — The system SHOULD support **scoped/targeted enumeration** (enumerate a
  subset) to control cost and footprint.
- **FR-C6 [M]** — Enumeration MUST degrade gracefully under **partial permissions**,
  recording access-denied gaps rather than failing the engagement.
  *AC:* an engagement with limited credentials still yields a partial graph plus a report of
  what could not be seen.

### 7.4 FR-D — Attack Graph Modeling

- **FR-D1 [M]** — The system MUST represent the environment as a **graph** of principals,
  permissions, resources, and trust edges, as a first-class, queryable model.
- **FR-D2 [M]** — The graph model MUST be **provider-agnostic** in its core node/edge
  semantics; provider specifics are attributes supplied by the adapter (see §9).
  *AC:* the graph schema contains no AWS-only concepts at its core level.
- **FR-D3 [M]** — Each edge MUST carry the **reason it exists** (the permission/trust that
  enables it) to support explainability (FR-E3).
- **FR-D4 [S]** — The system SHOULD allow the graph to be **exported** in an open,
  machine-readable format for external analysis/visualization.
- **FR-D5 [S]** — The system SHOULD support **ingesting external posture data** (e.g.,
  Prowler/ScoutSuite/Cartography findings) as supplementary graph attributes/nodes.
  *(Consume, don't re-implement, CSPM — Analysis §6.4/§9.)*
- **FR-D6 [C]** — The system COULD support graph **diffing** across enumeration runs to show
  environmental change over time.

### 7.5 FR-E — Attack-Path Computation & Explanation

- **FR-E1 [M]** — The system MUST compute **reachable attack paths** (privilege escalation
  and lateral movement) over the graph from a given starting principal.
  *AC:* given a foothold principal, the system enumerates paths to higher-privilege
  principals/resources it can reach.
- **FR-E2 [M]** — The system MUST support **objective-directed** path-finding: given an
  operator-declared objective (e.g., access to a specified resource), it returns reachable
  paths to that objective or reports none exist. *(D4.)*
- **FR-E3 [M]** — Every returned path MUST be **explainable**: each step states the
  principal, the action, and the permission/trust that makes it possible.
  *AC:* a human can read a path and understand *why* each hop is possible without external
  lookup.
- **FR-E4 [S]** — The system SHOULD **rank** paths by operator-selectable criteria (e.g.,
  fewest steps, least noise/telemetry, lowest impact).
- **FR-E5 [S]** — Path computation SHOULD be based on **discovered facts**, clearly
  distinguishing *proven* edges from *inferred/uncertain* ones (e.g., where resource
  policies may override identity permissions — a known CloudFox limitation, Analysis §6.5).
- **FR-E6 [W] (deferred to v2 — see §13.1)** — The system MAY later provide an **AI-assisted
  planner** that proposes paths or next steps in natural language; if provided, it MUST be
  advisory and human-gated, and MUST NOT execute anything without passing the same
  safety/consent controls as any other action. *(D9 — deferred to protect safety/scope while
  the deterministic graph engine is proven first.)*

### 7.6 FR-F — Technique Library & Content Model

- **FR-F1 [M]** — Offensive actions MUST be expressed as **techniques** in a declarative,
  versioned content model — not as free-form imperative code the operator must trust. The
  **primary authoring surface is a declarative, fact-based step-DSL** (a sequence of
  host-brokered API calls with parameters and named fact inputs/outputs); **embedded
  scripting (Tier 2) and WASM (Tier 3) are escape hatches** for the minority of techniques
  needing arbitrary orchestration or heavy computation. *(OQ-7 resolved — §13.1; inverts
  Pacu's imperative-Python weakness, Analysis §8 G6 and Appendix A.)*
- **FR-F2 [M]** — Every technique definition MUST include: a unique ID, a description,
  **MITRE ATT&CK mapping** (aligned to the ATT&CK Cloud matrices; OWASP/CIS cross-mapping
  optional), its impact level (FR-A3), its **preconditions as consumed facts/graph edges**
  and **effects as produced facts/graph edges** (FR-F6), its expected telemetry signature
  (FR-I1), and a **mandatory revert contract** (FR-G4).
  *AC:* a technique missing any mandatory field is rejected by validation.
- **FR-F3 [M]** — The content model MUST be **extensible by contributors** without modifying
  core components; adding a technique is additive.
  *AC:* a new technique can be added and loaded without changes to enumeration, graph, or
  execution engine code.
- **FR-F4 [M]** — Every technique MUST be **individually testable**, including automated
  verification that its revert restores the pre-detonation state.
  *AC:* CI can run a per-technique test that detonates and asserts successful revert.
- **FR-F5 [S]** — The library SHOULD organize techniques by kill-chain phase and MITRE
  tactic and make the catalog **discoverable/queryable** by the operator.
- **FR-F6 [M]** — Techniques MUST declare their **preconditions (consumed facts/graph edges)
  and effects (produced facts/graph edges)** using a shared fact model. This single
  declaration MUST serve both **execution** (data-flow between steps) and **path-planning**
  (the path-finder treats techniques as planning actions with preconditions/effects — FR-E),
  so a technique is defined once and reused by both. *(Fact model per CALDERA / PDDL prior
  art; Appendix A.)*

### 7.7 FR-G — Safe Execution Engine

- **FR-G1 [M]** — The execution engine MUST support a **lifecycle** for every technique:
  *precondition-check → (optional warm-up) → execute → verify → revert → (cleanup)*.
  *(Adopts and extends the Stratus lifecycle to real targets; Analysis §5.2, D1/D3.)*
- **FR-G2 [M]** — The engine MUST provide a **dry-run / preview** mode that reports the
  intended actions and their estimated **blast radius** without mutating the target.
  *AC:* dry-run of a mutating technique performs no state-changing provider calls.
- **FR-G3 [M]** — The engine MUST estimate and present **blast radius** (what will be
  created/modified/affected) before any mutating execution and require consent (FR-A3).
- **FR-G4 [M]** — Every mutating technique MUST provide a **revert** that undoes its
  observable effects; the engine MUST support **automatic revert on failure** and
  **on-demand revert** of completed steps.
  *AC:* after a failed or aborted chain, the engine restores reversible steps to their prior
  state and reports any step it could not revert.
- **FR-G5 [M]** — Execution MUST be **idempotent and resumable** with explicit, inspectable
  per-step state (e.g., pending / done / reverted / failed).
- **FR-G6 [M]** — Irreversible or destructive techniques MUST be **opt-in, gated, and
  clearly labeled**, and MUST prefer a **simulated** variant where a real action cannot be
  reversed. *(Anti-goal alignment, Analysis §11.)*
- **FR-G7 [S]** — The engine SHOULD enforce **rate/footprint controls** during execution to
  avoid accidental denial-of-service or log flooding of the target (couples to NFR-PERF/SAF).

### 7.8 FR-H — Chaining & Objective Execution

- **FR-H1 [M]** — The system MUST support composing techniques into a **chain** that
  realizes a computed attack path.
  *AC:* a path from FR-E can be handed to the engine and executed step-by-step.
- **FR-H2 [M]** — Chain execution MUST **halt-and-report** on step failure and offer to
  **revert the chain** (respecting FR-G4).
- **FR-H3 [S]** — The system SHOULD support **objective-to-execution** in one guided flow:
  declare objective → compute paths → select path → preview → execute → verify → report.
  *(D4; the flagship unified-loop user story.)*
- **FR-H4 [S]** — The system SHOULD **re-enumerate/refresh** relevant graph state between
  chain steps when a step changes the environment (so later steps reflect new access).
- **FR-H5 [C]** — The system COULD support **chains derived from the live target graph**
  (not only from static templates), differentiating from template-only tools (Analysis §6.6).

### 7.9 FR-I — Purple-Team Telemetry & Detection Output

- **FR-I1 [M]** — Every technique MUST declare its **expected telemetry signature** (the
  provider log/event evidence and relevant fields an action should generate) and its MITRE
  mapping. *(D7; Pacu lacks this.)*
- **FR-I2 [W] (deferred to v2 — see §13.1)** — The system MAY later **capture and correlate**
  the actual telemetry generated by an action where the provider allows, and present
  observed-vs-expected. v1 ships expected signatures only (FR-I1). *(Generalizes Grimoire;
  Analysis §6.1.)*
- **FR-I3 [S]** — The system SHOULD emit **detection guidance / candidate detections** per
  technique to support blue-team consumers.
- **FR-I4 [C]** — The system COULD export a **detection dataset** (correlated logs for a
  detonation) for detection-engineering workflows.

### 7.10 FR-J — Reporting, Evidence & Audit

- **FR-J1 [M]** — The system MUST maintain an **audit-grade, append-only record** of every
  action in an engagement (who, what, when, target, result). *(D8; NFR-OBS.)*
- **FR-J2 [M]** — Engagements MUST be **reproducible**: the recorded run can be reviewed and
  understood, and re-executed against an equivalent target where safe.
- **FR-J3 [M]** — The system MUST generate a **report** covering discovered paths, executed
  chains, obtained access, telemetry signatures, and MITRE coverage.
- **FR-J4 [S]** — Reports SHOULD be produced in both a **human-readable** and a
  **machine-readable** form.
- **FR-J5 [S]** — Reports SHOULD provide both an **executive summary** and a **technical
  detail** view.
- **FR-J6 [M]** — Reports and logs MUST **redact secrets/loot by default**, with explicit
  opt-in to include sensitive material (NFR-SEC).
- **FR-J7 [S]** — Computed attack paths/chains SHOULD be **exportable in an open interchange
  format** aligned to **STIX 2.1 / MITRE Attack Flow**, so findings interoperate with the
  wider threat-informed-defense ecosystem. *(Appendix A.)*

### 7.11 FR-K — Interfaces & Operating Modes

- **FR-K1 [M]** — The system MUST provide an **interactive operator interface** (command-
  driven) for guided use.
- **FR-K2 [M]** — The system MUST provide a **non-interactive / scriptable mode** suitable
  for automation and CI pipelines. *(D9 CI-native.)*
- **FR-K3 [S]** — The system SHOULD expose a **programmatic interface (library/API)** so it
  can be embedded in other tools. *(Follows Stratus's library-usability strength.)*
- **FR-K4 [S]** — Machine-facing modes SHOULD emit **structured, stable output** suitable
  for pipeline consumption.
- **FR-K5 [C]** — The system COULD provide a **visualization** of the attack graph and
  computed paths.

### 7.12 FR-L — External Data Ingestion (optional inputs)

- **FR-L1 [S]** — The system SHOULD ingest **external posture/inventory findings** to enrich
  the graph, treating them as untrusted supplementary input.
- **FR-L2 [C]** — The system COULD **prioritize paths** using ingested severity/risk signals
  from external tools.

---

## 8. Non-Functional Requirements

### 8.1 NFR-SAF — Safety

- **NFR-SAF1 [M]** — Default posture MUST be **least blast radius**: read-only unless the
  operator explicitly elevates; no mutating action without consent (couples FR-A3/FR-G3).
- **NFR-SAF2 [M]** — Every mutating capability MUST ship with a **tested revert**; a
  technique without a passing revert test MUST NOT be included in a release (couples FR-F4).
- **NFR-SAF3 [M]** — The system MUST protect the target from **accidental denial-of-service**
  via rate limiting and concurrency caps (couples FR-G7).
- **NFR-SAF4 [M]** — The system MUST **fail safe**: on unexpected error it MUST NOT leave the
  target in an unknown mutated state without recording exactly what was done.
- **NFR-SAF5 [S]** — The system SHOULD warn on and prevent actions likely to be
  **irreversible or noisy** unless explicitly acknowledged.

### 8.2 NFR-SEC — Security of the Tool Itself

- **NFR-SEC1 [M]** — Credentials and loot MUST be stored **encrypted at rest** within
  engagement state.
- **NFR-SEC2 [M]** — Secrets MUST NOT be written to logs, reports, or terminal output by
  default (couples FR-J6).
- **NFR-SEC3 [M]** — The system MUST NOT transmit target data, credentials, or loot to any
  third party or external service without explicit operator opt-in.
- **NFR-SEC4 [S]** — Engagement state SHOULD be portable/exportable in a form that keeps
  secrets protected.
- **NFR-SEC5 [S]** — The technique content-loading mechanism SHOULD constrain what a
  technique can do (least privilege for content) to reduce supply-chain risk.

### 8.3 NFR-EXT — Extensibility *(the core architectural mandate)*

- **NFR-EXT1 [M]** — All provider-specific behavior MUST reside behind a **single, stable
  internal provider interface**. Core components (enumeration orchestration, graph model,
  path-finding, execution engine, reporting) MUST contain **no provider-specific logic**.
  *AC:* adding a new provider requires implementing the interface + provider techniques and
  changes **zero** lines in the named core components.
- **NFR-EXT2 [M]** — The core data model (graph nodes/edges, technique lifecycle, engagement
  state) MUST be **provider-neutral**; provider specifics attach as attributes/extensions.
  *AC:* the core schema review shows no AWS-only concept at the core level (couples FR-D2).
- **NFR-EXT3 [M]** — Adding a **new technique** MUST be additive (no core changes) and MUST
  go through the same validation/testing as built-in techniques (couples FR-F3/FR-F4).
- **NFR-EXT4 [M]** — The architecture MUST allow **multiple providers to coexist** in a
  future version; the v1 single-provider constraint MUST be a *configuration/scoping* choice,
  **not** an assumption baked into the core.
  *AC:* a design review confirms no core component assumes exactly one provider type exists.
- **NFR-EXT5 [S]** — Each stage of the loop (enumerate / graph / reason / execute / report)
  SHOULD be **independently replaceable/testable** behind clear internal boundaries.
- **NFR-EXT6 [S]** — The system SHOULD support loading techniques/content from **outside the
  core distribution** so the catalog can grow without a full release.
- **NFR-EXT7 [M]** — A **mock/test provider** MUST be supportable so the core can be
  exercised end-to-end without any real cloud account.
  *AC:* the full loop runs against a mock provider in automated tests.

> **v1 acceptance gate:** NFR-EXT1, EXT2, EXT3, EXT4, and EXT7 are **release-blocking**.
> They are how "drop multi-cloud but stay extensible" is verified, not asserted.

### 8.4 NFR-PERF — Performance & Footprint

- **NFR-PERF1 [M]** — Enumeration MUST support **concurrency** to complete large targets in
  reasonable time, bounded by NFR-SAF3 limits.
- **NFR-PERF2 [M]** — The system MUST minimize **redundant provider API calls** via caching
  (couples FR-C2) to control both time and target-side log volume.
- **NFR-PERF3 [S]** — Path computation SHOULD remain usable on **large graphs** (thousands
  of principals/resources) without unacceptable latency; graceful strategies (limits,
  scoping) SHOULD be available.
- **NFR-PERF4 [S]** — The system SHOULD expose the **API-call / log footprint** an operation
  is expected to produce, so operators can manage stealth and cost.

### 8.5 NFR-USE — Usability

- **NFR-USE1 [M]** — Common workflows (enumerate, view paths, preview, execute, revert,
  report) MUST be achievable without reading source or content definitions.
- **NFR-USE2 [S]** — Output MUST make **impact and reversibility of an action obvious**
  before the operator commits to it.
- **NFR-USE3 [S]** — Technique and path documentation SHOULD be **auto-generated** from
  definitions so docs never drift from behavior. *(Adopts a Stratus strength.)*
- **NFR-USE4 [S]** — Errors MUST be **actionable** (what failed, why, what to do), including
  permission-denied cases during enumeration/execution.

### 8.6 NFR-REL — Reliability & Resumability

- **NFR-REL1 [M]** — Engagement state MUST be **durable**; a crash or interruption MUST NOT
  lose the record of what was already done (couples FR-G5/FR-J1).
- **NFR-REL2 [M]** — Long-running operations (enumeration, chains) MUST be **resumable**
  after interruption.
- **NFR-REL3 [S]** — The system SHOULD handle provider **throttling/transient errors** with
  safe retry/backoff without breaching NFR-SAF3.

### 8.7 NFR-OBS — Observability & Auditability

- **NFR-OBS1 [M]** — Every action MUST be **traceable** end-to-end (input → provider call →
  result → state change) in the audit record (couples FR-J1).
- **NFR-OBS2 [M]** — Logs MUST be sufficient to **reconstruct an engagement** for review and
  evidence.
- **NFR-OBS3 [S]** — The audit trail SHOULD be **tamper-evident**.

### 8.8 NFR-PORT — Portability & Distribution

- **NFR-PORT1 [M]** — The tool MUST run on the primary operator platforms (Linux and macOS
  at minimum) and MUST be usable in a CI/pipeline runner.
- **NFR-PORT2 [S]** — Distribution SHOULD minimize install friction (a low-dependency,
  self-contained delivery is strongly preferred — a lesson from Stratus's single-binary
  advantage over Pacu's runtime, Analysis §4.4/§5.3).
- **NFR-PORT3 [S]** — The tool SHOULD be runnable in a **container**.

### 8.9 NFR-MNT — Maintainability & Testability

- **NFR-MNT1 [M]** — Technique content MUST be covered by **automated tests**, including
  revert verification, runnable in CI (couples FR-F4).
- **NFR-MNT2 [M]** — The provider seam MUST be **mockable** for testing the core without a
  real account (couples NFR-EXT7).
- **NFR-MNT3 [S]** — Content and core MUST be **versioned**, with a clear compatibility
  policy between the content model and the engine.
- **NFR-MNT4 [S]** — Adding a provider or technique SHOULD be documented as a **repeatable
  procedure**.

### 8.10 NFR-COMP — Legal, Ethical & Compliance

- **NFR-COMP1 [M]** — The system MUST require and record **operator affirmation of
  authorization** per engagement (couples FR-A1).
- **NFR-COMP2 [M]** — The system MUST enforce the **declared scope boundary** and refuse
  out-of-scope actions (couples FR-A1/C2).
- **NFR-COMP3 [S]** — The system SHOULD map its activity to recognized frameworks (MITRE
  ATT&CK at minimum) to support reporting and compliance narratives.
- **NFR-COMP4 [M]** — The system MUST NOT include capabilities whose *only* purpose is
  evasion of authorized-owner defenses for malicious use; defense-evasion techniques are
  supported only as **labeled, authorized, reversible** test actions.

---

## 9. Extensibility Architecture Requirements (multi-cloud without rewrite)

This section consolidates the mandate from §2 into normative, verifiable requirements. It is
the direct answer to *"drop multi-cloud for v1 but stay extensible."* (All requirements are
also listed under NFR-EXT / FR-B / FR-D for traceability.)

- **EXR-1 [M]** — There MUST be exactly one **provider seam**. Everything a provider does
  (auth, enumeration primitives, resource/permission semantics, technique execution
  primitives, telemetry access) is reached through it.
- **EXR-2 [M]** — The **core is provider-blind**. Enumeration orchestration, the graph, path-
  finding, the execution lifecycle, chaining, reporting, and audit MUST be implemented once,
  provider-agnostically, and reused unchanged across providers.
- **EXR-3 [M]** — **AWS is a plugin, not the platform.** The AWS adapter is the first
  implementation of the seam and MUST NOT be privileged in the core.
- **EXR-4 [M]** — The **technique content model is provider-parameterized**: a technique
  declares which provider(s) and capabilities it targets; the engine runs it via the seam.
- **EXR-5 [M]** — A **future second provider** (Azure/GCP/Kubernetes/identity plane) MUST be
  addable by: (a) implementing the seam, and (b) contributing provider techniques — with **no
  change to core components**. This MUST be demonstrable via the mock provider (NFR-EXT7)
  standing in as a "second provider" in tests today.
- **EXR-6 [S]** — Where cloud concepts genuinely differ, the core SHOULD model the **general
  abstraction** (principal, permission, resource, trust edge, objective) and let the adapter
  supply specifics — never leak one provider's vocabulary into the core.
- **EXR-7 [S]** — The system SHOULD be designed so a later version can run an engagement that
  spans **more than one provider** (e.g., identity-federation paths crossing planes) without
  re-architecting the graph or path-finder.

---

## 10. Data & State Requirements

- **DSR-1 [M]** — Engagement state (scope, credentials, cache, graph, execution ledger,
  loot, audit) MUST be **persisted durably** and be **inspectable** (not an opaque black
  box). *(Improves on Pacu's private SQLite and Stratus's Terraform-only state.)*
- **DSR-2 [M]** — State MUST be **scoped to an engagement** and isolated between engagements
  (couples FR-A2).
- **DSR-3 [S]** — State SHOULD be **exportable/importable** for review, hand-off, and
  archival, honoring secret protection (NFR-SEC4).
- **DSR-4 [S]** — The execution ledger SHOULD record enough to drive **revert** even after a
  restart (couples FR-G4/NFR-REL1).

---

## 11. Out of Scope for v1 (Anti-Requirements)

- **OUT-1 [W]** — **Multiple cloud providers.** v1 ships AWS only; the architecture supports
  more (§9) but no second adapter ships in v1.
- **OUT-2 [W]** — **CSPM / compliance scanning breadth.** Akumo consumes posture output; it
  does not aim to replace Prowler/ScoutSuite's thousands of config checks (Analysis §11).
- **OUT-3 [W]** — **Destructive-by-default operations.** Irreversible impact actions are
  simulated or gated, not default capabilities (couples FR-G6/NFR-COMP4).
- **OUT-4 [W]** — **Autonomous, unsupervised exploitation.** Any AI-assisted planning is
  advisory and human-gated in v1 (couples FR-E6).
- **OUT-5 [W]** — **Persistent in-target agents.** Out of scope (couples C1).
- **OUT-6 [W]** — **Continuous/scheduled monitoring product.** v1 targets engagement-style
  runs (interactive + CI-invokable), not always-on monitoring.

---

## 12. v1 Definition of Done (capability gates)

Akumo v1 is "done" when, against an authorized AWS target, an operator can:

1. Declare scope + authorization and open an isolated engagement. *(FR-A)*
2. Enumerate principals/permissions/resources/trust into a provider-agnostic graph. *(FR-C/FR-D)*
3. Compute and read **explainable** attack paths toward a stated objective. *(FR-E)*
4. Preview an action's blast radius, execute it through the safe lifecycle, verify it, and
   **revert** it. *(FR-G)*
5. Chain techniques along a computed path with halt-and-revert on failure. *(FR-H)*
6. Obtain, per action, its MITRE mapping and expected telemetry signature. *(FR-I)*
7. Produce an audit-grade, reproducible report. *(FR-J)*
8. Do all of the above **through the provider seam**, proven by the same core running the
   full loop against a **mock provider** with zero core changes. *(§9 / NFR-EXT — the
   extensibility gate.)*

---

## 13. Decisions & Remaining Open Questions

### 13.1 Resolved decisions (2026-09-21)

| # | Decision | Choice | Notes / impact |
|---|---|---|---|
| OQ-1 | v1 cloud provider | **AWS** | First provider adapter; core stays provider-agnostic (§9). |
| — | v1 intelligence scope | **Deterministic path-finding only** | Graph-based privesc + lateral, explainable (FR-E1–E5). AI planner deferred. |
| OQ-4 | AI-assisted planner | **Deferred to v2** | FR-E6 reclassified `[W]`; safety/human-gating requirements retained for when it lands. |
| OQ-3 | Objective types in v1 | **"Reach admin privilege" + "reach a specified resource"** | Other objective classes deferred (FR-E2). |
| OQ-2 | Telemetry depth in v1 | **Expected signatures only** | FR-I1 stays `[M]`; live observed-vs-expected correlation deferred (FR-I2 → `[W]`). |
| OQ-5 | External posture ingestion | **Deferred (post-v1)** | FR-D5/FR-L remain `[S]`/`[C]`; Akumo's own enumeration stands alone. |
| — | Core/adapter language & distribution | **Rust, single static binary** | Memory/type-safe core for a safety-critical mutating tool; best-in-class sandbox host (`wasmtime`); low-friction single-binary (NFR-PORT2). v1 rides the mature (GA, 300+ services) AWS-Rust SDK with no penalty. Known v2 cost: Azure + identity-plane adapters lean on REST + auth crates rather than fat SDKs, isolated behind the provider seam (§9). |
| — | Technique content model | **Hybrid (metadata + action body)** | Declarative metadata (ID, MITRE, impact, telemetry, revert) + a sandboxed action body (FR-F, NFR-SEC5). Authoring format resolved — see OQ-7. |
| OQ-7 | Technique authoring format | **Declarative fact-based step-DSL (primary); Starlark (Tier 2) + WASM (Tier 3) escape hatches** | Chosen over scripting-primary after a prior-art survey (Appendix A). Classification aligns to **MITRE ATT&CK Cloud** (+ optional OWASP/CIS cross-map); chain interchange to **STIX 2.1 / MITRE Attack Flow** (FR-J7). The fact model (preconditions/effects) is shared by execution *and* the path-planner (FR-F6, FR-E) — a unification no incumbent has. All cloud calls host-brokered via the provider seam (NFR-SEC5). |
| OQ-6 | Graph backend | **Embedded-first, with export** | Keeps the single-binary promise (FR-D4 export); revisit only if graph scale demands an external DB. |
| OQ-8 | Revert verification in CI | **Hybrid tiering** | Mock provider on every PR (fast, free, contributor-friendly; covers all technique logic + the full loop) + a release-gated integration suite that detonates & reverts against a live ephemeral AWS account (fidelity backstop). Mandatory orphaned-resource leak detector. Satisfies release-blocking NFR-SAF2/FR-F4. |
| OQ-9 | Reproducibility scope | **Audit-grade replay (always) + path re-verification (where safe)** | "Equivalent target" defined graph-structurally, scoped to the subgraph the engagement path touched. No promise of byte-identical full re-execution against arbitrary live targets (cloud drift); full re-execution offered only for IaC/lab targets as a bonus (FR-J2). |
| OQ-10 | Default path ranking | **Composite: confidence → length → detectability** | Named presets (shortest / stealthiest / safest / most-reliable); default composite ordered confidence→length→detectability; reversibility/blast-radius always surfaced as a safety annotation even when not the sort key (FR-E4). |

### 13.2 Remaining open questions (for the design phase)

- **OQ-7 — RESOLVED (§13.1; research in Appendix A):** declarative fact-based step-DSL as the
  primary authoring surface, with Starlark (Tier 2) and WASM (Tier 3) escape hatches;
  classification aligned to MITRE ATT&CK Cloud, chain interchange to STIX 2.1 / MITRE Attack
  Flow. Residual design detail: the exact DSL grammar and the precise Tier-1 → Tier-2
  boundary.
- **OQ-8 — RESOLVED (§13.1).**
- **OQ-9 — RESOLVED (§13.1).**
- **OQ-10 — RESOLVED (§13.1).**

---

## 14. Traceability (requirements → analysis)

| Analysis driver | Addressed by |
|---|---|
| **G1** discovery→execution seam unbridged | FR-E, FR-G, FR-H, §12 DoD |
| **G2** safety not first-class in offense | FR-A3/A4, FR-G2/G3/G4/G6, NFR-SAF* |
| **G3** no attack-graph reasoning | FR-D, FR-E |
| **G4** offense fragmented per-cloud | §2, §9 EXR-*, NFR-EXT* |
| **G5** purple-team value bolted on | FR-I, FR-F2 |
| **G6** imperative/untrusted content | FR-F, NFR-MNT1, NFR-SEC5 |
| **G7** weak evidence/CI-native | FR-J, FR-K2/K3, NFR-OBS* |
| **D1** unified loop | FR-C→FR-J, §12 |
| **D2** attack-graph-native | FR-D, FR-E |
| **D3** safety as primitive | FR-G, NFR-SAF* |
| **D4** objective-seeking | FR-E2, FR-H3 |
| **D5** multi-cloud/identity parity (future) | §9 EXR-*, NFR-EXT4/EXT7 |
| **D6** declarative safe-by-construction content | FR-F, NFR-MNT1 |
| **D7** native purple-team dual output | FR-I1 |
| **D8** evidence-grade auditability | FR-J, NFR-OBS* |
| **D9** CI-native + optional AI planner | FR-K2/K3, FR-E6 |
