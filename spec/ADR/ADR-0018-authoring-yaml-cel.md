# ADR-0018: Authoring format — YAML + CEL

- **Status:** Accepted
- **Date:** 2026-09-22
- **Ref:** SPEC-D7 · specification §3.5

## Context
OQ-7 prioritized human+AI authorability. The recurring pain of every YAML DSL in this field is
expressions, control-flow, and inter-step data-flow (Leonidas fell back to stringly-typed Jinja).

## Decision
Author techniques in a **YAML container** with **CEL (Common Expression Language)** for
predicates, conditions, and templating. CEL is safe, typed, non-Turing-complete, and embeddable
(used in Kubernetes admission and Cloud IAM conditions). Anything CEL can't express escapes to
Starlark (ADR-0020).

## Consequences
- (+) Highest authorability + mature tooling; a real, safe expression layer instead of Jinja.
- (+) Lowest build cost (compose two mature pieces).
- (−) Expressiveness ceiling at CEL → escape to Tier 2 for complex logic.

## Alternatives considered
- **Purpose-built grammar** — rejected: high parser/tooling build+maintenance cost.
- **HCL / CUE** — rejected: heavier and less universally AI-familiar than YAML.
