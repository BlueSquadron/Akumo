# ADR-0009: Revert verification via hybrid CI tiering

- **Status:** Accepted
- **Date:** 2026-09-21
- **Ref:** OQ-8 · requirements FR-F4, NFR-SAF2 (release-blocking)

## Context
Every mutating technique must ship a *tested* revert (release-blocking). A mock-only test gives
false confidence (can't replicate IAM eval, eventual consistency); a real-AWS-only test is slow,
costly, and contributor-hostile.

## Decision
Adopt **hybrid tiering**: a fast **mock-provider** tier on every PR (covers all technique logic +
the full loop) **plus** a **release-gated integration suite** that detonates & reverts against a
live ephemeral AWS account (fidelity backstop). A mandatory **orphaned-resource leak detector**
runs after integration.

## Consequences
- (+) Fast, contributor-friendly inner loop with a real-fidelity backstop; satisfies NFR-SAF2.
- (−) Requires AWS sandbox/vending + teardown safety net and some spend.

## Alternatives considered
- **Mock-only** — rejected: false confidence on the #1 safety property.
- **Real-AWS-only** — rejected: slow, flaky, costly, contributor-hostile.
