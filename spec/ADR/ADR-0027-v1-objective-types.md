# ADR-0027: v1 objective types — reach-admin + reach-resource

- **Status:** Accepted
- **Date:** 2026-09-21
- **Ref:** OQ-3 · specification §5.1 · requirements FR-E2

## Context
Objective-directed path-finding needs a bounded, concrete set of goal types for v1 without
over-scoping.

## Decision
v1 supports two objective classes, expressed as **goal predicates** over the graph/fact views:
**reach administrative privilege** and **reach a specified resource**. Objectives are predicates,
so new classes are additive with no planner change.

## Consequences
- (+) Covers the headline user story; extensible by construction.
- (−) Other objective classes (e.g., specific data exfil conditions) deferred (tracked v2).

## Alternatives considered
- **Broader objective catalog in v1** — rejected: over-scopes the first release.
