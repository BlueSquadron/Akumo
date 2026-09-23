# ADR-0020: Per-step escape hatches + always-declarative contract

- **Status:** Accepted
- **Date:** 2026-09-22
- **Ref:** SPEC-D9 (resolves the OQ-7 residual) · specification §3.7

## Context
Starlark/WASM escape hatches must integrate with the DSL without letting the planner/validator
depend on understanding an escape-hatch language.

## Decision
Escape hatches are **per-step** (a DSL technique may have individual `script`/`wasm` steps), but
the technique's **metadata + contract (preconditions/effects) MUST stay declarative Tier-0**.
The planner and validator read only the contract, never the step bodies.

## Consequences
- (+) Fine-grained author power while the planner never parses Starlark/WASM (FR-F6/FR-E intact).
- (+) Closes the OQ-7 residual entirely.
- (−) Paradigm mixing within a file, bounded by the contract invariant.

## Alternatives considered
- **Per-step escape (unrestricted)** — rejected: a technique's contract could become opaque.
- **Whole-technique tiers** — rejected: one computed value forces the whole technique to script.
