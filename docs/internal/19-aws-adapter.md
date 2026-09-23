# 19 — AWS Adapter

> Task G16 · ADR-0002 · FR-B2 · spec §2.5. In `akumo-provider-aws`. **The AWS SDK is confined to
> this crate** (NFR-EXT1) — adding it changed zero core lines (EXR-3), the proof that a provider is a
> plugin, not the platform.

## Scope (v1, minimal-but-real)

- **Identity (G16.1):** `IdentityResolver` calls STS `GetCallerIdentity` and maps the caller ARN to
  a `Principal` (kind inferred from the ARN). Credentials come from the **standard AWS provider
  chain** (env vars, shared config/profile, assumed roles), covering the credential input types of
  FR-B3/B4.
- **Metadata (G16.4):** `MetadataProvider` reports provider id `aws` and the active region.
- **Enumeration (G16.2):** `ResourceEnumerator` maps descriptor `iam.ListUsers` / `iam.ListRoles` to
  the SDK calls and returns an intermediate JSON; `GraphMapper` (pure, in `mapper.rs`) turns it into
  `Principal` assertions with the IAM type kept as inventory.
- **Actions (G16.3):** `ActionExecutor` implements the `iam.CreateAccessKey` / `iam.DeleteAccessKey`
  pair (matching the example technique), enforcing the per-step capability grant (NFR-SEC5). The
  created **secret is never surfaced** — only the key id is returned (NFR-SEC2).
- **Telemetry:** the default `Unsupported` collector (v2).

## Error handling

`sdk_error` classifies SDK failures: access/authorization errors become the first-class
`AccessDenied` gap (so enumeration degrades per FR-C6); everything else is a `Provider` error.

## CLI wiring (G16.5)

The CLI now builds `Box<dyn Provider>` from `--provider` (`mock` or `aws`); `--provider aws` calls
`AwsProvider::connect()`. This is a CLI/wiring change only — the core and the whole loop are
unchanged, which is the "AWS is a plugin" proof (together with `dep-lint` keeping the SDK out of the
core).

## Deferred (additive follow-ups)

Pagination, more services and techniques, explicit assume-role chaining, and provider-assisted
validation (AWS `DryRun` / IAM policy simulator, ADR-0022) are additive — new descriptors/capability
code in this crate, no core changes.

## Tests

The response→graph `mapper` is pure and unit-tested **offline** (users/roles → principals; arn-less
skipped; empty → nothing), plus ARN-kind classification. Live AWS behavior (real detonate + revert)
is the release-gated fidelity tier (G19.2), not a unit test.
