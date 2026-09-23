# Architecture Decision Records (ADR)

Durable, one-decision-per-file log of the architecture decisions for **Akumo**, in the
Michael Nygard format. A record is immutable once **Accepted**; a later change of course gets a
new ADR that *supersedes* the old one (never edit an accepted decision's substance).

Full reasoning and trade-off tables live in [`../specification.md`](../specification.md) and
[`../requirements.md`](../requirements.md); these ADRs are the concise index of *what* was
decided and *why*.

| ADR | Decision | Ref |
|-----|----------|-----|
| [0001](ADR-0001-hexagonal-architecture.md) | Hexagonal (ports & adapters) architecture | Spec §1.1 |
| [0002](ADR-0002-aws-only-v1.md) | v1 targets AWS only; provider-neutral core | Req §2 |
| [0003](ADR-0003-rust-single-binary.md) | Rust core, single static binary | Req §13.1 |
| [0004](ADR-0004-hybrid-content-model.md) | Hybrid technique content model | Req §13.1 |
| [0005](ADR-0005-step-dsl-primary.md) | Declarative fact-based step-DSL primary; Starlark/WASM escape hatches | OQ-7 |
| [0006](ADR-0006-embedded-graph-backend.md) | Embedded-first graph backend | OQ-6 |
| [0007](ADR-0007-deterministic-planning-v1.md) | v1 deterministic path-finding only; AI planner deferred | Req §13.1 |
| [0008](ADR-0008-expected-telemetry-v1.md) | v1 expected-telemetry signatures only; live correlation deferred | Req §13.1 |
| [0009](ADR-0009-revert-verification-tiering.md) | Revert verification via hybrid CI tiering | OQ-8 |
| [0010](ADR-0010-reproducibility-two-tier.md) | Reproducibility: audit-grade replay + path re-verification | OQ-9 |
| [0011](ADR-0011-path-ranking-composite.md) | Default path ranking: composite + presets | OQ-10 |
| [0012](ADR-0012-provider-seam-capability-descriptor.md) | Provider seam: capability + descriptor hybrid | SPEC-D1 |
| [0013](ADR-0013-provider-scoped-techniques.md) | Provider-scoped techniques + shared helpers | SPEC-D2 |
| [0014](ADR-0014-event-sourced-ledger.md) | Event-sourced, hash-chained engagement ledger | SPEC-D3 |
| [0015](ADR-0015-layered-graph-ontology.md) | Layered attack-graph ontology | SPEC-D4 |
| [0016](ADR-0016-facts-as-graph-views.md) | Facts as derived views over the property graph | SPEC-D5 |
| [0017](ADR-0017-explicit-epistemic-status.md) | Explicit epistemic status on assertions | SPEC-D6 |
| [0018](ADR-0018-authoring-yaml-cel.md) | Authoring format: YAML + CEL | SPEC-D7 |
| [0019](ADR-0019-revert-saga-compensations.md) | Revert: saga / recorded compensations | SPEC-D8 |
| [0020](ADR-0020-per-step-escape-declarative-contract.md) | Per-step escape hatches + always-declarative contract | SPEC-D9 |
| [0021](ADR-0021-no-warmup-compensated-setup.md) | No warm-up phase; setup = compensated steps | SPEC-D10 |
| [0022](ADR-0022-pre-exec-analysis-tiered.md) | Pre-execution analysis: static default + opt-in provider-assisted | SPEC-D11 |
| [0023](ADR-0023-chain-failure-policy.md) | Chain failure: policy-driven safe default | SPEC-D12 |
| [0024](ADR-0024-planning-hybrid.md) | Planning: hybrid reachability + technique-as-action | SPEC-D13 |
| [0025](ADR-0025-unknown-edges-dual-mode.md) | UNKNOWN edges: dual-mode honest default | SPEC-D14 |
| [0026](ADR-0026-search-tiered.md) | Search: tiered bounded + opt-in exhaustive | SPEC-D15 |
| [0027](ADR-0027-v1-objective-types.md) | v1 objective types: reach-admin + reach-resource | OQ-3 |
| [0028](ADR-0028-sigma-aligned-signatures.md) | Detection signatures: Sigma-aligned | SPEC-D16 |
| [0029](ADR-0029-cli-command-per-invocation.md) | CLI: command-per-invocation + optional shell | SPEC-D17 |
| [0030](ADR-0030-mock-provider-stub.md) | Mock-provider fidelity: behavioral/scriptable stub | SPEC-D18 |
| [0031](ADR-0031-content-signed-bundle.md) | Content distribution: versioned, signed bundle | SPEC-D19 |

**Status legend:** Proposed · Accepted · Superseded · Deprecated

> v2 / future evolutions are tracked as GitHub issues (label `v2`), not as ADRs — an ADR
> records a decision *taken*, not a deferred intention.
