# ADR-0030: Mock-provider fidelity — behavioral/scriptable stub

- **Status:** Accepted
- **Date:** 2026-09-22
- **Ref:** SPEC-D18 · specification §7.2 · requirements NFR-EXT7, OQ-8

## Context
The mock provider serves as the extensibility gate's "second provider" (NFR-EXT7) and the fast
test tier. Its fidelity to AWS sets how much the fast tier catches before the live-AWS tier.

## Decision
The mock is a **behavioral / scriptable stub**: it models graph state
(principals/permissions/resources/trust) and returns scripted responses, exercising
enumeration/plan/execute/revert **logic** and the extensibility gate. It is deliberately **not**
an AWS simulator; the **live-AWS release tier (ADR-0009) is the semantic-fidelity backstop**.

## Consequences
- (+) Cheap, deterministic, contributor-runnable; clean division of labor (mock = logic,
  live-AWS = semantics); no false confidence.
- (−) Real AWS authz/consistency nuances only caught at the live-AWS tier.

## Alternatives considered
- **High-fidelity simulator** — rejected: LocalStack-class maintenance sink; risks silent divergence.
- **Stub + pluggable authz-eval module** — deferred: add only if the tier gap proves costly.
