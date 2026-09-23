# 02 — Domain Model

> Task G1.2 · glossary mirrors `spec/requirements.md` §4. Types live in `akumo-domain`.

The domain crate holds only provider-neutral value objects — no I/O, no cloud SDK. These are the
vocabulary every other crate shares.

## Value objects

| Type (module) | Meaning | Notes |
|---|---|---|
| `EngagementId`, `ProviderId`, `TechniqueId`, `Region`, `Actor`, `EventHash` (`ids`) | Transparent `String` newtypes | `serde(transparent)`; `Display`; `From<&str>`/`From<String>` |
| `Seq`, `Timestamp` (`ids`) | Monotonic sequence; Unix-epoch-millis time | dependency-free time to keep the leaf crate light |
| `ImpactLevel` (`impact`) | `read` < `mutating-reversible` < `mutating-irreversible` < `destructive` | **Ordering is meaningful** — consent is required "at or above" a level (FR-A3); derives `Ord` from declaration order |
| `EpistemicStatus` / `Epistemic` (`epistemic`) | `Proven` / `Absent` / `Unknown` / `Inferred` (+ optional confidence) | ADR-0017; `Unknown` ≠ `Absent` — the honesty invariant |
| `Scope` / `ScopeSelector` (`scope`) | Authorized boundary as `kind`/`value` pairs | empty scope = authorizes nothing (fail-closed), FR-A1 |
| `Principal` / `PrincipalKind` (`principal`) | Attack-core identity | provider inventory attaches later at the graph layer |
| `AkumoError` / `Result` (`error`) | The single typed error surface | first-class `AccessDenied` (FR-C6) and `OutOfScope` (FR-A1); not `Clone` |
| `EventEnvelope` (`event`) | One ledger record | hash scheme + typed payload finalized in G2.1 |
| Seam stubs (`seam`) | `EnumerationDescriptor`, `ActionDescriptor`, `RawResponse`, `ActionResult`, `Assertion`, `CapabilityGrant` | opaque JSON now; schemas frozen in G3.1/G4 |

## Design notes

- **`ImpactLevel` ordering is load-bearing.** `impact.requires_consent()` is simply `impact > Read`.
  Do not reorder the variants.
- **Epistemics are categorical, not a bare score.** Conflating "denied/unseen" with "absent"
  produces false negatives and dishonest reports; the planner (G12) defines explicit behavior over
  `Unknown` edges.
- **Errors degrade, they don't panic.** `AkumoError::AccessDenied` is a *coverage gap*
  (`is_access_gap()`), letting enumeration continue under partial permissions (FR-C6).
- **Seam stubs are deliberately opaque.** They let the port traits and the Mock adapter be built now
  without prematurely freezing the descriptor/graph schema (that happens in Increment 2).
