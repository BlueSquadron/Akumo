# ADR-0024: Planning — hybrid reachability + technique-as-action

- **Status:** Accepted
- **Date:** 2026-09-22
- **Ref:** SPEC-D13 · specification §5.2 · requirements FR-E, FR-F6

## Context
We built both a graph (ADR-0015) and a precondition/effect model (ADR-0016/FR-F6). Pure graph
reachability can't represent techniques that *create* new state (make key → use it); pure
symbolic planning is heavier and ignores the fast graph frontier.

## Decision
Use a **hybrid** planner: fast **graph reachability** over existing identity/trust/permission
edges, **plus** symbolic **technique-as-action** expansion where a technique's effects generate
new edges/facts that extend the frontier.

## Consequences
- (+) Represents multi-step, state-creating chains (Akumo's differentiator) while scaling.
- (+) Uses both models as intended; mirrors research attack-graph planners (logical graphs + PDDL).
- (−) More engine complexity than pure reachability.

## Alternatives considered
- **Graph reachability only** — rejected: can't model state-creating chains.
- **Symbolic planning only** — rejected: heavier; ignores the fast graph frontier.
