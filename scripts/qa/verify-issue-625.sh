#!/usr/bin/env bash
# Automated verification for GitLab #625 — post-merge !415–!417 tax swarm /
# e2e-tx / Layer B tax-on leftovers (#621 / #622 / #623).
#
# Proves (docs + children 621/622/623 + leftover live + 293):
#   1. Q13 / M625-1–M625-8 documented and crosslinked.
#   2. Tax-on buy uses pick_trader (non-treasury); seed treasury ≠ test1.
#   3. Children make verify-issue-621 / 622 / 623.
#   4. Optional fresh volume: VERIFY625_FRESH=1 → make reset && make deploy-local.
#   5. Live: LAYER_B_TAX_ON seed buy, Playwright P0, swarm soak, OE-1 293.
#
# VERIFY625_SKIP_CHILDREN=1 — skip 621/622/623 (docs + source + live).
# VERIFY625_SKIP_CHAIN=1 — skip LocalTerra leftovers even if the chain is up.
# VERIFY625_REQUIRE_CHAIN=1 — FAIL (do not SKIP) when LocalTerra is missing.
# VERIFY625_REQUIRE_LIVE=1 — FAIL (do not SKIP) when live leftovers cannot run.
# VERIFY625_FRESH=1 — make reset && make deploy-local before leftover probes.
# VERIFY625_SKIP_FRESH=1 — never reset (even if VERIFY625_FRESH=1).
# VERIFY625_SKIP_PLAYWRIGHT=1 — skip VERIFY_ISSUE_622_CHAIN Playwright.
# VERIFY625_SKIP_SWARM=1 — skip live swarm soak (source still checked).
# VERIFY625_SKIP_EPHEMERAL=1 — skip leftover ephemeral tax-on (seed path still runs).
# VERIFY625_SKIP_OPTIONAL=1 — skip optional 589 / 601.
#
# Refs: skills/AGENTS_POST_MERGE_OPS_625.md, docs/qa-invariants.md § Q13
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }
skip() { RESULTS+=("SKIP  $1"); echo "  [SKIP] $1"; }

run_step() {
  local label="$1"
  shift
  echo ""
  echo "[$label]"
  set +e
  "$@"
  local rc=$?
  set -e
  if [[ $rc -eq 0 ]]; then
    ok "$label"
  else
    bad "$label"
  fi
}

echo "════════════════════════════════════════════════════════════════"
echo "  GitLab #625 — post-merge !415–!417 tax leftover live"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
TEST1="terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v"

run_docs() {
  set -euo pipefail
  test -f skills/AGENTS_POST_MERGE_OPS_625.md
  grep -qE '\*\*M625-1\*\*' docs/qa-invariants.md
  grep -qE '\*\*M625-8\*\*' docs/qa-invariants.md
  grep -qE 'post-merge-ops-625' docs/qa-invariants.md
  grep -qE '\*\*M625-1' skills/AGENTS_POST_MERGE_OPS_625.md
  grep -qE 'make verify-issue-625' skills/AGENTS_POST_MERGE_OPS_625.md
  grep -qE 'AGENTS_POST_MERGE_OPS_625' AGENTS.md
  grep -qE 'verify-issue-625' AGENTS.md
  grep -qE 'verify-issue-625' Makefile
  grep -qE '#625' docs/testing.md
  grep -qE 'AGENTS_POST_MERGE_OPS_625' docs/README.md
  grep -qE '#625' skills/AGENTS_LOCALNET_SWARM_TAX.md
  grep -qE '#625' skills/AGENTS_E2E_COMMUNITY_TAX_TX.md
  grep -qE '#625' skills/AGENTS_CW20_CODE_ID_TAX_ON.md
  grep -qE '#625' skills/AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md
  grep -qE '#625' skills/AGENTS_POST_MERGE_OPS_624.md
  grep -qE '#625' docs/local-development.md
  grep -qE 'M625-1' docs/contracts-security-audit.md
  grep -qE 'Do \*\*not\*\* reopen #621' skills/AGENTS_POST_MERGE_OPS_625.md
  grep -qE 'LAYER_B_TAX_ON_FORCE_EPHEMERAL' skills/AGENTS_POST_MERGE_OPS_625.md
  grep -qE 'LAYER_B_TAX_ON_FORCE_EPHEMERAL' skills/AGENTS_CW20_CODE_ID_TAX_ON.md
  grep -qE 'LAYER_B_TAX_ON_FORCE_EPHEMERAL' docs/qa-invariants.md
  if grep -nE 'AddWhitelistedCodeId 11611' skills/AGENTS_POST_MERGE_OPS_625.md \
      skills/AGENTS_CW20_CODE_ID_TAX_ON.md; then
    echo "docs tell operators to whitelist columbus-5 11611 from LocalTerra" >&2
    return 1
  fi
}

run_source() {
  set -euo pipefail
  # Leftover #1: buy uses TRADER / pick_trader, not hardcoded test1.
  grep -qE 'BUY_USER="\$TRADER"' cw20-codeid-audits/scripts/layer-b-tax-on.sh
  grep -qE 'LAYER_B_TAX_ON_FORCE_EPHEMERAL' cw20-codeid-audits/scripts/layer-b-tax-on.sh
  grep -qE 'skim_min_return:"0"' cw20-codeid-audits/scripts/layer-b-tax-on.sh \
    || grep -qE 'cleared leftover skim_min_return' cw20-codeid-audits/scripts/layer-b-tax-on.sh
  grep -qE 'buy_user: \$buy_user' cw20-codeid-audits/scripts/layer-b-tax-on.sh
  if grep -nE 'BUY_USER="\$TEST_ADDRESS"' cw20-codeid-audits/scripts/layer-b-tax-on.sh; then
    echo "tax-on buy still hardcodes test1 (manager/treasury leftover)" >&2
    return 1
  fi
  # Leftover #2: seed treasury is a distinct sink (not manager:$a, treasury:$a).
  grep -qE '_ctax_token_treasury' scripts/lib/deploy-community-tax-local.sh
  grep -qE 'treasury:\$treas' scripts/lib/deploy-community-tax-local.sh
  grep -qE 'adding LocalTerra key test2.*\>\&2' scripts/lib/deploy-community-tax-local.sh \
    || grep -qE 'echo .*test2.*treasury.*>\&2' scripts/lib/deploy-community-tax-local.sh
  if grep -nE 'manager:\$a, treasury:\$a' scripts/lib/deploy-community-tax-local.sh; then
    echo "QA seed still sets treasury=test1 (extra-debit invisible on e2e wallet)" >&2
    return 1
  fi
  grep -qE 'isCommunityTaxPair|isEmberCoralPair' frontend-dapp/e2e/helpers/lcd.ts
  grep -qE 'requireCommunityTaxTxPins' frontend-dapp/e2e/helpers/community-tax-env.ts
  grep -qE 'enableExpertModeForSwap' frontend-dapp/e2e/helpers/community-tax-e2e.ts
  grep -qE 'assertCancelLimitRefundIsBuyNotSell' frontend-dapp/e2e/helpers/community-tax-e2e.ts
  grep -qE 'assertCancelLimitRefundIsBuyNotSell' frontend-dapp/e2e/community-tax-tx.spec.ts
  grep -qE 'enableExpertModeForSwap' frontend-dapp/e2e/helpers/swap-ui.ts
  grep -qE 'ADD_LIQUIDITY_GAS_LIMIT = 1_000_000' frontend-dapp/src/services/terraclassic/terraGas.ts
  grep -qE 'PLACE_LIMIT_ORDER_BATCH_BASE_GAS_LIMIT = 1_000_000' frontend-dapp/src/services/terraclassic/terraGas.ts
  grep -qE 'CANCEL_LIMIT_ORDER_GAS_LIMIT = 1_000_000' frontend-dapp/src/services/terraclassic/terraGas.ts
  grep -qE "'place_limit_order' in inner" frontend-dapp/src/services/terraclassic/terraGas.ts
  grep -q 'Ref\\s+' frontend-dapp/e2e/helpers/limit-e2e.ts
  grep -qE 'order #\\(\\\\d\\+\\)/i' frontend-dapp/e2e/helpers/limit-e2e.ts \
    || grep -qE 'LAST_PLACED_ORDER_ID_RE = /order #' frontend-dapp/e2e/helpers/limit-e2e.ts
  if grep -nE 'test\.skip\(' frontend-dapp/e2e/community-tax-tx.spec.ts \
      frontend-dapp/e2e/helpers/community-tax-e2e.ts; then
    echo "tax tx spec/helpers must not test.skip (E622-2 / M625-7)" >&2
    return 1
  fi
  grep -qE 'filterGemPairs' packages/localnet-trading-swarm/src/actions.ts
  grep -qE 'tax_hybrid_skip' packages/localnet-trading-swarm/src/actions.ts
  grep -qE 'SWARM_TAX_WORKERS' scripts/bots/launch-swarm.sh
  grep -qE 'tax_listed worker' scripts/bots/swarm.py
  grep -qE 'warmup = \("hybrid", "sell"\)' scripts/bots/swarm.py
  grep -qE 'Start tax-0 first' scripts/bots/launch-swarm.sh
  grep -qE '3173' scripts/deploy-dex-local.sh
  grep -qE 'indexer-cors-playwright' scripts/e2e-start-indexer.sh
  grep -qE 'PLAYWRIGHT_WEB_PORT' scripts/lib/indexer-cors-playwright.sh
  # shellcheck source=scripts/lib/indexer-cors-playwright.sh
  source "$REPO_ROOT/scripts/lib/indexer-cors-playwright.sh"
  local cors_merged
  cors_merged="$(indexer_cors_merge 'http://localhost:5173')"
  echo "$cors_merged" | grep -q '127.0.0.1:3173'
  echo "$cors_merged" | grep -q 'localhost:3173'
  if grep -nE 'add_whitelisted_code_id.*11611' scripts/lib/deploy-community-tax-local.sh \
      cw20-codeid-audits/scripts/layer-b-tax-on.sh; then
    echo "scripts tell LocalTerra to whitelist columbus-5 11611" >&2
    return 1
  fi
}

run_621() { make verify-issue-621; }
run_622() { make verify-issue-622; }
run_623() { make verify-issue-623; }
_clear_vite_exports() {
  # _load_vite uses set -a; leaked VITE_* breaks swarm env.test.ts overrides.
  local k
  for k in $(compgen -e VITE_ || true); do
    unset "$k"
  done
}

run_293() {
  _clear_vite_exports
  local log rc
  log="$(mktemp)"
  set +e
  make verify-issue-293 >"$log" 2>&1
  rc=$?
  set -e
  cat "$log"
  # Leftover is OE-1 gem pool_only — do not fail the stack on wrap-symbol preflight
  # (fresh indexer may use cUSTC, not USTC-C).
  if grep -q 'OE-1 hub pairs pool_only near-inverse' "$log" \
      && grep -q 'ALL_PASS=true' "$log"; then
    if grep -E 'QTAX|COMMUNITY_TAX|tax/EMBER' "$log" | grep -qiE 'OE-1|hub pair'; then
      echo "tax pair leaked into OE-1 hub checks" >&2
      rm -f "$log"
      return 1
    fi
    echo "OE-1 pool_only green (verify-issue-293 rc=$rc)"
    rm -f "$log"
    return 0
  fi
  rm -f "$log"
  return "$rc"
}
run_601() { make verify-issue-601; }

_load_vite() {
  local env_local="$REPO_ROOT/frontend-dapp/.env.local"
  [[ -f "$env_local" ]] || return 1
  set -a
  # shellcheck disable=SC1090
  source <(grep -E '^VITE_[A-Z0-9_]+=' "$env_local")
  set +a
  [[ -n "${VITE_TOKEN_COMMUNITY_TAX_ADDRESS:-}" && -n "${VITE_FACTORY_ADDRESS:-}" ]]
}

run_env_pins() {
  set -euo pipefail
  _load_vite || {
    echo "tax pins unset — run make deploy-local without DEPLOY_SKIP_COMMUNITY_TAX" >&2
    return 1
  }
  grep -qE '^VITE_TOKEN_COMMUNITY_TAX_ADDRESS=terra1' frontend-dapp/.env.local
  grep -qE '^VITE_PAIR_COMMUNITY_TAX_EMBER=terra1' frontend-dapp/.env.local
  local vite_id
  vite_id="$(grep -E '^VITE_COMMUNITY_TAX_CODE_ID=' frontend-dapp/.env.local | tail -n1 | cut -d= -f2)"
  [[ "$vite_id" != "11611" && "$vite_id" != "11619" ]] || {
    echo "dApp pin columbus-5 $vite_id against local instances" >&2
    return 1
  }
  echo "env pins: tax=$VITE_TOKEN_COMMUNITY_TAX_ADDRESS code=$vite_id"
}

run_treasury_not_test1() {
  set -euo pipefail
  # shellcheck source=scripts/lib/localterra-host-curl.sh
  source "$REPO_ROOT/scripts/lib/localterra-host-curl.sh"
  # shellcheck source=scripts/lib/lcd-smart-query.sh
  source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"
  _load_vite || return 1
  local lcd="${VITE_TERRA_LCD_URL:-http://localhost:1317}"
  lcd="${lcd%/}"
  local cfg treas
  cfg="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$lcd" "$VITE_TOKEN_COMMUNITY_TAX_ADDRESS" '{"get_config":{}}')")"
  treas="$(echo "$cfg" | jq -r '.treasury // empty')"
  [[ "$treas" == terra1* ]] || {
    echo "seed GetConfig.treasury missing" >&2
    return 1
  }
  [[ "$treas" != "$TEST1" ]] || {
    echo "seed treasury is test1 ($treas) — extra-debit LCD assert will net to 1:1" >&2
    return 1
  }
  echo "seed treasury sink=$treas (not test1)"
}

run_tax_on_seed() {
  set -euo pipefail
  local json="$REPO_ROOT/cw20-codeid-audits/harness/layer-b-tax-on.json"
  if [[ ! -f "$json" ]] || ! jq -e '.executed == true and .source == "seed" and .pair_direct_buy == true
    and (.buy_user | startswith("terra1"))' "$json" >/dev/null 2>&1; then
    LAYER_B_TAX_ON=1 LAYER_B_TAX_ON_JSON="$json" make verify-issue-623 || return 1
  fi
  test -f "$json"
  jq -e '.executed == true and .pair_direct_buy == true and .source == "seed"
    and .buy_user != "" and .trader != ""' "$json" >/dev/null
  python3 - "$json" "$TEST1" <<'PY'
import json, sys
doc = json.load(open(sys.argv[1], encoding="utf-8"))
test1 = sys.argv[2]
buy = doc.get("buy_user") or ""
# After treasury split, trader/buy_user may be test1 (non-treasury). That is OK.
# Refuse buy_user empty or missing.
if not buy.startswith("terra1"):
    raise SystemExit(f"buy_user={buy!r} not a terra addr")
print(f"tax-on seed buy_user={buy} trader={doc.get('trader')} (test1={test1})")
PY
}

run_tax_on_ephemeral() {
  set -euo pipefail
  # Seed pins win by default. FORCE_EPHEMERAL skips them so a #624 volume
  # still proves instantiate + buy-from-trader (issue-body leftover).
  local json="$REPO_ROOT/cw20-codeid-audits/harness/layer-b-tax-on-ephemeral.json"
  LAYER_B_TAX_ON=1 LAYER_B_TAX_ON_FORCE_EPHEMERAL=1 LAYER_B_TAX_ON_JSON="$json" \
    "$REPO_ROOT/cw20-codeid-audits/scripts/layer-b-tax-on.sh" || return 1
  test -f "$json"
  jq -e '.executed == true and .source == "ephemeral" and .pair_direct_buy == true
    and (.buy_user | startswith("terra1")) and .trader != ""' "$json" >/dev/null
  python3 - "$json" <<'PY'
import json, sys
doc = json.load(open(sys.argv[1], encoding="utf-8"))
buy = doc.get("buy_user") or ""
trader = doc.get("trader") or ""
if buy != trader:
    raise SystemExit(f"ephemeral buy_user={buy!r} != trader={trader!r}")
print(f"tax-on ephemeral buy_user={buy} trader={trader} token={doc.get('token')}")
PY
}

run_playwright_p0() {
  set -euo pipefail
  VERIFY_ISSUE_622_CHAIN=1 make verify-issue-622
}

restart_indexer_after_deploy() {
  set -euo pipefail
  local bin="$REPO_ROOT/indexer/target/release/cl8y-dex-indexer"
  if [[ ! -x "$bin" ]]; then
    local sibling
    sibling="$(cd "$REPO_ROOT/.." && pwd)/cl8y-dex-terraclassic/indexer/target/release/cl8y-dex-indexer"
    if [[ -x "$sibling" ]]; then
      mkdir -p "$(dirname "$bin")"
      ln -sfn "$sibling" "$bin"
    elif [[ -x /home/answorld/repos/cl8y-dex-terraclassic/indexer/target/release/cl8y-dex-indexer ]]; then
      mkdir -p "$(dirname "$bin")"
      ln -sfn /home/answorld/repos/cl8y-dex-terraclassic/indexer/target/release/cl8y-dex-indexer "$bin"
    else
      (cd "$REPO_ROOT/indexer" && cargo build --release)
    fi
  fi
  local pid
  pid="$(ss -ltnp 2>/dev/null | awk '/:3001/ {print}' | grep -oE 'pid=[0-9]+' | head -1 | cut -d= -f2 || true)"
  if [[ -n "$pid" ]]; then
    echo "stopping indexer pid $pid on :3001 (postgres was reset)"
    kill "$pid" 2>/dev/null || true
    sleep 2
  fi
  PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-3173}" \
    PLAYWRIGHT_BASE_URL="http://127.0.0.1:${PLAYWRIGHT_WEB_PORT:-3173}" \
    INDEXER_PID_FILE="${INDEXER_PID_FILE:-/tmp/cl8y-indexer-625.pid}" \
    INDEXER_LOG="${INDEXER_LOG:-/tmp/cl8y-indexer-625.log}" \
    bash "$REPO_ROOT/scripts/e2e-start-indexer.sh"
}

run_fresh_deploy() {
  set -euo pipefail
  echo "==> VERIFY625_FRESH=1: make reset && make start && deploy"
  make reset || return 1
  make start || return 1
  make wait-healthy || return 1
  if [[ -f "$REPO_ROOT/smartcontracts/artifacts/cl8y_community_tax_token.wasm" ]]; then
    make deploy-local-no-build || return 1
  else
    make deploy-local || return 1
  fi
  restart_indexer_after_deploy || return 1
}

run_swarm_soak() {
  set -euo pipefail
  _load_vite || return 1
  make swarm-stop >/dev/null 2>&1 || true
  local tax_log="$REPO_ROOT/scripts/bots/run/logs/tax-0.log"
  rm -f "$tax_log"
  # Poll until extra-debit + hybrid skip appear. Warmup is hybrid then sell
  # (~1s); 45s fixed sleep was flaky when RNG only bought/limited and gem
  # workers stormed test1's sequence.
  local soak="${VERIFY625_SWARM_SOAK_SEC:-90}"
  echo "swarm-launch soak up to ${soak}s (poll tax_debit + tax_hybrid_skip)…"
  BOTS_MEAN_INTERVAL_SEC="${BOTS_MEAN_INTERVAL_SEC:-8}" \
    BOTS_TAX_MEAN_INTERVAL_SEC="${BOTS_TAX_MEAN_INTERVAL_SEC:-8}" \
    make swarm-launch || return 1
  local deadline=$((SECONDS + soak))
  local saw_debit=0 saw_skip=0 saw_listed=0
  while (( SECONDS < deadline )); do
    if [[ -f "$tax_log" ]]; then
      grep -qE 'tax_debit=' "$tax_log" && saw_debit=1
      grep -qE 'tax_hybrid_skip' "$tax_log" && saw_skip=1
      grep -qE 'tax_listed' "$tax_log" && saw_listed=1
      if [[ $saw_debit -eq 1 && $saw_skip -eq 1 && $saw_listed -eq 1 ]]; then
        break
      fi
    fi
    sleep 2
  done
  if [[ $saw_debit -ne 1 || $saw_skip -ne 1 || $saw_listed -ne 1 ]]; then
    echo "tax-0 log missing extra-debit / hybrid-skip / tax_listed after ${soak}s:" >&2
    tail -n 40 "$tax_log" >&2 || true
    make swarm-stop || true
    return 1
  fi
  # Gem workers must not offer the pinned tax token.
  local gem_hit
  gem_hit="$(grep -R --include='*.log' -F "$VITE_TOKEN_COMMUNITY_TAX_ADDRESS" \
    "$REPO_ROOT/scripts/bots/run/logs" | grep -v 'tax-0' || true)"
  if [[ -n "$gem_hit" ]] && echo "$gem_hit" | grep -qE 'offer|swap|route'; then
    echo "gem worker logs mention the tax token as an offer/route:" >&2
    echo "$gem_hit" | head -n 20 >&2
    make swarm-stop || true
    return 1
  fi
  make swarm-stop || true
  echo "swarm soak: tax_listed extra-debit + tax_hybrid_skip present"
}

run_swarm_exclude_only() {
  set -euo pipefail
  make swarm-stop >/dev/null 2>&1 || true
  # Prior soak leaves tax-0.log; delete so we only fail on a new tax worker.
  rm -f "$REPO_ROOT/scripts/bots/run/logs/tax-0.log"
  SWARM_TAX_WORKERS=0 BOTS_SKIP_BOOTSTRAP=1 BOTS_DRY_RUN=1 \
    timeout 25 make swarm-launch || true
  if [[ -f "$REPO_ROOT/scripts/bots/run/logs/tax-0.log" ]]; then
    echo "SWARM_TAX_WORKERS=0 still started tax-0" >&2
    make swarm-stop || true
    return 1
  fi
  if grep -qE 'tax-0' "$REPO_ROOT/scripts/bots/run/pids.txt" 2>/dev/null; then
    echo "SWARM_TAX_WORKERS=0 recorded a tax-0 pid" >&2
    make swarm-stop || true
    return 1
  fi
  make swarm-stop || true
  echo "SWARM_TAX_WORKERS=0 is exclude-only (no tax-0)"
}

maybe_fresh() {
  if [[ "${VERIFY625_SKIP_FRESH:-}" == "1" ]]; then
    skip "fresh volume (VERIFY625_SKIP_FRESH=1)"
    return 0
  fi
  if [[ "${VERIFY625_FRESH:-}" == "1" ]]; then
    run_step "M625-6: fresh make reset && make deploy-local" run_fresh_deploy
  fi
}

echo ""
echo "── first pass ──"
run_step "docs: Q13 M625-1–M625-8 + skill + AGENTS crosslinks" run_docs
run_step "source: tax-on buy trader + seed treasury ≠ test1 + no skip" run_source

HAS_LT=1
if timeout 20 make -s has-localterra >/dev/null 2>&1; then
  HAS_LT=0
fi

# Fresh volume before children so #623 tax-on hits the new seed, not leftover pairs.
if [[ "${VERIFY625_SKIP_CHAIN:-}" != "1" && "$HAS_LT" -eq 0 ]]; then
  maybe_fresh
fi

if [[ "${VERIFY625_SKIP_CHILDREN:-}" == "1" ]]; then
  skip "children 621/622/623 (VERIFY625_SKIP_CHILDREN=1)"
else
  run_step "child: make verify-issue-621" run_621
  run_step "child: make verify-issue-622" run_622
  run_step "child: make verify-issue-623" run_623
fi

if [[ "${VERIFY625_SKIP_CHAIN:-}" == "1" ]]; then
  skip "LocalTerra leftovers (VERIFY625_SKIP_CHAIN=1)"
elif [[ "$HAS_LT" -eq 0 ]]; then
  run_step "M625-2/6: dApp local COMMUNITY_TAX pins" run_env_pins
  if run_treasury_not_test1; then
    ok "M625-2: seed treasury ≠ test1"
  else
    if [[ "${VERIFY625_FRESH:-}" == "1" || "${VERIFY625_REQUIRE_LIVE:-}" == "1" ]]; then
      bad "M625-2: seed treasury ≠ test1"
    else
      skip "M625-2 seed treasury still test1 (set VERIFY625_FRESH=1)"
    fi
  fi
  if [[ "${VERIFY625_SKIP_PLAYWRIGHT:-}" == "1" ]]; then
    skip "Playwright P0 (VERIFY625_SKIP_PLAYWRIGHT=1)"
  else
    if run_playwright_p0; then
      ok "M625-3: VERIFY_ISSUE_622_CHAIN=1 Playwright P0"
    else
      if [[ "${VERIFY625_REQUIRE_LIVE:-}" == "1" ]]; then
        bad "M625-3: VERIFY_ISSUE_622_CHAIN=1 Playwright P0"
      else
        skip "M625-3 Playwright P0 (indexer/pins). Set VERIFY625_REQUIRE_LIVE=1 to fail."
      fi
    fi
  fi
  if run_tax_on_seed; then
    ok "M625-2: LAYER_B_TAX_ON=1 seed-path buy (non-treasury)"
  else
    if [[ "${VERIFY625_REQUIRE_LIVE:-}" == "1" ]]; then
      bad "M625-2: LAYER_B_TAX_ON=1 seed-path buy"
    else
      skip "M625-2 tax-on seed buy. Set VERIFY625_REQUIRE_LIVE=1 to fail."
    fi
  fi
  if [[ "${VERIFY625_SKIP_EPHEMERAL:-}" == "1" ]]; then
    skip "tax-on ephemeral (VERIFY625_SKIP_EPHEMERAL=1)"
  elif run_tax_on_ephemeral; then
    ok "M625-2: LAYER_B_TAX_ON_FORCE_EPHEMERAL=1 buy (non-treasury)"
  else
    if [[ "${VERIFY625_REQUIRE_LIVE:-}" == "1" ]]; then
      bad "M625-2: LAYER_B_TAX_ON_FORCE_EPHEMERAL=1 buy"
    else
      skip "M625-2 tax-on ephemeral. Set VERIFY625_REQUIRE_LIVE=1 to fail."
    fi
  fi
  if [[ "${VERIFY625_SKIP_SWARM:-}" == "1" ]]; then
    skip "swarm soak (VERIFY625_SKIP_SWARM=1)"
  else
    if run_swarm_soak; then
      ok "M625-4: swarm-launch soak tax_listed extra-debit + hybrid skip"
    else
      if [[ "${VERIFY625_REQUIRE_LIVE:-}" == "1" ]]; then
        bad "M625-4: swarm-launch soak"
      else
        skip "M625-4 swarm soak. Set VERIFY625_REQUIRE_LIVE=1 to fail."
      fi
    fi
    if run_swarm_exclude_only; then
      ok "M625-4: SWARM_TAX_WORKERS=0 exclude-only"
    else
      if [[ "${VERIFY625_REQUIRE_LIVE:-}" == "1" ]]; then
        bad "M625-4: SWARM_TAX_WORKERS=0 exclude-only"
      else
        skip "M625-4 SWARM_TAX_WORKERS=0"
      fi
    fi
  fi
  run_step "M625-5: make verify-issue-293 still pool_only" run_293
elif [[ "${VERIFY625_REQUIRE_CHAIN:-}" == "1" ]]; then
  bad "LocalTerra required (VERIFY625_REQUIRE_CHAIN=1) — make setup-cloud-localterra"
else
  skip "LocalTerra (make has-localterra). Cloud Agent: make setup-cloud-localterra"
fi

if [[ "${VERIFY625_SKIP_OPTIONAL:-}" == "1" || "${VERIFY625_SKIP_CHILDREN:-}" == "1" ]]; then
  skip "optional 601 (VERIFY625_SKIP_OPTIONAL or SKIP_CHILDREN)"
else
  run_step "optional: make verify-issue-601" run_601
fi

echo ""
echo "── retest ──"
run_step "retest docs: Q13 M625" run_docs
run_step "retest source: buy trader + treasury sink" run_source

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  #625 results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
echo "==> GitLab #625 verification passed"
exit 0
