# 12 — Chaining & Objective Execution

> Task G11 · FR-H · ADR-0023 (chain failure policy) · spec §4.7. In `akumo-core::chain`.

A chain runs an ordered sequence of techniques (realizing a computed path) through the safe execution
engine (G9), one technique at a time.

## Chain executor (G11.1)

`ChainExecutor::execute(engagement, actor, chain, config)` iterates `ChainStep`s (a `Technique` + its
`inputs`), calling `ExecutionEngine::detonate` for each. Per-technique consent gating (FR-A3) is
honored by the engine; the chain checks the engagement is still active (kill-switch / closure) before
each technique.

## Failure policy (G11.2 / ADR-0023 = SPEC-D12 = C)

The default is **halt + auto-revert of the completed techniques** — CI-safe, leaving the target
clean without a human. On a technique failure (or a consent block), the chain reverts each completed
technique's detonation in reverse order (scoped precisely by the `detonation_id` each `Completed`
outcome carries), producing a combined `RevertReport`.

`ChainFailurePolicy::HaltAndHold` instead leaves completed techniques in place so an interactive
operator can inspect, fix, and resume. The kill-switch and on-demand `ExecutionEngine::revert` remain
available throughout.

Statuses: `Completed`, `DryRun`, `HaltedConsent`, `FailedAndReverted`, `FailedAndHeld`,
`FailedRevertIncomplete` (a compensation could not be undone — surfaced, never hidden).

## Inter-step re-enumeration (G11.3 / FR-H4)

When `ChainConfig.reenumerate` carries enumeration descriptors, the executor re-runs enumeration
after each successful technique, so a later technique's preconditions reflect access created by an
earlier one. Empty (the default) disables it.

## Precise revert scoping

Each `Completed` `ExecutionOutcome` now carries its `detonation_id`. The chain reverts exactly the
techniques it completed — a failure never unwinds a detonation from before the chain.

## Tests

A two-technique chain completes; a chain whose second technique fails **auto-reverts** the first
(reverted = 1, no failures); and the same failure under **hold** mode leaves the first in place
(no revert). All run the real engine + ledger + Mock.
