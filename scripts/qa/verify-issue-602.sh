#!/usr/bin/env bash
# Automated verification for GitLab #602 — post-merge !402 Coolify + launcher
# + LocalTerra Create Token QA.
#
# Proves (docs + children + live Coolify + optional LocalTerra):
#   1. Q9 / M602-1–M602-8 documented and crosslinked.
#   2. Child make verify-issue-{593,594}.
#   3. P402-1/2 live Coolify pins 11611 + canonical launcher address (not unused 11612).
#   4. P402-3 columbus-5 launcher CreateToken (code 11622); factory does not list sisters.
#   5. P402-4 LocalTerra community-tax smoke when chain is up.
#   6. P402-5 /create has no query prefill; Create Token next-link is /create.
#   7. P402-6 extra-debit Max + attested_cmm default catalog.
#
# VERIFY602_SKIP_CHILDREN=1 — docs + live + source only.
# VERIFY602_SKIP_LIVE=1 — skip dex.cl8y.com / indexer.dex.cl8y.com probes.
# VERIFY602_REQUIRE_CHAIN=1 — FAIL (do not SKIP) when LocalTerra smoke is missing.
# VERIFY602_SKIP_CHAIN=1 — skip LocalTerra smoke even if the chain is up.
#
# Refs: skills/AGENTS_POST_MERGE_OPS_602.md, docs/qa-invariants.md § Q9
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
echo "  GitLab #602 — post-merge !402 Coolify + Create Token QA"
echo "════════════════════════════════════════════════════════════════"

export PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-31602}"

LAUNCHER_C5="terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze"
UNUSED_11612="terra1af9xm63mev4hnf4z0nmmcsnd9f4lpac2vs205rmaeg3kdqlqudhq894lyz"
FACTORY_C5="terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea"
LCD_C5="${VERIFY602_LCD:-https://terra-classic-lcd.publicnode.com}"
DAPP_URL="${VERIFY602_DAPP_URL:-https://dex.cl8y.com}"
INDEXER_URL="${VERIFY602_INDEXER_URL:-https://indexer.dex.cl8y.com}"
STORE_TX="33F6A49F7221A377132D0A2B534A48D5AC64A5CA1F30D20BBE8A34086D3A45B8"
INSTANTIATE_TX="041E3C4379E88CE073B2EEED0125BEC58BCCAC31711AC15500352581763287FE"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_docs() {
  set -euo pipefail
  test -f skills/AGENTS_POST_MERGE_OPS_602.md
  grep -qE '\*\*M602-1\*\*' docs/qa-invariants.md
  grep -qE '\*\*M602-8\*\*' docs/qa-invariants.md
  grep -qE 'post-merge-ops-602' docs/qa-invariants.md
  grep -qE '\*\*M602-1' skills/AGENTS_POST_MERGE_OPS_602.md
  grep -qE 'make verify-issue-602' skills/AGENTS_POST_MERGE_OPS_602.md
  grep -qE 'AGENTS_POST_MERGE_OPS_602' AGENTS.md
  grep -qE 'verify-issue-602' AGENTS.md
  grep -qE 'verify-issue-602' Makefile
  grep -qE '#602' docs/testing.md
  grep -qE 'AGENTS_POST_MERGE_OPS_602' docs/README.md
  grep -qE '#602' skills/AGENTS_FRONTEND_CREATE_TOKEN.md
  grep -qE '#602' skills/AGENTS_INDEXER_COMMUNITY_TOKENS.md
  grep -qE '#602' skills/AGENTS_COMMUNITY_TAX_CW20.md
  grep -qF "$LAUNCHER_C5" skills/AGENTS_POST_MERGE_OPS_602.md
  grep -qF "$STORE_TX" skills/AGENTS_POST_MERGE_OPS_602.md
  grep -qF "$INSTANTIATE_TX" skills/AGENTS_POST_MERGE_OPS_602.md
  grep -qE 'Do \*\*not\*\* bake unused|Do not bake unused' skills/AGENTS_POST_MERGE_OPS_602.md
  # Product docs must not tell operators to point Coolify at unused 11612.
  if grep -nF "VITE_COMMUNITY_TOKEN_LAUNCHER=$UNUSED_11612" \
      docs/frontend.md skills/AGENTS_FRONTEND_CREATE_TOKEN.md \
      deployments/mainnet-ust1-wrap/coolify.env.example frontend-dapp/.env.example indexer/.env.example; then
    echo "docs still pin unused 11612 launcher for Coolify" >&2
    return 1
  fi
  if grep -nF "AddWhitelistedCodeId 11612" docs/runbooks/cw20-whitelist-policy.md skills/AGENTS_COMMUNITY_TAX_CW20.md; then
    echo "docs tell operators to whitelist launcher 11612" >&2
    return 1
  fi
}

run_p402_5_source() {
  set -euo pipefail
  grep -qE 'to="/create"' frontend-dapp/src/pages/CreateTokenPage.tsx
  if grep -nE 'to=\{`/create\?' frontend-dapp/src/pages/CreateTokenPage.tsx; then
    echo "Create Token next-link must not prefill /create query" >&2
    return 1
  fi
  if grep -nE 'useSearchParams' frontend-dapp/src/pages/CreatePairPage.tsx; then
    echo "Create Pair must not read URL search params (#542 / P402-5)" >&2
    return 1
  fi
  grep -qE 'create-token-next-create-pair' frontend-dapp/src/pages/CreateTokenPage.tsx
  grep -qE 'does not prefill Token A/B from /create query' frontend-dapp/src/pages/CreatePairPage.test.tsx
}

run_p402_6_source() {
  set -euo pipefail
  grep -q extraDebitSellBps frontend-dapp/src/pages/SwapPage.tsx
  grep -q extraDebitSellBps frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx
  grep -qE 'attested_cmm' indexer/src/api/community_tokens.rs
  grep -qE 'include_unattested' indexer/src/api/community_tokens.rs
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
  if ! printf '%s' "$js" | grep -qE 'VITE_COMMUNITY_TAX_CODE_ID:"(11611|11619)"'; then
    echo "dex.cl8y.com bundle missing VITE_COMMUNITY_TAX_CODE_ID 11611 or 11619" >&2
    return 1
  fi
  printf '%s' "$js" | grep -qF "$LAUNCHER_C5"
  printf '%s' "$js" | grep -q '/token/create'
  printf '%s' "$js" | grep -q 'Create Token'
  if printf '%s' "$js" | grep -qF "$UNUSED_11612"; then
    echo "Coolify frontend still bakes unused 11612 launcher" >&2
    return 1
  fi
  echo "dex.cl8y.com bundle pins community-tax code id + canonical launcher address"
}

run_live_indexer() {
  set -euo pipefail
  local body
  body="$(curl -fsS --max-time 20 "${INDEXER_URL%/}/api/v1/community-tokens")"
  echo "$body" | jq -e '.configured == true and (.code_id == 11611 or .code_id == 11619)' >/dev/null
  echo "$body" | jq -e '(.items | type) == "array"' >/dev/null
  echo "indexer community-tokens configured=true code_id=$(echo "$body" | jq -r '.code_id') items=$(echo "$body" | jq '.total // (.items|length)')"
}

run_c5_launcher() {
  set -euo pipefail
  # shellcheck source=scripts/lib/lcd-smart-query.sh
  source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"
  local raw ids admin
  raw="$(localterra_host_curl "${LCD_C5%/}/cosmwasm/wasm/v1/contract/${LAUNCHER_C5}" 2>/dev/null \
    || curl -fsS --max-time 30 "${LCD_C5%/}/cosmwasm/wasm/v1/contract/${LAUNCHER_C5}")"
  echo "$raw" | jq -e '.contract_info.code_id == "11622" or (.contract_info.code_id|tonumber) == 11622' >/dev/null
  admin="$(echo "$raw" | jq -r '.contract_info.admin // empty')"
  [[ "$admin" == "terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7" ]]
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
  echo "columbus-5 launcher 11622 admin=$admin whitelist=$(echo "$ids" | jq -c '.code_ids')"
}

run_593() { make verify-issue-593; }
run_594() { make verify-issue-594; }

run_smoke() {
  chmod +x "$REPO_ROOT/scripts/qa/localterra-community-tax-smoke.sh"
  "$REPO_ROOT/scripts/qa/localterra-community-tax-smoke.sh"
  local json="${VERIFY601_SMOKE_JSON:-/tmp/cl8y-601-smoke.json}"
  jq -e '.executed == true
    and .free_profile_create == true
    and .launcher_admin_cmm == true
    and .sell_extra_debit == true
    and .sku_unlock_50_ust1 == true
    and .settings_batch_50_ust1 == true' "$json" >/dev/null
}

run_e2e_smoke() {
  # UI-only: skip LocalTerra mint provision (shared account / mint-disabled tokens).
  PLAYWRIGHT_SKIP_CHAIN=1 bash "$REPO_ROOT/scripts/with-node.sh" --cwd frontend-dapp -- \
    ./node_modules/.bin/playwright test --project=e2e-smoke e2e/create-token-602.spec.ts
}

echo ""
echo "── first pass ──"
run_step "docs: Q9 M602-1–M602-8 + skill + AGENTS crosslinks" run_docs
run_step "source: P402-5 /create copy-address only (no query prefill)" run_p402_5_source
run_step "source: P402-6 extra-debit Max + attested_cmm default" run_p402_6_source

if [[ "${VERIFY602_SKIP_CHILDREN:-}" == "1" ]]; then
  skip "children 593/594 (VERIFY602_SKIP_CHILDREN=1)"
else
  run_step "child: make verify-issue-593" run_593
  run_step "child: make verify-issue-594" run_594
fi

if [[ "${VERIFY602_SKIP_LIVE:-}" == "1" ]]; then
  skip "live Coolify probes (VERIFY602_SKIP_LIVE=1)"
else
  run_step "P402-1: dex.cl8y.com bakes 11611 + canonical launcher" run_live_dapp
  run_step "P402-2: indexer.dex.cl8y.com community-tokens configured" run_live_indexer
  run_step "P402-3: columbus-5 launcher 11622 CreateToken; no sister whitelist" run_c5_launcher
fi

HAS_LT=1
if timeout 20 make -s has-localterra >/dev/null 2>&1; then
  HAS_LT=0
fi
if [[ "${VERIFY602_SKIP_CHAIN:-}" == "1" ]]; then
  skip "LocalTerra smoke (VERIFY602_SKIP_CHAIN=1)"
elif [[ "$HAS_LT" -eq 0 ]]; then
  run_step "P402-4: LocalTerra community-tax smoke (free create + invoices)" run_smoke
  if [[ -x "$REPO_ROOT/frontend-dapp/node_modules/.bin/playwright" ]]; then
    run_step "P402-4/5: Playwright create-token-602 e2e-smoke (5 workers)" run_e2e_smoke
  else
    skip "Playwright create-token-602 (no frontend-dapp/node_modules)"
  fi
else
  if [[ "${VERIFY602_REQUIRE_CHAIN:-}" == "1" ]]; then
    bad "LocalTerra required (VERIFY602_REQUIRE_CHAIN=1) — make setup-cloud-localterra"
  else
    skip "LocalTerra smoke (make has-localterra). Cloud Agent: make setup-cloud-localterra"
  fi
fi

echo ""
echo "── retest ──"
run_step "retest docs: Q9 M602" run_docs
run_step "retest source: P402-5 /create no query prefill" run_p402_5_source

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  #602 results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
echo "==> GitLab #602 verification passed"
exit 0
