#!/usr/bin/env bash
set -euo pipefail

git fetch origin main
git diff origin/main > diff.patch

# Run cargo mutants on the diff
RUSTFLAGS="${RUSTFLAGS:--D warnings}" \
cargo mutants \
  -j3 \
  --in-diff diff.patch \
  "$@"
