# 18 — Interfaces

> Task G15 · FR-K · ADR-0029 · spec §6.4. Library facade in `akumo-core::api`; CLI + shell in
> `akumo-cli`; binary in `bin/akumo`.

## Library facade (G15.1 — `akumo-core::api::Akumo`)

One entry point that wires the ports and exposes the unified loop as high-level, **stateless**
operations over a persistent engagement: `open_engagement` / `close_engagement` / `kill_switch` /
`record_consent` / `engagement_state`, `authcheck`, `enumerate`, `graph`, `paths`, `preview`, `run`,
`revert`, `report`. It holds no mutable session state — everything lives in the ledger (FR-A6). This
is the intended public surface (FR-K3), versioned with the crate (1.0.0).

## CLI (G15.2 — `akumo-cli`)

Command-per-invocation over the ledger (ADR-0029), so switching engagements is just a flag and there
is no Pacu-style "restart to switch sessions" pitfall. Global flags: `--state-dir`, `--provider`
(v1: `mock`; `aws` reports "not yet available"), `--json`.

Commands: `engagement open|list|show|close`, `authcheck`, `enumerate`, `paths`, `catalog`,
`technique` (metadata + generated Sigma), `preview` (dry-run + blast radius), `run` (detonate, with
`--consent`), `revert`, `report` (`--format markdown|json`), and `shell`.

The binary (`bin/akumo`) is a thin `process::exit(akumo_cli::run())`; `run()` builds a Tokio runtime
and dispatches. Exit codes are stable (`OK` 0, `FAILURE` 1, `USAGE` 2 — FR-K4).

## Machine output (G15.3)

`--json` emits versioned, structured output where supported (e.g. `authcheck`, `report`); `report`
also honors `--format json`. Machine-facing output carries a `version` field so pipelines can depend
on it.

## Interactive shell (G15.4)

`akumo shell` layers the **same command grammar** over the persistent ledger via a
`no_binary_name` clap parser, holding no mutable state of its own — each line is a stateless command,
exactly like a separate invocation. `exit`/`quit`/EOF leave.

## UX & errors (G15.5)

`preview` surfaces the impact and **reversibility** before any commit; `run` on a mutating technique
without consent returns `ConsentRequired` with a hint to re-run with `--consent`. Errors are
actionable (what failed and how to fix), and secrets are never printed.

## v1 provider note

The CLI wires the **Mock** provider so the whole surface is exercisable without a cloud account; the
AWS adapter (G16) slots in behind the same `Provider` seam with no CLI changes.
