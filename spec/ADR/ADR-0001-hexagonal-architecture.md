# ADR-0001: Hexagonal (ports & adapters) architecture

- **Status:** Accepted
- **Date:** 2026-09-21
- **Ref:** specification.md §1.1 · requirements NFR-EXT1–EXT7, FR-K

## Context
The requirements mandate a provider-neutral core with a single provider seam (NFR-EXT), a mock
provider (NFR-EXT7), and three driving surfaces — CLI, library, CI (FR-K). We need a structure
that keeps cloud/IO/human concerns out of the core so adding a provider is additive, not a
rewrite.

## Decision
Adopt a **hexagonal (ports & adapters)** architecture. A provider-agnostic domain core depends
only on **ports** (Provider, Persistence, Telemetry, driving ports); everything cloud-specific,
storage-specific, or human-facing is an **adapter**. A CI dependency-direction lint (core MUST
NOT import any adapter or cloud SDK) is release-blocking.

## Consequences
- (+) Directly realizes NFR-EXT1–4; core is unit-testable against the Mock adapter alone (EXT7).
- (+) CLI/library/CI are three driving adapters over one core (FR-K).
- (−) Requires boundary discipline and some indirection; contributors must respect the seam.

## Alternatives considered
- **Layered monolith** — rejected: provider logic inevitably leaks into the core, breaking the
  provider-neutral mandate.
