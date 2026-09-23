# ADR-0016: Facts as derived views over the property graph

- **Status:** Accepted
- **Date:** 2026-09-22
- **Ref:** SPEC-D5 · specification §2.3 · requirements FR-F6

## Context
FR-F6 requires a shared fact model feeding both execution (inter-step data-flow) and planning
(preconditions/effects). A separate flat fact layer (CALDERA-style) is ergonomic but risks a
second source of truth that drifts from the graph.

## Decision
The **property graph is the single store** (a ledger projection); **facts are named, typed
views/predicates over it** for ergonomic templating and requirement-matching. Preconditions
compile to graph patterns; effects are graph mutations surfaced as predicates.

## Consequences
- (+) Single source of truth + authoring ergonomics; natural fit for "one declaration feeds
  execution and the planner".
- (−) Must design the predicate/view layer.

## Alternatives considered
- **Unified graph-as-facts** — viable; slightly more ceremony for scalar templating.
- **Separate flat fact layer** — rejected: dual-model drift; weaker planner.
