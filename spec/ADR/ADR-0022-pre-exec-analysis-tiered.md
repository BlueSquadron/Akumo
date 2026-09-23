# ADR-0022: Pre-execution analysis — static default + opt-in provider-assisted

- **Status:** Accepted
- **Date:** 2026-09-22
- **Ref:** SPEC-D11 · specification §4.4 · requirements FR-G2/G3, FR-E5, NFR-PERF4

## Context
Dry-run and blast-radius preview must balance fidelity against footprint. Offline analysis is
free but bounded by declarations; provider-assisted analysis is higher-fidelity but makes calls.

## Decision
**Offline static preview by default** (from descriptors + contract effects + the graph; zero
footprint, full coverage). **Opt-in provider-assisted validation** (AWS `DryRun`, IAM policy
simulator, read-only probes) for higher confidence before real detonation — which also promotes
`INFERRED`/`UNKNOWN` edges toward `PROVEN` (FR-E5).

## Consequences
- (+) Safe, cheap default; high-fidelity when asked; footprint stays operator-controlled (NFR-PERF4).
- (−) Provider-assisted coverage is partial (not all APIs support dry-run).

## Alternatives considered
- **Offline only** — rejected: won't catch real permission/resource-policy denials.
- **Provider-assisted always** — rejected: unnecessary footprint; partial coverage.
