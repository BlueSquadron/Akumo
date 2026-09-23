# 16 — Reporting, Evidence & Export

> Task G14 · FR-J · ADR-0010 · spec §6.2–6.3. In `akumo-core::report`.

Reports are rendered from **ledger projections** (SPEC-D14) — never a separate narrative that could
drift from what happened.

## Report generation (G14.1)

`build_report(events, catalog)` folds an engagement's event stream into an `EngagementReport`:

- Engagement status/provider (via the `EngagementView` projection).
- `facts_asserted` and `coverage_gaps` (blind spots) from enumeration events.
- `computed_paths` from `PathComputed` events (`PathSummary` payloads).
- `techniques_used` from `DryRunPerformed` events; `steps_detonated`/`steps_reverted`/`failures`
  from execution events.
- `mitre_coverage` resolved from the exercised techniques against a supplied `Catalog`.

Rendering: `to_json()` (machine-readable, FR-J4) and `to_markdown()` with an **executive summary** and
a **technical detail** view (FR-J5) — computed paths (with explanations), techniques exercised, and
blind spots.

## Redaction & reproducibility (G14.2/G14.4)

Redaction is inherent (FR-J6): the report model holds no raw secrets; loot is represented as redacted
references, and the projections never carry secret material. The Markdown ends with an explicit
redaction note. Reproducibility (FR-J2, ADR-0010) is the ledger replay itself (see `projection`);
encryption of stored secrets (NFR-SEC1) is a packaging-phase concern.

## STIX / Attack Flow export (G14.3 / FR-J7)

`attack_flow(objective, path)` exports a computed path as a STIX 2.1 / MITRE **Attack Flow** bundle:
an `attack-flow` object plus one `attack-action` per hop (carrying the technique id where applicable),
linked by `effect_refs`. Ids are deterministic slugs in v1 rather than random UUIDs. `PathSummary`
(shared with the `PathComputed` event payload) is the report-side view of a planner path.

## Tests

A folded report over a real ledger (open → detonate) resolves status/provider/techniques/MITRE and
renders valid Markdown + JSON; an Attack Flow export yields a `bundle` with an `attack-flow` object
and per-hop actions.
