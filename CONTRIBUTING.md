# Contributing to Akumo

Thanks for helping build Akumo. This guide is the short version; the authoritative build order is
[`spec/tasks.md`](spec/tasks.md) and the design rationale is under [`spec/`](spec/).

## Ground rules

- **Authorized use only.** Akumo is offensive tooling for owned/contracted/lab targets. Do not
  contribute capabilities whose *only* purpose is malicious evasion (see [`SECURITY.md`](SECURITY.md)
  and `spec/requirements.md` NFR-COMP4).
- **The ADRs are law.** Decisions in [`spec/ADR/`](spec/ADR/) are settled. If code and an accepted
  ADR disagree, the ADR wins. A change of course needs a *new* ADR that supersedes the old one —
  never edit an accepted decision's substance.
- **Safety is release-blocking.** Every mutating technique ships a *passing revert test*
  (NFR-SAF2). No passing revert test ⇒ it does not ship.
- **The core is provider-blind.** `akumo-domain` and `akumo-core` must not depend on any adapter
  crate or any cloud SDK. This is enforced by `xtask dep-lint` in CI (ADR-0001 / NFR-EXT1).

## Building & testing (from `code/`)

```bash
cd code
cargo build --workspace
cargo test  --workspace          # fast tier: runs against the Mock provider, no cloud needed
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo run -p xtask -- dep-lint   # release-blocking dependency-direction check
```

The **live-AWS** fidelity tier is release-gated and runs separately (see `spec/tasks.md` G19.2).

## Adding a technique

Techniques are declarative YAML (+ CEL). You do not need to touch core code. See the progressive
guide in [`docs/external/`](docs/external/) — start at `02-first-technique.md`. Every technique
needs metadata (MITRE, impact, expected telemetry), a precondition/effect contract, steps, a revert
contract, and a passing detonate-and-revert test against the Mock.

## Commit & PR

- Small, focused commits; reference the task id (e.g. `G4.3`) and requirement/ADR ids where useful.
- PRs must pass the fast CI tier (fmt, clippy, build, test, `dep-lint`, `cargo-deny`).
- Update the relevant `docs/internal/` page — a task is not done without its **Docs** deliverable.
