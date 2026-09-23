# ADR-0004: Hybrid technique content model (metadata + action body)

- **Status:** Accepted
- **Date:** 2026-09-21
- **Ref:** requirements.md FR-F, §13.1

## Context
Techniques must be safe-by-construction, auditable, testable, MITRE-tagged, and AI/human
authorable, yet expressive enough for real cloud exploitation — Pacu's imperative Python is too
opaque to trust; Atomic Red Team's pure command strings are too weak for cloud orchestration.

## Decision
Adopt a **hybrid content model**: declarative **metadata** (ID, MITRE, impact, telemetry,
revert contract) + a sandboxed **action body**. The action body's authoring format is decided in
ADR-0005.

## Consequences
- (+) Combines auditability/testability of declaration with the power needed for real techniques.
- (+) Metadata is always machine-readable for the planner and validator.
- (−) Two things to specify (metadata schema + action body format).

## Alternatives considered
- **Pure declarative** — rejected: expressiveness ceiling (Leonidas-style limits).
- **Pure imperative/plugin code** — rejected: hard to audit/sandbox/trust (Pacu's weakness).
