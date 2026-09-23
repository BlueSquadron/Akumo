# 4 — CEL Expressions

> **Rung 4.** You'll use Akumo's safe expression language for step conditions, bounded iteration,
> and parameter templating.

Akumo techniques use a small, **safe, non-Turing-complete** expression language (CEL-aligned). It has
**no assignment, no loops, no I/O** — it can compute values and make decisions, but it cannot run
away or reach the network. Values are JSON (strings, numbers, bools, null, objects, arrays).

## What's available

- **Literals:** `"text"`, `42`, `true`, `null`.
- **Variables & fields:** `user`, `result.AccessKey.AccessKeyId` (a missing field is `null`).
- **Operators:** `! - * / % + - < <= > >= == != && ||` (with real short-circuit).
- **Built-ins:** `has(x)`, `size(x)`, `contains(s, sub)`, `starts_with(s, p)`, `ends_with(s, p)`,
  `lower(s)`, `upper(s)`.

The variables in scope are your **inputs**, the previous step's **`result`**, and any facts you've
**bound** (below). See the [CEL surface reference](reference/cel-surface.md) for the full list.

## Templating: `$` and `${}`

Step params and predicate args are templated:

- `"$user"` — a bare `$` + a path resolves to the **value**, preserving its type.
- `"user-${user}-key"` — `${expr}` **interpolates** an expression into a string.
- `"iam:CreateAccessKey"` — anything else is a literal.

```yaml
params:
  UserName: "$user"                       # → the value of input `user`
  Description: "created by ${upper(user)}" # → "created by ALICE"
```

## Binding results

A step can bind fields of its result into named facts for later steps and for its revert:

```yaml
steps:
  - id: create-key
    body: { type: call, service: iam, operation: CreateAccessKey, params: { UserName: "$user" } }
    bind:
      - name: key_id
        from: result.AccessKeyId     # a CEL path into the step result
    revert:
      service: iam
      operation: DeleteAccessKey
      params: { UserName: "$user", AccessKeyId: "$key_id" }   # uses the bound fact
```

## Conditions and bounded iteration

```yaml
steps:
  - id: maybe-tag
    condition: "has(result.AccessKeyId) && user != 'root'"   # runs only if true
    body: { type: call, service: iam, operation: TagUser, params: { UserName: "$user" } }
```

`for_each` iterates a collection (with an explicit bound); anything the expression language can't
express is where the **escape hatches** (Rungs 7–8) come in.

## You can now…

- [x] Write conditions and templated params with CEL.
- [x] Bind step results into facts and reference them (including in a revert).

**Next:** [5 — Chaining & Objectives](05-chaining-and-objectives.md).
