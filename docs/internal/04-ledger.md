# 04 — Event Ledger

> Tasks G2.1–G2.3 · ADR-0014 / SPEC-D3 · spec §2.1, §2.7. Types in `akumo-domain` (`event`,
> `hash`); store in `akumo-ledger`; folding in `akumo-core::projection`.

The engagement's entire state is an **append-only, hash-chained event ledger**. The ledger *is* the
source of truth; the attack graph, coverage map, execution status, and reports are all deterministic
**projections** folded from it. One mechanism thereby delivers audit (FR-J1), resume (NFR-REL2),
replay (FR-J2), revert-after-restart (DSR-4), tamper-evidence (NFR-OBS3), and isolation (DSR-2).

```
        append-only, hash-chained
 ┌───────────────────────────────────┐
 │            EVENT LEDGER            │   ← source of truth
 └───────────────┬───────────────────┘
                 │ fold / replay (pure, deterministic)
   ┌─────────┬───┴─────┬──────────┬──────────┐
   ▼         ▼         ▼          ▼          ▼
 Attack   Coverage  Execution   Loot       Audit /
 Graph     Map      Ledger      Store      Report view
 (G4)      (G6)     (G9)        (G14)      (G14)
```

## The event envelope (`event.rs`)

`EventEnvelope { id, engagement_id, seq, timestamp, actor, prev_hash, hash, event_type, payload }`.

- **Sealing:** `EventEnvelope::seal(...)` computes `hash = H(prev_hash ‖ canonical(payload))` where
  `H` is SHA-256 (`hash.rs`) and `canonical(payload)` is the compact JSON serialization. `serde_json`
  object maps are `BTreeMap` (no `preserve_order` feature), so keys are sorted and the bytes are
  deterministic — a prerequisite for a content hash.
- **Verification:** `verify_hash()` recomputes and compares; a mismatch is `AkumoError::Integrity`.
- The v1 `event_type` is a string; it becomes a typed per-category enum (spec §2.7) as the event
  producers land (G5/G6/G9/G14).

## The store (`akumo-ledger::FileEventStore`)

Implements the `EventStore` port. Substrate: **one JSON-Lines file per engagement**
(`<root>/<engagement>.jsonl`), append + `fsync`. Chosen for inspectability (DSR-1) — an operator can
read the ledger with `cat`/`jq`. Increment 7 may swap in an embedded KV store (`redb`) behind the
same port with no core change.

- **`append`** validates the chain before writing: `prev_hash` must equal the current head and `seq`
  must be the next in sequence, and the event's own `verify_hash()` must pass. This makes malformed
  or out-of-order appends impossible.
- **`open`** rebuilds each engagement's chain head from disk, so appends resume correctly after a
  restart (NFR-REL2).
- **Isolation** is structural: separate files per engagement (DSR-2).
- **`verify_chain`** re-checks every hash and the `prev`/`seq` linkage end-to-end (audit, NFR-OBS3).

## Projections (`akumo-core::projection::Projection`)

A projection provides only `apply(state, event)`; `replay(stream)` and `replay_to(stream, seq)` are
derived. Because `apply` is pure, replaying the same stream always yields the same state — the
determinism that audit-grade replay (ADR-0010) depends on. Snapshots are a cache only, never a
second source of truth.

## Tests

`hash.rs` (determinism, chaining), `event.rs` (seal/verify, tamper detection, chain linkage),
`file_store.rs` (append/read, reject wrong prev/seq, reopen-rebuilds-head + isolation),
`projection.rs` (replay determinism, inclusive `replay_to`).
