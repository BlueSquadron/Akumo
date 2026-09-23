# 1 — Running Attacks Safely

> **Rung 1.** You'll enumerate a target, compute an attack path toward an objective, preview an
> action's blast radius, run a reversible technique with consent, and revert it — the whole safe
> loop, still on the Mock.

Assumes you've done [Rung 0](00-getting-started.md) and have an open engagement `demo`.

## Impact levels & consent

Every technique declares an **impact level**, and consent is gated on it:

| Impact | Meaning | Consent |
|---|---|---|
| `read` | observes only | none needed |
| `mutating-reversible` | changes state, ships a tested revert | consent required |
| `mutating-irreversible` | no clean inverse | opt-in, prefers a simulated variant |
| `destructive` | destructive | opt-in, gated |

The default posture is **least blast radius**: nothing above `read` runs without a recorded consent
decision at or above its impact.

## Enumerate → graph

```bash
akumo enumerate --engagement demo --descriptor iam.ListUsers --descriptor iam.ListRoles
```

Enumeration discovers principals/permissions/resources and folds them into the **attack graph**. A
call you're not allowed to make becomes a *coverage gap* (a recorded blind spot), never a crash — so
you always get a partial graph plus an honest list of what you couldn't see.

## Compute attack paths

```bash
# reach administrative privilege from your foothold
akumo paths --engagement demo --objective admin

# or reach a specific resource
akumo paths --engagement demo --objective resource:arn:aws:s3:::prod-data
```

Each path is **explainable**: every hop names the principal, the edge or technique, the enabling
permission, its epistemic status (`Proven` vs `Candidate`), and what it produces. Paths that rely on
edges you couldn't fully prove are surfaced as **candidates** — never hidden, never presented as
certain.

## Preview before you commit

```bash
akumo preview --engagement demo --technique aws.iam.persistence.create-access-key --input user=alice
```

`preview` is a **dry-run**: it prints the intended provider calls, the declared effects, and the
**impact and reversibility** — performing **zero** mutating calls. Always preview first.

## Run, then revert

```bash
# run it — --consent records authorization at the technique's impact level
akumo run --engagement demo --technique aws.iam.persistence.create-access-key --input user=alice --consent

# undo it — replays the recorded compensation (here, DeleteAccessKey)
akumo revert --engagement demo
```

If a multi-step technique fails partway, Akumo **auto-reverts the completed steps** — the target is
left clean without you asking. You can also revert on demand at any time.

## The kill-switch

At any point, halting the engagement stops new actions and triggers revert of completed reversible
steps. It's always available and is your safety brake.

## You can now…

- [x] Enumerate a target and read coverage gaps.
- [x] Compute explainable attack paths toward an objective.
- [x] Preview blast radius, run a reversible technique with consent, and revert.

**Next:** [2 — Your First Technique](02-first-technique.md) — author a technique of your own.
