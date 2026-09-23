# 7 — Starlark Escape Hatch (Tier 2)

> **Rung 7 (advanced).** When the Tier-1 DSL + CEL can't express a computation or branch, drop a
> single step into Starlark — without losing the planner's view of your technique.

## When to reach for it

Stay in the Tier-1 DSL for host-brokered calls, fact binding, and CEL-expressible
conditions/iteration. Escape to **Tier-2 Starlark** only for **computed values or branching CEL
can't express** — e.g. building a derived parameter, transforming a list non-trivially, or
conditional logic beyond simple predicates.

## The invariant that keeps you safe

**Escapes are per-step; the metadata and contract stay declarative Tier-0.** The planner and
validator read your contract, never your script. So a script step can never hide what your technique
requires or produces — the planner still chains it correctly.

## Shape of a script step

```yaml
steps:
  - id: compute-name
    body:
      type: script
      language: starlark
      capabilities: ["compute"]     # you must declare what the step may use (least privilege)
      source: |
        # inputs and prior facts are in scope; return a value to bind
        candidate = lower(user) + "-" + suffix
        result = candidate
    bind:
      - name: derived_name
        from: result
  - id: create-user
    body:
      type: call
      service: iam
      operation: CreateUser
      params: { UserName: "$derived_name" }
    revert:
      service: iam
      operation: DeleteUser
      params: { UserName: "$derived_name" }
```

- **`capabilities`** are mandatory (validation rejects a script step without them) — a step only ever
  gets what it declares (host-brokering, least privilege). A script **cannot** perform I/O or reach a
  cloud SDK; it computes values that later host-brokered steps use.
- Bind the script's `result` into a fact, then reference it from a normal Tier-1 call — the mutation
  and its **revert** still go through the declarative, compensated path.

## Availability

The Starlark runtime is enabled behind the `ScriptHost` seam. In a build where it isn't enabled, a
script step reports *unsupported* rather than running — techniques that stay in Tier-1 are unaffected.
Keep escapes rare and small; they are the exception, not the tool.

## You can now…

- [x] Decide when a step needs Tier-2 and keep the contract declarative.
- [x] Declare capabilities, compute a value, bind it, and keep mutation on the compensated path.

**Next:** [8 — WASM Escape Hatch](08-wasm-escape.md) for heavy computation.
