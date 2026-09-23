<!-- Thanks for contributing to Akumo. Keep changes focused; reference the task id (e.g. G4.3) and
relevant requirement/ADR ids where useful. -->

## What & why

<!-- What does this change do, and why? -->

## Checklist

- [ ] `cargo test --workspace` passes.
- [ ] `cargo fmt --all -- --check` and `cargo clippy --workspace --all-targets -- -D warnings` clean.
- [ ] `cargo run -p xtask -- dep-lint` green (the core stays provider-blind).
- [ ] New/changed **mutating techniques** have a passing detonate-and-revert test (NFR-SAF2).
- [ ] Relevant docs updated (`docs/internal/` and/or `docs/external/`).
- [ ] No secrets, credentials, or real loot in the diff.

## Notes for reviewers

<!-- Anything that needs attention: trade-offs, follow-ups, out-of-scope items. -->
