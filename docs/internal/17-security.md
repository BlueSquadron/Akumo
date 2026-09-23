# 17 — Security of the Tool Itself

> NFR-SEC · FR-J6. How Akumo protects credentials, loot, and the operator — and what is deferred.

Akumo is a security tool; its own posture matters. Current state, honestly:

## Implemented

- **Secrets stay out of output (NFR-SEC2, FR-J6).** Loot/secret material is never placed in the
  ledger projections or reports. The AWS adapter, for example, returns only an access **key id** from
  `CreateAccessKey` — the secret is dropped, never surfaced. Reports carry a redaction note.
- **No third-party transmission (NFR-SEC3).** Akumo talks only to the target's provider APIs. There
  is no telemetry/callhome; nothing leaves the operator's machine except calls to the authorized
  target.
- **Least-privilege content (NFR-SEC5).** Techniques are host-brokered: a step can only invoke the
  capability it declared and was granted (`CapabilityGrant`), and content never holds a cloud SDK.
  Escape-hatch (Starlark/WASM) steps must declare their capabilities and run sandboxed.
- **Supply-chain integrity.** The content bundle is versioned and **Ed25519-signed** with
  per-technique SHA-256 pinning; a tampered technique or wrong key fails verification.
- **Tamper-evident audit (NFR-OBS3).** The engagement ledger is hash-chained; any edit is detectable
  (`verify_chain` / `verify_hash`).
- **Authorized-use enforcement.** Opening an engagement records an authorization affirmation; scope
  is fail-closed; the kill-switch halts and reverts.

## Deferred (documented, not silently missing)

- **Encryption at rest of stored secrets (NFR-SEC1).** The ledger currently stores redacted
  references, not raw secrets; a dedicated encrypted loot store is a packaging-phase follow-up. Until
  then, treat the state directory as sensitive (see `.gitignore` excludes `*.loot`, engagement
  state, and signing keys).
- **Portable encrypted state export (NFR-SEC4).** Export/import with secret protection is a follow-up.

## Operator guidance

- Keep the `--state-dir` on trusted, access-controlled storage; it is the engagement's evidence.
- Distribute only the **verifying** key for bundles; keep signing keys offline.
- Prefer short-lived credentials (the AWS adapter uses the standard provider chain, including
  assumed roles / OIDC).
