# ADR-0023: Chain failure — policy-driven safe default

- **Status:** Accepted
- **Date:** 2026-09-22
- **Ref:** SPEC-D12 · specification §4.7 · requirements FR-H2, FR-G4, FR-K2

## Context
When a step fails mid-chain, the engine must be safe unattended (CI) yet controllable
interactively. Fully atomic auto-revert can't inspect-before-undo; pure halt-and-hold needs a
human.

## Decision
**Policy-driven with a safe default**: default = **halt + auto-revert of completed steps**
(CI-safe, leaves the target clean); interactive operators may opt into **halt-and-hold** to
inspect, fix, and resume. Kill-switch and on-demand revert remain available throughout.

## Consequences
- (+) Safe for unattended/CI runs (FR-K2); interactive control retained; satisfies FR-H2 + FR-G4.
- (−) Two behaviors to implement and document.

## Alternatives considered
- **Atomic auto-revert whole chain** — rejected: no inspect-before-undo, no resume.
- **Halt-and-hold (manual)** — rejected: leaves target mutated pending a human; poor for CI.
