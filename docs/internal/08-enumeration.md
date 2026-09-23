# 08 — Enumeration Service

> Task G6 · FR-C · spec §2.6. In `akumo-core::enumeration`.

The Enumeration Service turns provider reads into ledger events, from which the attack graph (G4) is
folded. It is descriptor-driven and degrades gracefully under partial permissions.

## Flow (G6.1)

`enumerate(engagement, actor, descriptors)`:

1. Load the `EngagementContext`; refuse if the engagement is missing or not active (kill-switch/
   closed).
2. Record `EnumerationStarted`.
3. For each `EnumerationDescriptor`: `ResourceEnumerator::execute` → `GraphMapper::map` → one
   `FactAsserted` event per `Assertion`. All provider access goes through the `Provider` aggregate
   (`provider.enumerator()`, `provider.mapper()`), so calls are unambiguous and host-brokered.
4. Record `EnumerationCompleted` with an `EnumerationSummary` (descriptors run, facts asserted,
   coverage gaps).

## Partial-permission degradation (G6.5)

A denied call (`AkumoError::AccessDenied`, `is_access_gap()`) is **not** a failure: it records an
`AccessDenied` event carrying a `CoverageGap { scope, reason, observed_at }` and continues with the
remaining descriptors (FR-C6). Only non-access errors propagate. The graph projection turns these
into coverage-map entries — "unknown ≠ absent."

## Caching & provenance

- **Cache (G6.2):** a per-run cache keyed by `service.operation` + params avoids re-issuing an
  identical provider call within a batch (FR-C2, NFR-PERF2). Cross-invocation caching is a later
  refinement.
- **Provenance (G6.4):** each assertion carries `Provenance` (source operation, permission, time),
  stamped by the adapter's `GraphMapper`. The Mock authors it directly; the AWS adapter will set it
  from the producing call.

## Deferred in this increment

Bounded concurrency (G6.3, currently sequential) and cross-invocation/engagement-scoped caching and
resumable incremental enumeration (G6.4 full) are refinements layered on once the loop is proven.

## M2 vertical slice (proven by test)

`m2_open_enumerate_and_build_graph` wires the real pieces end-to-end — `FileEventStore` +
`MockProvider` + `EngagementManager` + `EnumerationService` + `GraphProjection` — and asserts: open
→ enumerate (one readable call, one denied) → a folded graph with 2 nodes, 1 `CAN_ASSUME` edge, and
1 coverage gap. A second test confirms enumeration is refused after the kill-switch. This is the
**M2 milestone** (enumerate → populated graph); the full CLI-driven slice is G10.
