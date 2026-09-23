# Akumo — Technical Specification

> **Status:** Living specification, built **incrementally**. Derived from `requirements.md`
> (the WHAT) and the decisions in its §13. This document is the HOW: architecture patterns
> and component specifications that provably satisfy the requirements.
> **Date:** 2026-09-21
>
> **Method note.** Where this spec references prior art (Terraform's provider-plugin model,
> Steampipe/CloudQuery plugins, Cartography's intel modules, MITRE CALDERA's plugin/executor
> model, hexagonal architecture, event sourcing, PDDL planning), those are drawn from
> established engineering knowledge — live web retrieval was unavailable in the authoring
> session, so treat external specifics as "to re-verify at design-lock," not freshly cited.
>
> **How decisions work here.** Routine, standard-practice patterns are chosen and stated with
> rationale. Anything **consequential, opinion-dependent, or hard to reverse** is left as an
> explicit **open decision** (`SPEC-Dn`) with context, options, trade-offs, and impact — never
> silently assumed. Open decisions are resolved with the user before the affected detail is
> frozen.

---

## 0. Increment Roadmap

The spec is delivered in increments that mirror the unified loop. Each increment is
self-contained enough to review, and surfaces its own open decisions.

| # | Increment | Primary requirements | Status |
|---|---|---|---|
| **1** | **Foundational architecture & the provider seam** | NFR-EXT*, FR-B, DSR, NFR-REL/OBS | **This document** |
| 2 | Core data model: engagement state, attack graph, fact model, enumeration | FR-A, FR-C, FR-D | Planned |
| 3 | Technique content model & the step-DSL | FR-F | Planned |
| 4 | Safe execution engine: lifecycle, dry-run, blast radius, revert, transactions | FR-G, FR-H | Planned |
| 5 | Path-finding & planning + ranking | FR-E | Planned |
| 6 | Telemetry/detection, reporting/audit, interfaces | FR-I, FR-J, FR-K | Planned |
| 7 | Testing strategy, mock provider fidelity, packaging/distribution | NFR-MNT, NFR-PORT, OQ-8 | Planned |

> The order is a proposal — increments 2–7 can be reordered. Each will follow the same
> pattern: settled specs + surfaced open decisions.

---

# Increment 1 — Foundational Architecture & the Provider Seam

This increment fixes the skeleton every later increment hangs off: the architecture style,
the component decomposition, the provider seam (the release-blocking extensibility gate), the
engagement state model, and the mock provider.

## 1.1 Architecture style — Hexagonal (Ports & Adapters) *(settled)*

**Decision:** Akumo uses a **hexagonal (ports-and-adapters) architecture**. A
provider-agnostic **domain core** sits in the centre; everything cloud-specific, I/O-specific,
or human-facing lives in **adapters** that plug into **ports**.

**Why this is not a real choice.** It is the direct structural expression of the requirements
we already ratified:

- NFR-EXT1 (single provider seam) → the **Provider Port**.
- NFR-EXT2 (provider-neutral core) → the domain core depends on *ports*, never on adapters
  (dependency inversion).
- NFR-EXT3/EXT6 (additive content/providers) → new adapters/content plug in without touching
  the core.
- NFR-EXT7 (mock provider) → a Mock adapter behind the same Provider Port.
- FR-K (CLI / library / CI) → three **Driving Adapters** over one core.

```
                       DRIVING ADAPTERS (inbound)
                ┌───────────┬───────────┬───────────┐
                │ Interactive│  Library  │  CI / non-│
                │    CLI     │   / API   │ interactive│
                └─────┬──────┴─────┬─────┴─────┬─────┘
                      │            │           │
                 ┌────▼────────────▼───────────▼────┐
                 │            DOMAIN CORE            │
                 │  (provider-agnostic; pure logic)  │
                 │                                   │
                 │  Engagement · Attack Graph +      │
                 │  Fact Store · Planner · Technique │
                 │  Registry · Execution Engine ·    │
                 │  Telemetry Model · Reporting      │
                 └──┬─────────┬──────────┬───────────┘
                    │         │          │        ...ports (interfaces)
            Provider│ Persist.│ Telemetry│
              Port  │  Port   │  Port    │
            ┌───────▼──┐ ┌────▼────┐ ┌───▼──────────┐
            │ AWS      │ │ Event   │ │ Telemetry     │   DRIVEN ADAPTERS (outbound)
            │ adapter  │ │ ledger  │ │ collector adpt│
            ├──────────┤ │ store   │ └──────────────┘
            │ Mock     │ └─────────┘
            │ adapter  │
            └──────────┘
```

**Ports (v1 set):**
- **Provider Port** — all cloud interaction (§1.3). Adapters: AWS, Mock.
- **Persistence Port** — durable engagement state (§1.5).
- **Telemetry Port** — retrieving/observing provider telemetry (thin in v1; FR-I1 is
  declared-signature-only, so the live-collection adapter is a v2 seam we shape now but leave
  unimplemented).
- **Driving ports** — the operations the CLI/library/CI invoke on the core.

**Rule (enforced in CI, NFR-EXT1 acceptance):** the domain-core module MUST NOT depend on any
adapter module or any cloud SDK crate. A dependency-direction lint is release-blocking.

## 1.2 Component decomposition *(settled)*

Domain-core services and their home increment:

| Component | Responsibility | Requirements | Detailed in |
|---|---|---|---|
| **Engagement Manager** | Sessions, scope/authorization gating, consent, kill-switch | FR-A, NFR-COMP | Inc. 2 |
| **Provider Port** | The one seam to all cloud interaction | FR-B, NFR-EXT1 | §1.3 (this) |
| **Enumeration Service** | Discovery orchestration, caching, provenance, partial-perm degradation | FR-C | Inc. 2 |
| **Attack Graph + Fact Store** | Provider-neutral graph & fact model | FR-D, FR-F6 | Inc. 2 |
| **Planner** | Reachable/objective-directed path-finding + ranking | FR-E | Inc. 5 |
| **Technique Registry + Content Engine** | Load/validate/run DSL·Starlark·WASM techniques | FR-F | Inc. 3 |
| **Execution Engine** | Lifecycle, dry-run, blast radius, transactional revert, chaining | FR-G, FR-H | Inc. 4 |
| **Telemetry/Detection** | Expected-signature emission (v1); observed correlation (v2) | FR-I | Inc. 6 |
| **Reporting/Audit** | Reports + STIX/Attack Flow export from the ledger | FR-J | Inc. 6 |
| **State/Persistence** | Durable, inspectable engagement state (see §1.5) | DSR, NFR-REL/OBS | §1.5 (this) |

**Dependency inversion:** every component talks to *ports*, never to concrete adapters. The
core is compiled and unit-tested with the Mock adapter alone (NFR-EXT7 gate).

## 1.3 The Provider Seam — **SPEC-D1 (RESOLVED: capability + descriptor hybrid)**

The seam is the most consequential design choice in Akumo: it decides how provider-neutral the
core truly is, how hard multi-cloud v2 becomes, and how techniques reach the cloud.

**What the seam must carry (agreed, independent of granularity):**
1. **Credential resolution** — accept the credential/identity types of the provider (FR-B3),
   support a low-privilege starting foothold (FR-B4), expose the *current principal*.
2. **Enumeration primitives** — read principals, permissions/policies, resources, trust
   edges (FR-C1), with paging/throttling handled centrally (FR-B5, NFR-REL3).
3. **Action execution** — perform a technique's mutating/read steps as **host-brokered**
   calls (the content never holds a cloud SDK; NFR-SEC5), returning results as facts.
4. **Telemetry access** — (v2) retrieve events a detonation produced (FR-I2).
5. **Region/partition handling** — centralized so content and enumeration don't repeat it.

**The fork — how "thick" the seam is:**

| | **A. Semantic seam** | **B. Thin transport seam** | **C. Capability + descriptor (hybrid)** |
|---|---|---|---|
| Seam speaks | rich domain ops (`EnumeratePrincipals`, `ExecuteAction`, `ResolveCredential`) | auth + signed request/response + paging | a *small* set of capability interfaces; enumeration & actions driven by **declarative descriptors** the adapter maps to concrete API calls |
| Core/content sees | fully cloud-agnostic ops | raw provider API shapes | agnostic capabilities; provider-native *descriptors* live in adapter + provider-scoped content |
| Add a provider | **large** (implement every op) | **trivial** (wire auth+transport) | **moderate** (implement capabilities + a descriptor catalog) |
| Provider leakage into core | low, but risk one cloud's model becomes "the domain" | **high** — core/content becomes cloud-aware (violates spirit of NFR-EXT2) | low — specifics stay in adapter/descriptors |
| Best fit for | few, very uniform providers | a single provider forever | provider-agnostic core + provider-scoped techniques (our stated model) |

**Impact of the choice:**
- On **NFR-EXT** (multi-cloud without rewrite): A and C protect the core; B erodes it.
- On the **content model** (Inc. 3): with B, techniques embed provider API detail directly;
  with C, techniques compose adapter-provided **capabilities/descriptors** and stay declarative.
- On **v2 cost**: B is cheapest to add a *transport* but most expensive to keep the core clean;
  A is most expensive per provider; C spreads cost but adds up-front design.
- On **testability**: C and A make the Mock adapter straightforward (implement capabilities);
  B pushes provider logic up into code that is harder to mock cleanly.

**Recommendation:** **Option C (capability + descriptor hybrid).** It is the only option that
simultaneously keeps the core provider-neutral (NFR-EXT2), keeps adding a provider *additive*
(NFR-EXT3), and lets techniques stay declarative and host-brokered (FR-F, NFR-SEC5). Its cost
is up-front design of the capability interfaces and the descriptor format — which we would do
anyway to support the step-DSL. **This decision is tightly coupled to SPEC-D2.**

> **Resolved (2026-09-21): Option C.** The user expressed no preference and delegated the
> call; C is selected as the only option consistent with SPEC-D2 (provider-scoped techniques)
> and the NFR-EXT provider-neutral-core mandate. The concrete capability interfaces and the
> descriptor format are specified in Increment 2.

## 1.4 Technique provider-scoping — **SPEC-D2 (RESOLVED: provider-scoped + shared helpers)**

**The question:** are techniques **provider-scoped** (a technique targets AWS and may name
AWS-native operations, while the *engine* stays agnostic) or **cross-cloud portable** (one
technique definition runs on multiple clouds)?

| | **A. Provider-scoped techniques** | **B. Cross-cloud portable techniques** |
|---|---|---|
| A technique declares | `provider: aws` + AWS-native capability/descriptor references | abstract capabilities only; engine maps to each cloud |
| Reality check | matches how cloud attacks actually are (IAM privesc *is* AWS-specific) — Leonidas/CALDERA precedent | most cloud techniques have no faithful cross-cloud equivalent |
| Catalog reuse across clouds | low (but honest) | high in theory, lowest-common-denominator in practice |
| Engine/graph/planner | fully provider-agnostic regardless | fully provider-agnostic regardless |
| Content-model complexity | lower | higher (forces an abstraction layer over divergent clouds) |

**Impact:** determines the content model's shape (Inc. 3), how much of SPEC-D1's descriptor
catalog is provider-specific, and what "multi-cloud" honestly promises (many techniques per
cloud, not one technique for all clouds).

**Recommendation:** **Option A (provider-scoped techniques) + shared building blocks.** The
*engine* is provider-agnostic (the extensibility promise); *techniques* are provider-scoped
(the honest reality), with common helpers (signing, encoding, fact-shaping) shared across
providers. This is consistent with SPEC-D1=C.

> **Resolved (2026-09-21): Option A** (provider-scoped techniques + shared helpers).

## 1.5 Engagement state & audit — **SPEC-D3 (RESOLVED: event-sourced ledger)**

Multiple release-blocking requirements converge on the state model: durable & inspectable
(DSR-1), engagement-isolated (DSR-2), append-only audit (FR-J1), resumable (NFR-REL2),
reproducible replay (FR-J2), revert-drivable after restart (DSR-4), tamper-evident (NFR-OBS3).

| | **A. Event-sourced ledger** | **B. Mutable state + separate audit log** |
|---|---|---|
| Model | engagement = append-only ordered **events**; current state (graph, facts, execution status) is a **projection** | a mutable state store + a side audit log |
| Audit (FR-J1) | **intrinsic** — the log *is* the state's history | separate; completeness not guaranteed by construction |
| Resume/replay (NFR-REL2/FR-J2) | replay events to any point | bespoke snapshot/restore logic |
| Revert after restart (DSR-4) | executed-effect events carry revert data | must be tracked separately |
| Tamper-evidence (NFR-OBS3) | natural via **hash-chained** events | bolt-on |
| Cost | higher up-front (events, projections, schema evolution) | lower up-front; risk of state/audit divergence |

**Impact:** touches reliability, auditability, reproducibility, and revert — several of them
release-blocking safety properties.

**Recommendation:** **Option A (event-sourced ledger).** One mechanism satisfies audit,
resume, replay, revert, and tamper-evidence together, and its append-only nature is exactly
what evidence-grade engagements need. The cost (event schema discipline, projections) is real
and is accepted deliberately. The Persistence Port abstracts the storage substrate; the
substrate choice (embedded store) is deferred to Increment 2/7 as an implementation detail,
not an architecture decision.

> **Resolved (2026-09-21): Option A** (event-sourced, hash-chained ledger).

## 1.6 The Mock Provider *(settled — mandated by NFR-EXT7)*

A **Mock adapter** behind the Provider Port is a first-class, always-shipped component, not a
test fixture afterthought. It:
- **Proves extensibility** — stands in as the "second provider" so CI shows the full loop runs
  with zero core changes (the NFR-EXT release gate, requirements §12 item 8).
- **Powers the fast test tier** — deterministic, in-memory, scriptable environment state for
  per-PR tests of enumeration, planning, execution, and revert logic (OQ-8 tier 1).
- Must implement the *same* capability interfaces (SPEC-D1) as the AWS adapter, backed by an
  authorable synthetic environment (principals, permissions, resources, trust edges) so tests
  can construct arbitrary attack-graph shapes.

## 1.7 Cross-cutting patterns *(settled)*

- **Dependency inversion everywhere** — core→ports only; adapters implement ports.
- **Capability-passing for host-brokering** — content receives only the narrow, per-technique
  capabilities the host grants for that step (realizes NFR-SEC5 least-privilege content).
- **Typed, explicit errors** — including a first-class `AccessDenied`/partial-permission
  outcome so enumeration degrades gracefully (FR-C6) rather than throwing.
- **Everything through the Engagement context** — scope, credentials, and the kill-switch are
  ambient to every operation so no code path can act outside the authorized boundary (FR-A1/A4,
  NFR-COMP2).

## 1.8 Open decisions raised in Increment 1

| ID | Decision | Resolution (2026-09-21) | Coupled to |
|---|---|---|---|
| **SPEC-D1** | Provider-seam granularity | **C — capability + descriptor hybrid** (delegated; no user preference) | SPEC-D2 |
| **SPEC-D2** | Technique scoping | **A — provider-scoped + shared helpers** | SPEC-D1 |
| **SPEC-D3** | Engagement state model | **A — event-sourced, hash-chained ledger** | — |

**All three Increment 1 decisions are RESOLVED.** The seam interface (capability + descriptor)
and the state/event schema can now be frozen in Increment 2.

---

# Increment 2 — Core Data Model: State, Graph, Facts & Enumeration

Builds on Increment 1 (hexagonal core; **SPEC-D1=C** capability+descriptor seam;
**SPEC-D2=A** provider-scoped techniques; **SPEC-D3=A** event-sourced ledger). This increment
specifies the data every later component reads and writes: the event ledger and its
projections, the attack graph, the fact model, the concrete provider capability interfaces +
descriptor format, and the enumeration engine.

## 2.1 Data-model overview — one source of truth, many projections *(settled)*

Per SPEC-D3, the **event ledger is the single source of truth**; everything else is a
**projection** rebuilt by folding the event stream:

```
                append-only, hash-chained
        ┌───────────────────────────────────────┐
        │              EVENT LEDGER              │   ← source of truth (SPEC-D3)
        └───────────────┬───────────────────────┘
                        │  fold / replay
   ┌───────────┬────────┼───────────┬────────────┐
   ▼           ▼        ▼           ▼            ▼
 Attack     Coverage  Execution   Loot        Audit /
 Graph      Map       Ledger      Store       Report view
 (§2.2)    (gaps,§2.4)(status)   (redacted)   (Inc. 6)
```

This one mechanism yields audit (FR-J1), resume (NFR-REL2), replay (FR-J2),
revert-after-restart (DSR-4), tamper-evidence (NFR-OBS3), and engagement isolation (DSR-2).
Projections are **deterministic functions of the event stream** — always reconstructable,
never a competing source of truth. Snapshots of a projection are a cache/optimization only.

## 2.2 Attack-graph ontology — **SPEC-D4 (RESOLVED: layered)**

The graph is what the planner reasons over (FR-E) and what techniques' preconditions/effects
reference (FR-F6). The fork is **how canonical vs. provider-specific** its type system is.

| | **A. Rich canonical ontology** | **B. Minimal core + provider-defined types** | **C. Layered (canonical attack-core + provider inventory)** |
|---|---|---|---|
| Core defines | a full fixed set of neutral node/edge types (Principal, Credential, Resource, Grant, TrustRel; edges CAN_ASSUME/HAS_PERMISSION/TRUSTS/CAN_ACCESS…) | only abstract Node/Edge + a type string; providers declare their own types | a **small canonical set of attack-relevant** node/edge types the planner reasons over **+** a provider-attributed inventory layer beneath |
| Planner semantics | rich & stable | none stable (each provider invents edges) | stable over the attack-core; inventory is attributes |
| Risk | forcing AWS concepts into a canon Azure may not fit | weak/again-provider-specific path-finding | must draw the attack-core/inventory line well |
| Precedent | — | — | BloodHound / Cartography (canonical attack edges + rich node props) |
| Upfront cost | high | low | medium |

**Impact:** determines planner power and cross-provider consistency (NFR-EXT), the target shape
of descriptor response-mapping (§2.5), and how honestly "multi-cloud" generalizes.

**Recommendation: C (layered).** A compact, stable **attack-relevant core ontology** (identity,
credential, permission, trust, and the derived reachability edges the planner needs) keeps
path-finding strong and provider-neutral; everything else is **provider-attributed inventory**
hanging off those nodes. This is exactly how the proven graph tools (BloodHound/Cartography)
scale, and it avoids both a bloated premature canon (A) and a semantically empty core (B).

> **Resolved (2026-09-22): Option C** (layered attack-core + provider inventory). Concrete
> attack-core node/edge types are enumerated in Increment 5 (planner) alongside the edges it
> traverses.

## 2.3 Fact model & its relation to the graph — **SPEC-D5 (RESOLVED: graph + derived fact views)**

FR-F6 mandates a *shared fact model* feeding both execution (data-flow between steps) and
planning (preconditions/effects). The fork is **what a "fact" physically is**.

| | **A. Unified property graph (graph-as-facts)** | **B. Separate flat fact layer (CALDERA-style)** | **C. Property graph + derived fact/predicate views** |
|---|---|---|---|
| A "fact" is | a graph element (node/edge/property exists) | a typed key-value tuple (name,value,source) separate from the graph | a **named typed projection/predicate over the graph** exposed to DSL & planner |
| Preconditions | graph pattern query | flat requirement match | predicate query (compiles to graph pattern) |
| Effects | graph mutation | assert/retract facts | graph mutation surfaced as predicates |
| Templating a step (e.g. inject a role ARN) | query the graph (slightly more ceremony) | ergonomic (flat lookup) | ergonomic (named fact view) |
| Divergence risk | none (one store) | two models can drift | none (single store; views derived) |
| Planner strength | strong (graph) | weak (flat) | strong (graph) |

**Impact:** shapes the DSL grammar (Inc. 3), how the planner encodes actions, and how loot
(scalar secrets) is represented (a node property vs a fact tuple).

**Recommendation: C.** Single source of truth (the graph, per SPEC-D3 projection), with
**facts as named, typed views/predicates** over it for ergonomic templating and requirement
matching. It gives B's authoring ergonomics without B's dual-model drift, and it is the natural
fit for "one declaration feeds execution *and* the planner." Couples to SPEC-D4.

> **Resolved (2026-09-22): Option C** (property graph as source of truth + named typed
> fact/predicate views). The predicate/view vocabulary is specified with the DSL in Increment 3.

## 2.4 Epistemic model: proven / absent / unknown / inferred — **SPEC-D6 (RESOLVED: explicit status)**

Offensive enumeration is almost always **partial** (limited-privilege foothold, throttling,
denied calls). How we model *what we know vs. what we couldn't see* is not cosmetic — it drives
honest path-finding, confidence ranking (FR-E5), and blind-spot reporting (FR-C6).

| | **A. Explicit epistemic status on assertions** | **B. Presence-only + side "gaps" list** | **C. Presence + numeric confidence only** |
|---|---|---|---|
| Every attack-relevant assertion carries | `PROVEN` / `ABSENT` / `UNKNOWN` (denied/unseen) / `INFERRED` (derived) | just present edges; gaps in a separate coverage list | present edges + a 0–1 confidence |
| "unseen ≠ absent" preserved | **yes** | partially (gaps side-list) | no |
| Planner can reason under uncertainty | yes (optimistic/pessimistic modes) | no (blind to unknowns) | fuzzily |
| Explainability (FR-E3) | high (categorical) | medium | low (opaque score) |
| Confidence ranking (FR-E5) | derives from status (+ optional score) | hard | native but fuzzy |
| Cost | modeling + planner must handle unknowns | lowest | low |

**Impact:** central to honesty and to FR-E5/FR-C6; changes planner semantics (how it treats
`UNKNOWN` edges) and report content (declared blind spots).

**Recommendation: A** (categorical epistemic status, optionally carrying a confidence score for
`INFERRED`). Conflating "denied/unseen" with "absent" — as B and C effectively do — produces
both **false negatives** (missed real paths) and **dishonest reports**, which is unacceptable
for an authorized-testing tool. The cost is that the planner must define behavior over
`UNKNOWN` edges — specified in Increment 5.

> **Resolved (2026-09-22): Option A** (categorical PROVEN / ABSENT / UNKNOWN / INFERRED, with
> an optional confidence score on INFERRED). Planner behavior over UNKNOWN edges is specified
> in Increment 5.

## 2.5 Provider capability interfaces + descriptor format *(settled — freezes SPEC-D1=C)*

The Provider Port is a **small set of capability interfaces**; provider-specific knowledge
lives in **declarative descriptors** (a catalog the adapter ships) plus adapter code only where
response-mapping needs it.

**Capability interfaces (v1):**

| Capability | Responsibility | Serves |
|---|---|---|
| **IdentityResolver** | Resolve supplied credentials → current principal; enumerate credential-derived identities; support a low-priv foothold | FR-B3/B4, FR-A |
| **ResourceEnumerator** | Execute *enumeration descriptors*; return raw responses + provenance; central paging/throttle/region | FR-C, FR-B5 |
| **ActionExecutor** | Execute an *action descriptor* with host-brokered params + a per-step capability grant; return structured result | FR-G, NFR-SEC5 |
| **GraphMapper** | Map raw responses → canonical graph facts (target shape per SPEC-D4/D5/D6) | FR-D |
| **MetadataProvider** | Region/partition catalog, service/operation catalog, permission model | FR-B5 |
| **TelemetryCollector** *(v2 stub)* | Given a window + correlation id, fetch produced events | FR-I2 |

**Descriptor shape (declarative):**

- *Enumeration descriptor* — `{ service, operation, params(static|templated), pagination,
  required_permission, response_mapping → graph assertions (with epistemic status per D6) }`.
- *Action descriptor* — `{ service, operation, params(templated from facts/inputs),
  required_permission, impact_level, expected_telemetry_ref, result_mapping → facts,
  revert_ref }`.

Descriptors are the concrete realization of SPEC-D1=C: **enumeration and technique steps
compose descriptors**; the core never holds a cloud SDK, and adding a provider = ship a
descriptor catalog + implement the six capabilities. *The exact `response_mapping` /
`result_mapping` target vocabulary is finalized once SPEC-D4/D5/D6 land.*

## 2.6 Enumeration engine *(settled)*

Descriptor-driven orchestration over the ResourceEnumerator capability:

- **Concurrency** (NFR-PERF1) bounded by the safety rate/concurrency caps (NFR-SAF3).
- **Caching** (FR-C2, NFR-PERF2): results cached per `(operation, params)` within an
  engagement to avoid redundant calls and reduce target-side log footprint; explicit refresh.
- **Provenance** (FR-C4): every asserted fact records the descriptor/permission/time that
  produced it (an event field).
- **Incremental & resumable** (FR-C3): enumeration progress is events; interruption resumes
  from the ledger.
- **Scoped/targeted** (FR-C5): enumerate a subset by scope selector.
- **Partial-permission degradation** (FR-C6): a denied call emits an `AccessDenied` gap event
  → recorded in the **Coverage Map** and reflected as `UNKNOWN` assertions (SPEC-D6), never as
  a hard failure.

## 2.7 Engagement & event schema *(settled)*

**Event envelope:** `{ id, engagement_id, seq, timestamp, actor, prev_hash, hash, type,
payload }`. `hash = H(prev_hash ‖ canonical(payload))` gives an immutable, tamper-evident chain
(NFR-OBS3). `seq` + `engagement_id` enforce ordering and isolation (DSR-2).

**Event categories (v1):**
- *Engagement*: `EngagementOpened` (scope, authorization affirmation NFR-COMP1, provider,
  credential ref), `ScopeAmended`, `KillSwitchInvoked`, `EngagementClosed`.
- *Enumeration*: `EnumerationStarted`, `FactAsserted`, `AccessDenied`(gap), `EnumerationCompleted`.
- *Planning*: `PathComputed`.
- *Content*: `TechniqueValidated`.
- *Execution*: `DryRunPerformed`, `BlastRadiusEstimated`, `ConsentRecorded`, `StepDetonated`,
  `StepVerified`, `StepReverted`, `ExecutionFailed` (Inc. 4 details).
- *Loot*: `LootRecorded` (redacted reference; secret material handled per FR-J6/NFR-SEC1).
- *Reporting*: `ReportGenerated`, `ChainExported` (STIX/Attack Flow, FR-J7).

**Persistence Port** abstracts the storage substrate; substrate selection (embedded store) is
an Increment 7 implementation detail, not an architecture decision.

## 2.8 Open decisions (Increment 2)

| ID | Decision | Resolution (2026-09-22) | Coupled to |
|---|---|---|---|
| **SPEC-D4** | Attack-graph ontology normalization | **C — layered attack-core + provider inventory** | D5 |
| **SPEC-D5** | Fact model representation | **C — property graph + derived fact/predicate views** | D4 |
| **SPEC-D6** | Epistemic modeling | **A — explicit PROVEN / ABSENT / UNKNOWN / INFERRED** | D4 |

**All three Increment 2 decisions are RESOLVED.** The graph, fact, and descriptor-mapping
vocabulary is now fixed: a layered property graph (attack-core + inventory) that is a ledger
projection, with named fact/predicate views and categorical epistemic status on attack-relevant
assertions. Concrete attack-core node/edge types are enumerated with the planner in Increment 5;
the fact/predicate vocabulary with the DSL in Increment 3.

---

# Increment 3 — Technique Content Model & the Step-DSL

Makes FR-F concrete. Builds on: **OQ-7** (declarative fact-based step-DSL primary; Starlark
Tier 2 + WASM Tier 3 escape hatches), **SPEC-D2=A** (provider-scoped techniques), **SPEC-D1=C**
(steps compose descriptors; host-brokered), **SPEC-D5=C** (facts = named views over the graph),
**SPEC-D6=A** (epistemic status), **FR-F6** (one declaration feeds execution *and* the planner).

## 3.1 Technique anatomy *(settled structure)*

A technique is one versioned unit with four parts. The first two (**metadata**, **contract**)
are **always declarative Tier-0** — the planner and validator read them and never execute code.
The last two (**steps**, **revert**) are where the tiers apply.

```
technique
├─ metadata     (Tier 0, always declarative)
│    id · name · description · author · version · provider:aws
│    mitre:[T…]  (+ optional owasp/cis cross-map) · impact · expected_telemetry[]
├─ contract     (Tier 0, always declarative — the planner's view, §3.3)
│    preconditions[]  (fact/graph predicates + required epistemic status)
│    inputs[]         (typed parameters)
│    effects[]        (facts/edges asserted on success)
├─ steps        (Tier 1 DSL primary; per-step Tier-2/3 escape — §3.4, SPEC-D9)
│    <descriptor ref> + params → bind result → facts ; conditions ; loops
└─ revert       (SPEC-D8)
```

**Invariant (couples to SPEC-D9):** the **contract is always declarative**, even when a step
drops to Starlark/WASM — so the planner never has to understand an escape-hatch language
(FR-F6, FR-E).

## 3.2 Metadata (Tier 0) *(settled)*

Realizes FR-F2. Mandatory fields: `id`, `description`, **MITRE ATT&CK mapping** (aligned to the
Cloud matrices; OWASP-WSTG / CIS cross-map optional), `impact` level (FR-A3:
`read` / `mutating-reversible` / `mutating-irreversible` / `destructive`), `expected_telemetry`
(signatures → candidate detections, FR-I1), `version`, and the `revert` contract (§3.6).
Validation rejects a technique missing any mandatory field (FR-F2 AC).

## 3.3 Precondition / effect contract *(settled model; syntax pends SPEC-D7)*

The contract is the shared declaration that FR-F6 requires — read by **both** the execution
engine (to gate a step) and the planner (to treat the technique as a planning action):

- **Preconditions** — predicates over the fact/graph views (SPEC-D5), each with a **required
  epistemic status** (SPEC-D6): e.g. *requires a `PROVEN` CAN_ASSUME edge from the current
  principal to a role*. A precondition may accept `INFERRED`/`UNKNOWN` when the author marks it
  as opportunistic (drives optimistic planning, Inc. 5).
- **Inputs** — typed parameters (with defaults) the step body and templating consume.
- **Effects** — the facts/edges asserted **on verified success** (e.g. *asserts a new
  `HAS_CREDENTIAL` edge*). Effects are what let the planner chain techniques and what the graph
  records post-execution.

This precondition/effect shape is the PDDL-style action model (Analysis Appendix A) expressed
over our property graph. The concrete predicate/expression syntax is fixed by SPEC-D7.

## 3.4 The step model (Tier 1) *(settled structure; control-flow/expressions pend SPEC-D7)*

A step is the atomic unit of a technique body. Settled semantics:
- A Tier-1 step **references a provider descriptor** (SPEC-D1=C) and supplies params (static or
  templated from inputs/facts); the host brokers the call (NFR-SEC5) and **binds the structured
  result into named facts** (SPEC-D5) available to later steps — this is the inter-step
  data-flow FR-F6/Leonidas-gap requires.
- Steps support **conditions** (run-if predicate) and **bounded iteration** over enumerated
  results. The *expressiveness* of conditions/iteration and the templating language is exactly
  SPEC-D7; anything beyond it escapes to Tier 2 (SPEC-D9).
- Each mutating step contributes its **compensation** to the revert contract (SPEC-D8).

## 3.5 Authoring format — **SPEC-D7 (RESOLVED: YAML + CEL)**

How the technique (metadata + contract + steps) is physically written. The OQ-7 driver was
human+AI authorability; the pain point of every YAML DSL in the field is expressions,
control-flow, and inter-step data-flow (Leonidas resorted to stringly-typed Jinja).

| | **A. YAML container + safe expression language (CEL)** | **B. Purpose-built DSL grammar** | **C. Existing expressive config language (HCL / CUE)** |
|---|---|---|---|
| Container | YAML (ubiquitous, huge tooling, top AI familiarity) | our own textual grammar | HCL (Terraform) or CUE |
| Expressions / predicates | **CEL** — safe, typed, non-Turing-complete, embeddable (used in k8s admission, Cloud IAM conditions) | native to the grammar | native typed expressions |
| Authorability / AI-familiarity | highest | medium (AI must learn a small grammar) | medium-high |
| Tooling (LSP, highlight, schema) | reuse YAML + a CEL lib | build ourselves | reuse HCL/CUE tooling |
| Build/maintenance cost | low (compose two mature pieces) | **high** (parser + tooling) | medium |
| Expressiveness ceiling | medium → escape to Tier 2 | high | high |

**Impact:** sets the everyday authoring experience, how much tooling we build, and where the
Tier-1→Tier-2 line naturally falls (SPEC-D9).

**Recommendation: A — YAML + CEL.** It maximizes the authorability that drove OQ-7, inherits
mature tooling, and replaces stringly-typed templating with a **real, safe, typed expression
layer** for predicates/conditions/templating — while the Starlark escape hatch covers whatever
CEL can't express. Lowest build cost, highest reach.

> **Resolved (2026-09-22): Option A** (YAML container + CEL for predicates/conditions/templating).
> The concrete technique schema and the CEL function/variable surface are specified in Increment 4.

## 3.6 Revert-contract model — **SPEC-D8 (RESOLVED: saga / recorded compensations)**

The safety linchpin (NFR-SAF2, release-blocking; FR-G4 auto-revert). How a technique guarantees
its effects can be undone.

| | **A. Explicit revert block** | **B. Auto-derived inverse** | **C. Saga / recorded compensations** |
|---|---|---|---|
| Author writes | explicit compensating steps | nothing (engine inverts) | usually nothing; overrides where needed |
| Mechanism | run the block | descriptors declare inverses (CreateX↔DeleteX); engine inverts executed steps in reverse | each detonated step **records a concrete compensation** (from descriptor inverse by default, else author-supplied); revert replays compensations in reverse |
| Partial-failure safety | author must handle | engine reverses only completed steps | **only undoes what actually happened** (intrinsic) |
| Restart-safe revert (DSR-4) | if block is data | needs executed-step record | **yes** — compensations live in the event ledger |
| Messy/irreversible actions | author handles | breaks (no clean inverse) | explicit compensation or `mutating-irreversible` flag |
| Author burden / error risk | high | low but brittle | low and robust |

**Impact:** determines revert correctness (the #1 safety property), how revert survives
restarts, and author burden.

**Recommendation: C — saga / recorded compensations.** Each mutating step, when detonated,
writes a concrete compensating action into the ledger (derived from the descriptor's declared
inverse by default; overridable per step for messy cases). Revert = replay compensations in
reverse order. This is transactional, undoes only what happened, survives restarts (compensations
are events, DSR-4), and directly powers FR-G4 auto-revert-on-failure. Irreversible actions must
declare `mutating-irreversible` and provide a simulated variant (FR-G6).

> **Resolved (2026-09-22): Option C** (saga / recorded compensations, ledger-backed). The
> compensation event shape and the execution-engine revert algorithm are specified in Increment 4.

## 3.7 Tier boundary & integration — **SPEC-D9 (RESOLVED: per-step escape + declarative contract)**

How the Tier-2 (Starlark) / Tier-3 (WASM) escape hatches integrate with the Tier-1 DSL.

| | **A. Per-step escape** | **B. Whole-technique tiers** | **C. Per-step escape + contract-always-declarative** |
|---|---|---|---|
| Granularity | a DSL technique may have individual `script`/`wasm` steps | a technique is entirely DSL *or* entirely scripted | per-step escapes (A) **plus** an invariant: metadata+contract stay declarative Tier-0 |
| Planner visibility | may be lost if steps opaque | lost for scripted techniques | **always preserved** (planner reads the contract, never the steps) |
| Author flexibility | high | coarse (one computed value ⇒ fully scripted) | high |
| Paradigm mixing | yes (in one file) | no | yes, but bounded by the invariant |

**Boundary criterion (all options):** stay in **Tier-1 DSL** for host-brokered calls + fact
binding + CEL-expressible conditions/iteration; escape to **Tier-2 Starlark** for computed
values or branching CEL can't express; **Tier-3 WASM** for heavy computation or libraries
(crypto, binary/protocol work, payload generation).

**Impact:** decides whether the planner's contract can ever be undermined by an escape hatch,
and how fine-grained authors can be.

**Recommendation: C.** Per-step escapes give authors fine-grained power, while the
**contract-always-declarative invariant** guarantees the planner (FR-E/FR-F6) and the validator
never need to understand Starlark/WASM. This is the precise, safe resolution of the OQ-7
residual.

> **Resolved (2026-09-22): Option C** (per-step Starlark/WASM escapes; metadata + contract stay
> declarative Tier-0). This closes the OQ-7 residual entirely.

## 3.8 Validation, testing, versioning & catalog *(settled)*

- **Validation** (FR-F2): schema + contract validation at load; a technique failing validation
  is not loaded. Escape-hatch steps are validated for capability declarations (NFR-SEC5).
- **Testing** (FR-F4, NFR-MNT1, release-blocking): every technique ships an automated test that
  detonates and asserts successful **revert** against the mock provider (fast tier) and, for
  release, the live-AWS tier (OQ-8). No passing revert test ⇒ not shipped (NFR-SAF2).
- **Versioning** (NFR-MNT3): techniques are versioned; a declared compatibility range binds a
  technique to engine/content-model versions.
- **Catalog** (FR-F5): techniques are discoverable/queryable by MITRE tactic/technique,
  provider, impact, and required preconditions (the last enables the planner to propose relevant
  techniques for a discovered path).
- **Extensibility** (FR-F3, NFR-EXT3/EXT6): adding a technique is additive — a catalog entry,
  no core changes; techniques may load from outside the core distribution.

## 3.9 Open decisions (Increment 3)

| ID | Decision | Resolution (2026-09-22) | Coupled to |
|---|---|---|---|
| **SPEC-D7** | Authoring format | **A — YAML + CEL** | D5 |
| **SPEC-D8** | Revert-contract model | **C — saga / recorded compensations** | D3(state), G4 |
| **SPEC-D9** | Tier boundary & integration | **C — per-step escape + contract-always-declarative** | OQ-7 |

**All three Increment 3 decisions are RESOLVED**, and the **OQ-7 residual is now fully closed.**
Techniques are authored in YAML with CEL expressions; revert uses ledger-backed saga
compensations; escape hatches are per-step with an always-declarative contract. The concrete
technique schema, CEL surface, and revert algorithm are specified in Increment 4.

---

# Increment 4 — Safe Execution Engine

The safety-critical core (FR-G, FR-H). Builds on: **SPEC-D3=A** (event ledger), **SPEC-D8=C**
(saga compensations), **SPEC-D9=C** (per-step tiers), **SPEC-D6=A** (epistemic status), the
technique contract (Inc. 3). Design bias throughout: **least blast radius, fail safe,
reversible-by-default** (NFR-SAF1/SAF4).

## 4.1 Per-step execution lifecycle *(settled)*

Every step moves through an explicit, ledger-recorded state machine. Nothing mutates the target
before consent, and nothing is recorded as an effect before verification.

```
 PLANNED
   │  check preconditions (contract vs current graph + epistemic status, SPEC-D6)
   ▼
 ANALYZED            ── dry-run + blast-radius (SPEC-D11); no mutation
   │  consent gate if impact ≥ threshold (§4.2)
   ▼
 CONSENTED
   │  detonate via ActionExecutor (host-brokered, SPEC-D1)
   ▼
 DETONATED           ── compensation recorded to ledger IMMEDIATELY (SPEC-D8)
   │  verify (CEL/descriptor check, §4.5)
   ▼
 VERIFIED            ── effects asserted into the graph (facts/edges)
   ┊
 (any failure after CONSENTED) → FAILED → REVERTING → REVERTED   (per SPEC-D12 policy)
```

Each transition emits a ledger event (Inc. 2 §2.7): `DryRunPerformed`, `BlastRadiusEstimated`,
`ConsentRecorded`, `StepDetonated`, `StepVerified`, `StepReverted`, `ExecutionFailed`.
Execution is **idempotent & resumable** (FR-G5): state lives in the ledger, so an interrupted
step resumes from its last recorded transition (NFR-REL2).

## 4.2 Consent, authorization gating & kill-switch *(settled)*

- **Impact-gated consent** (FR-A3): no step above `read` executes without a recorded consent
  decision at or above its impact level. Two modes serve the two operating contexts (FR-K):
  - *Interactive* — per-step confirmation at/above the operator's threshold, showing the
    blast-radius preview (§4.4).
  - *Non-interactive / CI* — **per-chain pre-authorization** with a declared **impact ceiling**;
    any step exceeding the ceiling halts rather than proceeding unattended.
- **Scope enforcement** (FR-A1, NFR-COMP2): every ActionExecutor call is checked against the
  engagement's authorized scope; out-of-scope targets are refused and logged.
- **Kill-switch** (FR-A4): a global safe-abort halts new steps and triggers revert of completed
  reversible steps (§4.6). Always available in every mode.

## 4.3 Warm-up / setup semantics — **SPEC-D10 (RESOLVED: no warm-up; setup = compensated steps)**

Stratus warms up *synthetic* prerequisites because it detonates against your own account to
validate detection. Akumo's primary job is different — demonstrate *real* exploitability against
the target's *actual* resources — so a Stratus-style warm-up doesn't fit the main use case. The
question: does the lifecycle keep a distinct warm-up/cleanup phase, or not?

| | **A. No warm-up; setup = ordinary compensated steps** | **B. Keep an explicit warm-up phase (Stratus-style)** | **C. Hybrid: no warm-up in v1 engagement mode; warm-up only in an optional lab/validation mode** |
|---|---|---|---|
| Prereq creation | just normal steps, each with a saga compensation (SPEC-D8) | separate stage with its own tracked artifacts + cleanup | normal steps for real targets; synthetic warm-up available for detection-validation |
| Teardown mechanism | **one** (saga compensations) | **two** (revert + cleanup) | one for engagements; two only in lab mode |
| Fit for real-target v1 | strong | weak (built for synthetic targets) | strong, keeps a v2 door open |
| Complexity | lowest | higher (two teardown paths) | medium |

**Impact:** whether the engine has one teardown mechanism or two; how honestly the lifecycle
matches the real-target use case; whether detection-validation (a v2/optional mode) is
pre-accommodated.

**Recommendation: A** for v1. The saga model already gives clean setup+teardown via
compensations, so a separate warm-up/cleanup phase is redundant and adds a second teardown path
to get right. A dedicated synthetic warm-up (option B behavior) can return later as an explicit
**lab/detection-validation mode** without disturbing the engagement lifecycle — but it is not
v1. (This keeps the door to C open while shipping A.)

> **Resolved (2026-09-22): Option A** (no warm-up phase in v1; setup is ordinary compensated
> steps). A synthetic warm-up returns only with an optional lab/detection-validation mode (v2).

## 4.4 Pre-execution safety analysis: dry-run + blast radius — **SPEC-D11 (RESOLVED: static + opt-in provider-assisted)**

Before any mutation the engine must preview intended actions and estimate blast radius (FR-G2/G3).
The fork is **fidelity vs. footprint**.

| | **A. Offline / static only** | **B. Provider-assisted** | **C. Static default + opt-in provider-assisted validation** |
|---|---|---|---|
| Source | technique descriptors + contract effects + the graph | + provider dry-run APIs (e.g. AWS `DryRun`), IAM policy simulation, read-only probes | static always; provider-assisted on opt-in |
| Fidelity | bounded by declarations (won't catch a real permission/resource-policy denial) | validates permissions & reachability | static baseline, high-fidelity when asked |
| Footprint | **zero** provider calls | small (read-only/no-op) calls | zero by default; small when enabled |
| Bonus | — | can upgrade `INFERRED` edges toward `PROVEN` (FR-E5) | same, on demand |
| Coverage | full (all techniques) | partial (not all APIs support dry-run) | full baseline + partial deepening |

**Impact:** confidence before detonation, the assessment's log/footprint (NFR-PERF4), and how
`INFERRED` edges get validated (FR-E5).

**Recommendation: C.** Offline static preview by default (cheap, safe, full coverage, zero
footprint), with an **opt-in provider-assisted validation** (AWS `DryRun` / IAM policy
simulator / read-only probes) for higher confidence before a real detonation — which also
doubles as the mechanism to promote `INFERRED` edges toward `PROVEN`.

> **Resolved (2026-09-22): Option C** (offline static preview by default; opt-in
> provider-assisted validation via AWS `DryRun` / IAM policy simulator / read-only probes).

## 4.5 Verification model *(settled)*

The lifecycle's `verify` phase confirms a step actually achieved its effect **before** effects
are asserted into the graph and the chain proceeds. Verification is a **CEL predicate** (SPEC-D7)
or a descriptor-provided success check evaluated over the step result and current facts. Ordering
is deliberate: *detonate → record compensation → verify → assert effects*. Recording the
compensation **before** verification guarantees that a step which mutated but failed verification
is still revertible (fail-safe, NFR-SAF4).

## 4.6 Revert engine *(settled — realizes SPEC-D8)*

- **Mechanism:** revert replays the recorded compensations (SPEC-D8) of detonated steps **in
  reverse order**. Because compensations are ledger events, revert is **restart-safe** (DSR-4):
  a crashed engagement can be reopened and reverted.
- **Triggers:** step failure (per §4.7 policy), operator on-demand revert, or kill-switch (§4.2).
- **Undo-only-what-happened:** only steps that reached `DETONATED` have compensations, so revert
  never attempts to undo work that did not occur (FR-G4).
- **Irreversible steps:** a `mutating-irreversible` step cannot be compensated; the engine
  refuses it unless explicitly consented, and prefers the technique's simulated variant (FR-G6).
- **Revert failures:** a compensation that itself fails is recorded and surfaced prominently; the
  engine reports exactly which effects it could not undo (NFR-SAF4, FR-G4 AC).

## 4.7 Chain transaction & auto-revert semantics — **SPEC-D12 (RESOLVED: policy-driven, safe default)**

A chain (FR-H) executes a computed path step-by-step. The fork is **what happens on failure**.

| | **A. Atomic: auto-revert whole chain on any failure** | **B. Halt-and-hold: revert nothing automatically** | **C. Policy-driven, safe default** |
|---|---|---|---|
| On step failure | undo all completed steps automatically | halt, leave completed steps in place, present options | default = halt + auto-revert *completed* steps; operator may configure "halt-and-hold" |
| State cleanliness | always clean | operator-dependent | clean by default |
| CI / unattended fit | good | **poor** (needs a human) | good (safe default) |
| Interactive control | low (can't inspect before undo) | high | high (hold mode) |
| Resume after fix | no (already reverted) | yes | yes (in hold mode) |

**Impact:** whether chains are safe unattended (FR-K2 CI mode), how much interactive control
operators get, and how FR-H2 (halt-and-report + offer revert) / FR-G4 (auto-revert) are realized.

**Recommendation: C.** Default to **halt + auto-revert of completed steps** on failure — safe and
CI-friendly (leaves the target clean without a human) — while letting interactive operators opt
into **halt-and-hold** to inspect, fix, and resume. Kill-switch and on-demand revert remain
available throughout. This satisfies FR-H2 and FR-G4 without sacrificing interactive control.

> **Resolved (2026-09-22): Option C** (default halt + auto-revert of completed steps; opt-in
> interactive halt-and-hold to inspect/fix/resume; kill-switch + on-demand revert always available).

## 4.8 Rate limiting, concurrency & footprint *(settled)*

- **Global rate & concurrency caps** (NFR-SAF3) protect the target from accidental DoS; they
  bound the enumeration concurrency (NFR-PERF1) and execution parallelism alike.
- **Footprint awareness** (NFR-PERF4): the engine can report the expected API-call / log
  footprint of an operation (drawn from descriptor metadata) so operators manage stealth and cost.
- **Throttle handling** (NFR-REL3): provider throttling triggers safe backoff/retry within the
  caps, never a breach of them.

## 4.9 Open decisions (Increment 4)

| ID | Decision | Resolution (2026-09-22) | Coupled to |
|---|---|---|---|
| **SPEC-D10** | Warm-up / setup semantics | **A — no warm-up; setup = compensated steps** (lab-mode warm-up deferred to v2) | D8 |
| **SPEC-D11** | Pre-execution safety-analysis fidelity | **C — static default + opt-in provider-assisted** | E5, NFR-PERF4 |
| **SPEC-D12** | Chain transaction & auto-revert semantics | **C — policy-driven, safe default (halt + auto-revert completed)** | G4, H2, K2 |

**All three Increment 4 decisions are RESOLVED.** The execution engine has one teardown
mechanism (saga compensations), static-by-default pre-execution analysis with opt-in
provider-assisted validation, and a CI-safe halt-and-auto-revert chain policy with an
interactive hold option.

---

# Increment 5 — Path-Finding & Planning

The attack-graph-native differentiator (FR-E). Builds on: **SPEC-D4=C** (layered graph),
**SPEC-D5=C** (facts as views), **SPEC-D6=A** (epistemic status), the **precondition/effect
contract** (FR-F6, PDDL-shaped), **OQ-10** (ranking), **OQ-3** (objective types). This increment
also delivers the concrete **attack-core ontology** promised in Increment 2.

## 5.1 Objective model *(settled — realizes OQ-3)*

An objective is a **goal predicate** over the graph/fact views (SPEC-D5). v1 ships two classes
(OQ-3):
- **Reach administrative privilege** — a principal reachable from the foothold satisfies an
  "administrative" predicate (holds a permission set meeting an admin definition).
- **Reach a specified resource** — a principal reachable from the foothold obtains a named
  access (read/write/etc.) to a specified resource/data store.

Objectives are predicates, so new objective classes are additive (no planner change). The
planner answers: *from the current principal(s), does a path exist that makes the goal predicate
true, and if so which paths* (FR-E1/E2).

## 5.2 Planning approach — **SPEC-D13 (RESOLVED: hybrid reachability + technique-as-action)**

How the planner computes paths. We deliberately built *both* a graph (D4) and a
precondition/effect model (FR-F6); this decides how they combine.

| | **A. Graph reachability (BloodHound-style)** | **B. Symbolic planning (PDDL-style)** | **C. Hybrid: reachability + technique-as-action planning** |
|---|---|---|---|
| A path is | a chain of **existing** attack-core edges | a state-space search over facts, applying techniques whose preconditions hold | fast graph frontier over static edges **+** symbolic expansion for state-creating techniques |
| Handles state-*creating* steps (e.g. create key → use it) | no (unless edge pre-materialized) | **yes** (effects add facts) | **yes** |
| Speed / scale | fast, mature | heavier (search can explode) | fast base + planning only where needed |
| Leverages our models | graph only | fact model only | **both** |
| Fit for multi-step chaining (Akumo's point) | weak | strong | strong |

**Impact:** whether the planner can represent the multi-step, state-creating techniques that are
Akumo's entire reason to exist (chaining), and how it scales.

**Recommendation: C (hybrid).** The layered graph gives fast base reachability over existing
identity/trust/permission edges; techniques are treated as **planning actions** whose effects
generate new edges/facts, extending the frontier. Pure A cannot express state-creating chains
(the differentiator); pure B is heavier and ignores the fast graph frontier. C is why both
models exist, and mirrors research attack-graph planners (logical attack graphs + PDDL).

> **Resolved (2026-09-22): Option C** (hybrid: graph reachability for static edges + symbolic
> technique-as-action expansion for state-creating steps). Concrete search mechanics per D15.

## 5.3 Handling UNKNOWN edges — **SPEC-D14 (RESOLVED: dual-mode, honest default)**

Offensive enumeration is partial (SPEC-D6), so many edges are `UNKNOWN`/`INFERRED`. How the
planner treats them decides false-negative vs false-positive balance.

| | **A. Optimistic** | **B. Pessimistic** | **C. Dual-mode, honest default** |
|---|---|---|---|
| Traverses | PROVEN + UNKNOWN/INFERRED | PROVEN only | PROVEN prominently **+** candidate (unproven) paths, labeled |
| False negatives (missed real paths) | low | **high** (blind to gaps) | low |
| False positives (paths that won't work) | higher | none | flagged as "candidate — needs validation" |
| Ties to | — | — | D6 status, FR-E5 (proven vs inferred), D11 (validate → promote) |

**Impact:** what paths surface under partial permissions (the normal case) and how honest the
output is.

**Recommendation: C (dual-mode).** Default surfaces **proven paths prominently and candidate
paths labeled as needing validation** (never hidden — false negatives are unacceptable for an
offensive tool, never presented as certain — dishonesty is unacceptable either). Provider-assisted
dry-run (D11) promotes candidates by upgrading `INFERRED`/`UNKNOWN` toward `PROVEN`. Operators
may force strict-PROVEN (B) or full-optimistic (A) modes.

> **Resolved (2026-09-22): Option C** (dual-mode: PROVEN paths prominent + labeled candidate
> paths; strict-PROVEN and full-optimistic available as modes).

## 5.4 Search strategy & termination — **SPEC-D15 (RESOLVED: tiered bounded + opt-in exhaustive)**

| | **A. Exhaustive** | **B. Bounded best-first (heuristic)** | **C. Tiered: bounded default + opt-in exhaustive** |
|---|---|---|---|
| Returns | all paths | top-K within depth/time/expansion budget, guided by the ranking score | bounded best-first default; exhaustive on request |
| Scale (NFR-PERF3) | explodes on large graphs | scales | scales, with a completeness escape |
| Completeness | complete | may miss some | complete when needed |
| Uses ranking as | post-sort only | **search heuristic + sort** | both |

**Impact:** usability on large graphs (NFR-PERF3) and whether the ranking objective doubles as
the search heuristic.

**Recommendation: C.** Bounded **best-first search guided by the ranking score** (§5.5) returning
top-K within a budget by default — the OQ-10 ranking criteria *are* the search heuristic — with
an opt-in deeper/exhaustive mode for smaller graphs or when completeness matters.

> **Resolved (2026-09-22): Option C** (bounded best-first default, ranking score as heuristic;
> opt-in exhaustive mode).

## 5.5 Ranking integration *(settled — realizes OQ-10)*

The OQ-10 composite score (**confidence → length → detectability**) is **both the sort key and
the best-first heuristic** (§5.4). Named presets (`shortest` / `stealthiest` / `safest` /
`most-reliable`) swap the weighting; `confidence` derives from the epistemic status of the path's
edges (SPEC-D6), `detectability` from the techniques' expected-telemetry signatures (FR-I1).
**Reversibility / blast-radius is always attached as a safety annotation**, shown even when not
the sort key (OQ-10).

## 5.6 Explainability *(settled — FR-E3)*

Every path is emitted as an ordered, human-readable chain; each step states: the **principal**,
the **action/technique**, the **enabling fact/edge and its epistemic status** (PROVEN / INFERRED
/ UNKNOWN), and the **produced effect**. A reader can understand *why each hop is possible*
without external lookup (FR-E3 AC). Paths export to STIX / Attack Flow (FR-J7).

## 5.7 The attack-core ontology *(settled — realizes SPEC-D4=C)*

The compact, provider-neutral node/edge vocabulary the planner traverses (provider inventory
hangs off nodes as attributes; every edge carries epistemic status + provenance + the enabling
permission reference):

- **Nodes:** `Principal` (user / role / service-identity / group), `Credential`, `Resource`
  (typed), `PermissionSet`, `ExternalIdentity` (federation / IdP).
- **Edges:** `MEMBER_OF`, `HAS_PERMISSION`, `CAN_ASSUME`, `TRUSTS`, `HAS_CREDENTIAL`,
  `FEDERATED_AS`, and the **derived** `CAN_ACCESS` and `CAN_ESCALATE_TO` (the privesc/lateral
  edges the planner both consumes and, via technique effects, produces).

Provider adapters map their inventory into these types (GraphMapper, §2.5); the set is small and
attack-relevant by design (SPEC-D4=C) so it stays stable across future providers.

## 5.8 Open decisions (Increment 5)

| ID | Decision | Resolution (2026-09-22) | Coupled to |
|---|---|---|---|
| **SPEC-D13** | Planning approach | **C — hybrid reachability + technique-as-action** | D4, F6 |
| **SPEC-D14** | UNKNOWN-edge handling | **C — dual-mode, honest default** | D6, E5, D11 |
| **SPEC-D15** | Search strategy & termination | **C — bounded best-first default + opt-in exhaustive** | OQ-10, NFR-PERF3 |

**All three Increment 5 decisions are RESOLVED.** The planner is a hybrid engine (graph
reachability + technique-as-action expansion), bounded best-first by default with the ranking
score as heuristic, surfacing proven paths prominently and candidate paths honestly labeled.

---

# Increment 6 — Telemetry/Detection, Reporting/Audit & Interfaces

Realizes FR-I, FR-J, FR-K. Most of this increment is **settled by earlier decisions** (the event
ledger is the audit trail per SPEC-D3/D14; hexagonal driving adapters give CLI/library/CI per
ADR-0001; STIX/Attack Flow export was aligned in FR-J7/Analysis Appendix A). Two genuine forks
remain: the detection-signature format and the CLI interaction model.

## 6.1 Expected-telemetry & detection output — **SPEC-D16 (RESOLVED: Sigma-aligned)**

v1 emits expected telemetry signatures + MITRE mapping only (ADR-0008). The fork is **how the
signature is represented** — which decides interoperability and whether we can auto-generate
candidate detections (FR-I3).

| | **A. Sigma-aligned** | **B. Akumo-native signature format** | **C. MITRE-mapping-only** |
|---|---|---|---|
| A signature is | expressed in / compilable to **Sigma** (the SIEM-agnostic detection standard; has AWS CloudTrail taxonomy) | a custom representation of expected API calls / log fields | just ATT&CK technique IDs, no field-level detail |
| Auto-generate candidate detections (FR-I3) | **yes** (Leonidas precedent) | yes, but we build converters | no |
| SIEM interop (Elastic/Sentinel/Splunk) | **native** (via sigma converters) | none until we build it | none |
| Expressiveness for cloud specifics | good (Sigma cloud taxonomy) | highest | lowest |
| Build cost | low (reuse Sigma + converters) | high | lowest |
| Blue-team value | high | high | low |

**Impact:** purple-team value (FR-I1/I3), SIEM interoperability, and build cost.

**Recommendation: A (Sigma-aligned).** Expected-telemetry signatures are expressed in a
Sigma-compatible form so Akumo can auto-generate candidate detections and interoperate with the
detection ecosystem out of the box (Leonidas proved this for cloud). A small native superset is
added only if a cloud field genuinely falls outside Sigma's taxonomy.

> **Resolved (2026-09-22): Option A** (Sigma-aligned signatures; auto-generate candidate
> detections; native superset only where Sigma's taxonomy falls short). See ADR-0028.

## 6.2 Reporting & evidence *(settled)*

- **Source:** reports are rendered from **ledger projections** (SPEC-D14) — never a separate
  narrative that could drift from what happened.
- **Content** (FR-J3): discovered paths, executed chains, obtained access, per-action telemetry
  signatures, and MITRE coverage.
- **Formats** (FR-J4): **human-readable** (Markdown / HTML) and **machine-readable** (JSON),
  plus **STIX 2.1 / MITRE Attack Flow** export of chains (FR-J7).
- **Views** (FR-J5): an **executive summary** and a **technical detail** view.
- **Redaction** (FR-J6, NFR-SEC): secrets/loot redacted by default, explicit opt-in to include.

## 6.3 Audit & query *(settled)*

The hash-chained event ledger (SPEC-D14) **is** the audit trail (FR-J1): every action is
traceable end-to-end (NFR-OBS1), an engagement is reconstructable for review (NFR-OBS2), and the
chain is tamper-evident (NFR-OBS3). Reproducibility follows ADR-0010 (audit-grade replay + path
re-verification). The ledger is queryable for review and reporting.

## 6.4 Interfaces & operating modes — CLI model **SPEC-D17 (RESOLVED: command-per-invocation + optional shell)**

**Settled:** the domain core is a **library** (FR-K3); the **CLI** and **CI/non-interactive**
modes are driving adapters over it (ADR-0001); machine-facing modes emit **versioned, stable,
structured output** (FR-K4, NFR-USE) so pipelines can depend on it.

**The fork — the CLI interaction model:**

| | **A. Command-per-invocation** | **B. Interactive REPL/shell** | **C. Both (command-per-invocation canonical + optional shell)** |
|---|---|---|---|
| Shape | `akumo enumerate` / `paths` / `run …`; state in the ledger | persistent session, commands within (Pacu/Metasploit-style) | command-per-invocation is canonical; an optional interactive shell layers on top |
| CI / scriptability | **native** | awkward | native |
| Exploratory UX | good | **best** | best |
| Switch engagements (FR-A6) | trivial (a flag; state is in the ledger) | needs in-shell handling | trivial |
| Avoids Pacu's "restart to switch sessions" flaw | yes (stateless invocations over persistent ledger) | risk of Pacu-style statefulness | yes |
| Build cost | low | medium | medium |

**Impact:** operator UX, CI-friendliness, and whether we inherit Pacu's session-statefulness
pitfalls (FR-A6).

**Recommendation: C.** Command-per-invocation is the **canonical** interface — stateless
invocations over the persistent engagement ledger, which is CI-friendly (FR-K2) and makes
switching engagements a flag (cleanly solving FR-A6, a Pacu pain point). An **optional
interactive shell** layers on top for exploratory work (the Pacu/Metasploit ergonomics) without
holding mutable session state of its own.

> **Resolved (2026-09-22): Option C** (command-per-invocation canonical over the ledger; optional
> stateless interactive shell on top). See ADR-0029.

## 6.5 Open decisions (Increment 6)

| ID | Decision | Resolution (2026-09-22) | Coupled to |
|---|---|---|---|
| **SPEC-D16** | Detection-signature format | **A — Sigma-aligned** (ADR-0028) | FR-I1/I3 |
| **SPEC-D17** | CLI interaction model | **C — command-per-invocation canonical + optional shell** (ADR-0029) | FR-K, FR-A6 |

**Both Increment 6 decisions are RESOLVED.** Expected telemetry is Sigma-aligned (auto-generates
candidate detections and interoperates with SIEMs); the CLI is command-per-invocation over the
ledger with an optional interactive shell.

---

# Increment 7 — Testing, Mock Fidelity & Distribution

The final planned increment (NFR-MNT, NFR-PORT, OQ-8). Much is settled: hybrid CI tiering
(OQ-8 / ADR-0009), single static binary (ADR-0003), the mock provider as a first-class shipped
adapter (ADR-0001), and per-technique revert tests (FR-F4). Two genuine forks: mock-provider
fidelity and the content distribution/update model.

## 7.1 Test architecture & tiers *(settled — OQ-8 / ADR-0009)*

```
        ▲  release-gated: LIVE-AWS integration (real detonate+revert; fidelity backstop)
       ╱ ╲     + orphaned-resource LEAK DETECTOR after every run
      ╱   ╲ per-PR: MOCK-provider tests (full loop; enumeration→plan→execute→revert logic)
     ╱─────╲ unit tests (core services, CEL eval, saga/ledger, planner) against the mock
```

- **Fast tier (every PR):** the entire loop against the **mock provider** — deterministic,
  free, contributor-runnable. Covers technique *logic* and revert *logic*.
- **Fidelity tier (release-gated):** real **detonate + revert** against a live ephemeral AWS
  account; the source of truth for AWS semantics; **leak detector** cleans stragglers after.
- Division of labor: **mock = logic fidelity; live-AWS = semantic fidelity.**

## 7.2 Mock-provider fidelity — **SPEC-D18 (RESOLVED: behavioral/scriptable stub)**

The mock serves two roles: the extensibility gate's "second provider" (NFR-EXT7) and the fast
test tier. The fork is **how faithfully it emulates AWS**.

| | **A. Behavioral / scriptable stub** | **B. High-fidelity simulator** | **C. Stub + pluggable authz-eval module** |
|---|---|---|---|
| Models | graph state (principals/permissions/resources/trust) + scripted responses | + real IAM policy evaluation, eventual consistency, service quirks | stub + an optional module for IAM authz decisions only |
| Catches | logic bugs (enumeration/plan/execute/revert) | + real AWS authz/consistency nuances | logic + the authz nuances that matter most |
| Build/maintenance | low | **very high** (LocalStack-class money pit) | medium |
| False-confidence risk | low (clearly not AWS) | real (diverges silently) | low-medium |
| Backstopped by | live-AWS tier | — | live-AWS tier |

**Impact:** how much the fast tier catches before the (slower, costlier) live-AWS tier, and how
much simulator maintenance we sign up for.

**Recommendation: A (behavioral/scriptable stub).** Deliberately *not* an AWS simulator — the
live-AWS tier (ADR-0009) is the semantic-fidelity backstop, and chasing simulator fidelity is a
well-known maintenance sink that risks silent divergence. The stub fully covers logic and the
extensibility gate. A targeted authz-eval module (C) can be added later only if the tier gap
proves costly.

> **Resolved (2026-09-22): Option A** (behavioral/scriptable stub; live-AWS tier is the
> semantic-fidelity backstop; authz-eval module deferred). See ADR-0030.

## 7.3 Safety & extensibility test gates *(settled)*

Release-blocking automated gates:
- **Revert correctness** (NFR-SAF2, FR-F4): every mutating technique has a test that detonates
  and asserts successful revert — mock tier always, live-AWS tier for release. No passing revert
  test ⇒ not shipped.
- **Scope & kill-switch** (FR-A1/A4, NFR-COMP2): tests that out-of-scope actions are refused and
  that the kill-switch halts + reverts.
- **Rate/concurrency caps** (NFR-SAF3): tests that caps hold under load.
- **Dependency-direction lint** (NFR-EXT1): core MUST NOT import any adapter/cloud SDK — CI-fails
  otherwise.
- **Extensibility gate** (NFR-EXT7): the full loop runs against the mock as a stand-in "second
  provider" with zero core changes — the proof that adding a real provider is additive.
- **Content validation** (FR-F2): schema/contract validation + escape-hatch capability checks.

## 7.4 Content distribution & update model — **SPEC-D19 (RESOLVED: signed versioned bundle)**

How the technique catalog ships and grows without a full engine release (NFR-EXT6, FR-F3).

| | **A. Bundled in the binary** | **B. Separate versioned, signed content bundle** | **C. Pull-able registry** |
|---|---|---|---|
| Ships as | techniques embedded in the release | a default catalog shipped alongside, loaded from a content path; updatable independently | engine fetches/updates from a registry |
| Update without engine release (NFR-EXT6) | no | **yes** | yes |
| Community contribution (FR-F3) | hard | good | good |
| Offline / sensitive-env friendly | yes | **yes** (local bundle) | no (network dependency) |
| Supply-chain surface | minimal | bounded (signed bundle) | largest (needs strong signing/trust) |
| Build cost | lowest | medium (versioning + signing) | highest |

**Impact:** how the catalog evolves, community extensibility, offline operation, and
supply-chain surface.

**Recommendation: B (separate versioned, signed content bundle).** The engine ships with a
default catalog but loads techniques from a content bundle that is **versioned (NFR-MNT3) and
signed (NFR-SEC5)** and updatable independently of the engine — satisfying NFR-EXT6 and community
extensibility while staying **offline-capable** (a local bundle, right for sensitive
environments). A pull-able registry (C) is a v2 convenience layered on B; pure bundling (A) is
too rigid for a content-driven tool.

> **Resolved (2026-09-22): Option B** (separate versioned, signed content bundle loaded by the
> engine; default catalog shipped alongside; registry deferred to v2). See ADR-0031.

## 7.5 Packaging & distribution *(settled — ADR-0003)*

- **Single static binary** (NFR-PORT2) on Linux/macOS (Windows where feasible), container-native
  (NFR-PORT3), distributed via binary release + Homebrew + Docker (+ `cargo install`).
- **Reproducible builds**; releases and the content bundle are **signed** (integrity for a
  security tool).
- **Versioning/compat** (NFR-MNT3): engine and content model are versioned with a declared
  compatibility range; a technique declares the engine/content-model versions it supports.

## 7.6 Open decisions (Increment 7)

| ID | Decision | Resolution (2026-09-22) | Coupled to |
|---|---|---|---|
| **SPEC-D18** | Mock-provider fidelity | **A — behavioral/scriptable stub** (ADR-0030) | OQ-8, ADR-0009 |
| **SPEC-D19** | Content distribution & update model | **B — separate versioned, signed bundle** (ADR-0031) | NFR-EXT6, FR-F3 |

**Both Increment 7 decisions are RESOLVED.** The mock is a behavioral stub (live-AWS is the
semantic backstop); techniques ship as a versioned, signed, independently-updatable bundle.

---

## Specification status

**All 7 planned increments are complete; every SPEC-D decision (D1–D19) is resolved** and
recorded in `ADR/`. The specification covers the full unified loop — authorize → enumerate →
graph → plan → safely execute → report — plus the provider seam, content model, safety/revert
engine, and testing/distribution. The natural next artifact is an **implementation/build plan**
(crate/module layout, milestones, and the first end-to-end vertical slice against the mock
provider), which moves from *what/how* into *build order*.
