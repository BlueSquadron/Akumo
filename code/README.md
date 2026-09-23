# Akumo — Code

The Rust implementation. This directory is the **Cargo workspace root** (its `Cargo.toml` is
added by task **G1.1**). Build it top-to-bottom following [`../spec/tasks.md`](../spec/tasks.md).

**Layout** (created as their groups are implemented; the dependency-direction lint of G1.4 depends
on these boundaries):

```
code/
├─ Cargo.toml                      # workspace manifest
├─ rust-toolchain.toml             # pinned toolchain
├─ deny.toml                       # cargo-deny config
├─ crates/
│  ├─ akumo-domain/                # pure types + PORT TRAITS + errors. Zero I/O, zero SDK.
│  ├─ akumo-core/                  # domain SERVICES over ports. Depends only on akumo-domain.
│  ├─ akumo-dsl/                   # technique schema + YAML + CEL + Starlark/WASM host.
│  ├─ akumo-ledger/                # Persistence Port impl (event-sourced, hash-chained store).
│  ├─ akumo-provider-mock/         # Mock Provider Port adapter (first-class, always shipped).
│  ├─ akumo-provider-aws/          # AWS Provider Port adapter (aws-sdk-* lives ONLY here).
│  ├─ akumo-content/               # content-bundle load/verify/sign.
│  └─ akumo-cli/                   # CLI + optional shell driving adapters.
├─ bin/
│  └─ akumo/                       # thin binary entrypoint wiring adapters → core.
├─ content/                        # default technique catalog (YAML), by provider/tactic.
│  └─ aws/…
└─ xtask/                          # dev automation: dep-lint, docgen, leak-detector, bundle-sign.
```

**Hard rule (ADR-0001 / NFR-EXT1, CI-enforced):** `akumo-domain` and `akumo-core` MUST NOT depend
on any adapter crate or any cloud SDK crate. The core is proven end-to-end against the **Mock**
provider before the AWS adapter exists.

Cross-cutting, language-agnostic and end-to-end tests live one level up in
[`../tests/`](../tests/); per-crate unit and integration tests live inside each crate as usual.
