# 6 — Revert Contracts

> **Rung 6.** You'll write correct compensations, handle irreversible actions, and write the revert
> test every mutating technique must pass.

Revert is Akumo's safety linchpin. The model is a **saga of recorded compensations** (not a
best-effort "undo"): when a step detonates, the engine records a concrete compensating action into
the ledger *before* it even verifies the step. Revert then replays those compensations **in reverse
order**, undoing **only what actually happened** — and because compensations live in the ledger, it
survives a crash/restart.

## Writing a compensation

For each mutating step, provide a `revert` that inverts it. It can reference facts bound from the
step's result:

```yaml
steps:
  - id: attach
    body:
      type: call
      service: iam
      operation: AttachUserPolicy
      params: { UserName: "$user", PolicyArn: "$policy_arn" }
    revert:
      service: iam
      operation: DetachUserPolicy
      params: { UserName: "$user", PolicyArn: "$policy_arn" }
```

Rules of thumb:

- Use the operation's **natural inverse** (`Create↔Delete`, `Attach↔Detach`, `Start↔Stop`).
- Reference **bound results** for ids the inverse needs (e.g. `$key_id` from a `CreateAccessKey`).
- A **read-only** step needs no revert.

## Irreversible & destructive actions

If an action has no clean inverse, don't fake one. Set the impact accordingly and provide a
**simulated variant** instead:

```yaml
metadata:
  impact: mutating-irreversible          # or: destructive
  simulated_variant: aws.s3.impact.delete-bucket.simulated
```

Validation **rejects** an irreversible/destructive technique that lacks a `simulated_variant`
(FR-G6). The engine refuses to run an irreversible step unless explicitly consented and prefers the
simulated variant. Prefer building the *simulated* technique first.

## The revert test (required)

Every mutating technique ships an automated test that **detonates and asserts a clean revert** — no
passing revert test, no shipping (NFR-SAF2, release-blocking). Against the Mock:

1. script the action and its compensation in a `MockEnvironment`;
2. open an engagement, record consent, `run` the technique → assert `Completed`;
3. `revert` → assert `reverted == <steps>` and `failed` is empty;
4. (bonus) assert the target state matches pre-detonation.

The mock tier proves the *logic*; the release-gated live-AWS tier proves the *semantics* and runs an
orphaned-resource leak detector after every run.

## You can now…

- [x] Write compensations that reference bound results.
- [x] Correctly classify and gate irreversible/destructive actions with a simulated variant.
- [x] Write a passing detonate-and-revert test.

**Next:** [7 — Starlark Escape Hatch](07-starlark-escape.md) for logic the DSL can't express.
