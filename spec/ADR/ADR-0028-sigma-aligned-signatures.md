# ADR-0028: Detection signatures — Sigma-aligned

- **Status:** Accepted
- **Date:** 2026-09-22
- **Ref:** SPEC-D16 · specification §6.1 · requirements FR-I1, FR-I3

## Context
v1 emits expected-telemetry signatures only (ADR-0008). Their representation decides SIEM
interoperability and whether Akumo can auto-generate candidate detections.

## Decision
Express expected-telemetry signatures in a **Sigma-aligned** form (the SIEM-agnostic detection
standard, which has an AWS CloudTrail taxonomy). This lets Akumo **auto-generate candidate
detections** (FR-I3) and interoperate with Elastic / Sentinel / Splunk via Sigma converters. A
small **native superset** is added only where a cloud field falls outside Sigma's taxonomy.

## Consequences
- (+) Out-of-the-box detection ecosystem interop; high blue-team value; low build cost.
- (+) Matches Leonidas precedent for cloud attack → Sigma generation.
- (−) Bounded by Sigma's expressiveness (mitigated by the native superset).

## Alternatives considered
- **Akumo-native format** — rejected: no ecosystem interop until we build converters.
- **MITRE-mapping-only** — rejected: no field-level detail; low blue-team value.
