#!/usr/bin/env bash
# Verification for GitLab #328 — pair search degraded typed search + token name tier 2 QA.
#
# Layers:
#   1. Frontend unit tests (pairSearchQuery + PairSearchSelect degraded)
#   2. Live indexer (optional): GET /api/v1/pairs?q=Ember&sort=relevance returns hits with non-empty asset names
#
# Refs: docs/frontend.md § Pair search combobox, issue #328
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
SKIP=0
declare -a RESULTS=()

ok()   { RESULTS+=("PASS  $1"); PASS=$((PASS+1)); echo "  [PASS] $1"; }
bad()  { RESULTS+=("FAIL  $1"); FAIL=$((FAIL+1)); echo "  [FAIL] $1" >&2; }
skip() { RESULTS+=("SKIP  $1"); SKIP=$((SKIP+1)); echo "  [SKIP] $1"; }

INDEXER="${VERIFY328_INDEXER_URL:-http://127.0.0.1:3001}"

echo "════════════════════════════════════════════════════════════════"
echo "  GitLab #328 — pair search degraded + token name relevance"
echo "════════════════════════════════════════════════════════════════"

echo ""
echo "[1] Frontend unit tests (pair search)"
if (cd frontend-dapp && npm run test:run -- src/utils/__tests__/pairSearchQuery.test.ts src/components/trade/__tests__/PairSearchSelect.degraded.test.tsx); then
  ok "pairSearchQuery + PairSearchSelect.degraded Vitest"
else
  bad "pairSearchQuery + PairSearchSelect.degraded Vitest"
fi

echo ""
echo "[2] Indexer pair search API — token name tier (q=Ember)"
if ! command -v curl >/dev/null 2>&1; then
  skip "curl not installed"
elif ! curl -sf "${INDEXER}/health" >/dev/null 2>&1; then
  skip "indexer not reachable at ${INDEXER} (start indexer after deploy-local)"
else
  RESP="$(curl -sf "${INDEXER}/api/v1/pairs?q=Ember&sort=relevance&limit=5" 2>/dev/null || true)"
  if [[ -z "$RESP" ]]; then
    bad "GET /api/v1/pairs?q=Ember failed"
  elif command -v jq >/dev/null 2>&1; then
    COUNT="$(echo "$RESP" | jq '[.items[] | select(
      (.asset_0.name // "" | test("ember"; "i")) or
      (.asset_1.name // "" | test("ember"; "i")) or
      (.asset_0.symbol // "" | test("ember"; "i")) or
      (.asset_1.symbol // "" | test("ember"; "i"))
    )] | length')"
    if [[ "$COUNT" -ge 1 ]]; then
      ok "indexer q=Ember relevance returned ${COUNT} matching pair(s)"
    else
      bad "indexer q=Ember returned no pairs with Ember name/symbol metadata (re-index assets after deploy)"
      echo "$RESP" | jq '.items[0] // empty' 2>/dev/null || true
    fi
  else
    if echo "$RESP" | grep -qi ember; then
      ok "indexer q=Ember response contains ember (jq not installed — shallow check)"
    else
      bad "indexer q=Ember response has no ember match (install jq for strict check)"
    fi
  fi
fi

echo ""
echo "────────────────────────────────────────────────────────────────"
printf 'Summary: %s PASS, %s FAIL, %s SKIP\n' "$PASS" "$FAIL" "$SKIP"
for r in "${RESULTS[@]}"; do echo "  $r"; done
echo "────────────────────────────────────────────────────────────────"

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
