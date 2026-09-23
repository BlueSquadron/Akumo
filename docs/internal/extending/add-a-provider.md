# Extending — Add a Provider

> Task G18.8 · NFR-MNT4 · EXR-3/EXR-5. The repeatable procedure for adding a cloud/identity provider
> behind the seam, with **zero core changes**.

Adding a provider is additive: implement the Provider Port's capabilities in a new adapter crate and
(optionally) contribute provider-scoped techniques. The core (`akumo-domain`, `akumo-core`) does not
change — enforced by `xtask dep-lint`.

## 1. Create the adapter crate

`code/crates/akumo-provider-<name>/`, depending on `akumo-domain` (+ that cloud's SDK). The SDK stays
**confined to this crate** — the dep-lint fails the build if it leaks into the core.

## 2. Implement the six capabilities

From `akumo_domain::ports::provider` (see [03-ports.md](../03-ports.md) and the AWS adapter,
[19-aws-adapter.md](../19-aws-adapter.md), as a worked example):

| Capability | Do |
|---|---|
| `IdentityResolver` | resolve supplied credentials → current `Principal` (support a low-priv foothold). |
| `ResourceEnumerator` | map enumeration descriptors (`service.operation`) to API calls → raw response. |
| `GraphMapper` | map raw responses → attack-core `Assertion`s (keep this **pure** so it's unit-testable offline). |
| `ActionExecutor` | execute action descriptors under the per-step `CapabilityGrant` (host-brokering). |
| `MetadataProvider` | provider id + regions/partitions. |
| `TelemetryCollector` | v1: the default `Unsupported` impl. |

Then implement the aggregate `Provider` returning `self` for each. Classify auth/permission errors as
`AkumoError::AccessDenied` so enumeration degrades gracefully (FR-C6).

## 3. Wire it into the driving layer

The CLI selects a provider into `Box<dyn Provider>` (see `akumo-cli`); add your `--provider <name>`
arm there. This is a CLI/wiring change, not a core change.

## 4. Contribute provider-scoped techniques

Techniques are provider-scoped (`provider: <name>`) — see the external
[add-a-technique](../../external/advanced/add-a-technique.md) guide.

## 5. Prove "zero core changes"

- `cargo run -p xtask -- dep-lint` stays green (the SDK didn't reach the core).
- The extensibility-gate test (the full loop on the Mock) is unchanged; your provider runs the *same*
  loop.
- Review confirms the diff touches only your adapter crate (+ content/CLI wiring), not the named core
  components.

## Checklist

- [ ] New adapter crate; cloud SDK confined to it.
- [ ] Six capabilities implemented; `GraphMapper` pure + offline-tested.
- [ ] Errors classified (`AccessDenied` for auth failures).
- [ ] CLI `--provider` arm added.
- [ ] `dep-lint` green; core diff = zero.
