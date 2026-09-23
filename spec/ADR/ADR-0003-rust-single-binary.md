# ADR-0003: Rust core, single static binary

- **Status:** Accepted
- **Date:** 2026-09-21
- **Ref:** requirements.md §13.1 · Analysis.md §"language deep dive"

## Context
Akumo mutates real cloud environments and runs semi-trusted content, so core safety matters;
low-friction distribution (a lesson from Pacu's Python install friction vs. Stratus's single
binary) matters; and it must host a content sandbox well.

## Decision
Implement the **core and adapters in Rust**, distributed as a **single static binary**.
Technique authors do not write core Rust — they use the content model (ADR-0004/0005).

## Consequences
- (+) Memory/type-safe core for a safety-critical tool; single-binary distribution (NFR-PORT2);
  best-in-class WASM sandbox host (`wasmtime`).
- (+) v1 AWS-Rust SDK is fully mature.
- (−) Core authoring is harder for AI first-pass than Go/Python (mitigated: compiler as oracle;
  content is not authored in Rust).
- (−) Known v2 cost: Azure/identity-plane adapters lean on REST + auth crates rather than fat
  SDKs (isolated behind the seam).

## Alternatives considered
- **Go** — strongest multi-cloud SDKs and AI-authorability, but weaker core safety and a less
  mature sandbox host. **Python** — best ecosystem/authorability but reproduces Pacu's
  distribution/safety weaknesses. **TypeScript** — top authorability + SDKs but weaker runtime
  safety/single-binary maturity. (User delegated; Rust chosen.)
