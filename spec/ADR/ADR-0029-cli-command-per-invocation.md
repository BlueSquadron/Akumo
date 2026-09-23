# ADR-0029: CLI model — command-per-invocation + optional shell

- **Status:** Accepted
- **Date:** 2026-09-22
- **Ref:** SPEC-D17 · specification §6.4 · requirements FR-K, FR-A6

## Context
The CLI must be CI-friendly and scriptable yet good for interactive exploration, without
inheriting Pacu's session-statefulness pitfalls (e.g. restart-to-switch-engagements, FR-A6).

## Decision
**Command-per-invocation is the canonical CLI** (`akumo enumerate|paths|run …`), with all state
in the engagement ledger (SPEC-D14) — so invocations are stateless over persistent state and
switching engagements is just a flag. An **optional interactive shell** layers on top for
exploratory work, holding no mutable session state of its own.

## Consequences
- (+) CI-native and scriptable (FR-K2); cleanly solves FR-A6; keeps Pacu's REPL ergonomics as an
  option without its statefulness flaws.
- (−) Two front-ends to build/maintain (shell is thin over the canonical commands).

## Alternatives considered
- **Command-per-invocation only** — viable; omits the exploratory shell.
- **Interactive REPL only** — rejected: awkward for CI; risks Pacu-style session statefulness.
