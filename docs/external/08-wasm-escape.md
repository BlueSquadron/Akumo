# 8 — WASM Escape Hatch (Tier 3)

> **Rung 8 (advanced).** For heavy computation or library work — crypto, encoding, binary/protocol
> parsing, payload generation — drop a step into WebAssembly.

## When to reach for it

Tier-3 WASM is for what neither CEL (Tier-1) nor Starlark (Tier-2) should do: **heavy computation or
reusing a compiled library.** Typical cases: computing a signature/HMAC, base64/DER encoding, parsing
a binary blob, or generating a payload. If you only need a derived string or a branch, use Tier-2.

## Same invariant, same discipline

As with Starlark, WASM is **per-step**, and your **metadata and contract stay declarative Tier-0** —
the planner never parses WASM. A WASM step computes a value; the actual cloud mutation and its revert
still flow through the declarative, host-brokered, compensated path.

## Shape of a WASM step

```yaml
steps:
  - id: build-payload
    body:
      type: script
      language: wasm
      capabilities: ["compute"]     # mandatory; sandbox denies filesystem/network
      source: |
        # a WASM module (referenced/embedded per your build); fact-in / fact-out ABI:
        # it receives the current facts and returns a value to bind.
    bind:
      - name: payload
        from: result
  - id: use-payload
    body:
      type: call
      service: lambda
      operation: UpdateFunctionCode
      params: { FunctionName: "$fn", ZipFile: "$payload" }
    revert:
      service: lambda
      operation: UpdateFunctionCode
      params: { FunctionName: "$fn", ZipFile: "$previous_code" }
```

- The WASM sandbox has **no ambient capabilities** beyond what's granted — no filesystem, no network.
  It is pure compute over the facts you pass in.
- Prefer **deterministic** modules so runs are reproducible.
- Everything a WASM step touches externally must still be a host-brokered Tier-1 call with a revert.

## Availability

Like Starlark, the WASM runtime plugs in behind the `ScriptHost` seam; a build without it reports
*unsupported* for WASM steps. Reserve Tier-3 for genuine heavy computation.

## You can now…

- [x] Decide when computation belongs in Tier-3.
- [x] Keep the mutation + revert on the declarative path while WASM only computes.

**Next:** [9 — Telemetry & Detections](09-telemetry-and-detections.md) — the purple-team payoff.
