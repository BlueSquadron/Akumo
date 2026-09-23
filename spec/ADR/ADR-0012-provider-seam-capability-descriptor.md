# ADR-0012: Provider seam — capability + descriptor hybrid

- **Status:** Accepted
- **Date:** 2026-09-21
- **Ref:** SPEC-D1 · specification §1.3, §2.5 · requirements NFR-EXT

## Context
The seam granularity decides how provider-neutral the core stays and how hard multi-cloud v2 is.
A semantic seam makes each provider a large lift; a thin transport seam leaks cloud specifics
into the core.

## Decision
Use a **capability + descriptor hybrid**: a small set of capability interfaces (IdentityResolver,
ResourceEnumerator, ActionExecutor, GraphMapper, MetadataProvider, and a v2 TelemetryCollector)
where enumeration and actions are driven by **declarative descriptors** the adapter maps to
concrete API calls. Adding a provider = ship a descriptor catalog + implement the capabilities.

## Consequences
- (+) Core stays neutral (NFR-EXT2), adding a provider is additive (NFR-EXT3), techniques stay
  declarative and host-brokered (NFR-SEC5).
- (−) Up-front design of the capability interfaces + descriptor format.

## Alternatives considered
- **Semantic seam** — rejected: large per-provider lift; risks one cloud's model becoming "the domain".
- **Thin transport seam** — rejected: leaks provider specifics into the core.

> User expressed no preference and delegated; C selected as the only option consistent with
> ADR-0013 and the provider-neutral-core mandate.
