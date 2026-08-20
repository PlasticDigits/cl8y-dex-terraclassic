#!/usr/bin/env bash
# Verification for GitLab #580: indexer oracle CEX FDUSD identity under path vfdusd.
#
# Requires: make setup-indexer-postgres (Postgres + indexer/.env; no wasm deploy).
# Does not require live CEX / CoinGecko.
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
echo "  GitLab #580 — CEX FDUSD identity under path vfdusd"
echo "════════════════════════════════════════════════════════════════"

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_step "docs: identity is CEX FDUSD, not vFDUSD/USD" \
  bash -c '
    set -euo pipefail
    grep -q "FDUSD/USD" indexer/src/indexer/oracle.rs
    grep -q "quote_asset" indexer/src/api/oracle.rs
    grep -q "CEX FDUSD" indexer/src/api/oracle.rs
    if grep -n "Vfdusd => \"vFDUSD/USD\"" indexer/src/indexer/oracle.rs; then
      echo "display_name still vFDUSD/USD" >&2
      exit 1
    fi
    grep -q "not.*vFDUSD" docs/runbooks/indexer-external-oracle.md
    grep -q "FDUSD/USD" docs/runbooks/indexer-external-oracle.md
    grep -q "quote_asset=FDUSD" docs/runbooks/indexer-external-oracle.md
    grep -q "#580" docs/indexer-invariants.md
    grep -q "never \`vFDUSD/USD\`" skills/AGENTS_INDEXER_EXTERNAL_ORACLE.md
    grep -q "OracleTicker::Vfdusd" skills/AGENTS_INDEXER_EXTERNAL_ORACLE.md
    grep -q "usd_per_human" skills/AGENTS_INDEXER_EXTERNAL_ORACLE.md
    grep -q "verify-issue-580" AGENTS.md
    grep -q "#580" docs/frontend.md
    grep -q "P550-9" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
  '

run_step "source: parse(fdusd) None; P522-Q ignores VFDUSD" \
  bash -c '
    set -euo pipefail
    grep -q "parse(\"fdusd\")" indexer/src/indexer/oracle.rs
    grep -q "quote_usd_kind(\"VFDUSD\"" indexer/src/indexer/pair_price_usd.rs
    grep -q "HubTicker::parse(\"vfdusd\")" indexer/src/indexer/hub_usd.rs
  '

run_step "indexer lib: oracle display_name + pair USD + hub parse" \
  bash -c '
    set -euo pipefail
    cd indexer
    cargo test --lib oracle -- --quiet
    cargo test --lib pair_price_usd -- --quiet
    cargo test --lib hub_usd -- --quiet
  '

run_step "indexer integration: api_oracle identity (catalog/snapshot/history/fdusd 400)" \
  bash -c '
    set -euo pipefail
    cd indexer
    cargo test --test api_oracle -- --test-threads=1 --quiet oracle_price_catalog
    cargo test --test api_oracle -- --test-threads=1 --quiet oracle_price_vfdusd
    cargo test --test api_oracle -- --test-threads=1 --quiet oracle_history_vfdusd
    cargo test --test api_oracle -- --test-threads=1 --quiet oracle_price_fdusd
  '

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
