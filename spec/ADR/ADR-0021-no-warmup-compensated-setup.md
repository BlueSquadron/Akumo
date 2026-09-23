# ADR-0021: No warm-up phase; setup = compensated steps

- **Status:** Accepted
- **Date:** 2026-09-22
- **Ref:** SPEC-D10 · specification §4.3

## Context
Stratus warms up *synthetic* prerequisites to validate detection against its own account.
Akumo's primary job is to demonstrate *real* exploitability against the target's actual
resources, so a Stratus-style warm-up doesn't fit — and it would add a second teardown path.

## Decision
v1 has **no distinct warm-up/cleanup phase**. Any prerequisite creation is an **ordinary step
with a saga compensation** (ADR-0019). A synthetic warm-up returns only later as an optional
**lab / detection-validation mode**.

## Consequences
- (+) One teardown mechanism (compensations); simplest, honest fit for real-target v1.
- (−) No synthetic detection-validation mode in v1 (tracked as a v2 evolution).

## Alternatives considered
- **Keep an explicit warm-up phase** — rejected: built for synthetic targets; two teardown paths.
- **Hybrid now** — rejected: more surface than v1 needs.
