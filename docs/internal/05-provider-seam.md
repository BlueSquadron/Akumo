# 05 — Provider Seam & the Mock

> Tasks G3.1–G3.3 · ADR-0012 (capability + descriptor) · ADR-0013 (provider-scoped) · ADR-0030
> (mock stub). Descriptor types in `akumo-domain::seam`; ports in `akumo-domain::ports::provider`;
> Mock in `akumo-provider-mock`.

## The seam shape (ADR-0012 = capability + descriptor)

The Provider Port is a **small set of capability traits** (see [03-ports.md](03-ports.md)); provider
knowledge lives in **declarative descriptors** the adapter interprets. This keeps the core
provider-blind while letting techniques stay declarative and host-brokered.

## Descriptors (G3.1 — `akumo-domain::seam`)

- **`EnumerationDescriptor`** `{ service, operation, params, required_permission }` — identifies a
  read call. `key()` = `"service.operation"`.
- **`ActionDescriptor`** `{ service, operation, params, impact, required_permission }` — identifies a
  mutating/read action; carries its `ImpactLevel` (gates consent, FR-A3).

The **response-/result-mapping language** (how a raw response becomes graph assertions/facts) is
designed with the step-DSL (G7/G8); until then an adapter's `GraphMapper` maps in code. `RawResponse`,
`ActionResult`, and `CapabilityGrant` remain thin.

## Host-brokering (G3.3)

`CapabilityGrant` is the narrow set of capabilities the host grants one technique step (NFR-SEC5).
The `ActionExecutor` refuses any action whose `key()` the grant does not `permit`. Content never
holds a cloud client — it can only invoke what it declared and was granted. (The Mock enforces this
today; the execution engine wires per-step grants in G9.)

## The Mock provider (G3.2 — `akumo-provider-mock`)

A behavioral, scriptable stub (ADR-0030), **not** an AWS simulator. Its `MockEnvironment` is authored
directly in attack-core vocabulary via a builder:

```rust
MockEnvironment::builder("mock", foothold_principal)
    .region("mock-region-1")
    .enumeration("iam", "ListPrincipals", vec![/* Assertions */])
    .denied_enumeration("kms", "ListKeys", "access denied: kms:ListKeys")
    .action("iam", "CreateAccessKey", ActionResult { /* ... */ })
    .build();
```

`MockProvider` implements all six capabilities + the aggregate `Provider`:

- `IdentityResolver` → the scripted current principal.
- `ResourceEnumerator` → scripted assertions, or `AccessDenied` for a denied selector (exercises the
  partial-permission path, FR-C6), or an empty result for anything unscripted.
- `GraphMapper` → decodes the assertions the env authored (straight decode, since the Mock speaks the
  canonical vocabulary directly).
- `ActionExecutor` → enforces the capability grant, then returns the scripted result.
- `MetadataProvider` → provider id + regions.
- `TelemetryCollector` → default `Unsupported` (v1).

Its two jobs: the **extensibility gate's "second provider"** (NFR-EXT7 — the full loop must run
against it with zero core changes, proven in G10.2) and the **fast test tier** (OQ-8).

## Tests

`akumo-provider-mock` unit tests: resolve principal; enumerate→map yields assertions; denied
enumeration is an access gap; action refused without a grant and executed with one.
