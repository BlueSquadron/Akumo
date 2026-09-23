# ADR-0014: Event-sourced, hash-chained engagement ledger

- **Status:** Accepted
- **Date:** 2026-09-21
- **Ref:** SPEC-D3 · specification §1.5, §2.1, §2.7 · requirements DSR, FR-J1, NFR-REL/OBS

## Context
Several release-blocking properties converge on the state model: append-only audit (FR-J1),
resumability (NFR-REL2), reproducible replay (FR-J2), revert-after-restart (DSR-4), and
tamper-evidence (NFR-OBS3).

## Decision
The engagement is an **append-only, hash-chained event ledger** and is the single source of
truth; all other state (attack graph, coverage map, execution ledger, loot store) is a
deterministic **projection**. Event envelope carries `prev_hash`/`hash` for tamper-evidence and
`seq`/`engagement_id` for ordering and isolation.

## Consequences
- (+) One mechanism yields audit, resume, replay, revert, tamper-evidence, and isolation.
- (+) Compensations (ADR-0019) live in the ledger → restart-safe revert.
- (−) Event-schema discipline, projection maintenance, and schema evolution to manage.

## Alternatives considered
- **Mutable state + separate audit log** — rejected: two sources of truth can diverge; audit
  completeness not guaranteed; replay/resume/revert become bespoke.
