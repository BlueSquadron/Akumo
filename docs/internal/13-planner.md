# 13 — Planner

> Task G12 · FR-E · ADR-0007/0011/0024/0025/0026/0027 · spec §5. In `akumo-core::planner`.
> The attack-graph-native differentiator. v1 is **deterministic** (ADR-0007; AI planner is v2).

The planner computes ranked, explainable attack paths from a foothold toward an objective, over the
attack graph (G4). It is pure (no ledger); a driving layer records a `PathComputed` event.

## Objectives (G12.1 / ADR-0027)

Goal predicates (OQ-3): `ReachAdmin` (a controlled principal satisfies the admin predicate) and
`ReachResource { resource }` (a controlled principal can access the resource). New objective classes
are additive — no planner change.

The **admin predicate**: a principal whose node carries `attributes.admin == true`, or which holds a
`HAS_PERMISSION` edge to a `PermissionSet` node flagged `admin` (the edge subject to the mode filter).

## Hybrid search (G12.2 / ADR-0024)

Two kinds of transition extend the frontier of *controlled* principals:

- **Static edges** — control-transfer edges (`CAN_ASSUME`, `CAN_ESCALATE_TO`, `MEMBER_OF`,
  `FEDERATED_AS`): graph reachability.
- **Technique-as-action** — a `PlanningAction` whose `requires_control` principals are all
  controlled grants control of its `grants_control` principals. This expresses **state-creating**
  chains (create-a-credential-then-assume) that pure reachability cannot. Techniques (G7) will map
  their precondition/effect contracts to `PlanningAction`, so the planner stays DSL-agnostic (FR-F6).

`ReachResource` is satisfied when a controlled principal has a `CAN_ACCESS` edge to the target.

## Uncertainty (G12.3 / ADR-0025)

Dual-mode, honest by default:

- **`StrictProven`** — traverse only proven edges.
- **`HonestDefault`** — traverse proven + unproven; any path using an unproven edge is labeled
  `Candidate` (proven paths are `Proven`). Never hides a real path, never presents an unproven one
  as certain.
- **`FullOptimistic`** — traverse all non-`ABSENT` edges.

A path's `confidence` is `Proven` unless any hop used an unproven edge.

## Search & ranking (G12.4 / G12.5 · ADR-0026/0011)

Bounded **best-first (uniform-cost)** search: a min-heap keyed by accumulated cost, a visited set of
controlled-sets, and `max_depth` / `max_expansions` / `max_results` bounds (NFR-PERF3), with an
opt-in `exhaustive` mode. The **cost is the ranking preset**, so the search *is* the ranking:

- `Composite` (default): `confidence → length → detectability` (unproven hops dominate cost).
- `Shortest`, `Stealthiest`, `Safest`, `MostReliable` swap the weighting.

Reversibility and `max_impact` are always carried on the path as a **safety annotation**, shown even
when they are not the sort key (OQ-10).

## Explainability (G12.6 / FR-E3)

Each `PathStep` names the acting principal, the edge/technique, the enabling permission/trust, its
epistemic status, and the produced effect. `AttackPath::explain()` renders one line per hop plus the
confidence/reversibility/impact annotation — understandable without external lookup.

## Tests

Reach-admin over a static edge; no-path reported honestly; an `UNKNOWN` edge hidden under
`StrictProven` but surfaced as a `Candidate` under `HonestDefault`; a technique action creating an
otherwise-unreachable path; reach-resource after an escalation hop.
