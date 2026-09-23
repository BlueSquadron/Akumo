# Reference — Fact Vocabulary

Fact predicates are named views over the attack graph, used in a technique's `preconditions` and
`effects` (Rung 3) and by the planner. A predicate is `name(args…)`; args are input refs (`$user`) or
literals, resolved at execution/planning time.

## Attack-core predicates (planner-aware)

These map to attack-core edges the planner reasons over:

| Predicate | Meaning | Backing edge |
|---|---|---|
| `can_assume(principal, role)` | principal can assume the role | `CAN_ASSUME` |
| `has_permission(principal, permission)` | principal holds a permission/permission-set | `HAS_PERMISSION` |
| `has_credential(principal, credential)` | principal holds a credential | `HAS_CREDENTIAL` |
| `can_access(principal, resource)` | principal can access a resource | `CAN_ACCESS` |
| `member_of(principal, group)` | group membership | `MEMBER_OF` |
| `trusts(a, b)` | trust relationship | `TRUSTS` |
| `federated_as(external, principal)` | federation mapping | `FEDERATED_AS` |

Each carries an **epistemic status** (`proven` / `absent` / `unknown` / `inferred`); a precondition's
`min_status` says how certain the evidence must be.

## Free-form predicates

You may also declare predicates the planner does not (yet) reason over — they still document intent
and appear in reports (e.g. `logging_disabled(trail)`, `exists_principal(name)`). Only the
attack-core predicates above influence path-finding today; others are descriptive.

> The set of planner-aware predicates grows as the attack-core ontology does; see
> [internal graph docs](../../internal/06-graph-and-facts.md).
