#!/usr/bin/env bash
# Automated verification for Forgejo #1213 — fee-ledger home (docs/trail).
#
# Proves (docs lockstep; no LocalTerra / no FeeSource ingest):
#   L1213-1  indexer is the ledger; marketing is not a fee service
#   L1213-2  pair_creation ingest is #1209; no instantiate-gas
#   L1213-3  SKU/settings invoices are #1210; catalog ≠ fees
#   L1213-4  cohort split is #1211; unregistered tier 0 is not MM
#   L1213-5  marketing #1 / #4 / #5 named as closed trackers
#   L1213-6  GET O(1) / omit-unconfigured / pin language
#   L1213-7  no #1204 / #1202 expansion; no fourth ingest copy
#   L1213-8  skill + AGENTS.md + verify target
#
# Refs: skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md
#       docs/qa/issue-1213/README.md
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
echo "  Forgejo #1213 — fee-ledger home"
echo "════════════════════════════════════════════════════════════════"

run_step "skill: home map + L1213-1–L1213-8 + children" \
  bash -c '
    grep -qE "\*\*L1213-1\*\*" skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md && \
    grep -qE "\*\*L1213-8\*\*" skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md && \
    grep -qE "make verify-issue-1213" skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md && \
    grep -qE "issues/1209" skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md && \
    grep -qE "issues/1210" skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md && \
    grep -qE "issues/1211" skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md && \
    grep -qE "cl8y-marketing/issues/1" skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md && \
    grep -qE "cl8y-marketing/issues/4" skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md && \
    grep -qE "cl8y-marketing/issues/5" skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md && \
    grep -qE "does \*\*not\*\* add a \`FeeSource\`|does \*\*not\*\* add a \*\*FeeSource" \
      skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md && \
    grep -qE "instantiate gas" skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md && \
    grep -qE "community_token_events" skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md && \
    grep -qE "tier_id = 0" skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md && \
    grep -qE "stay \*\*closed\*\*" skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md && \
    grep -qE "O\(1\) rollup" skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md && \
    grep -qE "issues/1204" skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md && \
    grep -qE "issues/1202" skills/AGENTS_INDEXER_FEE_LEDGER_HOME.md
  '

run_step "docs: invariants + audit item 11 + QA" \
  bash -c '
    grep -qE "Fee-ledger home \(#1213\)" docs/indexer-invariants.md && \
    grep -qE "L1213-1" docs/indexer-invariants.md && \
    grep -qE "issues/1209" docs/indexer-invariants.md && \
    grep -qE "issues/1210" docs/indexer-invariants.md && \
    grep -qE "issues/1211" docs/indexer-invariants.md && \
    grep -qE "AGENTS_INDEXER_FEE_LEDGER_HOME" docs/indexer-invariants.md && \
    grep -qE "1209" docs/audits/factory-treasury-bank-send.md && \
    grep -qE "1213" docs/audits/factory-treasury-bank-send.md && \
    grep -qE "L1213-1" docs/qa/issue-1213/README.md && \
    grep -qE "does \*\*not\*\* implement" docs/qa/issue-1213/README.md && \
    grep -qE "verify-issue-1213" docs/testing.md && \
    grep -qE "AGENTS_INDEXER_FEE_LEDGER_HOME" docs/testing.md && \
    grep -qE "AGENTS_INDEXER_FEE_LEDGER_HOME" docs/README.md
  '

run_step "docs: skill + AGENTS.md playbook #1213" \
  bash -c '
    grep -qE "AGENTS_INDEXER_FEE_LEDGER_HOME" AGENTS.md && \
    grep -qE "verify-issue-1213" AGENTS.md && \
    grep -qE "L1213-1" AGENTS.md && \
    grep -qE "AGENTS_INDEXER_FEE_LEDGER_HOME|#1213" skills/AGENTS_INDEXER_WRAP_FEE_INGEST.md && \
    grep -qE "AGENTS_INDEXER_FEE_LEDGER_HOME|#1213" skills/AGENTS_INDEXER_UST1_WINDOW_FEES.md && \
    grep -qE "AGENTS_INDEXER_FEE_LEDGER_HOME|#1213|#1210" skills/AGENTS_INDEXER_COMMUNITY_TOKENS.md && \
    grep -qE "AGENTS_INDEXER_FEE_LEDGER_HOME|#1213" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
  '

run_step "no ingest on this epic: FeeSource / CHECK unchanged" \
  bash -c '
    grep -qE "swap_amm" indexer/src/indexer/protocol_fees.rs && \
    grep -qE "ust1_redeem" indexer/src/indexer/protocol_fees.rs && \
    ! grep -qE "PairCreation|SkuUnlock|SettingsFee" indexer/src/indexer/protocol_fees.rs && \
    ! grep -qE "pair_creation" indexer/src/indexer/protocol_fees.rs && \
    ! grep -qE "sku_unlock|settings_fee|pair_creation" indexer/migrations/*protocol_fees*.sql
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
