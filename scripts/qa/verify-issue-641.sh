#!/usr/bin/env bash
# Automated verification for GitLab #641 — Hexxagon / Galaxy Station CW20 pack.
#
# Proves (docs + fragment lockstep; no LocalTerra / no GitHub PR):
#   H641-1  file is cw20/tokens/mainnet/terra.js (Classic, not phoenix)
#   H641-2  five submit rows; USTR omitted (already live); gems excluded
#   H641-3  schema protocol/symbol/name/token/icon/decimals; coinGeckoID Hexxagon spelling
#   H641-4  coinGeckoID ceramicliberty-com on CL8Y only
#   H641-5  UST1 unstablecoin; USTR not a stablecoin (verify-only)
#   H641-6  do not PR terra-money/assets
#   H641-7  no-CI-PR
#   H641-8  skill + README + verify target
#
# Refs: skills/AGENTS_HEXXAGON.md
#       docs/listings/hexxagon/README.md
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
echo "  GitLab #641 — Hexxagon / Galaxy Station CW20 pack"
echo "════════════════════════════════════════════════════════════════"

run_step "pack: schema, Keplr lockstep, USTR omitted, gems excluded" \
  python3 scripts/qa/hexxagon_cw20_validate.py

run_step "docs: invariants H641-1–H641-8" \
  bash -c '
    grep -qE "\*\*H641-1\*\*" docs/listings/hexxagon/README.md && \
    grep -qE "\*\*H641-8\*\*" docs/listings/hexxagon/README.md && \
    grep -qE "cw20/tokens/mainnet/terra.js" docs/listings/hexxagon/README.md && \
    grep -qE "already listed|already live" docs/listings/hexxagon/README.md && \
    grep -qE "unstablecoin" docs/listings/hexxagon/README.md && \
    grep -qE "terra-money/assets" docs/listings/hexxagon/README.md && \
    grep -qE "make verify-issue-641" docs/listings/hexxagon/README.md && \
    grep -qE "hexxagon-io/chain-registry/pull/68" docs/listings/hexxagon/README.md
  '

run_step "docs: skill + AGENTS.md + integrators + QA" \
  bash -c '
    grep -qE "\*\*H641-1" skills/AGENTS_HEXXAGON.md && \
    grep -qE "\*\*H641-8" skills/AGENTS_HEXXAGON.md && \
    grep -qE "make verify-issue-641" skills/AGENTS_HEXXAGON.md && \
    grep -qE "AGENTS_LISTINGS|#639" skills/AGENTS_HEXXAGON.md && \
    grep -qE "AGENTS_COSMOSTATION|#640" skills/AGENTS_HEXXAGON.md && \
    grep -qE "AGENTS_HEXXAGON" AGENTS.md && \
    grep -qE "verify-issue-641" AGENTS.md && \
    grep -qE "hexxagon-cw20-gitlab-641" docs/integrators.md && \
    grep -qE "verify-issue-641" docs/testing.md && \
    grep -qE "H641-1" docs/qa/issue-641/README.md && \
    grep -qE "hexxagon-io/chain-registry/pull/68" docs/qa/issue-641/README.md
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
