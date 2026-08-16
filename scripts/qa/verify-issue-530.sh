#!/usr/bin/env bash
# Automated verification for GitLab #530 — My open limits Cancel vs stale ● row.
#
# Proves (unit + docs; no chain required):
#   1. Open-row reconcile: Active / ParkedRefund / Unknown+fill / Unknown+gone / LCD fail ≠ Unknown.
#   2. Panel: report-class Filled fixture, already-cancelled label, parked Claim, pause/restricted.
#   3. Cancel mutation: indexed cancel + LCD Unknown/ParkedRefund do not broadcast.
#   4. queryOrderStatus payload is order_id only (invert-safe).
#   5. /trade compact panel is above sticky Place limit (not a child).
#   6. Docs/skills F530-1–F530-8 crosslinked; AGENTS playbook present.
#
# Refs: skills/AGENTS_FRONTEND_LIMIT_CANCEL_OPEN.md,
#       frontend-dapp/src/utils/limitPlacementOpenReconcile.ts,
#       docs/limit-orders.md § Open-row Cancel reconciliation
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
echo "  GitLab #530 — My open limits Cancel / stale ● row"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: reconcile + lifecycle + cancel mutation + pair OrderStatus" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/limitPlacementOpenReconcile.test.ts \
    src/utils/__tests__/limitPlacementLifecycle.test.ts \
    src/hooks/__tests__/useLimitOrderCancelMutation.test.tsx \
    src/services/terraclassic/__tests__/pair.test.ts -t "530"'

run_step "frontend: placements panel + /trade compact above footer" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/components/trade/__tests__/LimitOrderMyPlacementsPanel.test.tsx \
    src/pages/TradePage.test.tsx -t "530|GitLab #530"'

run_step "code: queryOrderStatus + cancel_limit_order.order_id only" \
  grep -qE 'order_status: \{ order_id: orderId \}' frontend-dapp/src/services/terraclassic/pair.ts && \
  grep -qE 'cancel_limit_order: \{ order_id: orderId \}' frontend-dapp/src/services/terraclassic/pair.ts && \
  grep -qE 'parsePairOrderStatus' frontend-dapp/src/services/terraclassic/pair.ts

run_step "code: compact panel above ticket footer (not a child)" \
  grep -qE 'trade-ticket-placements-anchor' frontend-dapp/src/components/trade/TradeOrderTicket.tsx && \
  python3 - <<'PY'
from pathlib import Path
text = Path("frontend-dapp/src/components/trade/TradeOrderTicket.tsx").read_text()
footer = Path("frontend-dapp/src/components/trade/TradeTicketSubmitFooter.tsx").read_text()
a = text.find('data-testid="trade-ticket-placements-anchor"')
s = text.find("<TradeTicketSubmitFooter")
if a < 0 or s < 0 or a > s:
    raise SystemExit("placements anchor must appear before TradeTicketSubmitFooter in TradeOrderTicket.tsx")
if "trade-ticket-submit-footer" not in footer:
    raise SystemExit("TradeTicketSubmitFooter must keep trade-ticket-submit-footer")
if "trade-limit-submit-sticky" in text:
    raise SystemExit("do not resurrect trade-limit-submit-sticky after #527")
PY

run_step "docs: limit-orders.md F530-1–F530-8" \
  grep -qE 'open-row-cancel-reconciliation-gitlab-530' docs/limit-orders.md && \
  grep -qE '\*\*F530-1\*\*' docs/limit-orders.md && \
  grep -qE '\*\*F530-8\*\*' docs/limit-orders.md

run_step "docs: frontend.md open-limits-cancel-reconciliation" \
  grep -qE 'open-limits-cancel-reconciliation' docs/frontend.md && \
  grep -qE 'queryOrderStatus' docs/frontend.md

run_step "skill: AGENTS_FRONTEND_LIMIT_CANCEL_OPEN" \
  grep -qE '\*\*F530-1' skills/AGENTS_FRONTEND_LIMIT_CANCEL_OPEN.md && \
  grep -qE 'queryOrderStatus|useLimitOrderStatuses' skills/AGENTS_FRONTEND_LIMIT_CANCEL_OPEN.md && \
  grep -qE 'make verify-issue-530' skills/AGENTS_FRONTEND_LIMIT_CANCEL_OPEN.md

run_step "skill: OrderStatus + parked + CTA dock crosslinks #530" \
  grep -qE 'AGENTS_FRONTEND_LIMIT_CANCEL_OPEN|#530' skills/AGENTS_ORDER_STATUS_QUERY.md && \
  grep -qE 'AGENTS_FRONTEND_LIMIT_CANCEL_OPEN|#530' skills/AGENTS_FRONTEND_LIMIT_PARKED_EXPIRED.md && \
  grep -qE 'AGENTS_FRONTEND_LIMIT_CANCEL_OPEN|#530' skills/AGENTS_FRONTEND_TRADE_TICKET_CTA_DOCK.md

run_step "AGENTS.md playbook link #530" \
  grep -qE 'AGENTS_FRONTEND_LIMIT_CANCEL_OPEN|#530' AGENTS.md && \
  grep -qE 'verify-issue-530' AGENTS.md

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
