# ADR-0025: UNKNOWN edges — dual-mode, honest default

- **Status:** Accepted
- **Date:** 2026-09-22
- **Ref:** SPEC-D14 · specification §5.3 · requirements FR-E5, FR-C6

## Context
Under partial enumeration many edges are `UNKNOWN`/`INFERRED` (ADR-0017). Optimistic traversal
surfaces paths that may not work; pessimistic (PROVEN-only) is blind to gaps and misses real paths.

## Decision
**Dual-mode with an honest default**: surface **PROVEN paths prominently** *and* **candidate
(unproven) paths clearly labeled "needs validation"**. Provider-assisted dry-run (ADR-0022)
promotes candidates. Operators may force strict-PROVEN or full-optimistic modes.

## Consequences
- (+) Low false negatives without false confidence; ties to epistemic status and validation.
- (−) Must render and reason over two path classes.

## Alternatives considered
- **Pessimistic (PROVEN only)** — rejected: blind to enumeration gaps; misses real paths.
- **Optimistic** — rejected: surfaces too many paths that won't work, unlabeled.
