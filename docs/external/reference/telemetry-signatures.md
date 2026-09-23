# Reference — Telemetry Signatures

Every technique declares `expected_telemetry` (Rung 9). Each entry is one observable event the action
should produce; Akumo turns each into a **Sigma-aligned** candidate detection.

## Entry shape

```yaml
expected_telemetry:
  - source: cloudtrail            # required — log source
    event_name: CreateAccessKey   # required — the event/API a defender sees
    fields:                       # optional — discriminating fields
      eventSource: iam.amazonaws.com
      # e.g. userIdentity.type, requestParameters.userName
    detection: "<sigma ref>"      # optional — author-supplied detection
```

## Generated Sigma rule

`akumo technique --id <id>` renders, per signature:

| Sigma field | From |
|---|---|
| `title` | technique name + `event_name` |
| `id` | deterministic `akumo-<technique>-<event>` slug |
| `tags` | `attack.tXXXX` from the technique's `mitre` |
| `logsource` | `product` = provider, `service` = `source` |
| `detection.selection` | `eventName` + your `fields` |
| `detection.condition` | `selection` |
| `level` | from impact (`read`→low, `mutating-reversible`→medium, else high) |

## Tips

- Put **discriminating** fields (not just the API name) in `fields` for higher-fidelity detections.
- Keep `mitre` accurate — it becomes the rule tags and the report's MITRE coverage.
- Declare **multiple** signatures if the action produces multiple observable events.

> v1 emits **expected** signatures. Live observed-vs-expected correlation is a v2 capability.
