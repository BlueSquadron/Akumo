# ADR-0005: Declarative fact-based step-DSL primary; Starlark/WASM escape hatches

- **Status:** Accepted
- **Date:** 2026-09-21
- **Ref:** OQ-7 · requirements FR-F1/F6 · Analysis.md Appendix A

## Context
A prior-art survey (Atomic Red Team, CALDERA, Leonidas, Sigma/Nuclei, MITRE Attack Flow, PDDL)
showed a declarative DSL is the industry norm and the most AI/human-authorable, auditable, and
testable surface. An earlier provisional choice of "Starlark-primary" was made without this
survey and is corrected here.

## Decision
The **primary authoring surface is a declarative, fact-based step-DSL** (sequenced host-brokered
API calls with named fact inputs/outputs). **Starlark (Tier 2)** and **WASM (Tier 3)** are
escape hatches for the minority of techniques needing arbitrary orchestration or heavy compute.
CALDERA's fact model + PDDL's precondition/effect model unify content with the planner (FR-F6).

## Consequences
- (+) Maximizes authorability; unifies execution and planning from one declaration.
- (+) Aligns with MITRE ATT&CK (classify) and STIX/Attack Flow (interchange).
- (−) We design and maintain a DSL (more up-front work than embedding a scripting language).

## Alternatives considered
- **Scripting-primary (Starlark)** — rejected: loses planner unification and some auditability.
- **Pure declarative only** — rejected: expressiveness ceiling.
