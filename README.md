<p align="center">
  <img src="assets/akumo-logo.png" alt="Akumo" width="200">
</p>

<h1 align="center">Akumo</h1>

<p align="center"><em>Offense-minded security for the cloud.</em></p>

---

**Akumo** fuses **Akuma** (悪魔, *"devil"* in Japanese) and **Kumo** (雲, *"cloud"*) — a security
testing framework for cloud-hosted infrastructure that thinks like an attacker but is built to be
safe.

Akumo is a **multi-cloud-ready, attack-graph-native, safe-by-construction offensive framework**. It
unifies *discovery → attack-path reasoning → safe execution → purple-team output* into one loop with
one data model — closing the seam that today forces operators to stitch together a scanner, a
path-enumerator, an exploitation tool, and a detection validator by hand. **v1 targets AWS**, behind
a provider-neutral core designed so additional providers are additive, never a rewrite.

> ### ⚠️ Authorized use only
> Akumo is for infrastructure you are **authorized to test**: owned accounts, contracted
> engagements, and lab/CTF environments. Authorization gating, scope enforcement, and audit are
> first-class, enforced requirements — not features. See [`spec/requirements.md`](spec/requirements.md)
> §3 and [`spec/Analysis.md`](spec/Analysis.md) §11.

## Repository layout

| Path | Contents |
|------|----------|
| [`spec/`](spec/) | **Specification phase** — analysis, requirements, technical specification, the 31 ADRs, the build task list, references, and the v2 backlog. Start at [`spec/README.md`](spec/README.md). |
| [`docs/`](docs/) | **Documentation** — [`internal/`](docs/internal/) (how Akumo works) and [`external/`](docs/external/) (a progressive, beginner→advanced guide to writing scenarios and attacks). |
| [`code/`](code/) | **Implementation** — the Rust workspace (hexagonal core, provider adapters, DSL engine, CLI). See [`code/README.md`](code/README.md). |
| [`tests/`](tests/) | **Cross-cutting tests** — end-to-end loop tests, CLI transcripts, safety/release-gate tests, and fixtures. |
| [`assets/`](assets/) | Brand assets (the logo above). |
| [`scripts/`](scripts/) | Project automation. |

## Where to start

- **Understand the design:** [`spec/README.md`](spec/README.md) → `Analysis.md` → `requirements.md`
  → `specification.md` → `ADR/`.
- **Build it:** follow [`spec/tasks.md`](spec/tasks.md) top-to-bottom (groups G0–G20).
- **Use / extend it:** [`docs/external/`](docs/external/) (once implementation lands).

## Roadmap & v2

v1 targets a single provider (AWS) behind a provider-neutral core. Everything deliberately deferred
beyond v1 — additional cloud/identity providers, an AI-assisted planner, live telemetry correlation,
external posture ingestion, and more — is tracked in [`spec/V2_BACKLOG.md`](spec/V2_BACKLOG.md) and
can be filed as GitHub issues with [`scripts/create-v2-issues.sh`](scripts/create-v2-issues.sh). The
v1 completion criteria and their proofs are in [`docs/internal/22-v1-done.md`](docs/internal/22-v1-done.md).

## License

See [`LICENSE`](LICENSE).
