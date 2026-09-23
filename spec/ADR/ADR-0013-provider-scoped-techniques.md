# ADR-0013: Provider-scoped techniques + shared helpers

- **Status:** Accepted
- **Date:** 2026-09-21
- **Ref:** SPEC-D2 · specification §1.4

## Context
Most cloud attack techniques are inherently provider-specific (an IAM privesc *is* AWS). Forcing
cross-cloud portability would impose a lowest-common-denominator abstraction over clouds that
don't share faithful equivalents.

## Decision
Techniques are **provider-scoped** (declare `provider: aws`, may reference that cloud's
operations); the **engine/graph/planner/lifecycle stay provider-agnostic**. Common helpers
(signing, encoding, fact-shaping) are shared across providers.

## Consequences
- (+) Honest, simpler content model; matches Leonidas/CALDERA precedent.
- (+) Multi-cloud means "many techniques per cloud", which is the real world.
- (−) Low cross-cloud technique reuse (acceptable/honest).

## Alternatives considered
- **Cross-cloud portable techniques** — rejected: high theoretical reuse, low practical value,
  higher content-model complexity.
