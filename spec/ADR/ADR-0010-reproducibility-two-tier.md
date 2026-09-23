# ADR-0010: Reproducibility — audit-grade replay + path re-verification

- **Status:** Accepted
- **Date:** 2026-09-21
- **Ref:** OQ-9 · requirements FR-J2

## Context
Cloud targets drift, so "re-run and get identical results" is dishonest as a general promise.
We need a reproducibility guarantee that is both meaningful and achievable.

## Decision
Promise **two tiers**: (1) **audit-grade replay** — the recorded run is a complete, reviewable
narrative (always achievable); (2) **path re-verification** where safe — confirm a discovered
path still holds, with **"equivalent target" defined graph-structurally, scoped to the subgraph
the path touched**. Full byte-identical re-execution is offered only for IaC/lab targets as a
bonus, never guaranteed.

## Consequences
- (+) Honest about cloud drift; leans on the graph model; evidence-grade audit always holds.
- (−) No universal "replay against any live target" guarantee.

## Alternatives considered
- **Full deterministic re-execution** — rejected: unachievable against drifting live targets.
- **Audit replay only** — rejected: misses the useful "does the path still hold?" check.
