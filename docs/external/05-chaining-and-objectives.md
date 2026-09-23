# 5 — Chaining & Objectives

> **Rung 5.** You'll understand how techniques chain via their effects, how to aim at an objective,
> and how ranking picks the path.

## Objectives

An objective is a goal predicate. v1 ships two:

- **reach-admin** — control a principal that satisfies the "administrative" predicate.
- **reach-resource** — control a principal that can access a named resource.

```bash
akumo paths --engagement demo --objective admin
akumo paths --engagement demo --objective resource:arn:aws:s3:::prod-data
```

## How chaining works

The planner extends a frontier of *controlled* principals two ways:

1. **Existing edges** in the graph (`CAN_ASSUME`, `MEMBER_OF`, trust, …) — graph reachability.
2. **Techniques as actions** — if a technique's **preconditions** hold, applying it produces its
   **effects** (new edges/facts), which may unlock the next hop. This is why your contract (Rung 3)
   matters: it's what makes your technique *chainable*.

So a path can interleave "you already can do X" with "run technique Y to make Z possible" — the
state-creating chains that are Akumo's whole point.

## Ranking & modes

Paths are ranked; pick the emphasis with a preset (default is a composite of
**confidence → length → detectability**):

- `shortest` · `stealthiest` · `safest` · `most-reliable`

Reversibility and max impact are **always shown** as a safety annotation, even when they aren't the
sort key. Uncertainty handling has three modes: honest default (proven prominent, candidates
labeled), strict-proven, and full-optimistic.

## Executing a chain

A computed path becomes a **chain** of techniques. Running a chain:

- executes each technique through the safe lifecycle (preview → consent → detonate → verify);
- on failure, the default policy **halts and auto-reverts the completed techniques** (CI-safe), or
  with hold mode leaves them for you to inspect/fix/resume;
- can **re-enumerate between steps** so a later technique sees access an earlier one created.

## Authoring for chainability — checklist

- Declare **effects** that match the **preconditions** of the techniques you expect to follow.
- Keep impact honest so ranking's safety annotation is meaningful.
- Give every mutating step a **revert** so the chain can unwind cleanly.

## You can now…

- [x] Aim at the two v1 objectives and read ranked paths.
- [x] Explain how effects→preconditions make techniques chain, and how failure is handled.

**Next:** [6 — Revert Contracts](06-revert-contracts.md).
