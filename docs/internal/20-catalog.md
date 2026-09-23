# 20 — Technique Catalog & Content Bundle

> Task G17 · ADR-0031 (signed bundle) · FR-F · spec §7.4. Content under `code/content/`; bundle in
> `akumo-content`.

## The default AWS catalog (G17.1)

The seed catalog lives under [`code/content/aws/`](../../code/content/aws/), organized by **tactic**
(mirroring the [AWS Threat Technique Catalog](https://aws-samples.github.io/threat-technique-catalog-for-aws/matrix.html)
columns). v1 ships five reversible techniques:

| Tactic | Technique | MITRE | Action / revert |
|---|---|---|---|
| Persistence | Create IAM Access Key | T1098 | `CreateAccessKey` / `DeleteAccessKey` |
| Persistence | Create Console Login Profile | T1098 | `CreateLoginProfile` / `DeleteLoginProfile` |
| Persistence | Create IAM User | T1136.003 | `CreateUser` / `DeleteUser` |
| Privilege Escalation | Attach Managed Policy to User | T1098 | `AttachUserPolicy` / `DetachUserPolicy` |
| Defense Evasion | Stop CloudTrail Logging | T1562.008 | `StopLogging` / `StartLogging` |

Each carries MITRE ATT&CK as the machine-readable classification, an AWS Threat Technique Catalog
URL in `references` (the human cross-map — exact ATC ids can be filled in there), expected telemetry
(a CloudTrail signature → Sigma via G13), a precondition/effect contract, and a **revert contract**.
All are `mutating-reversible`, so none needs a simulated variant (FR-G6).

> The AWS adapter (G16) currently implements the `CreateAccessKey`/`DeleteAccessKey` calls; the other
> techniques validate, dry-run, and run on the Mock today, and execute live once the adapter grows
> the corresponding operations — a purely additive change behind the seam.

## The signed content bundle (G17.2 — `akumo-content::bundle`)

- **`build_manifest`** produces a `BundleManifest { name, version, engine_compat, entries }` where
  each `BundleEntry` pins a technique by id, version, and **SHA-256** of its canonical serialization.
- **`sign_manifest` / `verify_manifest`** sign and verify the canonical manifest bytes with
  **Ed25519** (NFR-SEC5); the signature is a detached hex string.
- **`verify_bundle`** checks every technique's hash against the manifest *and* the manifest
  signature — a swapped or tampered technique fails (proven by tests), as does a wrong key.
- **`verifying_key_to_hex` / `verifying_key_from_hex`** distribute/pin a trusted key.

This is versioned (NFR-MNT3), signed (NFR-SEC5), offline-capable, and updatable independently of the
engine (NFR-EXT6). A pull-able registry (SPEC-D19 option C) is a v2 layer on top.

## Loading & validation

Techniques are loaded and validated by [`akumo_dsl::Catalog`] (G7) — adding one is additive (drop a
YAML file). A crate test loads the shipped `content/` tree and asserts it validates.

## Deferred (G17.3)

The `xtask`/CLI bundle **tooling** (build/sign/verify/install wrappers over the library above) is a
thin follow-up; the library API is complete and tested now.
