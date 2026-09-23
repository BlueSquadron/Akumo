# ADR-0006: Embedded-first graph backend

- **Status:** Accepted
- **Date:** 2026-09-21
- **Ref:** OQ-6 · requirements FR-D4

## Context
The attack graph must be queryable and portable. An external graph database scales and offers
rich queries but breaks the single-binary promise (ADR-0003) and adds operational burden.

## Decision
Use an **embedded, in-process graph** with an **export** path (FR-D4). Revisit an external graph
DB only if graph scale genuinely demands it.

## Consequences
- (+) Preserves single-binary distribution; zero external dependencies to run an engagement.
- (+) Export path keeps interop open.
- (−) Very large graphs may eventually need an external store (tracked as a v2 evolution).

## Alternatives considered
- **External graph DB from day one** — rejected: contradicts single-binary, premature for v1
  scale.
