# Reference — Technique Schema

The complete YAML schema for a technique. Fields marked **required** are enforced at load; a
technique failing validation is not loaded.

## Top level

```yaml
metadata: { … }     # required
contract: { … }     # optional (defaults to empty)
steps: [ … ]        # optional (a technique may be metadata-only, e.g. a simulated variant)
```

## `metadata` (Tier-0, always declarative)

| Field | Req | Type | Notes |
|---|:--:|---|---|
| `id` | ✅ | string | Unique technique id. |
| `name` | ✅ | string | Human-friendly name. |
| `description` | ✅ | string | What it does. |
| `version` | ✅ | string | Content semver. |
| `provider` | ✅ | string | Provider scope (e.g. `aws`). |
| `mitre` | ✅ | string[] | ≥1 MITRE ATT&CK id (e.g. `T1098`). |
| `impact` | ✅ | enum | `read` · `mutating-reversible` · `mutating-irreversible` · `destructive`. |
| `expected_telemetry` | ✅ | list | ≥1 signature (see below). |
| `author` | | string | Attribution. |
| `owasp` / `cis` | | string[] | Optional cross-maps. |
| `references` | | string[] | Free-form (e.g. AWS Threat Technique Catalog URLs). |
| `simulated_variant` | (req. if irreversible/destructive) | string | Id of a simulated variant (FR-G6). |
| `engine_compat` | | string | Engine/content-model compatibility range. |

### `expected_telemetry` entry

```yaml
- source: cloudtrail          # required — log source
  event_name: CreateAccessKey # required — event/API
  fields: { eventSource: iam.amazonaws.com }   # optional — discriminating fields
  detection: "<sigma ref>"    # optional — author-supplied detection
```

## `contract` (Tier-0)

```yaml
contract:
  inputs:
    - name: user               # required
      type: principal          # string | number | bool | principal | resource
      required: true           # default false
      default: <value>         # optional
  preconditions:
    - predicate: { name: has_permission, args: ["$foothold", "iam:CreateAccessKey"] }
      min_status: proven       # proven (default) | absent | unknown | inferred
      opportunistic: false     # allow inferred/unknown to satisfy it (default false)
  effects:
    - predicate: { name: has_credential, args: ["$user", "new-access-key"] }
```

Predicate `name`s are fact views over the graph (`has_permission`, `can_assume`, `has_credential`,
`can_access`, …). See the [fact vocabulary](fact-vocabulary.md).

## `steps` (Tier-1, with per-step escapes)

```yaml
steps:
  - id: create-key             # required, unique
    condition: "<CEL>"         # optional run-if
    for_each:                  # optional bounded iteration
      items: "<CEL array>"
      var: item
      max: 50
    body:                      # required — one of:
      # Tier-1 host-brokered call:
      type: call
      service: iam
      operation: CreateAccessKey
      params: { UserName: "$user" }
      # — or — Tier-2/3 escape:
      # type: script
      # language: starlark | wasm
      # capabilities: ["compute"]   # required for script steps
      # source: "<code>"
    bind:                      # optional — result → named facts
      - name: key_id
        from: result.AccessKeyId
    revert:                    # optional override; else derived from the descriptor inverse
      service: iam
      operation: DeleteAccessKey
      params: { UserName: "$user", AccessKeyId: "$key_id" }
```

## Validation rules (enforced)

- Mandatory metadata present; `mitre` and `expected_telemetry` non-empty.
- Irreversible/destructive `impact` ⇒ `simulated_variant` required.
- Step ids unique and non-empty.
- `script` steps must declare non-empty `capabilities`.
- Predicate names non-empty.

See also: [CEL surface](cel-surface.md) · [technique-schema authoring guide](../02-first-technique.md).
