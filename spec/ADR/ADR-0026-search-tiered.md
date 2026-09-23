# ADR-0026: Search strategy — tiered bounded + opt-in exhaustive

- **Status:** Accepted
- **Date:** 2026-09-22
- **Ref:** SPEC-D15 · specification §5.4 · requirements NFR-PERF3, OQ-10

## Context
Path search must remain usable on large graphs (NFR-PERF3). Exhaustive enumeration explodes;
the ranking objective (ADR-0011) can double as a search heuristic.

## Decision
**Tiered**: default to **bounded best-first search** guided by the ranking score, returning
top-K within a depth/time/expansion budget; provide an **opt-in exhaustive/expanded mode** for
smaller graphs or when completeness matters.

## Consequences
- (+) Scales by default; ranking = heuristic; completeness available on demand.
- (−) Default is incomplete (acceptable; exhaustive escape exists).

## Alternatives considered
- **Exhaustive** — rejected: explodes on large graphs.
- **Bounded best-first only** — rejected: no completeness escape even on small graphs.
