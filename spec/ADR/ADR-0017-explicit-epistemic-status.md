# ADR-0017: Explicit epistemic status on assertions

- **Status:** Accepted
- **Date:** 2026-09-22
- **Ref:** SPEC-D6 · specification §2.4 · requirements FR-C6, FR-E5

## Context
Offensive enumeration is almost always partial (limited foothold, throttling, denied calls).
Conflating "unseen/denied" with "absent" causes both missed real paths and dishonest reports.

## Decision
Every attack-relevant assertion carries a **categorical epistemic status**:
`PROVEN` / `ABSENT` / `UNKNOWN` (denied/unseen) / `INFERRED` (derived), with an optional
confidence score on `INFERRED`. This drives honest planning, confidence ranking (FR-E5), and
blind-spot reporting (FR-C6).

## Consequences
- (+) Preserves "unseen ≠ absent"; enables dual-mode planning (ADR-0025) and honest reports.
- (−) The planner must define behavior over `UNKNOWN` edges.

## Alternatives considered
- **Presence-only + gaps list** — rejected: planner can't natively reason about uncertainty.
- **Confidence-only** — rejected: fuzzy, hard to explain, still loses unseen≠absent.
