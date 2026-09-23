# External Documentation — Writing Scenarios & Attacks

A **progressive curriculum**: start at rung 0 and climb. No rung assumes concepts from a later
one; each ends with a "You can now…" checklist and a link to the next. Populated by group **G18**
of [`../../spec/tasks.md`](../../spec/tasks.md).

### Beginner
- `00-getting-started.md` — install, authorize an engagement, first enumeration, read a report
- `01-running-attacks.md` — objectives, paths, preview blast radius, run a reversible technique, revert

### Intermediate
- `02-first-technique.md` — author a minimal metadata + single-step technique
- `03-preconditions-effects.md` — the fact model; how your contract feeds the planner
- `04-cel-expressions.md` — CEL predicates, conditions, iteration, templating
- `05-chaining-and-objectives.md` — how techniques chain via effects; ranking presets/modes

### Advanced
- `06-revert-contracts.md` — saga compensations; writing a passing revert test
- `07-starlark-escape.md` — Tier-2 per-step scripting
- `08-wasm-escape.md` — Tier-3 heavy computation
- `09-telemetry-and-detections.md` — Sigma-aligned expected telemetry & candidate detections
- `advanced/building-a-bundle.md` — package, version, and sign a content bundle
- `advanced/add-a-technique.md` — the full contributor path, end to end

### Reference (auto-generated where possible)
- `reference/technique-schema.md`, `reference/cel-surface.md`, `reference/descriptors.md`,
  `reference/fact-vocabulary.md`, `reference/telemetry-signatures.md`
- `reference/techniques/` — one auto-generated page per shipped technique (G18.7)
