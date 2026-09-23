# 0 — Getting Started

> **Rung 0 of the Akumo ladder.** By the end you can install Akumo, open an authorized engagement,
> enumerate a target, and read a report — safely, against the built-in Mock provider first.

## ⚠️ Before anything: authorized use only

Akumo is offensive tooling. Only run it against infrastructure you are **authorized to test** — owned
accounts, contracted engagements, or lab/CTF environments. Opening an engagement requires you to
affirm this (`--affirm`), and it is recorded in the audit ledger.

## Install

```bash
# from a release binary (recommended)
#   download the single static binary for your OS, then:
akumo --version

# or build from source
cd code && cargo build --release
./target/release/akumo --version
```

Everything below uses `akumo`. If you built from source, substitute `cargo run -p akumo --`.

## The mental model (30 seconds)

Akumo runs one loop: **authorize → enumerate → build a graph → compute paths → safely execute →
verify → revert → report.** All state lives in an append-only **ledger** on disk, so every command is
a stateless invocation — you select the engagement with a flag, never a long-lived session.

Two providers exist: **`mock`** (a built-in, scriptable practice target — the default) and **`aws`**
(a real AWS account via your standard credentials). **Start with `mock`.**

## Your first engagement (on the Mock)

```bash
# 1. Check who you are and where you can operate (no changes, no enumeration)
akumo authcheck

# 2. Open an authorized engagement (scope is fail-closed: nothing outside it is allowed)
akumo engagement open --id demo --scope account:123456789012 --affirm

# 3. List and inspect engagements
akumo engagement list
akumo engagement show --engagement demo

# 4. Read the (currently empty) report
akumo report --engagement demo
```

Global flags you'll use everywhere: `--state-dir <dir>` (where the ledger lives; default
`./.akumo-state`), `--provider mock|aws`, and `--json` for machine-readable output.

## What just happened

- `engagement open` recorded an `EngagementOpened` event — your scope and authorization affirmation —
  into the ledger. Nothing touched a target.
- Every later command reads and appends to that ledger. You can inspect it directly: the ledger is
  plain JSON-Lines under `--state-dir`.

## You can now…

- [x] Install Akumo and run `authcheck`.
- [x] Open, list, and show an authorized engagement.
- [x] Generate a report.

**Next:** [1 — Running Attacks Safely](01-running-attacks.md) — enumerate, compute attack paths,
preview blast radius, run a reversible technique, and revert it.
