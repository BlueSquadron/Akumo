# 07 — Engagement Manager

> Tasks G5.1–G5.5 · FR-A, NFR-COMP · spec §1.7, §4.2. In `akumo-core::engagement` (+ the shared
> `akumo-core::journal` event writer).

The Engagement Manager is the authorization gate every operation passes through. All state is
recorded as ledger events (ADR-0014), so it is durable, auditable, and engagement-isolated, and
switching engagements needs no restart (FR-A6).

## The journal (`akumo-core::journal`)

`Journal` is the single place events are written. Given `(engagement, actor, type, payload)` it
reads the chain head (`EventStore::head`), computes the next `seq` and `prev_hash`, seals the event,
and appends it. Every producer (engagement, enumeration, execution) records through it, so the hash
chain stays intact and sequencing is centralized.

## Lifecycle (G5.1)

- `open` — records `EngagementOpened` with scope, provider, credential reference, and the
  **authorization affirmation** (NFR-COMP1). Refuses a `false` affirmation and an **empty scope**
  (fail-closed), and refuses to re-open an existing engagement.
- `amend_scope`, `record_consent`, `invoke_kill_switch`, `close` — the remaining transitions.
- `load` folds the stream into an `EngagementState` via the `EngagementView` projection; `context`
  yields the ambient `EngagementContext`.

## Scope enforcement (G5.2)

`EngagementContext::ensure_in_scope(kind, value)` refuses out-of-scope targets (FR-A1, NFR-COMP2);
an empty scope authorizes nothing. Enumeration (G6) and execution (G9) consult the context before
every provider call. `ensure_active()` refuses actions after the kill-switch or close.

## Consent (G5.3)

Impact-gated (FR-A3), with two policies for the two operating contexts (spec §4.2):

- **`Interactive`** — a step needs a recorded consent at or above its `ImpactLevel`
  (`EngagementState::max_consent` tracks the highest granted).
- **`CiCeiling(impact)`** — steps at or below a declared ceiling are pre-authorized; anything above
  halts rather than proceeding unattended.

`consent_satisfied(policy, max_recorded, impact)` is the pure decision function (read-only never
needs consent). It relies on `ImpactLevel`'s ordering.

## Kill-switch (G5.4)

`invoke_kill_switch` records `KillSwitchInvoked`; the projection flips status to `Killed` and
`ensure_active()` then fails, halting new work. Revert of completed reversible steps is wired by the
execution engine (G9).

## Dry authorization check (G5.5)

`authcheck(provider)` resolves the current principal and provider metadata (regions) via the
`IdentityResolver`/`MetadataProvider` capabilities — **no enumeration, no ledger writes** (FR-A5).

## Tests

Auth/scope required to open; open→load→scope enforcement + no-double-open; kill-switch deactivates;
recorded consent raises the ceiling; the `consent_satisfied` truth table. A small in-memory
`EventStore` (in the test module) keeps these tests off the ledger adapter.
