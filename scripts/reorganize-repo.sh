#!/usr/bin/env bash
#
# Akumo — repository reorganization (one-shot).
#
# Moves the specification-phase documents into spec/ and the logo into assets/, so the root is
# grouped by phase (spec / docs / code / tests / assets) ready for implementation.
#
# The new folders (spec/, docs/, code/, tests/, assets/) and all placeholder/README files are
# ALREADY created. This script only performs the file MOVES that couldn't be done automatically,
# and it is safe to re-run (each move is guarded).
#
# NOTE: the spec documents were never committed (they are untracked), so we use plain `mv`, not
# `git mv`. They will be picked up when you `git add -A` before committing the reorganization.
#
# Usage:  bash scripts/reorganize-repo.sh            (from the repo root)
#         DRY_RUN=1 bash scripts/reorganize-repo.sh  (print what would happen)

set -euo pipefail

cd "$(git rev-parse --show-toplevel)"

move() { # $1=src  $2=dest-dir
  local src="$1" dest="$2"
  if [[ ! -e "$src" ]]; then
    echo "skip (already moved / absent): $src"
    return 0
  fi
  mkdir -p "$dest"
  if [[ "${DRY_RUN:-0}" == "1" ]]; then
    echo "DRY_RUN: mv $src $dest/"
  else
    mv "$src" "$dest"/
    echo "moved: $src -> $dest/"
  fi
}

echo "== Grouping the specification phase into spec/ =="
move Analysis.md       spec
move requirements.md   spec
move specification.md  spec
move tasks.md          spec
move REFERENCES.md     spec
move V2_BACKLOG.md     spec
move ADR               spec

echo "== Moving brand assets into assets/ =="
move akumo-logo.png    assets

echo
echo "Done. Review with:  git status"
echo "The root README, spec/README, docs/, code/, and tests/ scaffolding already point at these"
echo "new locations, so no link fixes remain. Stage everything with 'git add -A' and commit."
