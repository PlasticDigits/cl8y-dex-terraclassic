#!/usr/bin/env bash
# Automated verification for GitLab #640 — Cosmostation CW20 pack.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }

run_step() {
  local label="$1"
  shift
  echo ""
  echo "[$label]"
  if "$@"; then
    ok "$label"
  else
    bad "$label"
  fi
}

echo "════════════════════════════════════════════════════════════════"
echo "  GitLab #640 — Cosmostation CW20 pack"
echo "════════════════════════════════════════════════════════════════"

run_step "pack: schema, Keplr lockstep, logos" \
  python3 scripts/qa/cosmostation_cw20_validate.py

run_step "export: chain/terra tree (not phoenix)" \
  bash -c '
    tmp="$(mktemp -d)"
    trap "rm -rf \"$tmp\"" EXIT
    ./scripts/qa/export-cosmostation-cw20-pack.sh "$tmp"
    test -f "$tmp/chain/terra/cw20_2.fragment.json"
    test -f "$tmp/chain/terra/asset/cl8y.png"
    test ! -d "$tmp/chain/phoenix"
  '

run_step "docs: invariants C640-1–C640-8" \
  bash -c '
    grep -qE "\*\*C640-1\*\*" docs/listings/cosmostation/README.md && \
    grep -qE "\*\*C640-8\*\*" docs/listings/cosmostation/README.md && \
    grep -qE "chain/terra" docs/listings/cosmostation/README.md && \
    grep -qE "unstablecoin" docs/listings/cosmostation/README.md && \
    grep -qE "make verify-issue-640" docs/listings/cosmostation/README.md
  '

run_step "docs: skill + AGENTS.md + integrators" \
  bash -c '
    grep -qE "\*\*C640-1" skills/AGENTS_COSMOSTATION.md && \
    grep -qE "\*\*C640-8" skills/AGENTS_COSMOSTATION.md && \
    grep -qE "make verify-issue-640" skills/AGENTS_COSMOSTATION.md && \
    grep -qE "AGENTS_COSMOSTATION" AGENTS.md && \
    grep -qE "verify-issue-640" AGENTS.md && \
    grep -qE "cosmostation-cw20-gitlab-640" docs/integrators.md && \
    grep -qE "verify-issue-640" docs/testing.md && \
    grep -qE "C640-1" docs/qa/issue-640/README.md
  '

echo ""
echo "────────────────────────────────────────────────────────────────"
echo "  $PASS passed, $FAIL failed"
echo "────────────────────────────────────────────────────────────────"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
