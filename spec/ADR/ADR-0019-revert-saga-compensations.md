# ADR-0019: Revert — saga / recorded compensations

- **Status:** Accepted
- **Date:** 2026-09-22
- **Ref:** SPEC-D8 · specification §3.6, §4.6 · requirements NFR-SAF2 (release-blocking), FR-G4

## Context
Revert correctness is the #1 safety property. Explicit revert blocks put correctness on the
author; auto-derived inverses are brittle where actions have no clean inverse.

## Decision
Use the **saga / compensation** pattern: each mutating step, when detonated, records a concrete
**compensation** into the event ledger (derived from the descriptor's declared inverse by
default; overridable per step). Revert replays compensations in **reverse order**. Compensation
is recorded *before* verification so a step that mutated but failed verification is still
revertible.

## Consequences
- (+) Undoes only what actually happened; restart-safe (compensations are events, DSR-4); powers
  auto-revert-on-failure (FR-G4).
- (−) Irreversible actions must declare `mutating-irreversible` + provide a simulated variant (FR-G6).

## Alternatives considered
- **Auto-derived inverse** — rejected: brittle for actions without clean inverses.
- **Explicit revert block** — rejected: correctness/partial-failure handling rests on the author.
