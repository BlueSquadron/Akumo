# 15 — Telemetry & Detection

> Task G13 · FR-I · ADR-0008/0028 · spec §6.1. In `akumo-core::telemetry`.

Every technique carries its **expected telemetry signature** (FR-I1), and Akumo emits **Sigma-aligned**
candidate detections (ADR-0028) so findings interoperate with SIEMs out of the box. v1 emits expected
signatures only; live observed-vs-expected correlation is v2 (ADR-0008, `TelemetryCollector` stub).

## Sigma generation (G13.1/G13.2)

`sigma_rules_for(technique)` produces one `SigmaRule` per `ExpectedTelemetry` entry:

- **title** = technique name + event; **id** = deterministic `akumo-<technique>-<event>` slug.
- **tags** = `attack.tXXXX` from the technique's MITRE mapping.
- **logsource** = `product` (the provider) + `service` (the signature source, e.g. `cloudtrail`).
- **detection** = a `selection` of `eventName` + the signature's declared fields, `condition: selection`.
- **level** derived from impact (`read`→low, `mutating-reversible`→medium, irreversible/destructive→high).

`SigmaRule::to_yaml()` renders standard Sigma YAML. These rules are the **candidate detections**
(FR-I3) surfaced in reports and the catalog; where an author supplies a `detection` string it is kept
alongside. A native superset is added only where Sigma's taxonomy falls short.

## Tests

One rule per signature with correct tags/logsource/level and merged detection fields; YAML rendering
contains `logsource`/`detection`/`eventName`.
