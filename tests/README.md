# Akumo — Tests

Cross-cutting, end-to-end, and scenario tests that sit **above** any single crate. Per-crate unit
and integration tests live inside their crates under [`../code/`](../code/); this directory is for:

- **End-to-end loop tests** — the full `open → enumerate → plan → execute → verify → revert →
  report` loop against the **Mock** provider (the extensibility gate, G10.2) and, release-gated,
  against a live ephemeral AWS account (G19.2).
- **CLI transcript tests** — asserting the documented commands in `docs/external/` behave as shown.
- **Safety & release-gate tests** — revert correctness, scope refusal, kill-switch, rate caps,
  dependency-direction lint, content validation (G19.3).
- **`fixtures/`** — synthetic Mock environments, sample techniques, and scenario definitions used
  by the tests above.

See [`../spec/tasks.md`](../spec/tasks.md) groups **G10**, **G19**, and **G20.3** for what each
tier must prove. Nothing ships without its tests: every mutating technique needs a passing
detonate-and-revert test (NFR-SAF2, release-blocking).
