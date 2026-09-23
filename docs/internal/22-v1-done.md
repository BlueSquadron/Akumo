# 22 — v1 Definition of Done

> Task G20.3 · `requirements.md §12`. Akumo v1 is "done" when, against an authorized AWS target, an
> operator can do all of the following — each mapped here to the code and test that proves it.

## The eight capability gates

| # | §12 capability | Proven by |
|---|---|---|
| 1 | Declare scope + authorization and open an **isolated engagement** (FR-A) | `akumo-core::engagement` (`open` records the affirmation; fail-closed scope) · gates: scope refusal, kill-switch · `engagement::tests` |
| 2 | Enumerate principals/permissions/resources/trust into a **provider-agnostic graph** (FR-C/D) | `enumeration` + `graph` (`GraphProjection`) · `enumeration::tests::m2_open_enumerate_and_build_graph` |
| 3 | Compute and read **explainable** attack paths toward an objective (FR-E) | `planner` (hybrid, dual-mode, ranked, `explain()`) · `planner::tests` |
| 4 | Preview blast radius, execute through the safe lifecycle, verify, and **revert** (FR-G) | `execution` (dry-run → consent → detonate → record compensation → verify; saga revert) · `execution::tests` · gate: revert restores |
| 5 | **Chain** techniques along a path with halt-and-revert on failure (FR-H) | `chain` (`ChainExecutor`, halt-and-auto-revert / hold) · `chain::tests` |
| 6 | Per action, its **MITRE mapping + expected telemetry** (FR-I) | `telemetry` (Sigma-aligned generation) · technique metadata · `telemetry::tests` |
| 7 | Produce an **audit-grade, reproducible report** (FR-J) | `report` (ledger-folded, Markdown + JSON, STIX/Attack Flow export) · hash-chained ledger (`akumo-ledger`) · `report::tests` |
| 8 | Do all of the above **through the provider seam**, proven by the same core running the full loop against a **mock provider** with zero core changes (§9 / NFR-EXT) | `api::tests::extensibility_gate_full_loop_on_mock` + `xtask dep-lint` (structural) |

## Release-blocking gates (must be green to ship)

- **Revert correctness** — every mutating technique detonates and reverts (NFR-SAF2): per-technique
  tests + `gates::gate_revert_restores`; live-AWS tier + **leak detector** (G19.2).
- **Extensibility** — `dep-lint` green + the extensibility-gate loop test (NFR-EXT1/EXT7).
- **Safety negatives** — scope refusal, kill-switch halt, consent required, tamper detection
  (`akumo-core::gates`).
- **Content validation** — schema/contract/telemetry/revert checks (`akumo-dsl`).

Run them:

```bash
cd code
cargo test --workspace
cargo run -p xtask -- dep-lint
cargo clippy --workspace --all-targets
# after a live engagement:
cargo run -p xtask -- leak-detector <state-dir>
```

## Known deferrals (v2 backlog)

Encryption-at-rest of secrets (NFR-SEC1), the rate/concurrency governor (NFR-SAF3), provider-assisted
validation (ADR-0022), live telemetry correlation (FR-I2), the Starlark/WASM runtimes behind the
`ScriptHost` seam, and bundle CLI tooling (G17.3) — all tracked in [`../spec/V2_BACKLOG.md`](../../spec/V2_BACKLOG.md)
and documented at their seams. None blocks the §12 capability set.
