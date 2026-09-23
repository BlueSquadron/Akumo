# ADR-0011: Default path ranking — composite + named presets

- **Status:** Accepted
- **Date:** 2026-09-21
- **Ref:** OQ-10 · requirements FR-E4 · specification §5.5

## Context
The planner returns several paths; the default sort is the first thing an operator trusts and is
also a product-identity choice. Akumo uniquely has expected-telemetry data, enabling stealth
ranking.

## Decision
Default to a **composite score ordered confidence → length → detectability**, offered as named
presets **shortest / stealthiest / safest / most-reliable**. **Reversibility / blast-radius is
always shown as a safety annotation**, even when not the sort key. The composite doubles as the
best-first search heuristic (ADR-0026).

## Consequences
- (+) Default optimizes for a real, working, explainable path; stealth/safety are selectable lenses.
- (+) Leverages telemetry data (a differentiator) and epistemic status (confidence).
- (−) Composite weighting must be documented and defensible.

## Alternatives considered
- **Single-criterion default** (shortest/stealthiest/safest) — offered as presets, not the default.
