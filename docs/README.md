# Akumo — Documentation

Product documentation, split into two audiences (see `spec/tasks.md` group **G18**):

- **[`internal/`](internal/)** — *how Akumo works*. Architecture, ports, the event ledger, the
  provider seam, the graph & fact model, the execution lifecycle, the planner, telemetry,
  reporting, interfaces, the AWS adapter, testing, and security. Written for maintainers and
  contributors.

- **[`external/`](external/)** — *how to write scenarios and attacks*. A strictly **progressive,
  beginner → advanced** ladder: install & first engagement → running attacks safely → authoring
  your first technique → contracts, CEL, chaining, revert → Starlark/WASM escape hatches →
  telemetry & detections → building and signing content bundles. Plus an auto-generated
  per-technique **reference**.

Both sections are written *alongside* the code (each implementation task in `spec/tasks.md`
carries a **Docs** deliverable) and consolidated by group G18. The design intent — internal for
mechanics, external as a progressive authoring curriculum — is a project requirement, not an
afterthought.
