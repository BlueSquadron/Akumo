# ADR-0007: v1 deterministic path-finding only; AI planner deferred

- **Status:** Accepted
- **Date:** 2026-09-21
- **Ref:** requirements FR-E, FR-E6 (deferred) · §13.1

## Context
The attack-graph-native thesis requires strong deterministic path-finding first. An AI-assisted
planner is attractive but adds risk and a safety-review burden on top of an unproven graph engine.

## Decision
v1 ships **deterministic, explainable path-finding only** (graph + symbolic planning). The
**AI-assisted planner is deferred to v2** (FR-E6 → `[W]`); if it lands, it stays advisory,
human-gated, and subject to the same safety/consent controls as any other action.

## Consequences
- (+) Focuses effort on the differentiating core; keeps the safety story clean.
- (−) No natural-language objectives / AI next-step suggestions in v1 (tracked as a v2 evolution).

## Alternatives considered
- **Ship the AI planner in v1** — rejected: compounds risk on an unproven graph engine.
