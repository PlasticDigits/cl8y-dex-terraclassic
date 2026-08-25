#!/usr/bin/env bash
# Verification for GitLab #631: DeFiLlama UTC-day API + adapter copies.
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
echo "  GitLab #631 — DeFiLlama UTC-day volume/fees + adapter copies"
echo "════════════════════════════════════════════════════════════════"

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_step "docs: playbook + invariants + skill crosslinks" \
  bash -c '
    set -euo pipefail
    test -f docs/DEFILLAMA.md
    test -f skills/AGENTS_DEFILLAMA.md
    test -f scripts/defillama/README.md
    test -f indexer/migrations/20260825150000_defillama_daily.sql
    grep -q "L631-1" skills/AGENTS_DEFILLAMA.md
    grep -q "L631-9" skills/AGENTS_DEFILLAMA.md
    grep -q "L631-10" skills/AGENTS_DEFILLAMA.md
    grep -q "L631-11" skills/AGENTS_DEFILLAMA.md
    grep -q "unstablecoin" docs/DEFILLAMA.md
    test -f indexer/migrations/20260825160000_defillama_daily_assets.sql
    grep -q "DeFiLlama UTC-day (#631)" docs/indexer-invariants.md
    grep -q "DEFILLAMA.md" docs/README.md
    grep -q "AGENTS_DEFILLAMA" docs/README.md
    grep -q "Not DeFiLlama" docs/CG_CMC_COMPLIANCE.md
    grep -q "defillama/daily" docs/integrators-hybrid-volume.md
    grep -q "verify-issue-631" docs/testing.md
    grep -q "verify-issue-631" AGENTS.md
    grep -q "AGENTS_DEFILLAMA" AGENTS.md
    grep -q "terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea" docs/DEFILLAMA.md
    grep -q "liquidity_in_usd" docs/DEFILLAMA.md
  '

run_step "source: GET is rollup-only; no CG TVL; gems match #562" \
  bash -c '
    set -euo pipefail
    grep -q "/api/v1/defillama/daily" indexer/src/api/mod.rs
    grep -q refresh_defillama_daily indexer/src/indexer/volume_aggregator.rs
    grep -q COLUMBUS5_GEM_ADDRESSES indexer/src/indexer/defillama.rs
    grep -q COLUMBUS5_GEM_ADDRESSES frontend-dapp/src/utils/pairCatalogRank.ts
    grep -q COLUMBUS5_GEM_ADDRESSES scripts/defillama/gems.js
    python3 - <<'"'"'PY'"'"'
from pathlib import Path
import re
fe = Path("frontend-dapp/src/utils/pairCatalogRank.ts").read_text()
rs = Path("indexer/src/indexer/defillama.rs").read_text()
js = Path("scripts/defillama/gems.js").read_text()
pat = re.compile(r"terra1[a-z0-9]{38,}")
def gems(text):
    return {m.group(0) for m in pat.finditer(text.lower()) if "dmuruhht32x8f47nvm73pwp6q7uf2jtfhdt3nxcql4mmqkyfsraqn2dt94" in text.lower() or True}
# Narrow to the eight gem addrs that appear in all three files
need = {
    "terra1dmuruhht32x8f47nvm73pwp6q7uf2jtfhdt3nxcql4mmqkyfsraqn2dt94",
    "terra1k6cqupylk0wp4pj273pntwhv9py0q5guyqye8ssvukn9xq7mes7sdlmena",
    "terra1ejq3mjjgnklpa3pg4jterlfwsny055gpmcjf3fz0ev3ueajnzeysz6xxgr",
    "terra178fgrfzv7njtmdp9vghyf2dx77sah8u8jluzs7ym562chaxnmj2s6mn6m9",
    "terra1fga508hzx8dd7x8q4uhm6mdhkqv6fxrtsea3r27smdqmv5k2jgxq5zk9fc",
    "terra12k67cvfs7y7g8lca3qr4g4py6s6j69fu24gze5pjfamfpckv8mps7cymme",
    "terra17dpnjlpgsnm8muu4msfjra4f2hrptnjp2jdpkka4p0e3px42ayxq0pmc2z",
    "terra18fzufz8cs7ez49xjwgs248x85za5v50yug55fj7lyxp9hapxyr7qnh3czs",
}
for name, text in (("frontend", fe), ("indexer", rs), ("gems.js", js)):
    missing = [a for a in need if a not in text.lower()]
    if missing:
        raise SystemExit(f"{name} missing gems: {missing}")
print("gem addresses lockstep ok")
PY
    if grep -nE "FROM swap_events|FROM protocol_fee_events|FROM limit_order_fills" \
         indexer/src/api/defillama.rs 2>/dev/null; then
      echo "GET handler must not scan event tables" >&2
      exit 1
    fi
    grep -q "never use this indexer USD" indexer/src/api/defillama.rs || \
      grep -q "Do not use this indexer USD" indexer/src/api/defillama.rs
    grep -q unstablecoin indexer/src/api/defillama.rs
    grep -q reserve indexer/src/api/defillama.rs
    if grep -nE "liquidity_in_usd|total_liquidity_usd" \
         scripts/defillama/tvl/tvlCore.js \
         scripts/defillama/dimensions/mapDaily.js 2>/dev/null; then
      echo "TVL/volume helpers must not read indexer/CG USD fields" >&2
      exit 1
    fi
  '

run_step "lib: timestamp parse + gem set + usd null contract" \
  bash -c '
    set -euo pipefail
    cd indexer
    cargo test --lib defillama -- --test-threads=1
  '

run_step "integration: daily API gem/hybrid/fee/400/404/O(1)" \
  bash -c '
    set -euo pipefail
    cd indexer
    cargo test --test indexer_defillama -- --test-threads=1
  '

run_step "node: TVL helper + dimension mapping" \
  bash -c '
    set -euo pipefail
    node --test scripts/defillama/tvl/tvlCore.test.js \
      scripts/defillama/dimensions/mapDaily.test.js \
      scripts/defillama/stablecoins/ust1Core.test.js
  '

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  $PASS passed, $FAIL failed"
echo "════════════════════════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [ "$FAIL" -ne 0 ]; then
  exit 1
fi
