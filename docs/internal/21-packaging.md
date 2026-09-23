# 21 — Packaging, Distribution & Versioning

> Task G20.1/G20.2 · ADR-0003 · NFR-PORT · NFR-MNT3. Build config in `code/Cargo.toml`, `Dockerfile`,
> and `.github/workflows/release.yml`.

## Single static binary (G20.1)

Akumo ships as one self-contained binary (ADR-0003), built with a size/speed-tuned release profile
(`opt-level = "z"`, `lto`, `codegen-units = 1`, `strip`, `panic = "abort"` — see `code/Cargo.toml`).

Distribution channels:

| Channel | How |
|---|---|
| Binary release | `release.yml` builds musl-static Linux (x86_64/aarch64) + macOS (x86_64/aarch64), with SHA-256 checksums and **cosign** signatures, attached to the GitHub release. |
| Container | `Dockerfile` builds a musl binary on Alpine and ships it on a minimal Alpine runtime with CA certs; pushed to GHCR (`linux/amd64,linux/arm64`). |
| Homebrew | a `homebrew-akumo` tap formula points at the signed release tarballs. |
| Source | `cargo build --release --bin akumo` (or `cargo install`). |

Builds are reproducible-oriented and **signed** — appropriate for a security tool. Verifiers check the
checksum and cosign signature against the release identity.

## Versioning & compatibility (G20.2, NFR-MNT3)

- **Crate/binary version:** all workspace crates share `version.workspace` — currently **1.0.0**.
- **Engine contract version:** `akumo_core::ENGINE_CONTRACT_VERSION` tracks the content-model/event
  contract; bumped on a breaking change to the technique model or event schema.
- **Technique/bundle compatibility:** a technique declares `engine_compat`, and a content bundle's
  manifest carries an `engine_compat` range (NFR-EXT6). The engine refuses content it is not
  compatible with, with an actionable error.
- **Policy:** the public library surface (`akumo_core::api`) and the CLI's structured (`--json`)
  output follow semver — breaking changes bump the major version.

## Platforms (NFR-PORT)

Linux and macOS are tier-1 (release binaries + container); the binary runs in CI/pipeline runners
(FR-K2). Windows is best-effort where the toolchain and SDKs allow.
