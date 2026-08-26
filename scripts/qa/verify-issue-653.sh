#!/usr/bin/env bash
# Automated verification for GitLab #653 — one chrome layer / anti-nesting.
#
# Proves (unit + docs; no chain required):
#   1. StatBox default stays card-glass; variant=flat does not.
#   2. Charts overview + Trader summary tiles are flat (testids unchanged).
#   3. check_chrome_nesting.py + design tokens green.
#   4. Docs/skills C653-1–C653-8 + allowlist; no blanket nested-card yes.
#   5. Swap IO cards still card-glass; Trade chart not wrapped in card-glass.
#
# Refs: skills/AGENTS_FRONTEND_CHROME_NESTING.md,
#       frontend-dapp/src/components/ui/StatBox.tsx,
#       docs/frontend.md § One chrome layer
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
echo "  GitLab #653 — one chrome layer / anti-nesting"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: StatBox card vs flat" \
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/components/ui/__tests__/StatBox.test.tsx

run_step "frontend: Charts overview + Trader summary chrome" \
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/ChartsPage.test.tsx \
    src/components/trader/TraderSummaryStats.test.tsx

run_step "frontend: Protocol page stats still render" \
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/ProtocolPage.test.tsx

run_step "code: metric grids pass variant=flat" \
  bash -c 'grep -qE "variant=\"flat\"" frontend-dapp/src/pages/ChartsPage.tsx && \
  grep -qE "variant=\"flat\"" frontend-dapp/src/components/trader/TraderSummaryStats.tsx && \
  grep -qE "variant=\"flat\"" frontend-dapp/src/components/protocol/ProtocolGlobalStats.tsx && \
  grep -qE "variant=\"flat\"" frontend-dapp/src/components/protocol/ProtocolFeeStats.tsx && \
  grep -qE "variant=\"flat\"" frontend-dapp/src/components/protocol/ProtocolOracleCard.tsx && \
  ! grep -qE "card-glass" frontend-dapp/src/pages/ChartsPage.tsx && \
  ! grep -qE "card-glass" frontend-dapp/src/components/trader/TraderSummaryStats.tsx && \
  ! grep -qE "card-glass" frontend-dapp/src/components/protocol/ProtocolGlobalStats.tsx && \
  ! grep -qE "card-glass" frontend-dapp/src/components/protocol/ProtocolFeeStats.tsx'

run_step "code: Swap IO cards still card-glass" \
  bash -c 'grep -qE "card-glass swap-io-card-pay" frontend-dapp/src/pages/SwapPage.tsx && \
  grep -qE "card-glass swap-io-card-receive" frontend-dapp/src/pages/SwapPage.tsx'

run_step "code: TradeChartSlot does not wrap PriceChart in card-glass" \
  bash -c '! awk "/function TradeChartSlot/,/^export default function TradePage/" frontend-dapp/src/pages/TradePage.tsx | grep -q "card-glass"'

run_step "guard: check_chrome_nesting.py" \
  python3 scripts/check_chrome_nesting.py

run_step "design tokens" \
  python3 scripts/check_design_tokens.py

run_step "docs: frontend.md C653-1–C653-8 + L561-2 points at global §" \
  bash -c 'grep -qE "one-chrome-layer" docs/frontend.md && \
  grep -qE "\*\*C653-1\*\*" docs/frontend.md && \
  grep -qE "\*\*C653-8\*\*" docs/frontend.md && \
  grep -qE "one-chrome-layer" docs/frontend.md && \
  grep -qE "#653" docs/frontend.md && \
  grep -qE "C653" docs/frontend.md'

run_step "docs: design-system principle is allowlist, not blanket yes" \
  bash -c 'grep -qE "One chrome layer per region" docs/design-system.md && \
  grep -qE "#653" docs/design-system.md && \
  ! grep -qE "is OK for distinct inner blocks" docs/design-system.md'

run_step "skill: AGENTS_FRONTEND_CHROME_NESTING C653 + verify" \
  bash -c 'grep -qE "\*\*C653-1" skills/AGENTS_FRONTEND_CHROME_NESTING.md && \
  grep -qE "\*\*C653-8" skills/AGENTS_FRONTEND_CHROME_NESTING.md && \
  grep -qE "make verify-issue-653" skills/AGENTS_FRONTEND_CHROME_NESTING.md && \
  grep -qE "variant=\"flat\"" skills/AGENTS_FRONTEND_CHROME_NESTING.md'

run_step "skill: design-system rule 3 + trade L561-2 crosslink #653" \
  bash -c 'grep -qE "#653" skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md && \
  grep -qE "AGENTS_FRONTEND_CHROME_NESTING" skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md && \
  ! grep -qE "is OK for distinct inner blocks" skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md && \
  grep -qE "#653" skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md && \
  grep -qE "one-chrome-layer" skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md'

run_step "QA_TEMPLATE §10 nesting checkbox" \
  bash -c 'grep -qE "10.2.16" QA_TEMPLATE.md && grep -qE "#653" QA_TEMPLATE.md'

run_step "AGENTS.md + testing.md playbook" \
  bash -c 'grep -qE "AGENTS_FRONTEND_CHROME_NESTING|#653" AGENTS.md && \
  grep -qE "verify-issue-653" AGENTS.md && \
  grep -qE "verify-issue-653" docs/testing.md'

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
echo "==> GitLab #653 verification passed"
