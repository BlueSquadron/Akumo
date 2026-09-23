# 9 — Telemetry & Detections

> **Rung 9 (advanced).** Every technique you write carries its defensive meaning. You'll author
> expected-telemetry signatures and get Sigma-aligned candidate detections for free.

Akumo's purple-team thesis: **every offensive action emits both its result and its expected detection
signature.** That's the `expected_telemetry` block you've been writing since Rung 2 — here's how to
make it count.

## Authoring expected telemetry

```yaml
expected_telemetry:
  - source: cloudtrail
    event_name: CreateAccessKey
    fields:
      eventSource: iam.amazonaws.com
      # add the fields a detection engineer would key on:
      # e.g. userIdentity.type, requestParameters.userName
```

- **`source`** — the log source (e.g. `cloudtrail`).
- **`event_name`** — the API/event a defender would see.
- **`fields`** — the discriminating fields; the more specific, the better the generated detection.

A technique may declare **multiple** signatures (one per observable event).

## Generated candidate detections (Sigma)

Akumo turns each signature into a **Sigma-aligned** candidate detection:

```bash
akumo technique --id aws.iam.persistence.create-access-key --content content
```

prints the metadata and the generated Sigma rule(s) — `title`, `attack.tXXXX` tags from your MITRE
mapping, a `logsource` (product = provider, service = your `source`), a `detection` selection built
from `event_name` + your `fields`, and a severity derived from impact. These are candidate rules to
hand to the blue team or load into a SIEM — no converter needed.

## In reports

Executed techniques and their MITRE coverage appear in the engagement report
(`akumo report --engagement <id>`), so a run doubles as a **detection-coverage artifact**: what was
done, what it should have tripped, and which ATT&CK techniques it exercised.

## Tips for good signatures

- Prefer fields that are **specific to the malicious use**, not just the API name.
- Keep MITRE mappings accurate — they become the rule's tags and the report's coverage.
- Cross-reference the [AWS Threat Technique Catalog](https://aws-samples.github.io/threat-technique-catalog-for-aws/matrix.html)
  in `references` so defenders can trace the technique.

> v1 emits **expected** signatures. Live observed-vs-expected correlation (Grimoire-style) is a v2
> capability.

## You can now…

- [x] Author precise expected-telemetry signatures.
- [x] Generate Sigma candidate detections and read MITRE coverage in reports.

**You've reached the top of the ladder.** Next, ship your work:
[Building a Content Bundle](advanced/building-a-bundle.md) · [Contributing a Technique](advanced/add-a-technique.md).
