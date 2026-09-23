# 2 — Your First Technique

> **Rung 2.** You'll author a complete, minimal technique in YAML — metadata, a contract, one
> host-brokered step, and a revert — load it into the catalog, and preview it.

Techniques are **data, not code you must trust**: declarative YAML, versioned, validated, and always
carrying a revert contract. You never write a cloud SDK call by hand — you reference a provider
operation and the host brokers it.

## The four parts

```
technique
├─ metadata   — id, name, MITRE mapping, impact, expected telemetry   (always declarative)
├─ contract   — inputs, preconditions, effects                        (always declarative)
├─ steps      — what it does: host-brokered calls (+ optional escapes)
└─ revert     — how each step is undone
```

The first two are read by the **planner** and the **validator** and never execute code. That's what
lets Akumo reason about your technique before running it.

## A minimal technique

Create `content/aws/persistence/tag-user.yaml`:

```yaml
metadata:
  id: aws.iam.persistence.tag-user
  name: Tag IAM User
  description: Adds a tag to an IAM user (a benign, fully reversible example).
  version: "1.0.0"
  provider: aws
  mitre: ["T1098"]
  impact: mutating-reversible
  expected_telemetry:
    - source: cloudtrail
      event_name: TagUser
      fields:
        eventSource: iam.amazonaws.com

contract:
  inputs:
    - name: user
      type: principal
      required: true

steps:
  - id: tag
    body:
      type: call
      service: iam
      operation: TagUser
      params:
        UserName: "$user"
        Tags: [{ Key: "akumo", Value: "demo" }]
    revert:
      service: iam
      operation: UntagUser
      params:
        UserName: "$user"
        TagKeys: ["akumo"]
```

Key points:

- **`impact: mutating-reversible`** + a **`revert`** block = safe by construction. (An
  irreversible/destructive technique must declare a `simulated_variant` instead — Rung 6.)
- **`$user`** is a template reference to the input `user`. Anything starting with `$` resolves to a
  value; `${expr}` interpolates into a string (Rung 4).
- **`expected_telemetry`** is mandatory — every action carries its detection meaning (Rung 9).
- **`mitre`** is mandatory — at least one ATT&CK id.

## Load and preview it

```bash
akumo catalog --content content          # your technique now appears in the catalog
akumo technique --id aws.iam.persistence.tag-user --content content   # metadata + generated Sigma
akumo preview --engagement demo --technique aws.iam.persistence.tag-user --input user=alice
```

Validation runs at load time. If you omit a mandatory field (say `mitre` or `expected_telemetry`),
the technique is **rejected with an actionable message** — it never silently half-loads.

## You can now…

- [x] Write a technique with metadata, a contract input, a host-brokered step, and a revert.
- [x] Load it into the catalog and preview it.
- [x] Explain why the metadata/contract are "always declarative".

**Next:** [3 — Preconditions & Effects](03-preconditions-effects.md) — the fact model that lets the
planner chain your technique into attack paths.
