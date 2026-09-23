# Building a Content Bundle

> **Advanced.** Package, version, and **sign** a set of techniques so they can be distributed and
> loaded independently of the engine — offline-friendly and tamper-evident.

Akumo ships a default catalog, but the content model is meant to grow **without an engine release**
(NFR-EXT6). A **content bundle** is a versioned, signed manifest over a set of techniques.

## What a bundle is

- A **manifest** — `{ name, version, engine_compat, entries[] }`, where each entry pins a technique
  by `id`, `version`, and the **SHA-256** of its canonical form.
- A **detached signature** — Ed25519 over the canonical manifest bytes.

Because each technique is hash-pinned and the manifest is signed, a swapped or tampered technique, or
a wrong signing key, fails verification.

## Compatibility & versioning

- `version` — the bundle's own semver.
- `engine_compat` — the engine/content-model range the bundle targets (NFR-MNT3). The engine refuses
  a bundle it isn't compatible with.
- Each technique also carries its own `version` in metadata; bump it when you change a technique.

## The workflow

1. **Author & test** your techniques (Rungs 2–6); every mutating one has a passing revert test.
2. **Build** the manifest over your content directory (pins each technique by hash).
3. **Sign** the manifest with your Ed25519 signing key. Keep the key secret; distribute only the
   **verifying key** (as hex).
4. **Distribute** the content directory + manifest + signature.
5. **Verify on load** — the engine checks every technique hash against the manifest and the manifest
   signature against a trusted verifying key before loading anything.

Programmatically (the library API, `akumo-content::bundle`):

```rust
use akumo_content::bundle::{build_manifest, sign_manifest, verify_bundle};
let manifest = build_manifest("aws-default", "1.0.0", ">=0", &techniques)?;
let signature = sign_manifest(&manifest, &signing_key)?;      // hex, detached
verify_bundle(&manifest, &techniques, &signature, &verifying_key)?;  // fails on tamper/wrong key
```

> A CLI/`xtask` wrapper for build/sign/verify/install is planned; today the bundle library is the
> supported interface, and the engine loads/validates plain content directories via the catalog.

## Trust & distribution

- Pin the **verifying key** out of band (it's just hex); never ship the signing key.
- For sensitive/offline environments, distribute the bundle as files — no registry/network needed.
- A pull-able registry is a future convenience layered on top of signed bundles.

## You can now…

- [x] Explain a bundle's manifest, hash-pinning, and signature.
- [x] Build, sign, and verify a bundle, and reason about compatibility.
