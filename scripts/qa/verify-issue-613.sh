#!/usr/bin/env bash
# Verification for GitLab #613: /protocol Wrap/Unwrap ingest — captured mapper attrs.
#
# Requires: make setup-indexer-postgres (Postgres + indexer/.env; no wasm deploy).
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
echo "  GitLab #613 — Wrap/Unwrap protocol-fee ingest (captured attrs)"
echo "════════════════════════════════════════════════════════════════"

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_step "docs: invariants + skill + AGENTS crosslinks" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_INDEXER_WRAP_FEE_INGEST.md
    grep -q "I613-1" skills/AGENTS_INDEXER_WRAP_FEE_INGEST.md
    grep -q "I613-8" skills/AGENTS_INDEXER_WRAP_FEE_INGEST.md
    grep -q "notify_deposit" skills/AGENTS_INDEXER_WRAP_FEE_INGEST.md
    grep -q "AGENTS_INDEXER_WRAP_FEE_INGEST" AGENTS.md
    grep -q "verify-issue-613" AGENTS.md
    grep -q "verify-issue-613" Makefile
    grep -q "Protocol fees (#586)" docs/indexer-invariants.md
    grep -q "I613-1–I613-8" docs/indexer-invariants.md
    grep -q "notify_deposit" docs/indexer-invariants.md
    grep -q "I613-1–I613-8" docs/runbooks/overview-global-stats-brin.md
    grep -q "I613" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "AGENTS_INDEXER_WRAP_FEE_INGEST" skills/AGENTS_WRAP_MAPPER_SPLIT_FEES.md
    grep -q "GitLab #613" docs/frontend.md
  '

run_step "source: captured attrs + segment scan; GET does not SUM events" \
  bash -c '
    set -euo pipefail
    grep -q notify_deposit indexer/src/indexer/protocol_fees.rs
    grep -q parse_wrap_flattened_combo_keeps_wrap_when_last_action_is_swap indexer/src/indexer/protocol_fees.rs
    grep -q parse_unwrap_captured_fee_not_tax_amount indexer/src/indexer/protocol_fees.rs
    grep -q parse_wrap_never_infers_amount_times_bps indexer/src/indexer/protocol_fees.rs
    grep -q captured_wrap_ingest_rollup_and_get indexer/tests/indexer_protocol_fees.rs
    grep -q unconfigured_mapper_omits_wrap_family indexer/tests/indexer_protocol_fees.rs
    grep -q "GitLab #613" frontend-dapp/src/pages/ProtocolPage.test.tsx
    if grep -nE "SUM\\(.*protocol_fee_events|FROM protocol_fee_events" \
         indexer/src/api/overview.rs indexer/src/api/protocol_fees.rs 2>/dev/null; then
      echo "GET handlers must not SUM protocol_fee_events" >&2
      exit 1
    fi
  '

run_step "indexer lib: wrap pin + captured / flattened parse" \
  bash -c 'cd indexer && cargo test --lib protocol_fees -- --quiet'

run_step "indexer integration: wrap ingest + rollup + GET + omit + decay" \
  bash -c 'cd indexer && cargo test --test indexer_protocol_fees -- --test-threads=1 --quiet'

run_step "frontend: Protocol Wrap/Unwrap idle-hide + event_count>0" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run src/pages/ProtocolPage.test.tsx'

if [[ "${VERIFY_ISSUE_613_SKIP_586:-}" == "1" ]]; then
  echo ""
  echo "[related 586] skipped (VERIFY_ISSUE_613_SKIP_586=1)"
  ok "related verify-issue-586 (skipped)"
else
  run_step "related: verify-issue-586 (skip nested e2e/related)" \
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
