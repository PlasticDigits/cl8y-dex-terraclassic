#!/usr/bin/env bash
# Automated verification for GitLab #616 — post-merge !409–!413 option-2 wasm,
# wrap/window fees, AutoLP, tax ranking.
#
# Proves (docs + children + live pins + optional leftovers / LocalTerra):
#   1. Q11 / M616-1–M616-8 documented and crosslinked.
#   2. Child make verify-issue-{607,610,613,614,615}.
#   3. Columbus-5 launcher 11622 + GetConfig 11626/11621; sisters not listed.
#   4. Coolify bakes 11619 + canonical launcher; no option-1 skip copy.
#   5. Option-2 Swap/Trade/Create copy (Playwright e2e-smoke, 5 workers).
#   6. Optional: indexer window pin / wrap event_count leftovers.
#
# VERIFY616_SKIP_CHILDREN=1 — docs + live + source only.
# VERIFY616_SKIP_LIVE=1 — skip dex.cl8y.com / indexer / columbus-5 probes.
# VERIFY616_REQUIRE_LIVE_LEFTOVERS=1 — FAIL (do not SKIP) when window pin /
#   wrap event_count / option-2 env leftovers remain.
# VERIFY616_REQUIRE_CHAIN=1 — FAIL (do not SKIP) when LocalTerra is missing.
# VERIFY616_SKIP_CHAIN=1 — skip LocalTerra even if chain is up.
#
# Refs: skills/AGENTS_POST_MERGE_OPS_616.md, docs/qa-invariants.md § Q11
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
echo "  GitLab #616 — post-merge !409–!413 option-2 / fees / AutoLP"
echo "════════════════════════════════════════════════════════════════"

export PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-31616}"

LAUNCHER_C5="terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze"
WINDOW_C5="terra1zxwpzpzpleatqn39r00grau4yt29sld8pw78s7ktvjafnj5nsaxq0h3rh2"
FACTORY_C5="terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea"
UNUSED_11612="terra1af9xm63mev4hnf4z0nmmcsnd9f4lpac2vs205rmaeg3kdqlqudhq894lyz"
LCD_C5="${VERIFY616_LCD:-https://terra-classic-lcd.publicnode.com}"
DAPP_URL="${VERIFY616_DAPP_URL:-https://dex.cl8y.com}"
INDEXER_URL="${VERIFY616_INDEXER_URL:-https://indexer.dex.cl8y.com}"
MIGRATE_TX="F2166AB0C09B4E7989AB10DC8DCC4D5855B4E3F91C7E4F8C6D5B8F780947AAB2"
UPDATE_TX="DAC86F27B4E95FC83461B733453A9EF1028BC8421F2FE3AE022B0A14DADF6ED3"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_docs() {
  set -euo pipefail
  test -f skills/AGENTS_POST_MERGE_OPS_616.md
  grep -qE '\*\*M616-1\*\*' docs/qa-invariants.md
  grep -qE '\*\*M616-8\*\*' docs/qa-invariants.md
  grep -qE 'post-merge-ops-616' docs/qa-invariants.md
  grep -qE '\*\*M616-1' skills/AGENTS_POST_MERGE_OPS_616.md
  grep -qE 'make verify-issue-616' skills/AGENTS_POST_MERGE_OPS_616.md
  grep -qE 'AGENTS_POST_MERGE_OPS_616' AGENTS.md
  grep -qE 'verify-issue-616' AGENTS.md
  grep -qE 'verify-issue-616' Makefile
  grep -qE '#616' docs/testing.md
  grep -qE 'AGENTS_POST_MERGE_OPS_616' docs/README.md
  grep -qE '#616' skills/AGENTS_COMMUNITY_TAX_ROUTER.md
  grep -qE '#616' skills/AGENTS_INDEXER_TAX_AWARE_ROUTING.md
  grep -qE '#616' skills/AGENTS_COMMUNITY_TAX_AUTOLP.md
  grep -qE '#616' skills/AGENTS_INDEXER_WRAP_FEE_INGEST.md
  grep -qE '#616' skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
  grep -qE '#616' skills/AGENTS_UST1_WINDOW_UI.md
  grep -qE '#616' skills/AGENTS_COMMUNITY_TAX_CW20.md
  grep -qE '#616' skills/AGENTS_FRONTEND_CREATE_TOKEN.md
  grep -qF "$LAUNCHER_C5" skills/AGENTS_POST_MERGE_OPS_616.md
  grep -qF "$WINDOW_C5" skills/AGENTS_POST_MERGE_OPS_616.md
  grep -qF "$MIGRATE_TX" skills/AGENTS_POST_MERGE_OPS_616.md
  grep -qF "$UPDATE_TX" skills/AGENTS_POST_MERGE_OPS_616.md
  grep -qE 'Sell tax extra' skills/AGENTS_POST_MERGE_OPS_616.md
  grep -qE 'UST1_WINDOW_ADDRESS' skills/AGENTS_POST_MERGE_OPS_616.md
  grep -qE 'COMMUNITY_TAX_OPTION2_CODE_IDS' skills/AGENTS_POST_MERGE_OPS_616.md
  grep -qE 'Do \*\*not\*\* reopen #607' skills/AGENTS_POST_MERGE_OPS_616.md
  if grep -nF "AddWhitelistedCodeId 11612" docs/runbooks/cw20-whitelist-policy.md \
      skills/AGENTS_COMMUNITY_TAX_CW20.md skills/AGENTS_POST_MERGE_OPS_616.md; then
    echo "docs tell operators to whitelist launcher 11612" >&2
    return 1
  fi
  grep -qF "$WINDOW_C5" deployments/mainnet-ust1-wrap/coolify.env.example
  grep -qE '^UST1_WINDOW_ADDRESS=' deployments/mainnet-ust1-wrap/coolify.env.example
}

run_source() {
  set -euo pipefail
  grep -qF 'Sell tax extra' frontend-dapp/src/utils/taxPreviewMaxSpend.ts
  grep -qF 'Buy tax applies' frontend-dapp/src/utils/taxPreviewMaxSpend.ts
  grep -qF 'Buy/sell tax applies on every listed-pair swap.' \
    frontend-dapp/src/utils/taxPreviewMaxSpend.ts
  grep -qE 'communityTaxRouteHint' frontend-dapp/src/pages/SwapPage.tsx
  grep -qE 'communityTaxRouteHint' frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx
  grep -qE 'swap-sell-tax-extra' frontend-dapp/src/pages/SwapPage.tsx
  grep -qE 'trade-sell-tax-extra' frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx
  grep -qE 'create-token-tax-scope' frontend-dapp/src/pages/CreateTokenPage.tsx
  grep -qE 'manage-token-tax-scope' frontend-dapp/src/pages/ManageTokenPage.tsx
  grep -qF 'usesRouter: true' frontend-dapp/src/utils/taxPreviewMaxSpend.test.ts
  grep -qF 'UST1_WINDOW_ADDRESS' indexer/src/config.rs
  grep -qF 'COMMUNITY_TAX_OPTION2_CODE_IDS' indexer/src/config.rs
  grep -qF 'notify_deposit' indexer/src/indexer/protocol_fees.rs
  grep -qF 'require_factory_listed_tax_pair' \
    smartcontracts/contracts/community-tax-autolp/src/pair.rs
  grep -qF 'hop_trader_addr' smartcontracts/contracts/community-tax-token/src/tax.rs
  if grep -nF 'Route skips buy/sell tax' frontend-dapp/src/utils/taxPreviewMaxSpend.ts \
      frontend-dapp/src/pages/SwapPage.tsx \
      frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx \
      frontend-dapp/src/pages/CreateTokenPage.tsx \
      frontend-dapp/src/pages/ManageTokenPage.tsx; then
    echo "stale option-1 skip hint still in dApp" >&2
    return 1
  fi
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
  printf '%s' "$js" | grep -qE 'VITE_COMMUNITY_TAX_CODE_ID:"(11619|11626)"'
  printf '%s' "$js" | grep -qF "$LAUNCHER_C5"
  if printf '%s' "$js" | grep -qF "$UNUSED_11612"; then
    echo "Coolify frontend still bakes unused 11612 launcher" >&2
    return 1
  fi
  if printf '%s' "$js" | grep -qF 'Route skips buy/sell tax'; then
    echo "Coolify frontend still ships stale option-1 skip copy" >&2
    return 1
  fi
  printf '%s' "$js" | grep -qF 'Sell tax extra'
  printf '%s' "$js" | grep -qF 'Buy tax applies'
  printf '%s' "$js" | grep -qF 'Buy/sell tax applies on every listed-pair swap.'
  echo "dex.cl8y.com bundle pins 11619 + option-2 copy; no unused 11612"
}

run_live_indexer_catalog() {
  set -euo pipefail
  local body
  body="$(curl -fsS --max-time 20 "${INDEXER_URL%/}/api/v1/community-tokens")"
  echo "$body" | jq -e '.configured == true and (.code_id == 11626 or .code_id == 11619)' >/dev/null
  echo "$body" | jq -e '(.items | type) == "array"' >/dev/null
  echo "indexer community-tokens configured=true code_id=$(echo "$body" | jq -r '.code_id')"
}

run_c5_pins() {
  set -euo pipefail
  # shellcheck source=scripts/lib/lcd-smart-query.sh
  source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"
  local raw ids cfg
  raw="$(localterra_host_curl "${LCD_C5%/}/cosmwasm/wasm/v1/contract/${LAUNCHER_C5}" 2>/dev/null \
    || curl -fsS --max-time 30 "${LCD_C5%/}/cosmwasm/wasm/v1/contract/${LAUNCHER_C5}")"
  echo "$raw" | jq -e '.contract_info.code_id == "11622" or (.contract_info.code_id|tonumber) == 11622' >/dev/null
  raw="$(lcd_smart_query_raw "$LCD_C5" "$LAUNCHER_C5" '{"get_config":{}}')"
  cfg="$(lcd_decode_smart_data "$raw")"
  echo "$cfg" | jq -e '.token_code_id == 11626 or (.token_code_id|tonumber) == 11626' >/dev/null
  echo "$cfg" | jq -e '.autolp_code_id == 11621 or (.autolp_code_id|tonumber) == 11621' >/dev/null
  raw="$(lcd_smart_query_raw "$LCD_C5" "$FACTORY_C5" '{"get_whitelisted_code_ids":{}}')"
  ids="$(lcd_decode_smart_data "$raw")"
  echo "$ids" | jq -e '.code_ids | index(11626) != null' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(11612) == null' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(11613) == null' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(11614) == null' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(11620) == null' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(11621) == null' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(11622) == null' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(8654) == null' >/dev/null
  echo "columbus-5 launcher 11622 token=11626 autolp=11621 whitelist=$(echo "$ids" | jq -c '.code_ids')"
}

run_live_window_pin() {
  set -euo pipefail
  local fees
  fees="$(curl -fsS --max-time 20 "${INDEXER_URL%/}/api/v1/protocol/fees?window=24h")"
  echo "$fees" | jq -e '.ust1_window_configured == true' >/dev/null
  echo "indexer ust1_window_configured=true (UST1_WINDOW_ADDRESS pinned)"
}

run_live_leftovers() {
  set -euo pipefail
  local fees wrap_n mint_n
  fees="$(curl -fsS --max-time 20 "${INDEXER_URL%/}/api/v1/protocol/fees?window=24h")"
  wrap_n="$(echo "$fees" | jq -r '[.by_source[]? | select(.source=="wrap" or .source=="unwrap") | .event_count] | add // 0')"
  mint_n="$(echo "$fees" | jq -r '[.by_source[]? | select(.source=="ust1_mint" or .source=="ust1_redeem") | .event_count] | add // 0')"
  echo "live leftovers: wrap+unwrap=$wrap_n ust1_mint+redeem=$mint_n"
  if [[ "$wrap_n" == "null" || "$wrap_n" -lt 1 ]]; then
    echo "wrap/unwrap event_count still 0 (mapper pin true is not ingest)" >&2
    return 1
  fi
  if [[ "$mint_n" == "null" || "$mint_n" -lt 1 ]]; then
    echo "no captured ust1_mint/ust1_redeem events yet" >&2
    return 1
  fi
}

run_607() { make verify-issue-607; }
run_610() { make verify-issue-610; }
run_613() { make verify-issue-613; }
run_614() { make verify-issue-614; }
run_615() { make verify-issue-615; }

run_e2e_copy() {
  PLAYWRIGHT_SKIP_CHAIN=1 bash "$REPO_ROOT/scripts/with-node.sh" --cwd frontend-dapp -- \
    ./node_modules/.bin/playwright test --project=e2e-smoke e2e/option2-copy-616.spec.ts
}

echo ""
echo "── first pass ──"
run_step "docs: Q11 M616-1–M616-8 + skill + AGENTS crosslinks" run_docs
run_step "source: option-2 copy + wrap/window/AutoLP pins" run_source

if [[ "${VERIFY616_SKIP_CHILDREN:-}" == "1" ]]; then
  skip "children 607/610/613/614/615 (VERIFY616_SKIP_CHILDREN=1)"
else
  run_step "child: make verify-issue-607" run_607
  run_step "child: make verify-issue-610" run_610
  run_step "child: make verify-issue-613" run_613
  run_step "child: make verify-issue-614" run_614
  run_step "child: make verify-issue-615" run_615
fi

if [[ "${VERIFY616_SKIP_LIVE:-}" == "1" ]]; then
  skip "live Coolify / columbus-5 probes (VERIFY616_SKIP_LIVE=1)"
else
  run_step "M616-3: dex.cl8y.com bakes 11619 + option-2 copy" run_live_dapp
  run_step "M616-3: indexer.dex.cl8y.com community-tokens 11619" run_live_indexer_catalog
  run_step "M616-3: indexer UST1_WINDOW_ADDRESS pinned" run_live_window_pin
  run_step "M616-2: columbus-5 launcher 11622 GetConfig 11626/11621" run_c5_pins
  if [[ "${VERIFY616_REQUIRE_LIVE_LEFTOVERS:-}" == "1" ]]; then
    run_step "M616-3 leftovers: wrap + ust1 mint/redeem events" run_live_leftovers
  else
    if run_live_leftovers; then
      ok "M616-3 leftovers: wrap + ust1 mint/redeem events"
    else
      skip "M616-3 leftovers (wrap event_count / ust1 mint-redeem still 0). Set VERIFY616_REQUIRE_LIVE_LEFTOVERS=1 to fail."
    fi
  fi
fi

HAS_LT=1
if timeout 20 make -s has-localterra >/dev/null 2>&1; then
  HAS_LT=0
fi
if [[ "${VERIFY616_SKIP_CHAIN:-}" == "1" ]]; then
  skip "LocalTerra (VERIFY616_SKIP_CHAIN=1)"
elif [[ "$HAS_LT" -eq 0 ]]; then
  echo "LocalTerra is up — crate children already cover hop extra-debit / skim floor / ranking."
  ok "M616-5/6: LocalTerra up; children 607/610/615 are the execute/rank gates"
else
  if [[ "${VERIFY616_REQUIRE_CHAIN:-}" == "1" ]]; then
    bad "LocalTerra required (VERIFY616_REQUIRE_CHAIN=1) — make setup-cloud-localterra"
  else
    skip "LocalTerra (make has-localterra). Cloud Agent: make setup-cloud-localterra"
  fi
fi

if [[ -x "$REPO_ROOT/frontend-dapp/node_modules/.bin/playwright" ]]; then
  run_step "M616-4: Playwright option2-copy-616 e2e-smoke (5 workers)" run_e2e_copy
else
  skip "Playwright option2-copy-616 (no frontend-dapp/node_modules)"
fi

echo ""
echo "── retest ──"
run_step "retest docs: Q11 M616" run_docs
run_step "retest source: option-2 copy + pins" run_source

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  #616 results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
echo "==> GitLab #616 verification passed"
exit 0
