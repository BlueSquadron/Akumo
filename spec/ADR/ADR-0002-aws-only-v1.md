# ADR-0002: v1 targets AWS only, with a provider-neutral core

- **Status:** Accepted
- **Date:** 2026-09-21
- **Ref:** requirements.md §2, §13.1

## Context
Multi-cloud was dropped from v1 to go deep rather than wide, but the architecture must add
providers later without a rewrite. AWS has the deepest offensive research, the richest API
surface, and the most mature Rust SDK, and is the shared home of every flagship inspiration.

## Decision
Ship **AWS as the sole v1 provider adapter**. The core, graph, planner, execution lifecycle,
and content engine remain provider-agnostic (ADR-0001); adding Azure/GCP/Kubernetes later is an
additive adapter + technique effort.

## Consequences
- (+) Depth over breadth; rides the mature (GA, 300+ services) AWS-Rust SDK with no penalty.
- (+) The mock provider proves the extensibility gate today.
- (−) No multi-cloud in v1 (tracked as a v2 evolution).

## Alternatives considered
- **Azure or GCP first** — rejected: less mature offensive research/tooling and (for Rust)
  thinner SDKs at the time of decision.
- **Multi-cloud from v1** — rejected: breadth would starve the differentiating depth.
