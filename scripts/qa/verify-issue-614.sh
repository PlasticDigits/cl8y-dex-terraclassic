#!/usr/bin/env bash
# Verification for GitLab #614: /protocol UST1 window mint/redeem treasury fees.
#
# Requires: make setup-indexer-postgres (Postgres + indexer/.env; no wasm deploy).
# Worktree frontend: sibling node_modules symlink or npm ci.
# Optional live leftover: VERIFY614_REQUIRE_LIVE=1 (FAIL if pin/events missing).
# VERIFY614_SKIP_LIVE=1 skips the indexer.dex.cl8y.com probe.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS+1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL+1)); echo "  [FAIL] $1" >&2; }
skip() { RESULTS+=("SKIP  $1"); echo "  [SKIP] $1"; }

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

ensure_frontend_node_modules() {
  local sibling
  if [[ -x "$REPO_ROOT/frontend-dapp/node_modules/.bin/vitest" ]]; then
    return 0
  fi
  sibling="$(dirname "$REPO_ROOT")/cl8y-dex-terraclassic/frontend-dapp/node_modules"
  if [[ -x "$sibling/.bin/vitest" ]]; then
    ln -sfn "$sibling" "$REPO_ROOT/frontend-dapp/node_modules"
    echo "[bootstrap] linked frontend-dapp/node_modules from primary checkout"
    return 0
  fi
  echo "[bootstrap] frontend-dapp/node_modules missing — npm ci…"
  bash "$REPO_ROOT/scripts/with-node.sh" --cwd frontend-dapp -- npm ci
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
INDEXER_URL="${VERIFY614_INDEXER_URL:-https://indexer.dex.cl8y.com}"

run_step "docs: PFee-13 + I614 + CHECK + pin + crosslinks" \
  bash -c '
    set -euo pipefail
    grep -q "PFee-13" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "ust1_mint" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "PFee-13" skills/AGENTS_UST1_WINDOW_UI.md
    grep -q "I614-1" skills/AGENTS_INDEXER_UST1_WINDOW_FEES.md
    grep -q "I614-8" skills/AGENTS_INDEXER_UST1_WINDOW_FEES.md
    grep -q "AGENTS_INDEXER_UST1_WINDOW_FEES" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "AGENTS_INDEXER_UST1_WINDOW_FEES" AGENTS.md
    grep -q "Protocol fees (#586)" docs/indexer-invariants.md
    grep -q "ust1_mint" docs/indexer-invariants.md
    grep -q "PFee-13" docs/indexer-invariants.md
    grep -q "I614-1" docs/indexer-invariants.md
    grep -q "UST1_WINDOW_ADDRESS" docs/runbooks/overview-global-stats-brin.md
    grep -q "verify-issue-614" AGENTS.md
    grep -q "PFee-13" docs/frontend.md
    grep -q "PFee-1–PFee-12" docs/frontend.md
    grep -q "UST1_WINDOW_ADDRESS" indexer/.env.example
    grep -q "UST1_WINDOW_ADDRESS" deployments/mainnet-ust1-wrap/coolify.env.example
    grep -q "UST1_WINDOW_ADDRESS" deployments/mainnet-ust1-wrap/REGISTRY.md
    grep -q "11618" deployments/mainnet-ust1-wrap/REGISTRY.md
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

ensure_frontend_node_modules

run_step "frontend: Protocol RTL including UST1 mint/redeem labels" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run src/pages/ProtocolPage.test.tsx'

run_live_ingest() {
  set -euo pipefail
  local fees mint_n redeem_n
  fees="$(curl -fsS --max-time 20 "${INDEXER_URL%/}/api/v1/protocol/fees?window=24h")"
  echo "$fees" | jq -e '.ust1_window_configured == true' >/dev/null
  mint_n="$(echo "$fees" | jq -r '[.by_source[]? | select(.source=="ust1_mint") | .event_count] | add // 0')"
  redeem_n="$(echo "$fees" | jq -r '[.by_source[]? | select(.source=="ust1_redeem") | .event_count] | add // 0')"
  echo "live: ust1_window_configured=true ust1_mint=$mint_n ust1_redeem=$redeem_n"
  if [[ "$mint_n" == "null" || "$mint_n" -lt 1 ]]; then
    echo "no captured ust1_mint events" >&2
    return 1
  fi
  if [[ "$redeem_n" == "null" || "$redeem_n" -lt 1 ]]; then
    echo "no captured ust1_redeem events" >&2
    return 1
  fi
  echo "$fees" | jq -e '[.by_source[]? | select(.source=="ust1_mint" or .source=="ust1_redeem") | .amount_usd] | map(select(. != null and . != "0")) | length >= 1' >/dev/null
}

if [[ "${VERIFY614_SKIP_LIVE:-}" == "1" ]]; then
  echo ""
  echo "[live: Coolify pin + mint/redeem event_count] skipped (VERIFY614_SKIP_LIVE=1)"
  skip "live: Coolify pin + mint/redeem event_count"
elif ! curl -fsS --max-time 10 "${INDEXER_URL%/}/api/v1/protocol/fees?window=24h" >/dev/null; then
  if [[ "${VERIFY614_REQUIRE_LIVE:-}" == "1" ]]; then
    run_step "live: Coolify pin + mint/redeem event_count" run_live_ingest
  else
    echo ""
    echo "[live: Coolify pin + mint/redeem event_count] SKIP (indexer unreachable)"
    skip "live: Coolify pin + mint/redeem event_count"
  fi
else
  run_step "live: Coolify pin + mint/redeem event_count" run_live_ingest
fi

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
