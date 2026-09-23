# 09 — Technique Content Model

> Task G7 (schema/loader/validation) · FR-F · ADR-0004/0005/0018 · spec §3. In `akumo-dsl`.
> CEL evaluation and the Starlark/WASM hosts are G8; execution through the safe lifecycle is G9.

A technique is one versioned YAML unit with four parts (spec §3.1). The first two are **always
declarative Tier-0** (the planner and validator read them, never executing code); the last two are
where the tiers apply.

```
technique
├─ metadata   (Tier-0)  id · name · description · version · provider · mitre[] · impact
│                        · expected_telemetry[] · simulated_variant?
├─ contract   (Tier-0)  inputs[] · preconditions[] (predicate + min epistemic status)
│                        · effects[]           ← the PDDL-shaped action model (FR-F6)
├─ steps      (Tier-1)  body{call | script} · condition (CEL) · for_each · bind[] · revert?
└─ (revert)             per-step compensation override; else derived from the descriptor inverse
```

## Schema (G7.1/G7.2/G7.4 — `schema.rs`)

- **`Metadata`** — the mandatory declarative fields (FR-F2), including `mitre`, `impact`, and
  `expected_telemetry`. `simulated_variant` is required for irreversible/destructive impact (FR-G6).
- **`Contract`** — `inputs`, `preconditions` (a `FactPredicate` + required `EpistemicStatus`, with an
  `opportunistic` flag for optimistic planning), and `effects`. The contract is **structured**
  predicates (SPEC-D5), not free CEL, so the planner can reason over it without understanding CEL —
  this is the single declaration that feeds both execution and the planner (FR-F6).
- **`Step`** — a `StepBody` that is either a Tier-1 `Call` (host-brokered `service`/`operation` +
  params) or a Tier-2/3 `Script` (`starlark`/`wasm` with declared `capabilities`). Plus a CEL
  `condition`, bounded `for_each`, result `bind`ings, and an optional `revert` override.
- Expression fields (`condition`, `for_each.items`, templated params, `bind.from`) hold **CEL source
  as strings** here; they are parsed/evaluated in G8.

## Validation (G7.5 — `validate.rs`)

Structural and declarative (never runs a body). Rejects: missing id/name/description/version/
provider; **no MITRE mapping**; **no expected telemetry** (FR-I1); irreversible/destructive impact
without a `simulated_variant` (FR-G6); duplicate/empty step ids; a **script step without declared
capabilities** (NFR-SEC5); empty predicate names. A technique failing validation is not loaded (the
loading layer records `TechniqueValidated` for those that pass).

## Loader & catalog (G7.3 — `catalog.rs`)

- `parse_technique(yaml)` parses + validates one technique.
- `Catalog::load_dir(dir)` recursively loads every `.yaml`/`.yml` (deterministic order), validating
  each; adding a technique is additive — no core changes (NFR-EXT3/EXT6).
- Queryable by id, provider, MITRE id, and impact (FR-F5) — the last will let the planner propose
  relevant techniques for a discovered path (the G7↔G12 bridge, next).

## Expression engine (G8.1/G8.2 — `expr.rs`)

The Tier-1 expression language (ADR-0018 / SPEC-D7). **Self-contained by design** — no external
interpreter — for full sandbox control in a safety-critical tool; the surface is CEL-aligned so
`cel-interpreter` can be adopted later without changing authoring. Values are `serde_json::Value`.

- **`eval`/`eval_bool`** — evaluate an expression (conditions must be boolean). Supports
  number/string/bool/null literals, variables and `.field` access (missing → `null`), the operators
  `! - * / % + - < <= > >= == != && ||` (with real short-circuit), and the built-ins `has`, `size`,
  `contains`, `starts_with`, `ends_with`, `lower`, `upper`. No assignment, loops, or I/O
  (non-Turing-complete).
- **`eval_array`** — evaluate a `for_each` collection; the execution engine (G9) enforces the bound.
- **Templating** — `resolve_value` / `resolve_params`: `"$path"` yields the referenced value
  preserving its type; `"...${expr}..."` interpolates into a string; anything else is a literal.

## Escape-hatch host seam (G8.3/G8.4 — `script.rs`)

The `ScriptHost` trait is where per-step Starlark (Tier-2) / WASM (Tier-3) bodies run, receiving only
their granted capabilities (NFR-SEC5). The **contract stays declarative Tier-0** regardless (SPEC-D9),
so the planner never parses an escape language. The default `UnsupportedScriptHost` reports Tier-2/3
as not enabled; the real `starlark`/`wasmtime` runtimes plug in behind this trait as a focused,
separately-verified step (they are heavyweight dependencies deliberately isolated behind the seam).

## Tests

Parse + validate a full technique; reject missing MITRE; reject irreversible-without-simulated;
reject script-without-capabilities; catalog indexing/queries + duplicate-id refusal; load a
directory.
