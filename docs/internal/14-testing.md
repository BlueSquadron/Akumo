# 14 — Testing

> Tasks G1.5, G19 · ADR-0009 (hybrid CI tiering) · ADR-0030 (mock fidelity).

Akumo uses a **two-tier** test strategy. The division of labor: **mock = logic fidelity;
live-AWS = semantic fidelity.**

```
        ▲  release-gated: LIVE-AWS integration (real detonate+revert; fidelity backstop)
       ╱ ╲     + orphaned-resource LEAK DETECTOR after every run          (G19.2)
      ╱   ╲ per-PR: MOCK-provider tests (full loop: enumerate→plan→execute→revert)  (G10)
     ╱─────╲ unit tests (core services, CEL eval, saga/ledger, planner) vs the mock (all groups)
```

## Fast tier — every PR (`.github/workflows/ci.yml`)

Deterministic, cloud-free, contributor-runnable:

```bash
cd code
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --all-features   # -D warnings
cargo build --workspace --all-targets
cargo test  --workspace
cargo run -p xtask -- dep-lint                            # release-blocking (NFR-EXT1)
cargo deny check                                          # licenses / advisories / bans
```

Covers technique *logic* and revert *logic* against the Mock provider. Target runtime: a few
minutes.

## Fidelity tier — release-gated (`.github/workflows/ci-live-aws.yml`)

Real detonate + revert against a live, ephemeral AWS account; the source of truth for AWS semantics;
a mandatory orphaned-resource **leak detector** runs after every live run. Currently a stub —
enabled once the AWS adapter (G16) and the ephemeral account exist (G19.2).

## Release-blocking gates (G19.3)

Consolidated in `akumo-core::gates` (each proven to **fail when violated**, not just to pass), plus
the per-module tests they gather:

| Gate | Requirement | Where |
|---|---|---|
| Revert correctness | NFR-SAF2, FR-F4 | `gates::gate_revert_restores`, `execution`/`chain` tests |
| Scope refusal | FR-A1, NFR-COMP2 | `gates::gate_scope_refuses_out_of_scope` |
| Kill-switch halts | FR-A4 | `gates::gate_kill_switch_halts_execution` |
| Consent required | FR-A3 | `gates::gate_consent_required_before_mutation` |
| Tamper-evidence | NFR-OBS3 | `gates::gate_tamper_evidence` |
| Extensibility gate | NFR-EXT7 | `api::tests::extensibility_gate_full_loop_on_mock` |
| Dependency direction | NFR-EXT1 | `xtask dep-lint` (CI) |
| Content validation | FR-F2 | `akumo-dsl` validate tests |

**Deferred:** the rate/concurrency-cap gate (NFR-SAF3) lands with the execution governor (G9.7),
which is a documented follow-up.

## Leak detector (G19.2)

`xtask leak-detector [state_dir]` scans engagement ledgers for steps that **detonated but were never
reverted** (mutations still standing) and exits non-zero if any are found — the *local* proxy. The
release-gated live-AWS workflow (`ci-live-aws.yml`) runs it after the real detonate+revert suite, in
addition to scanning the AWS account itself. It runs only when `vars.AKUMO_LIVE_AWS == 'true'` with
the gated `live-aws` environment and OIDC-assumed, short-lived credentials — never on PRs.

## Conventions

- Unit tests live beside the code (`#[cfg(test)] mod tests`); cross-cutting/e2e/scenario tests live
  in the repo-root [`tests/`](../../tests/).
- No network in the fast tier. The Mock is deterministic and in-memory.
