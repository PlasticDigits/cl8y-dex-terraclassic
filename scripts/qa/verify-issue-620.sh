#!/usr/bin/env bash
# Automated verification for GitLab #620 — LocalTerra community-tax seed +
# Transfer funding + indexer env.
#
# Invariants L620-1–L620-8: skills/AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md
#
# Static + unit always. Optional LCD probes when LocalTerra is up and
# frontend-dapp/.env.local already has the QA tax pins (after make deploy-local).
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
echo "  GitLab #620 — LocalTerra community-tax seed + Transfer funding"
echo "════════════════════════════════════════════════════════════════"

run_docs() {
  set -euo pipefail
  rg -q "L620-1" skills/AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md
  rg -q "DEPLOY_SKIP_COMMUNITY_TAX" skills/AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md
  rg -q "AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED" skills/AGENTS_COMMUNITY_TAX_CW20.md
  rg -q "AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED" skills/AGENTS_COMMUNITY_TAX_AUTOLP.md
  rg -q "AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED" skills/AGENTS_INDEXER_COMMUNITY_TOKENS.md
  rg -q "AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED" skills/AGENTS_LOCALNET_TRADING_SWARM.md
  rg -q "AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED" skills/AGENTS_LOCAL_POSTGRES_DEV.md
  rg -q "verify-issue-620" AGENTS.md
  rg -q "verify-issue-620" docs/testing.md
  rg -q "L620-1" docs/contracts-security-audit.md
  rg -q "community-tax seed" docs/local-development.md
  rg -q "VITE_TOKEN_COMMUNITY_TAX_ADDRESS" frontend-dapp/.env.example
}

run_deploy_hooks() {
  set -euo pipefail
  rg -q "deploy_community_tax_local" scripts/deploy-dex-local.sh
  rg -q "DEPLOY_SKIP_COMMUNITY_TAX" scripts/deploy-dex-local.sh
  rg -q "VITE_TOKEN_COMMUNITY_TAX_ADDRESS" scripts/deploy-dex-local.sh
  rg -q "COMMUNITY_TAX_OPTION2_CODE_IDS" scripts/deploy-dex-local.sh
  rg -q "features:\\[\"auto_v2_lp\"\\]" scripts/lib/deploy-community-tax-local.sh
  rg -q "register_listed_pair" scripts/lib/deploy-community-tax-local.sh
  rg -q "update_config" scripts/lib/deploy-community-tax-local.sh
  bash -n scripts/deploy-dex-local.sh
  bash -n scripts/lib/deploy-community-tax-local.sh
  bash -n scripts/lib/cw20-funding-kind.sh
  bash -n scripts/e2e-provision-dev-wallet.sh
}

run_no_c5_whitelist() {
  set -euo pipefail
  # Local seed must never AddWhitelistedCodeId a columbus-5 pin.
  if rg -n "add_whitelisted_code_id.*11611" scripts/lib/deploy-community-tax-local.sh \
    scripts/deploy-dex-local.sh; then
    echo "deploy tells LocalTerra to whitelist columbus-5 11611" >&2
    return 1
  fi
  if rg -n "AddWhitelistedCodeId 11611" scripts/lib/deploy-community-tax-local.sh \
    docs/local-development.md skills/AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md; then
    echo "docs/scripts tell ops to whitelist 11611 from LocalTerra" >&2
    return 1
  fi
  rg -q "_CTAX_FORBIDDEN_CODE_IDS" scripts/lib/deploy-community-tax-local.sh
  rg -q "11611 11612 11613 11614 11619 11620 11621 11622 8654" \
    scripts/lib/deploy-community-tax-local.sh
}

run_funding_fork() {
  set -euo pipefail
  rg -q "classify_cw20_funding_kind" scripts/e2e-provision-dev-wallet.sh
  rg -q "fail-closed" scripts/e2e-provision-dev-wallet.sh
  rg -q '"transfer"' scripts/e2e-provision-dev-wallet.sh
  rg -q "classifyCw20FundingKind" packages/localnet-trading-swarm/src/funding.ts
  rg -q "fundingEnvFromVite" packages/localnet-trading-swarm/src/swarmRunner.ts
  # Transfer path must not fall back to Mint on the tax token.
  if rg -n 'features:\[.*"mint_control"' scripts/lib/deploy-community-tax-local.sh; then
    echo "QA seed must not enable MintControl" >&2
    return 1
  fi
  rg -q 'mint_control == false' scripts/lib/deploy-community-tax-local.sh
  chmod +x scripts/qa/test-cw20-funding-kind.sh
  scripts/qa/test-cw20-funding-kind.sh
}

run_swarm_unit() {
  set -euo pipefail
  if [[ ! -d packages/localnet-trading-swarm/node_modules ]]; then
    bash scripts/with-node.sh --cwd packages/localnet-trading-swarm -- npm ci
  fi
  bash scripts/with-node.sh --cwd packages/localnet-trading-swarm -- npm run test:run -- src/funding.test.ts
}

run_retest_static() {
  run_docs
  run_deploy_hooks
  run_no_c5_whitelist
  run_funding_fork
}

run_live_lcd() {
  set -euo pipefail
  # shellcheck source=scripts/lib/localterra-host-curl.sh
  source "$REPO_ROOT/scripts/lib/localterra-host-curl.sh"
  # shellcheck source=scripts/lib/lcd-smart-query.sh
  source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"
  local env_local="$REPO_ROOT/frontend-dapp/.env.local"
  [[ -f "$env_local" ]] || {
    echo "no .env.local" >&2
    return 1
  }
  set -a
  # shellcheck disable=SC1090
  source <(grep -E '^VITE_[A-Z0-9_]+=' "$env_local")
  set +a
  [[ -n "${VITE_TOKEN_COMMUNITY_TAX_ADDRESS:-}" && -n "${VITE_FACTORY_ADDRESS:-}" ]] || {
    echo "tax pins unset — run make deploy-local without DEPLOY_SKIP_COMMUNITY_TAX" >&2
    return 1
  }
  local lcd="${VITE_TERRA_LCD_URL:-http://localhost:1317}"
  lcd="${lcd%/}"
  local origin pair alp code_ids
  origin="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$lcd" "$VITE_TOKEN_COMMUNITY_TAX_ADDRESS" '{"get_launcher_origin":{}}')")"
  echo "$origin" | jq -e --arg l "$VITE_COMMUNITY_TOKEN_LAUNCHER" '.launcher == $l' >/dev/null
  pair="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$lcd" "$VITE_FACTORY_ADDRESS" \
    "$(jq -nc --arg a "$VITE_TOKEN_COMMUNITY_TAX_ADDRESS" --arg b "$VITE_TOKEN_EMBER_ADDRESS" \
      '{pair:{asset_infos:[{token:{contract_addr:$a}},{token:{contract_addr:$b}}]}}')")")"
  echo "$pair" | jq -e --arg p "$VITE_PAIR_COMMUNITY_TAX_EMBER" \
    '.contract_addr == $p or .pair.contract_addr == $p' >/dev/null
  local cfg
  cfg="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$lcd" "$VITE_TOKEN_COMMUNITY_TAX_ADDRESS" '{"get_config":{}}')")"
  alp="$(echo "$cfg" | jq -r '.autolp // empty')"
  [[ "$alp" == terra1* ]]
  local alp_cfg
  alp_cfg="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$lcd" "$alp" '{"get_config":{}}')")"
  echo "$alp_cfg" | jq -e --arg p "$VITE_PAIR_COMMUNITY_TAX_EMBER" '.pair == $p' >/dev/null
  # GetWhitelistedCodeIds is paginated; probe the local id + columbus-5 pins directly.
  local wl
  wl="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$lcd" "$VITE_FACTORY_ADDRESS" \
    "$(jq -nc --argjson c "${VITE_COMMUNITY_TAX_CODE_ID}" '{is_code_id_whitelisted:{code_id:$c}}')")")"
  echo "$wl" | jq -e '.whitelisted == true' >/dev/null
  for pin in 11611 11619 11612 11621 8654; do
    wl="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$lcd" "$VITE_FACTORY_ADDRESS" \
      "$(jq -nc --argjson c "$pin" '{is_code_id_whitelisted:{code_id:$c}}')")")"
    echo "$wl" | jq -e '.whitelisted == false' >/dev/null
  done
  echo "live LCD: token=$VITE_TOKEN_COMMUNITY_TAX_ADDRESS pair=$VITE_PAIR_COMMUNITY_TAX_EMBER autolp=$alp"
}

echo ""
echo "── first pass ──"
run_step "docs: L620 + skill crosslinks" run_docs
run_step "deploy: tax phase + skip flag + syntax" run_deploy_hooks
run_step "guard: no columbus-5 whitelist from LocalTerra seed" run_no_c5_whitelist
run_step "funding: Transfer fork + bash classify unit" run_funding_fork
run_step "swarm: fundingKind vitest" run_swarm_unit

HAS_LT=1
if timeout 20 make -s has-localterra >/dev/null 2>&1; then
  HAS_LT=0
fi
if [[ "$HAS_LT" -eq 0 ]] && grep -qE '^VITE_TOKEN_COMMUNITY_TAX_ADDRESS=terra1' \
  frontend-dapp/.env.local 2>/dev/null; then
  run_step "live LCD: factory pair + AutoLP pair + local whitelist" run_live_lcd
else
  echo "  [SKIP] live LCD (need LocalTerra + tax pins from make deploy-local)"
  RESULTS+=("SKIP  live LCD tax market")
fi

echo ""
echo "── retest ──"
run_step "retest docs + deploy hooks + funding classify" run_retest_static
run_step "retest swarm: fundingKind vitest" run_swarm_unit

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  #620 results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
echo "==> GitLab #620 verification passed"
exit 0
