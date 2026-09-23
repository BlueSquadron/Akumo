#!/usr/bin/env bash
#
# Create Akumo v2 / future-evolution issues on the repo's GitHub remote.
# Requires: authenticated `gh` (`gh auth status`) and a GitHub remote (`git remote -v`).
# Usage:  bash scripts/create-v2-issues.sh        (from the repo root)
#         DRY_RUN=1 bash scripts/create-v2-issues.sh   (print instead of create)
#
# Idempotency note: re-running creates DUPLICATE issues. Run once, or delete/close dupes.

set -euo pipefail

run() {
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    printf 'DRY_RUN: gh %s\n' "$*"
  else
    gh "$@"
  fi
}

# --- preflight -------------------------------------------------------------
if ! command -v gh >/dev/null 2>&1; then
  echo "error: gh CLI not found. Install GitHub CLI first." >&2; exit 1
fi
if [[ "${DRY_RUN:-0}" != "1" ]] && ! gh auth status >/dev/null 2>&1; then
  echo "error: gh not authenticated. Run 'gh auth login'." >&2; exit 1
fi
if [[ "${DRY_RUN:-0}" != "1" ]] && ! git remote get-url origin >/dev/null 2>&1; then
  echo "error: no 'origin' remote. Create/link a GitHub repo, e.g.:" >&2
  echo "       gh repo create <owner>/Akumo --private --source . --push" >&2; exit 1
fi

# --- labels (best-effort; ignore if they already exist) --------------------
run label create v2          --color 5319E7 --description "Deferred to v2 / future" 2>/dev/null || true
run label create epic        --color B60205 --description "Large multi-issue effort"  2>/dev/null || true
run label create enhancement --color A2EEEF --description "New feature or request"     2>/dev/null || true

# --- issues ----------------------------------------------------------------
issue() { # $1=title  $2=extra-label  $3=body
  run issue create --title "$1" --label "v2" --label "$2" --body "$3"
}

issue "v2: Multi-cloud provider adapters (Azure, GCP, Kubernetes)" "epic" \
"Implement Azure, GCP, and Kubernetes adapters behind the existing provider seam (ADR-0012) with **no core changes**, proving the NFR-EXT extensibility promise against real second/third providers.

Refs: ADR-0002, requirements NFR-EXT, OUT-1.
DoD: full loop runs on >=1 new provider; core diff = zero (dependency-direction lint stays green)."

issue "v2: Identity-plane providers & cross-provider engagements" "epic" \
"Add identity providers (Entra ID / Okta / OIDC federation) and support engagements whose paths cross planes (federation -> cloud).

Refs: ADR-0002, EXR-7. Depends on multi-cloud adapters."

issue "v2: AI-assisted planner (advisory, human-gated)" "enhancement" \
"Natural-language objectives and next-step suggestions. Strictly advisory and human-gated; every proposed action passes the same safety/consent controls as any other action.

Refs: ADR-0007, FR-E6."

issue "v2: Live telemetry correlation (observed-vs-expected)" "enhancement" \
"Capture the actual telemetry a detonation produced and present observed-vs-expected (Grimoire-style) via the TelemetryCollector capability stub.

Refs: ADR-0008, FR-I2."

issue "v2: External posture ingestion (Prowler / ScoutSuite / Cartography)" "enhancement" \
"Enrich the attack graph with external posture/inventory findings as untrusted supplementary input.

Refs: FR-D5, FR-L1. Enables path prioritization (see severity-ranking issue)."

issue "v2: Lab / detection-validation mode (synthetic warm-up)" "enhancement" \
"Optional Stratus-style synthetic warm-up mode for detection engineering, kept separate from the real-target engagement lifecycle.

Refs: ADR-0021, SPEC-D10."

issue "v2: Full re-execution against IaC / equivalent targets" "enhancement" \
"Beyond path re-verification: re-run a full engagement against an IaC-provisioned / lab-equivalent target.

Refs: ADR-0010, OQ-9."

issue "v2: Additional objective types" "enhancement" \
"New goal predicates (e.g. specific data-exfil conditions, persistence footholds) beyond v1's reach-admin / reach-resource.

Refs: ADR-0027, FR-E2."

issue "v2: Attack-graph diffing across enumeration runs" "enhancement" \
"Show environmental change between enumeration runs.

Refs: FR-D6."

issue "v2: Detection dataset export" "enhancement" \
"Export correlated logs for a detonation for detection-engineering workflows.

Refs: FR-I4. Depends on live telemetry correlation."

issue "v2: Path prioritization using ingested external severity" "enhancement" \
"Rank paths using severity/risk signals from ingested external tools.

Refs: FR-L2. Depends on external posture ingestion."

issue "v2: Attack-graph & path visualization" "enhancement" \
"Visual rendering of the attack graph and computed paths.

Refs: FR-K5."

issue "v2: External graph DB backend option" "enhancement" \
"Optional external graph store if the embedded-first backend's scale is exceeded.

Refs: ADR-0006, OQ-6 (revisit)."

issue "v2: Continuous / scheduled automated red-teaming mode" "enhancement" \
"Recurring, pipeline-driven offensive testing beyond one-off engagements.

Refs: OUT-6."

echo "Done. Created 14 v2 issues (or printed them, if DRY_RUN=1)."
