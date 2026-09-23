# Akumo — v2 / Future Evolutions Backlog

Everything deliberately deferred beyond v1, so we don't lose track of it. Each item is meant to
become a **GitHub issue** (label `v2`). See the note at the bottom on how these get filed.

Source of the deferrals: `requirements.md` §11 (out-of-scope) & §13, the `ADR/` records, and the
`specification.md` increments.

| # | Evolution | Type | Deferred by |
|---|-----------|------|-------------|
| 1 | Multi-cloud provider adapters (Azure, GCP, Kubernetes) | epic | ADR-0002, NFR-EXT, OUT-1 |
| 2 | Identity-plane providers & cross-provider engagements (Entra/Okta/OIDC federation) | epic | ADR-0002, EXR-7 |
| 3 | AI-assisted planner (advisory, human-gated) | feature | ADR-0007, FR-E6 |
| 4 | Live telemetry correlation (observed-vs-expected) | feature | ADR-0008, FR-I2 |
| 5 | External posture ingestion (Prowler / ScoutSuite / Cartography) | feature | FR-D5/L |
| 6 | Lab / detection-validation mode (synthetic warm-up) | feature | ADR-0021, SPEC-D10 |
| 7 | Full re-execution against IaC / equivalent targets | feature | ADR-0010, OQ-9 |
| 8 | Additional objective types beyond reach-admin / reach-resource | feature | ADR-0027, FR-E2 |
| 9 | Attack-graph diffing across enumeration runs | feature | FR-D6 |
| 10 | Detection dataset export | feature | FR-I4 |
| 11 | Path prioritization using ingested external severity | feature | FR-L2 |
| 12 | Attack-graph & path visualization | feature | FR-K5 |
| 13 | External graph DB backend option (scale) | feature | ADR-0006, OQ-6 |
| 14 | Continuous / scheduled automated red-teaming mode | feature | OUT-6 |

## Details

1. **Multi-cloud provider adapters** — implement Azure, GCP, and Kubernetes adapters behind the
   existing provider seam (ADR-0012) with no core changes. Validates the NFR-EXT extensibility
   promise against real second/third providers. *DoD:* the full loop runs on ≥1 new provider;
   core diff = zero.
2. **Identity-plane providers & cross-provider engagements** — Entra ID / Okta / OIDC federation
   as provider(s), plus engagements whose paths cross planes (federation → cloud). Depends on #1.
3. **AI-assisted planner** — natural-language objectives and next-step suggestions; strictly
   advisory, human-gated, subject to the same safety/consent controls as any action (ADR-0007).
4. **Live telemetry correlation** — capture actual telemetry a detonation produced and present
   observed-vs-expected (Grimoire-style) via the TelemetryCollector capability stub (FR-I2).
5. **External posture ingestion** — enrich the graph with Prowler / ScoutSuite / Cartography
   findings as untrusted supplementary input (FR-D5/L1); optional path prioritization (see #11).
6. **Lab / detection-validation mode** — an optional synthetic warm-up mode (Stratus-style) for
   detection engineering, kept separate from the real-target engagement lifecycle (ADR-0021).
7. **Full re-execution against equivalent targets** — beyond path re-verification: re-run a full
   engagement against an IaC-provisioned/lab-equivalent target (ADR-0010).
8. **Additional objective types** — new goal predicates (e.g., specific data-exfil conditions,
   persistence footholds) beyond v1's reach-admin / reach-resource (ADR-0027).
9. **Attack-graph diffing** — show environmental change between enumeration runs (FR-D6).
10. **Detection dataset export** — export correlated logs for a detonation for detection-
    engineering workflows (FR-I4); depends on #4.
11. **Path prioritization via ingested severity** — rank paths using severity/risk signals from
    ingested external tools (FR-L2); depends on #5.
12. **Attack-graph & path visualization** — visual rendering of the graph and computed paths
    (FR-K5).
13. **External graph DB backend** — optional external graph store if embedded-first scale is
    exceeded (ADR-0006 / OQ-6 revisit).
14. **Continuous / scheduled automated red-teaming mode** — recurring, pipeline-driven offensive
    testing beyond one-off engagements (OUT-6).

---

## How these get filed as GitHub issues

The authoring session could not run `gh` (the auto-mode safety classifier blocked all shell
commands for this session — it reacts to the conversation theme, not the command). To create the
issues yourself, run the generated script from this session's terminal:

```
! bash scripts/create-v2-issues.sh
```

(The `!` prefix runs it directly in your shell, bypassing the blocked tooling.) The script is
idempotent about labels and requires an authenticated `gh` and a GitHub remote on this repo
(`gh auth status`, `git remote -v`). If there is no remote yet, create/link one first, e.g.
`gh repo create <owner>/Akumo --private --source . --push`.
