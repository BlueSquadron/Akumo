# 01 — Architecture

> Task G1.1 · ADR-0001 (hexagonal) · NFR-EXT1/EXT5. See also `spec/specification.md` §1.

Akumo is a **hexagonal (ports & adapters)** application. A provider-agnostic **domain core** sits
in the centre; everything cloud-specific, storage-specific, or human-facing is an **adapter** that
plugs into a **port**.

```
                       DRIVING ADAPTERS (inbound)
                ┌───────────┬───────────┬────────────┐
                │ Interactive│  Library  │  CI / non- │
                │    CLI     │   / API   │ interactive│
                └─────┬──────┴─────┬─────┴─────┬──────┘
                      │            │           │
                 ┌────▼────────────▼───────────▼────┐
                 │            DOMAIN CORE            │
                 │  (provider-agnostic; pure logic) │
                 │  Engagement · Graph+Facts ·      │
                 │  Planner · Technique Registry ·  │
                 │  Execution Engine · Telemetry ·  │
                 │  Reporting                       │
                 └──┬─────────┬──────────┬──────────┘
              Provider   Persistence  Telemetry     ...ports (traits in akumo-domain::ports)
                Port        Port        Port
            ┌──────▼───┐ ┌───▼─────┐ ┌──▼───────────┐
            │ AWS/Mock │ │ Event   │ │ (v2 collector)│    DRIVEN ADAPTERS (outbound)
            │ adapters │ │ ledger  │ └──────────────┘
            └──────────┘ └─────────┘
```

## Crate map

The Cargo workspace lives under [`code/`](../../code/). Crates and the rule they exist to enforce:

| Crate | Role | Depends on |
|---|---|---|
| `akumo-domain` | Value objects, error type, event envelope, seam stubs, **port traits**. Internal *leaf*. | (external only: serde, thiserror, async-trait) |
| `akumo-core` | Provider-agnostic domain **services** (the unified loop). | `akumo-domain` (+ `akumo-dsl`) |
| `akumo-dsl` | Technique content model: YAML+CEL, validation, Starlark/WASM hosts. | `akumo-domain` |
| `akumo-ledger` | Persistence Port **adapter** (event store). | `akumo-domain` |
| `akumo-provider-mock` | Mock Provider Port **adapter** (first-class). | `akumo-domain` |
| `akumo-provider-aws` | AWS Provider Port **adapter** (AWS SDK confined here). | `akumo-domain` |
| `akumo-content` | Content bundle load/verify/sign. | `akumo-domain`, `akumo-dsl` |
| `akumo-cli` | Driving **adapters**: CLI + shell. | `akumo-core`, `akumo-domain` |
| `bin/akumo` | Thin binary; wires adapters → core. | `akumo-cli` |
| `xtask` | Dev automation (dep-lint, later docgen/leak-detector/bundle). | serde_json |

## Enforced rules

1. **The core is provider-blind.** `akumo-domain` and `akumo-core` MUST NOT depend — transitively,
   via normal deps — on any adapter crate (`akumo-provider-*`, `akumo-ledger`, `akumo-cli`,
   `akumo-content`) or any cloud SDK (`aws-*`, `azure*`, `google-cloud*`). Enforced by
   `cargo run -p xtask -- dep-lint` (task G1.4), a required CI check. `akumo-domain` is additionally
   an internal *leaf* (no sibling `akumo-*` deps).
2. **Dependency inversion.** Core services talk to *ports* (traits in `akumo_domain::ports`), never
   to concrete adapters. The binary is the only place that names concrete adapters.
3. **Mock-first.** The full loop is proven against `akumo-provider-mock` before the AWS adapter
   exists — the mechanism that proves adding a provider is additive (NFR-EXT7).
4. **`#![forbid(unsafe_code)]`** in the core crates (and, via `[lints]`, all first-party crates).

## Why hexagonal is not a real choice here

It is the direct structural expression of already-ratified requirements: NFR-EXT1 → the Provider
Port; NFR-EXT2 → the core depends on ports, not adapters; NFR-EXT3/EXT6 → adapters/content plug in
without touching the core; NFR-EXT7 → the Mock behind the same Provider Port; FR-K → three driving
adapters over one core.
