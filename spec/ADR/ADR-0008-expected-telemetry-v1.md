# ADR-0008: v1 expected-telemetry signatures only; live correlation deferred

- **Status:** Accepted
- **Date:** 2026-09-21
- **Ref:** requirements FR-I1 (v1), FR-I2 (deferred) · §13.1

## Context
Every technique declaring its expected telemetry signature is cheap and high-value. Live
capture and observed-vs-expected correlation (Grimoire-style) is valuable but adds real
complexity (log polling, timing, per-service quirks).

## Decision
v1 emits **expected telemetry signatures + MITRE mapping only** (FR-I1). **Live observed-vs-
expected correlation is deferred to v2** (FR-I2 → `[W]`).

## Consequences
- (+) Ships the headline purple-team value at low cost; keeps v1 scope tight.
- (−) No live detection feedback loop in v1 (tracked as a v2 evolution).

## Alternatives considered
- **Live correlation in v1** — rejected: complexity outweighs v1 value.
- **Defer all telemetry to v2** — rejected: drops a headline differentiator (D7 in Analysis).
