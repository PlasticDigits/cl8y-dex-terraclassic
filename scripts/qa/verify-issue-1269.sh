#!/usr/bin/env bash
# Verification for GitLab / Forgejo #1269: persist every multihop AMM hop in protocol_fee_events.
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
echo "  GitLab #1269 — persist every multihop AMM hop in protocol_fee_events"
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
    test -f skills/AGENTS_INDEXER_PROTOCOL_FEE_HOPS.md
    grep -q "F1269-1" skills/AGENTS_INDEXER_PROTOCOL_FEE_HOPS.md
    grep -q "F1269-8" skills/AGENTS_INDEXER_PROTOCOL_FEE_HOPS.md
    grep -q "PFee-14" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
    grep -q "AGENTS_INDEXER_PROTOCOL_FEE_HOPS" AGENTS.md
    grep -q "verify-issue-1269" AGENTS.md
    grep -q "verify-issue-1269" Makefile
    grep -q "Protocol fees (#586)" docs/indexer-invariants.md
    grep -q "#1269" docs/indexer-invariants.md
    grep -q "protocol_fee_events_pair_tx_source_ordinal_uidx" docs/indexer-invariants.md
    grep -q "#1269" docs/runbooks/overview-global-stats-brin.md
    grep -q "#1269" docs/runbooks/indexer-reorg-replay-dedup.md
    grep -q "widened" skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md
    test -f indexer/migrations/20260916120000_protocol_fee_events_pair_id.sql
    grep -q "F1269" docs/testing.md
    grep -q "PFee-14" docs/frontend.md
    grep -q "ADR 0005" docs/adr/0005-protocol-fee-multihop-hops.md
    grep -q "protocol_fee_events_pair_tx_source_ordinal_uidx" docs/adr/0005-protocol-fee-multihop-hops.md
    grep -q "Indexer protocol fee ledger" docs/architecture.md
    grep -q "20260916120000_usdt_quote_usd_null_backfill" docs/adr/0005-protocol-fee-multihop-hops.md
    grep -q "sqlx version leftover" docs/adr/0005-protocol-fee-multihop-hops.md
  '

run_step "source: pair-scoped unique; GET does not SUM events; no DO UPDATE" \
  bash -c '
    set -euo pipefail
    grep -q "protocol_fee_events_pair_tx_source_ordinal_uidx" \
      indexer/migrations/20260916120000_protocol_fee_events_pair_id.sql
    grep -q "protocol_fee_events_nopair_tx_source_ordinal_uidx" \
      indexer/migrations/20260916120000_protocol_fee_events_pair_id.sql
    grep -q "ON CONFLICT DO NOTHING" indexer/src/db/queries/protocol_fees.rs
    if grep -nE "ON CONFLICT[^\\n]*DO UPDATE SET" indexer/src/db/queries/protocol_fees.rs \
         indexer/migrations/20260916120000_protocol_fee_events_pair_id.sql 2>/dev/null; then
      echo "DO UPDATE is forbidden on protocol_fee_events" >&2
      exit 1
    fi
    grep -q ingest_swap_amm_fee indexer/src/indexer/parser.rs
    grep -q backfill_missing_swap_amm_fees indexer/src/db/queries/protocol_fees.rs
    grep -q hop_collision_distinct_pairs_both_persist indexer/tests/indexer_protocol_fees.rs
    grep -q wrap_two_ordinals_persist_and_replay_dedup indexer/tests/indexer_protocol_fees.rs
    grep -q hybrid_counts_amm_and_book_once indexer/tests/indexer_protocol_fees.rs
    grep -q parse_swaps_assigns_per_pair_swap_index indexer/src/indexer/parser.rs
    grep -q "GitLab #1269" indexer/src/indexer/parser.rs
    if grep -nE "SUM\\(.*protocol_fee_events|FROM protocol_fee_events" \
         indexer/src/api/overview.rs indexer/src/api/protocol_fees.rs 2>/dev/null; then
      echo "GET handlers must not SUM protocol_fee_events" >&2
      exit 1
    fi
    # Old three-column unique must not be the sole remaining insert conflict target.
    if grep -nF "ON CONFLICT (tx_hash, source, ordinal) DO NOTHING" \
         indexer/src/db/queries/protocol_fees.rs 2>/dev/null; then
      echo "insert_fee_event still uses the colliding 3-column unique" >&2
      exit 1
    fi
  '

run_step "indexer lib: protocol_fees math + wrap pin" \
  bash -c 'cd indexer && cargo test --lib protocol_fees -- --quiet'

run_step "indexer lib: parser per-pair swap_index still restarts at 0" \
  bash -c 'cd indexer && cargo test --lib parse_swaps_assigns_per_pair_swap_index -- --quiet'

run_step "indexer integration: hop insert + replay + wrap uniqueness + backfill" \
  bash -c 'cd indexer && cargo test --test indexer_protocol_fees -- --test-threads=1 --quiet'

if [[ "${VERIFY_ISSUE_1269_SKIP_RELATED:-}" == "1" ]]; then
  echo ""
  echo "[related 586/613/614/683] skipped (VERIFY_ISSUE_1269_SKIP_RELATED=1)"
  ok "related verifies (skipped)"
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
