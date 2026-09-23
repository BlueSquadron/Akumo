# 3 — Preconditions & Effects (the fact model)

> **Rung 3.** You'll declare what your technique *needs* and what it *produces*, so the planner can
> chain it into attack paths automatically.

This is Akumo's core idea: **one declaration drives both execution and planning.** You write the
contract once; the execution engine uses it to gate a step, and the planner uses it as a planning
action (a precondition/effect pair, PDDL-style).

## Preconditions and effects are predicates over the graph

A **fact predicate** is `name(args…)` over the attack graph's fact views — e.g.
`can_assume(principal, role)`, `has_permission(principal, permission)`,
`has_credential(principal, credential)`, `can_access(principal, resource)`.

```yaml
contract:
  inputs:
    - name: user
      type: principal
      required: true
  preconditions:
    - predicate: { name: has_permission, args: ["$foothold", "iam:CreateAccessKey"] }
      min_status: proven          # how certain the evidence must be
  effects:
    - predicate: { name: has_credential, args: ["$user", "new-access-key"] }
```

- **`preconditions`** must hold before the step runs. Each carries a required **epistemic status**
  (`proven` by default; `inferred`/`unknown` allowed only when you mark it `opportunistic: true`).
- **`effects`** are asserted on **verified success** — they're what let the planner chain your
  technique after the technique that produced its precondition.

## Why this matters for the planner

The planner treats each technique as an action: *if its preconditions hold in the current state,
applying it yields its effects.* That's how Akumo finds **state-creating chains** (e.g. *create a key
→ assume a role*) that pure graph reachability can't express. Your contract is the interface the
planner reasons over — it never has to read your steps.

## Epistemic honesty

`min_status` ties into Akumo's rule that **"unknown ≠ absent."** A precondition requiring `proven`
evidence won't be satisfied by an unseen edge; an `opportunistic` one may be, and any path using it
is flagged as a **candidate** needing validation. This keeps plans honest under partial permissions
(the normal case in real engagements).

## You can now…

- [x] Declare typed inputs, preconditions (with required epistemic status), and effects.
- [x] Explain how the contract feeds both execution and the planner (one declaration, two uses).

**Next:** [4 — CEL Expressions](04-cel-expressions.md) — conditions, iteration, and templating.
