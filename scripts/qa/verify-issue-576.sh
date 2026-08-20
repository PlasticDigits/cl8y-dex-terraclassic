#!/usr/bin/env bash
# Verification for GitLab #576: trailing 24h/7d/30d volume copy (not calendar-day reset).
#
# Frontend-only (no chain / Postgres required).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS+1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL+1)); echo "  [FAIL] $1" >&2; }

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
echo "  GitLab #576 — trailing 24h volume copy (not midnight reset)"
echo "════════════════════════════════════════════════════════════════"

run_step "docs: invariants + skill + AGENTS crosslinks" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_FRONTEND_TRAILING_WINDOW.md
    grep -q "Trailing window copy (#576)" docs/indexer-invariants.md
    grep -q "AGENTS_FRONTEND_TRAILING_WINDOW" AGENTS.md
    grep -q "verify-issue-576" AGENTS.md
    grep -q "W1" skills/AGENTS_FRONTEND_TRAILING_WINDOW.md
    grep -q "W5" skills/AGENTS_FRONTEND_TRAILING_WINDOW.md
    grep -q "trailingWindowCopy" docs/frontend.md
    grep -q "Last 24h Vol" docs/frontend.md
    grep -q "not a midnight reset" docs/design-system.md
    grep -q "#576" skills/AGENTS_FRONTEND_CHARTS_OVERVIEW.md
    grep -q "#576" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "#576" skills/AGENTS_FRONTEND_CHARTS_PAIR_STATS.md
    grep -q "#576" skills/AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md
    grep -q "#576" skills/AGENTS_FRONTEND_POOL_TABLE.md
    grep -q "verify-issue-576" Makefile
  '

run_step "code: shared copy + StatBox title/aria; no calendar-day window" \
  bash -c '
    set -euo pipefail
    grep -q "TRAILING_24H_VOLUME_TITLE" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "TRAILING_24H_VOLUME_LABEL" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "TRAILING_24H_TRADES_LABEL" frontend-dapp/src/pages/ChartsPage.tsx
    grep -q "TRAILING_24H_VOLUME_TITLE" frontend-dapp/src/components/protocol/ProtocolGlobalStats.tsx
    grep -q "TRAILING_7D_VOLUME_TITLE" frontend-dapp/src/components/protocol/ProtocolGlobalStats.tsx
    grep -q "TRAILING_30D_VOLUME_TITLE" frontend-dapp/src/components/protocol/ProtocolGlobalStats.tsx
    grep -q "POOL_VOL_HEADER_TITLE" frontend-dapp/src/components/pool/PoolPairsTable.tsx
    grep -q "volume_24h" frontend-dapp/src/components/pool/PoolPairsTable.tsx
    grep -q "aria-label={accessibleValue}" frontend-dapp/src/components/ui/StatBox.tsx
    grep -q "title={title}" frontend-dapp/src/components/ui/StatBox.tsx
    ! grep -qE "formatNum\(overview\.total_volume_24h" frontend-dapp/src/pages/ChartsPage.tsx
    ! grep -q "resets at 00:00" frontend-dapp/src/pages/ChartsPage.tsx
    ! grep -q "resets at 00:00" frontend-dapp/src/components/protocol/ProtocolGlobalStats.tsx
    if grep -nE "https?://|VITE_INDEXER_URL" frontend-dapp/src/utils/trailingWindowCopy.ts; then
      echo "trailingWindowCopy must not contain URLs or env names" >&2
      exit 1
    fi
  '

run_step "frontend: trailing copy + Charts + Protocol + StatBox + Pool Vol title" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/trailingWindowCopy.test.ts \
    src/components/ui/__tests__/StatBox.test.tsx \
    src/pages/ChartsPage.test.tsx \
    src/pages/ProtocolPage.test.tsx \
    src/pages/PoolPage.test.tsx'

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
