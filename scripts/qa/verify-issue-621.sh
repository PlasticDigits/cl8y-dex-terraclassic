#!/usr/bin/env bash
# Automated verification for GitLab #621 — tax-aware localnet swarm workers.
#
# Invariants S621-1–S621-8: skills/AGENTS_LOCALNET_SWARM_TAX.md
#
# Static + unit always. Optional TS --dry-run when LocalTerra + tax pins exist.
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
echo "  GitLab #621 — tax-aware localnet swarm workers"
echo "════════════════════════════════════════════════════════════════"

run_docs() {
  set -euo pipefail
  rg -q "S621-1" skills/AGENTS_LOCALNET_SWARM_TAX.md
  rg -q "tax_hybrid_skip" skills/AGENTS_LOCALNET_SWARM_TAX.md
  rg -q "SWARM_TAX_WORKERS" skills/AGENTS_LOCALNET_SWARM_TAX.md
  rg -q "AGENTS_LOCALNET_SWARM_TAX" skills/AGENTS_LOCALNET_TRADING_SWARM.md
  rg -q "AGENTS_LOCALNET_SWARM_TAX" skills/AGENTS_COMMUNITY_TAX_ROUTER.md
  rg -q "AGENTS_LOCALNET_SWARM_TAX" skills/AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md
  rg -q "AGENTS_LOCALNET_SWARM_TAX" skills/AGENTS_COMMUNITY_TAX_CW20.md
  rg -q "verify-issue-621" AGENTS.md
  rg -q "verify-issue-621" docs/testing.md
  rg -q "S621-1" docs/contracts-security-audit.md
  rg -q "tax_listed" packages/localnet-trading-swarm/README.md
  rg -q "SWARM_TAX_WORKERS" packages/localnet-trading-swarm/README.md
  rg -q "verify-issue-621" docs/local-development.md
  rg -qi "tax-aware" docs/local-development.md
}

run_ts_static() {
  set -euo pipefail
  rg -q "tax_hybrid_skip" packages/localnet-trading-swarm/src/actions.ts
  rg -q "TAX_LISTED_PROFILE_ID" packages/localnet-trading-swarm/src/profiles.ts
  rg -q "filterGemPairs" packages/localnet-trading-swarm/src/actions.ts
  rg -q "pairDirectSwapHook" packages/localnet-trading-swarm/src/actions.ts
  rg -q "requiredWalletDebit" packages/localnet-trading-swarm/src/taxPreview.ts
  rg -q "taxWorkersEnabled" packages/localnet-trading-swarm/src/swarmRunner.ts
  # Pair-direct must not assign a spoofed trader.
  if rg -n "trader: wallet" packages/localnet-trading-swarm/src/actions.ts \
    packages/localnet-trading-swarm/src/taxHooks.ts; then
    echo "pair-direct / hook builder spoofs trader" >&2
    return 1
  fi
}

run_py_static() {
  set -euo pipefail
  rg -q "filter_gem_pairs" scripts/bots/swarm.py
  rg -q -F -- '--worker tax' scripts/bots/launch-swarm.sh
  rg -q "tax_hybrid_skip" scripts/bots/swarm.py
  rg -q "SWARM_TAX_WORKERS" scripts/bots/launch-swarm.sh
  rg -q "pair_direct_swap_hook" scripts/bots/swarm_tax.py
  bash -n scripts/bots/launch-swarm.sh
  python3 -m py_compile scripts/bots/swarm_tax.py scripts/bots/swarm.py \
    scripts/bots/test_swarm_tax.py
}

run_ts_unit() {
  set -euo pipefail
  if [[ ! -d packages/localnet-trading-swarm/node_modules ]]; then
    bash scripts/with-node.sh --cwd packages/localnet-trading-swarm -- npm ci
  fi
  bash scripts/with-node.sh --cwd packages/localnet-trading-swarm -- npm run test:run
}

run_py_unit() {
  set -euo pipefail
  (cd "$REPO_ROOT/scripts/bots" && python3 -m unittest test_swarm_liquidity.py test_swarm_tax.py -v)
}

run_retest_static() {
  run_docs
  run_ts_static
  run_py_static
}

run_live_dry_run() {
  set -euo pipefail
  [[ -f frontend-dapp/.env.local ]] || {
    echo "no .env.local" >&2
    return 1
  }
  grep -qE '^VITE_TOKEN_COMMUNITY_TAX_ADDRESS=terra1' frontend-dapp/.env.local || {
    echo "tax pin unset" >&2
    return 1
  }
  local log
  log="$(mktemp)"
  set +e
  timeout 15 bash scripts/with-node.sh --cwd packages/localnet-trading-swarm -- \
    npm run start -- --dry-run >"$log" 2>&1
  local rc=$?
  set -e
  cat "$log"
  # 124 = timeout (expected: dry-run keeps scheduling). Require tax discovery logs.
  rg -q '"kind":"swarm_tax"' "$log" || rg -q 'swarm_tax' "$log"
  rg -q 'tax_listed|tax_pair_visible|skipped_broadcast' "$log"
  rm -f "$log"
  [[ "$rc" -eq 0 || "$rc" -eq 124 ]]
}

echo ""
echo "── first pass ──"
run_step "docs: S621 + skill crosslinks" run_docs
run_step "TS static: exclude + preview + no pair-direct trader" run_ts_static
run_step "Python static: tax worker + exclude + syntax" run_py_static
run_step "TS unit: npm run test:run" run_ts_unit
run_step "Python unit: test_swarm_liquidity + test_swarm_tax" run_py_unit

HAS_LT=1
if timeout 20 make -s has-localterra >/dev/null 2>&1; then
  HAS_LT=0
fi
if [[ "$HAS_LT" -eq 0 ]] && grep -qE '^VITE_TOKEN_COMMUNITY_TAX_ADDRESS=terra1' \
  frontend-dapp/.env.local 2>/dev/null; then
  run_step "live: TS --dry-run sees factory (incl. tax pair)" run_live_dry_run
else
  echo "  [SKIP] live TS --dry-run (need LocalTerra + tax pins from make deploy-local)"
  RESULTS+=("SKIP  live TS --dry-run")
fi

echo ""
echo "── retest ──"
run_step "retest docs + static" run_retest_static
run_step "retest TS unit" run_ts_unit
run_step "retest Python unit" run_py_unit

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  #621 results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
echo "==> GitLab #621 verification passed"
exit 0
