# 11 — The Vertical Slice & Extensibility Gate

> Task G10 · requirements §12 item 8 · NFR-EXT1/EXT7 (release-blocking). Test in
> `akumo-core::api` (`extensibility_gate_full_loop_on_mock`).

## The end-to-end loop (G10.1)

A single test drives the **entire unified loop** through the [`Akumo`](../../code/crates/akumo-core/src/api.rs)
library facade against the **Mock** provider — no cloud, no CLI:

1. **authorize** — `authcheck` resolves the foothold principal; `open_engagement` records the
   authorization affirmation and scope.
2. **enumerate** — a scripted `iam.ListPrincipals` yields a foothold, an admin role, and a
   `CAN_ASSUME` edge → three `FactAsserted` events.
3. **graph** — folded from the ledger: 2 nodes, 1 edge.
4. **plan** — `paths(ReachAdmin)` finds the foothold→admin path.
5. **execute** — consent recorded, then a `create-access-key` technique detonates and verifies.
6. **revert** — the recorded `DeleteAccessKey` compensation replays cleanly.
7. **report** — the folded report lists the technique used and resolves MITRE coverage `T1098`.
8. **close** — the engagement ends `Closed`.

This exercises every subsystem (engagement, enumeration, graph, planner, execution, revert, report)
together, over the real ledger.

## The extensibility gate (G10.2)

The same test **is** the NFR-EXT7 gate: the provider-blind core runs the full loop with the Mock
standing in as a "second provider" and **zero core changes**. The "zero core changes" half is
enforced *structurally and continuously* by `xtask dep-lint` (task G1.4) — the core cannot even
compile a dependency on an adapter or cloud SDK. Together they are the proof that adding a real
provider (AWS, G16) is purely additive:

- **fast tier / every PR:** this loop test passes on the Mock (`cargo test`), and `dep-lint` is green.
- when the AWS adapter lands, the *same* core and the *same* loop run against it — the adapter is the
  only new code.

## Note on test location

Cargo-discoverable tests live inside their crates (this one in `akumo-core`); the repo-root
[`tests/`](../../tests/) tree is reserved for cross-cutting, CLI-transcript, and scenario fixtures
that are not `cargo` unit/integration tests.
