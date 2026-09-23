# 06 — Attack Graph & Facts

> Tasks G4.1–G4.4 · ADR-0015 (layered ontology) · ADR-0016 (facts as views) · ADR-0017 (epistemic
> status). Ontology types in `akumo-domain::graph`; the graph engine in `akumo-core::graph`.

## Layered ontology (G4.1 — `akumo-domain::graph`)

A **compact, provider-neutral attack core** the planner reasons over, with a **provider inventory
layer** hanging off nodes as `attributes` (SPEC-D4 = layered). No provider vocabulary leaks into the
core (EXR-6).

- **Nodes** (`NodeKind`): `Principal`, `Credential`, `Resource`, `PermissionSet`, `ExternalIdentity`.
- **Edges** (`EdgeKind`): `MEMBER_OF`, `HAS_PERMISSION`, `CAN_ASSUME`, `TRUSTS`, `HAS_CREDENTIAL`,
  `FEDERATED_AS`, and the **derived** `CAN_ACCESS`, `CAN_ESCALATE_TO` (the privesc/lateral edges the
  planner consumes and, via technique effects, produces).
- Every `GraphNode`/`GraphEdge` carries an `Epistemic` status and `Provenance`; every edge records
  its `enabling_permission` (the reason it exists — explainability, FR-D3).
- An `Assertion` is `Node(GraphNode)` or `Edge(GraphEdge)` — what a `GraphMapper` produces and a
  `FactAsserted` event carries. A `CoverageGap` is what an `AccessDenied` event carries.

## Graph as a ledger projection (G4.2 — `akumo-core::graph`)

`GraphProjection` folds the event stream into an `AttackGraph` (nodes keyed by id, edges, coverage
map). The graph is **never a separate store** — it is always reconstructable from the ledger
(SPEC-D3), so it can never drift.

- `FactAsserted` → `apply_assertion` (upsert node/edge).
- `AccessDenied` → a coverage gap.
- **Merge rule:** repeated assertions merge attributes and keep the *more-certain* epistemic status
  (`certainty_rank`: `Proven`/`Absent` > `Inferred` > `Unknown`). A later blind spot never downgrades
  a proven fact.

## Fact/predicate views (G4.3)

Named predicates over the graph, the ergonomic surface the DSL (G8) and planner (G12) query. Each
compiles to an edge lookup and returns the edge's `Epistemic` status (or `None` if absent):

`can_assume(principal, role)` · `has_credential(principal, credential)` ·
`has_permission(principal, permission_set)` · `can_access(principal, resource)` — all built on the
`edge_status(from, to, kind)` primitive. One store, no dual-model drift (SPEC-D5 = C).

## Epistemic status & coverage (G4.4)

Categorical `PROVEN / ABSENT / UNKNOWN / INFERRED` on every attack-relevant assertion (ADR-0017).
The invariant that pays off later: **"unknown ≠ absent."** A denied enumeration call records a
`CoverageGap` (surfaced by `coverage_gaps()`, FR-C6) and, where relevant, `UNKNOWN` assertions —
never a missing fact silently read as "does not exist." The planner (G12) defines explicit behavior
over `UNKNOWN` edges; honest reporting depends on this distinction.

## Tests

`akumo-domain::graph` (edge-kind wire format, assertion round-trip); `akumo-core::graph` (fold nodes
+ edges + fact views, merge-never-downgrades-proven, access-denied-becomes-coverage-gap).
