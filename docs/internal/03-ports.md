# 03 — Ports

> Task G1.3 · ADR-0012 (provider seam) · ADR-0014 (persistence) · NFR-EXT1. Traits live in
> `akumo-domain::ports`.

Ports are the stable interfaces the core depends on. Adapters implement them; the core never names
a concrete adapter.

## Provider Port (`ports::provider`)

The one seam to all cloud interaction (ADR-0012 = capability + descriptor hybrid). It is a small set
of **capability traits**; provider-specific knowledge lives in declarative *descriptors* the adapter
ships (see `akumo_domain::seam`). Adding a provider = implement these + ship a descriptor catalog,
with zero core changes.

| Capability | Sync/Async | Responsibility | Serves |
|---|---|---|---|
| `IdentityResolver` | async | Resolve credentials → current principal; low-priv foothold | FR-B3/B4 |
| `ResourceEnumerator` | async | Execute enumeration descriptors → raw responses | FR-C, FR-B5 |
| `ActionExecutor` | async | Execute an action descriptor under a `CapabilityGrant` | FR-G, NFR-SEC5 |
| `GraphMapper` | sync | Map raw responses → graph assertions | FR-D |
| `MetadataProvider` | sync | Region/partition + provider id | FR-B5 |
| `TelemetryCollector` | async | Fetch produced telemetry — **v2**; v1 default = `Unsupported` | FR-I2 |

`Provider` aggregates the six (`identity()`, `enumerator()`, `actions()`, `mapper()`,
`metadata()`, `telemetry()`), so the core holds one `&dyn Provider`. Async traits use `async_trait`
for object safety.

## Persistence Port (`ports::persistence`)

`EventStore` — the append-only, hash-chained, engagement-isolated event store (ADR-0014):

- `append(event)` — durable, ordered append.
- `read_stream(engagement)` — the full ordered stream, to fold projections.
- `last_hash(engagement)` — the current chain head, used as the next event's `prev_hash`.

The storage substrate (embedded KV, IMPL-CHOICE `redb`) is an Increment 7 detail *behind* this
trait.

## Driving ports (`ports::driving`)

The operations the CLI/library/CI invoke on the core. Increment 1 ships only `EngagementApi`
(`open_engagement`, which requires the authorization affirmation, NFR-COMP1); the full set
(`enumerate`, `compute_paths`, `preview`, `execute_step`, `execute_chain`, `revert`, `report`, …)
is added as each capability lands (G5–G15).

## Rule

The domain-core module MUST NOT depend on any adapter module or cloud SDK crate. A dependency-
direction lint (`xtask dep-lint`) is release-blocking — see [01-architecture.md](01-architecture.md)
§Enforced rules.
