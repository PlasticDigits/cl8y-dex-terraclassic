#!/usr/bin/env bash
# Automated verification for GitLab #612 — post-merge !407/!408 Enable Feature
# migrate + LocalTerra QA.
#
# Proves (docs + children + live Coolify + optional LocalTerra):
#   1. Q10 / M612-1–M612-8 documented and crosslinked.
#   2. Child make verify-issue-{606,607}.
#   3. Columbus-5 launcher 11622 + GetConfig 11619/11621; sisters not listed.
#   4. Coolify bakes 11619 + canonical launcher; single communityTaxHint.
#   5. LocalTerra verify-issue-601 smoke: sku_unlock_via_launcher + paid
#      create + second SKU unlock via launcher.
#   6. Playwright Enable Feature chrome (e2e-smoke, 5 workers).
#
# Do NOT treat option-1 "Route skips buy/sell tax" as a pass (#616 / !409).
#
# VERIFY612_SKIP_CHILDREN=1 — docs + live + source only.
# VERIFY612_SKIP_LIVE=1 — skip dex.cl8y.com / indexer / columbus-5 probes.
# VERIFY612_REQUIRE_CHAIN=1 — FAIL (do not SKIP) when LocalTerra smoke is missing.
# VERIFY612_SKIP_CHAIN=1 — skip LocalTerra / verify-issue-601 even if chain is up.
#
# Refs: skills/AGENTS_POST_MERGE_OPS_612.md, docs/qa-invariants.md § Q10
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
  if "$@"; then
    ok "$label"
  else
    bad "$label"
  fi
}

echo "════════════════════════════════════════════════════════════════"
echo "  GitLab #612 — post-merge !407/!408 Enable Feature + LocalTerra"
echo "════════════════════════════════════════════════════════════════"

export PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-31612}"

LAUNCHER_C5="terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze"
UNUSED_11612="terra1af9xm63mev4hnf4z0nmmcsnd9f4lpac2vs205rmaeg3kdqlqudhq894lyz"
FACTORY_C5="terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea"
LCD_C5="${VERIFY612_LCD:-https://terra-classic-lcd.publicnode.com}"
DAPP_URL="${VERIFY612_DAPP_URL:-https://dex.cl8y.com}"
INDEXER_URL="${VERIFY612_INDEXER_URL:-https://indexer.dex.cl8y.com}"
MIGRATE_TX="F2166AB0C09B4E7989AB10DC8DCC4D5855B4E3F91C7E4F8C6D5B8F780947AAB2"
UPDATE_TX="DAC86F27B4E95FC83461B733453A9EF1028BC8421F2FE3AE022B0A14DADF6ED3"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_docs() {
  set -euo pipefail
  test -f skills/AGENTS_POST_MERGE_OPS_612.md
  grep -qE '\*\*M612-1\*\*' docs/qa-invariants.md
  grep -qE '\*\*M612-8\*\*' docs/qa-invariants.md
  grep -qE 'post-merge-ops-612' docs/qa-invariants.md
  grep -qE '\*\*M612-1' skills/AGENTS_POST_MERGE_OPS_612.md
  grep -qE 'make verify-issue-612' skills/AGENTS_POST_MERGE_OPS_612.md
  grep -qE 'AGENTS_POST_MERGE_OPS_612' AGENTS.md
  grep -qE 'verify-issue-612' AGENTS.md
  grep -qE 'verify-issue-612' Makefile
  grep -qE '#612' docs/testing.md
  grep -qE 'AGENTS_POST_MERGE_OPS_612' docs/README.md
  grep -qE '#612' skills/AGENTS_COMMUNITY_TAX_ENABLE_FEATURE.md
  grep -qE '#612' skills/AGENTS_COMMUNITY_TAX_CW20.md
  grep -qE '#612' skills/AGENTS_FRONTEND_CREATE_TOKEN.md
  grep -qE '#612' skills/AGENTS_COMMUNITY_TAX_ROUTER.md
  grep -qF "$LAUNCHER_C5" skills/AGENTS_POST_MERGE_OPS_612.md
  grep -qF "$MIGRATE_TX" skills/AGENTS_POST_MERGE_OPS_612.md
  grep -qF "$UPDATE_TX" skills/AGENTS_POST_MERGE_OPS_612.md
  grep -qE 'sku_unlock_via_launcher' skills/AGENTS_POST_MERGE_OPS_612.md
  grep -qE 'sku_second_unlock_via_launcher' skills/AGENTS_POST_MERGE_OPS_612.md
  grep -qE 'Do \*\*not\*\* run the stale option-1|Do not run the stale option-1' \
    skills/AGENTS_POST_MERGE_OPS_612.md
  if grep -nF "VITE_COMMUNITY_TOKEN_LAUNCHER=$UNUSED_11612" \
      docs/frontend.md skills/AGENTS_FRONTEND_CREATE_TOKEN.md \
      skills/AGENTS_POST_MERGE_OPS_612.md \
      deployments/mainnet-ust1-wrap/coolify.env.example frontend-dapp/.env.example indexer/.env.example; then
    echo "docs still pin unused 11612 launcher for Coolify" >&2
    return 1
  fi
  if grep -nF "AddWhitelistedCodeId 11612" docs/runbooks/cw20-whitelist-policy.md \
      skills/AGENTS_COMMUNITY_TAX_CW20.md skills/AGENTS_POST_MERGE_OPS_612.md; then
    echo "docs tell operators to whitelist launcher 11612" >&2
    return 1
  fi
  grep -qE 'stale option-1|Do \*\*not\*\* run the stale option-1' \
    skills/AGENTS_POST_MERGE_OPS_612.md
}

run_source() {
  set -euo pipefail
  # Official Enable Feature payee is the env launcher (T606-1 / C593-4).
  grep -qF 'payee: input.launcher' frontend-dapp/src/utils/communityTaxInvoice.ts
  grep -qF 'enable_feature: { token: input.token' frontend-dapp/src/utils/communityTaxInvoice.ts
  grep -qE 'manage-enable-feature' frontend-dapp/src/pages/ManageTokenPage.tsx
  grep -qE 'manage-unlock-sku' frontend-dapp/src/pages/ManageTokenPage.tsx
  grep -qE 'manage-token-invalid' frontend-dapp/src/pages/ManageTokenPage.tsx
  grep -qF 'send_cw20_hook "$UST1_ADDR" "$LAUNCHER_ADDR"' \
    scripts/qa/localterra-community-tax-smoke.sh
  grep -q 'sku_unlock_via_launcher' scripts/qa/localterra-community-tax-smoke.sh
  grep -q 'sku_second_unlock_via_launcher' scripts/qa/localterra-community-tax-smoke.sh
  grep -q 'paid_create_one_sku' scripts/qa/localterra-community-tax-smoke.sh
  # Single communityTaxHint binding (TS2451 leftover).
  local hints
  hints="$(grep -c 'const communityTaxHint' frontend-dapp/src/pages/SwapPage.tsx || true)"
  [[ "$hints" == "1" ]] || {
    echo "SwapPage must declare communityTaxHint once (got $hints)" >&2
    return 1
  }
  # Current policy is option 2 — do not ship option-1 skip copy.
  if grep -nF 'Route skips buy/sell tax' frontend-dapp/src/utils/taxPreviewMaxSpend.ts \
      frontend-dapp/src/pages/SwapPage.tsx \
      frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx; then
    echo "stale option-1 skip hint still in dApp" >&2
    return 1
  fi
  grep -qF 'Sell tax extra' frontend-dapp/src/utils/taxPreviewMaxSpend.ts
  grep -qF 'Buy tax applies' frontend-dapp/src/utils/taxPreviewMaxSpend.ts
}

run_live_dapp() {
  set -euo pipefail
  local html js_path js
  html="$(curl -fsS --max-time 30 "${DAPP_URL%/}/")"
  js_path="$(printf '%s' "$html" | grep -oE '/assets/index-[^"]+\.js' | head -1)"
  [[ -n "$js_path" ]] || {
    echo "no Vite index bundle in $DAPP_URL" >&2
    return 1
  }
  js="$(curl -fsS --max-time 45 "${DAPP_URL%/}${js_path}")"
  printf '%s' "$js" | grep -qE 'VITE_COMMUNITY_TAX_CODE_ID:"11619"'
  printf '%s' "$js" | grep -qF "$LAUNCHER_C5"
  printf '%s' "$js" | grep -q '/token/create'
  if printf '%s' "$js" | grep -qF "$UNUSED_11612"; then
    echo "Coolify frontend still bakes unused 11612 launcher" >&2
    return 1
  fi
  if printf '%s' "$js" | grep -qF 'Route skips buy/sell tax'; then
    echo "Coolify frontend still ships stale option-1 skip copy" >&2
    return 1
  fi
  echo "dex.cl8y.com bundle pins 11619 + canonical launcher; no unused 11612"
}

run_live_indexer() {
  set -euo pipefail
  local body
  body="$(curl -fsS --max-time 20 "${INDEXER_URL%/}/api/v1/community-tokens")"
  echo "$body" | jq -e '.configured == true and (.code_id == 11611 or .code_id == 11619)' >/dev/null
  echo "$body" | jq -e '(.items | type) == "array"' >/dev/null
  echo "indexer community-tokens configured=true code_id=$(echo "$body" | jq -r '.code_id')"
}

run_c5_launcher() {
  set -euo pipefail
  # shellcheck source=scripts/lib/lcd-smart-query.sh
  source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"
  local raw ids cfg
  raw="$(localterra_host_curl "${LCD_C5%/}/cosmwasm/wasm/v1/contract/${LAUNCHER_C5}" 2>/dev/null \
    || curl -fsS --max-time 30 "${LCD_C5%/}/cosmwasm/wasm/v1/contract/${LAUNCHER_C5}")"
  echo "$raw" | jq -e '.contract_info.code_id == "11622" or (.contract_info.code_id|tonumber) == 11622' >/dev/null
  raw="$(lcd_smart_query_raw "$LCD_C5" "$LAUNCHER_C5" '{"get_config":{}}')"
  cfg="$(lcd_decode_smart_data "$raw")"
  echo "$cfg" | jq -e '.token_code_id == 11619 or (.token_code_id|tonumber) == 11619' >/dev/null
  echo "$cfg" | jq -e '.autolp_code_id == 11621 or (.autolp_code_id|tonumber) == 11621' >/dev/null
  raw="$(lcd_smart_query_raw "$LCD_C5" "$FACTORY_C5" '{"get_whitelisted_code_ids":{}}')"
  ids="$(lcd_decode_smart_data "$raw")"
  echo "$ids" | jq -e '.code_ids | index(11611) != null' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(11619) != null' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(11612) == null' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(11613) == null' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(11614) == null' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(11620) == null' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(11621) == null' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(11622) == null' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(8654) == null' >/dev/null
  echo "columbus-5 launcher 11622 token=11619 autolp=11621 whitelist=$(echo "$ids" | jq -c '.code_ids')"
}

run_606() { make verify-issue-606; }
run_607() { make verify-issue-607; }

run_601() {
  VERIFY601_REQUIRE_CHAIN=1 make verify-issue-601
  local json="${VERIFY601_SMOKE_JSON:-/tmp/cl8y-601-smoke.json}"
  jq -e '.executed == true
    and .sku_unlock_via_launcher == true
    and .sku_unlock_50_ust1 == true
    and .paid_create_one_sku == true
    and .sku_second_unlock_via_launcher == true' "$json" >/dev/null
}

run_e2e_smoke() {
  PLAYWRIGHT_SKIP_CHAIN=1 bash "$REPO_ROOT/scripts/with-node.sh" --cwd frontend-dapp -- \
    ./node_modules/.bin/playwright test --project=e2e-smoke e2e/enable-feature-612.spec.ts
}

echo ""
echo "── first pass ──"
run_step "docs: Q10 M612-1–M612-8 + skill + AGENTS crosslinks" run_docs
run_step "source: launcher Enable Feature + no option-1 skip copy" run_source

if [[ "${VERIFY612_SKIP_CHILDREN:-}" == "1" ]]; then
  skip "children 606/607 (VERIFY612_SKIP_CHILDREN=1)"
else
  run_step "child: make verify-issue-606" run_606
  run_step "child: make verify-issue-607" run_607
fi

if [[ "${VERIFY612_SKIP_LIVE:-}" == "1" ]]; then
  skip "live Coolify / columbus-5 probes (VERIFY612_SKIP_LIVE=1)"
else
  run_step "M612-3: dex.cl8y.com bakes 11619 + canonical launcher" run_live_dapp
  run_step "M612-3: indexer.dex.cl8y.com community-tokens configured" run_live_indexer
  run_step "M612-2: columbus-5 launcher 11622 GetConfig 11619/11621" run_c5_launcher
fi

HAS_LT=1
if timeout 20 make -s has-localterra >/dev/null 2>&1; then
  HAS_LT=0
fi
if [[ "${VERIFY612_SKIP_CHAIN:-}" == "1" ]]; then
  skip "LocalTerra / verify-issue-601 (VERIFY612_SKIP_CHAIN=1)"
elif [[ "$HAS_LT" -eq 0 ]]; then
  run_step "M612-4/5: verify-issue-601 smoke (launcher SKU + second unlock)" run_601
  if [[ -x "$REPO_ROOT/frontend-dapp/node_modules/.bin/playwright" ]]; then
    run_step "M612-6: Playwright enable-feature-612 e2e-smoke (5 workers)" run_e2e_smoke
  else
    skip "Playwright enable-feature-612 (no frontend-dapp/node_modules)"
  fi
else
  if [[ "${VERIFY612_REQUIRE_CHAIN:-}" == "1" ]]; then
    bad "LocalTerra required (VERIFY612_REQUIRE_CHAIN=1) — make setup-cloud-localterra"
  else
    skip "LocalTerra smoke (make has-localterra). Cloud Agent: make setup-cloud-localterra"
  fi
fi

echo ""
echo "── retest ──"
run_step "retest docs: Q10 M612" run_docs
run_step "retest source: launcher Enable Feature" run_source

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  #612 results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
echo "==> GitLab #612 verification passed"
exit 0
