# 10 — Safe Execution Engine

> Task G9 · FR-G · ADR-0019/0021/0022/0023 · spec §4. In `akumo-core::execution`.
> Design bias: **least blast radius, fail safe, reversible-by-default** (NFR-SAF1/SAF4).

The engine executes a technique's steps within an active engagement, recording the full lifecycle to
the ledger, and auto-reverts completed steps on failure.

## Per-step lifecycle (G9.1, spec §4.1)

```
PLANNED
  │  ANALYZED — static dry-run + blast radius (no mutation)   → DryRunPerformed, BlastRadiusEstimated
  ▼
CONSENTED — impact-gated consent (FR-A3)
  │  detonate via ActionExecutor (host-brokered, capability-granted)
  ▼
DETONATED — compensation recorded to the ledger IMMEDIATELY   → StepDetonated (with compensation)
  │  verify (success-based in v1)
  ▼
VERIFIED                                                       → StepVerified
  ┊
(any failure after detonation) → ExecutionFailed → auto-revert completed steps → StepReverted
```

Every transition is a ledger event, so execution is idempotent and resumable (FR-G5), and revert
survives restarts (DSR-4).

## No warm-up (G9.2 / ADR-0021)

There is no warm-up phase: prerequisite creation is just ordinary compensated steps, so there is
**one** teardown mechanism (saga revert).

## Dry-run & blast radius (G9.3 / ADR-0022)

`ExecutionMode::DryRun` computes the blast radius **statically** — the provider calls
(`service.operation`) the technique would make, its declared effects, and its max impact — with
**zero** provider calls (FR-G2). Opt-in provider-assisted validation (AWS `DryRun` / IAM policy
simulator) is a noted follow-up (G9.4).

## Detonation, compensation & verification (G9.5)

For each step (honoring a CEL `condition`):

- **Call** steps resolve params via the expression engine, are granted **only** their own
  `service.operation` capability (NFR-SEC5), and execute through `ActionExecutor`.
- Results are bound into the environment; the **compensation is resolved and recorded to the ledger
  before verification** — so a step that mutated but failed verification is still revertible
  (fail-safe, NFR-SAF4).
- **Script** steps run via the `ScriptHost` (Tier-2/3; unsupported by default).

Consent is gated at the technique's impact level; without sufficient consent the outcome is
`ConsentRequired` and nothing is detonated.

## Revert engine (G9.6 / ADR-0019)

`revert` reads the recorded `StepDetonated` compensations from the ledger and replays those not yet
reverted **in reverse order**, so it is restart-safe and undoes **only what happened**. A detonation
id scopes an auto-revert to the current run (so a failure never unwinds a prior technique). A
compensation that itself fails is surfaced in the `RevertReport.failed` list (`FailedRevertIncomplete`),
never silently dropped (NFR-SAF4).

## Deferred in this increment

Provider-assisted validation (G9.4), rate/concurrency/footprint controls (G9.7), `for_each` step
iteration, a CEL `verify` predicate, and auto-asserting technique **effects** into the graph (the
effect→assertion mapping lands with the G7↔G12 bridge). Kill-switch-triggered revert reuses this
engine's `revert` (wired in the CLI/orchestration layer).

## Tests

Detonate→revert roundtrip (create-key → DeleteAccessKey compensation, idempotent second revert);
dry-run performs no detonation and reports the blast radius; a mutating technique without consent is
refused; and a two-step technique whose second step fails **auto-reverts** the first.

> **M4 reached:** a technique runs through dry-run → consent → detonate → verify → revert on the
> Mock, provably reversible.
