# ADR-0031: Content distribution — separate versioned, signed bundle

- **Status:** Accepted
- **Date:** 2026-09-22
- **Ref:** SPEC-D19 · specification §7.4 · requirements NFR-EXT6, FR-F3, NFR-MNT3, NFR-SEC5

## Context
The technique catalog must grow without a full engine release (NFR-EXT6) and support community
contribution (FR-F3), while staying safe for sensitive/offline environments.

## Decision
Techniques ship as a **separate, versioned, signed content bundle** loaded by the engine. A
default catalog is shipped alongside the binary, but the bundle is **updatable independently**,
**versioned** (NFR-MNT3), and **signed** (NFR-SEC5). A pull-able registry is deferred to v2 as a
convenience layered on this model.

## Consequences
- (+) Catalog evolves without an engine release; community-extensible; offline-capable; bounded
  supply-chain surface via signing.
- (−) Requires bundle versioning, compatibility ranges, and signing infrastructure.

## Alternatives considered
- **Bundled in the binary** — rejected: content can't update without an engine release; too rigid.
- **Pull-able registry** — deferred to v2: adds a network dependency and the largest
  supply-chain surface; inappropriate as a v1 default for a security tool.
