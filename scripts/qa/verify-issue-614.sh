#!/usr/bin/env bash
# Verification for GitLab #614: /protocol UST1 window mint/redeem treasury fees.
#
# Requires: make setup-indexer-postgres (Postgres + indexer/.env; no wasm deploy).
# Playwright e2e-smoke is optional when frontend-dapp/node_modules exists.
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
echo "  GitLab #614 — UST1 window mint/redeem protocol fees"
echo "════════════════════════════════════════════════════════════════"

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_step "docs: PFee-13 + CHECK + pin + crosslinks" \
  bash -c '
    set -euo pipefail
    grep -q "PFee-13" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "ust1_mint" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "PFee-13" skills/AGENTS_UST1_WINDOW_UI.md
    grep -q "Protocol fees (#586)" docs/indexer-invariants.md
    grep -q "ust1_mint" docs/indexer-invariants.md
    grep -q "PFee-13" docs/indexer-invariants.md
    grep -q "UST1_WINDOW_ADDRESS" docs/runbooks/overview-global-stats-brin.md
    grep -q "verify-issue-614" AGENTS.md
    grep -q "PFee-13" docs/frontend.md
    grep -q "PFee-1–PFee-12" docs/frontend.md
    grep -q "UST1_WINDOW_ADDRESS" indexer/.env.example
    grep -q "UST1_WINDOW_ADDRESS" deployments/mainnet-ust1-wrap/coolify.env.example
    grep -q "UST1_WINDOW_ADDRESS" deployments/mainnet-ust1-wrap/REGISTRY.md
    test -f indexer/migrations/20260824120000_ust1_window_protocol_fees.sql
    grep -q "ust1_mint" indexer/migrations/20260824120000_ust1_window_protocol_fees.sql
    grep -q "ust1_redeem" indexer/migrations/20260824120000_ust1_window_protocol_fees.sql
  '

run_step "source: pin + parse allowlist + omit-if-unconfigured; GET does not scan events" \
  bash -c '
    set -euo pipefail
    grep -q UST1_WINDOW_ADDRESS indexer/src/config.rs
    grep -q parse_ust1_window_address indexer/src/indexer/protocol_fees.rs
    grep -q parse_ust1_window_fees indexer/src/indexer/protocol_fees.rs
    grep -q parse_ust1_window_crate_attrs_without_fee_amount_fail_closed indexer/src/indexer/protocol_fees.rs
    grep -q ust1_window_configured indexer/src/db/queries/protocol_fees.rs
    grep -q ust1_window_configured indexer/src/api/protocol_fees.rs
    grep -q is_ust1_window_family indexer/src/db/queries/protocol_fees.rs
    grep -q "UST1 mint" frontend-dapp/src/components/protocol/ProtocolFeeStats.tsx
    grep -q "UST1 redeem" frontend-dapp/src/components/protocol/ProtocolFeeStats.tsx
    grep -q ust1_window_configured frontend-dapp/src/components/protocol/ProtocolFeeStats.tsx
    if grep -nE "SUM\\(.*protocol_fee_events|FROM protocol_fee_events" \
         indexer/src/api/overview.rs indexer/src/api/protocol_fees.rs 2>/dev/null; then
      echo "GET handlers must not SUM protocol_fee_events" >&2
      exit 1
    fi
    if grep -nE "amount_raw.*=.*fee_total_bps|ust1_out.*fee_bps|fee_cmm_protocol_bps \\*" \
         indexer/src/indexer/protocol_fees.rs; then
      echo "Window parser must not infer fee from bps" >&2
      exit 1
    fi
  '

run_step "indexer lib: protocol_fees window pin + parse" \
  bash -c 'cd indexer && cargo test --lib protocol_fees -- --quiet'

run_step "indexer integration: window ingest + omit + overview keys" \
  bash -c 'cd indexer && cargo test --test indexer_protocol_fees -- --test-threads=1 --quiet'

run_step "frontend: Protocol RTL including UST1 mint/redeem labels" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run src/pages/ProtocolPage.test.tsx'

if [[ "${VERIFY_ISSUE_614_SKIP_RELATED:-}" == "1" ]]; then
  echo ""
  echo "[related verify-issue-586] skipped (VERIFY_ISSUE_614_SKIP_RELATED=1)"
  ok "related verify-issue-586 (skipped)"
else
  run_step "related: verify-issue-586" \
    bash -c 'VERIFY_ISSUE_586_SKIP_E2E=1 VERIFY_ISSUE_586_SKIP_RELATED=1 make verify-issue-586'
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
