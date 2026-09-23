# Reference — Expression (CEL) Surface

Akumo's Tier-1 expression language is safe and **non-Turing-complete**: no assignment, no loops, no
I/O. Values are JSON (`string`, `number`, `bool`, `null`, `object`, `array`).

## Where expressions are used

| Location | Kind | Result |
|---|---|---|
| step `condition` | expression | must be **boolean** |
| step `for_each.items` | expression | must be an **array** |
| `bind[].from` | expression | any value (bound into a fact) |
| params / predicate args | **templating** (see below) | value or string |

## Variables in scope

- **Inputs** — by name (e.g. `user`).
- **`result`** — the previous step's result (object); missing fields are `null`.
- **Bound facts** — anything a prior `bind` produced.

## Operators (precedence high → low)

`!`, unary `-` · `* / %` · `+ -` · `< <= > >=` · `== !=` · `&&` · `||`

- `+` adds numbers or concatenates strings.
- `&&` / `||` short-circuit.
- Comparisons are numeric or string (lexicographic).

## Built-in functions

| Function | Meaning |
|---|---|
| `has(x)` | `x != null` |
| `size(x)` | length of a string / array / object |
| `contains(s, sub)` | substring test |
| `starts_with(s, p)` · `ends_with(s, p)` | prefix / suffix test |
| `lower(s)` · `upper(s)` | case conversion |

Field access is `a.b.c` (a missing field yields `null`, so guard with `has(...)`).

## Templating

Applied to string param/arg values:

| Form | Behavior |
|---|---|
| `"$path"` | the referenced **value**, preserving its type (`$user`, `$result.AccessKeyId`) |
| `"…${expr}…"` | **interpolate** the expression into a string |
| anything else | literal string |

Examples:

```yaml
UserName: "$user"                       # value of `user`
Note: "created-by-${upper(user)}"       # "created-by-ALICE"
Arn: "arn:aws:iam::123:policy/Admin"    # literal
```

## Not available (by design)

No variables assignment, no user-defined functions, no loops (`for_each` is bounded and lives in the
step, not the language), no I/O or network. When you need more, use a per-step
[Starlark](../07-starlark-escape.md) (Tier-2) or [WASM](../08-wasm-escape.md) (Tier-3) escape — the
contract stays declarative.
