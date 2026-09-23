# Contributing a Technique (end to end)

> **Advanced.** The full path from idea to merged, shippable technique — additive, no core changes.

Adding a technique is **additive**: drop a YAML file into the content tree and it's discovered. No
enumeration, graph, planner, or execution code changes (NFR-EXT3/EXT6).

## 1. Pick and classify

- Choose a real, authorized-testing-relevant AWS behavior.
- Map it to **MITRE ATT&CK** (at least one id) and organize the file by **tactic**:
  `content/aws/<tactic>/<name>.yaml`.
- Cross-reference the
  [AWS Threat Technique Catalog](https://aws-samples.github.io/threat-technique-catalog-for-aws/matrix.html)
  in `references`.

## 2. Write the technique

Follow the ladder: [metadata + step](../02-first-technique.md) →
[contract](../03-preconditions-effects.md) → [CEL where needed](../04-cel-expressions.md) →
[chainability](../05-chaining-and-objectives.md) → [revert](../06-revert-contracts.md) →
[telemetry](../09-telemetry-and-detections.md). Mandatory fields: `id`, `name`, `description`,
`version`, `provider`, `mitre`, `impact`, `expected_telemetry`, and — for anything mutating — a
`revert` (or, if irreversible, a `simulated_variant`).

See the full field list in the [technique schema reference](../reference/technique-schema.md) and the
expression surface in the [CEL reference](../reference/cel-surface.md).

## 3. Validate

```bash
akumo catalog --content content
akumo technique --id <your-id> --content content   # shows metadata + generated Sigma
akumo preview --engagement demo --technique <your-id> --input k=v
```

Validation is strict: a missing MITRE mapping or telemetry, a duplicate step id, a script step
without declared capabilities, or an irreversible technique without a simulated variant is
**rejected** with a clear message.

## 4. Test (required)

Every mutating technique ships a **detonate-and-revert test** — no passing revert test, no ship
(NFR-SAF2). Write it against the Mock: script the action + its compensation, `run` → `Completed`,
`revert` → clean. See [Rung 6](../06-revert-contracts.md).

## 5. Provider support

Your step references a provider `service`/`operation`. If the AWS adapter already implements that
call, it runs live; if not, the technique still validates, previews, and runs on the Mock — and
executes live once the adapter grows that operation (a small, isolated addition behind the seam; see
the internal [add-a-provider runbook](../../internal/extending/add-a-provider.md)).

## 6. Document & bundle

- The [auto-generated reference](../reference/techniques/) page for your technique is produced from
  its definition (docs never drift from behavior).
- To distribute outside the default catalog, package it in a signed
  [content bundle](building-a-bundle.md).

## Checklist

- [ ] MITRE mapping + tactic folder + `references`.
- [ ] Contract: inputs, preconditions (with epistemic status), effects.
- [ ] Steps host-brokered; escapes only where justified, with declared capabilities.
- [ ] Revert for every mutating step (or `simulated_variant` if irreversible).
- [ ] Expected telemetry (precise fields).
- [ ] Passing detonate-and-revert test.
