#!/usr/bin/env bash
# Automated verification for GitLab #639 — listing venue catalog.
#
# Proves (docs + catalog lockstep; no LocalTerra / no GitHub PR / no form submit):
#   L639-1  catalog only — no new indexer API claimed
#   L639-2  permanent six match Keplr pack; gems excluded
#   L639-3  exchange form drafts pin indexer.dex.cl8y.com /cg and /cmc
#   L639-4  owned surfaces #631 / #629 / #224 do_not_reopen
#   L639-5  UST1 unstablecoin; USTR not a stablecoin; one CG id
#   L639-6  skip list (Coinhall, DexScreener, LuncScan Telegram, …)
#   L639-7  no-CI-PR + human-gate language
#   L639-8  skill + AGENTS.md + verify target
#
# Refs: skills/AGENTS_LISTINGS.md
#       docs/listings/README.md
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
echo "  GitLab #639 — listing venue catalog"
echo "════════════════════════════════════════════════════════════════"

run_step "catalog: pins, Keplr lockstep, forms, skip list" \
  python3 scripts/qa/listings_catalog_validate.py

run_step "docs: listing catalog invariants L639-1–L639-8" \
  bash -c '
    grep -qE "\*\*L639-1\*\*" docs/listings/README.md && \
    grep -qE "\*\*L639-8\*\*" docs/listings/README.md && \
    grep -qE "make verify-issue-639" docs/listings/README.md && \
    grep -qE "unstablecoin" docs/listings/README.md && \
    grep -qE "Not a stablecoin|not a stablecoin" docs/listings/README.md && \
    grep -qE "indexer.dex.cl8y.com/cg/" docs/listings/README.md && \
    grep -qE "indexer.dex.cl8y.com/cmc/" docs/listings/README.md && \
    grep -qE "ceramicliberty-com" docs/listings/README.md && \
    grep -qE "Coinhall" docs/listings/README.md && \
    grep -qE "DexScreener" docs/listings/README.md && \
    grep -qE "archived" docs/listings/README.md && \
    ! grep -qE "https?://(pro-)?api\.coingecko\.com" docs/listings/forms/*.md
  '

run_step "docs: form drafts + no new indexer API" \
  bash -c '
    grep -qE "does \*\*not\*\* implement a new indexer API|does \*\*not\*\* add indexer" \
      docs/listings/README.md && \
    test -f docs/listings/forms/coingecko-exchange.md && \
    test -f docs/listings/forms/coingecko-terra-classic-platform.md && \
    test -f docs/listings/forms/coinmarketcap-exchange.md && \
    grep -qE "L639-3" docs/listings/forms/README.md && \
    grep -qE "listing-venue-catalog-gitlab-639" docs/integrators.md && \
    grep -qE "AGENTS_LISTINGS" docs/integrators.md && \
    grep -qE "listings/README" docs/README.md && \
    grep -qE "verify-issue-639" docs/testing.md && \
    grep -qE "L639-1" docs/qa/issue-639/README.md && \
    grep -qE "listings/README|listing venue catalog|#639" docs/CG_CMC_COMPLIANCE.md && \
    grep -qE "#639" docs/indexer-invariants.md
  '

run_step "docs: skill + AGENTS.md playbook #639" \
  bash -c '
    grep -qE "\*\*L639-1" skills/AGENTS_LISTINGS.md && \
    grep -qE "\*\*L639-8" skills/AGENTS_LISTINGS.md && \
    grep -qE "make verify-issue-639" skills/AGENTS_LISTINGS.md && \
    grep -qE "AGENTS_LISTINGS" AGENTS.md && \
    grep -qE "verify-issue-639" AGENTS.md && \
    grep -qE "AGENTS_LISTINGS|#639" skills/AGENTS_KEPLR_CW20_REGISTRY.md && \
    grep -qE "AGENTS_LISTINGS|#639" skills/AGENTS_DEFILLAMA.md
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
