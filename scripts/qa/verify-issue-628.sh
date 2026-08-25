#!/usr/bin/env bash
# Automated verification for GitLab #628 — post-merge !418 community-tax
# migrate leftovers (#626).
#
# Proves (docs + children 626/592/593/594 + leftover live):
#   1. Q14 / M628-1–M628-8 documented and crosslinked.
#   2. Children make verify-issue-626 / 592 / 593 / 594.
#   3. Source: migrate allowlist includes 8654; never factory-list 8654;
#      Create Token retail copy is code-id-free; Manage hides SKUs off-pin.
#   4. Columbus-5 / Coolify leftovers (SKIP unless reachable).
#   5. LocalTerra P3 / P7 / P11 (SKIP unless chain + tax pins).
#
# VERIFY628_SKIP_CHILDREN=1 — skip 626/592/593/594 (docs + source + live).
# VERIFY628_SKIP_LIVE=1 — skip columbus-5 / Coolify even if reachable.
# VERIFY628_SKIP_CHAIN=1 — skip LocalTerra leftovers even if the chain is up.
# VERIFY628_REQUIRE_CHAIN=1 — FAIL (do not SKIP) when LocalTerra is missing.
# VERIFY628_REQUIRE_LIVE=1 or VERIFY628_IID=628 — FAIL (do not SKIP) when
#   columbus-5 / Coolify / LocalTerra leftovers cannot run.
#
# Refs: skills/AGENTS_POST_MERGE_OPS_628.md, docs/qa-invariants.md § Q14
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }
skip() { RESULTS+=("SKIP  $1"); echo "  [SKIP] $1"; }

require_live() {
  [[ "${VERIFY628_REQUIRE_LIVE:-}" == "1" || "${VERIFY628_IID:-}" == "628" ]]
}

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
echo "  GitLab #628 — post-merge !418 migrate leftovers"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
LCD_C5="${VERIFY628_LCD:-https://terra-classic-lcd.publicnode.com}"
DAPP_URL="${VERIFY628_DAPP_URL:-https://dex.cl8y.com}"
INDEXER_URL="${VERIFY628_INDEXER_URL:-https://indexer.dex.cl8y.com}"
LAUNCHER_C5="terra126pr5323xkhwas7y03azv48sqr2fy3fxxg0sxu8xhmjdxr8v5tzqahzwze"
# Soft-launch factory (REGISTRY.md). Override with VERIFY628_FACTORY.
FACTORY_C5="${VERIFY628_FACTORY:-terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea}"

run_docs() {
  set -euo pipefail
  test -f skills/AGENTS_POST_MERGE_OPS_628.md
  grep -qE '\*\*M628-1\*\*' docs/qa-invariants.md
  grep -qE '\*\*M628-8\*\*' docs/qa-invariants.md
  grep -qE 'post-merge-ops-628' docs/qa-invariants.md
  grep -qE '\*\*M628-1' skills/AGENTS_POST_MERGE_OPS_628.md
  grep -qE 'make verify-issue-628' skills/AGENTS_POST_MERGE_OPS_628.md
  grep -qE 'AGENTS_POST_MERGE_OPS_628' AGENTS.md
  grep -qE 'verify-issue-628' AGENTS.md
  grep -qE 'verify-issue-628' Makefile
  grep -qE '#628' docs/testing.md
  grep -qE 'AGENTS_POST_MERGE_OPS_628' docs/README.md
  grep -qE '#628' skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md
  grep -qE '#628' skills/AGENTS_FRONTEND_CREATE_TOKEN.md
  grep -qE '#628' skills/AGENTS_COMMUNITY_TAX_CW20.md
  grep -qE '#628' skills/AGENTS_INDEXER_COMMUNITY_TOKENS.md
  grep -qE '#628' skills/AGENTS_CW20_CODE_ID_AUDIT.md
  grep -qE '#628' docs/local-development.md
  grep -qE 'M628-1' docs/contracts-security-audit.md
  grep -qE 'localterra-628-migrate-leftover' skills/AGENTS_POST_MERGE_OPS_628.md
  grep -qE 'Do \*\*not\*\* reopen #626' skills/AGENTS_POST_MERGE_OPS_628.md
  grep -qE '[Dd]o \*\*not\*\* append `3`' skills/AGENTS_POST_MERGE_OPS_628.md
  grep -qE 'Never `AddWhitelistedCodeId 8654`' skills/AGENTS_POST_MERGE_OPS_628.md
}

run_source() {
  set -euo pipefail
  grep -qE 'DEFAULT_COMMUNITY_MIGRATE_CODE_IDS = \[6036, 10184, 8266, 8654\]' \
    frontend-dapp/src/utils/communityTaxMigrate.ts
  grep -qE '8654' frontend-dapp/src/utils/communityTaxMigrate.ts
  grep -qE 'Migrate here' frontend-dapp/src/pages/CreateTokenPage.tsx
  grep -qE 'Already have a token' frontend-dapp/src/pages/CreateTokenPage.tsx
  # Retail Create Token stays code-id-free (#489 / M628-7).
  if grep -nE '6036|10184|8266|8654' frontend-dapp/src/pages/CreateTokenPage.tsx; then
    echo "Create Token page must not print migrate/factory code ids (M628-7)" >&2
    return 1
  fi
  grep -qE 'manage-token-wrong-template' frontend-dapp/src/pages/ManageTokenPage.tsx
  grep -qE 'Tax SKUs are hidden' frontend-dapp/src/pages/ManageTokenPage.tsx
  grep -qE 'queryByText\(/6036/\)' frontend-dapp/src/pages/CreateTokenPage.test.tsx
  grep -qE '8654' frontend-dapp/src/pages/CreateTokenPage.test.tsx
  grep -qE 'P11|wrong-template|6036' frontend-dapp/src/pages/ManageTokenPage.test.tsx
  grep -qE 'localterra-628-migrate-leftover' scripts/qa/localterra-628-migrate-leftover.sh
  grep -qE 'inbound Transfer 1:1' scripts/qa/localterra-628-migrate-leftover.sh
  grep -qE 'extra-debit sell after Refresh' scripts/qa/localterra-628-migrate-leftover.sh
  grep -qE 'GetFeatures' scripts/qa/localterra-628-migrate-leftover.sh
  bash -n scripts/qa/localterra-628-migrate-leftover.sh
  if grep -nRE 'AddWhitelistedCodeId 8654|add_whitelisted_code_id.*8654' \
      smartcontracts/contracts/community-tax-token \
      scripts/qa/localterra-628-migrate-leftover.sh \
      scripts/upgrade-611-community-tax.sh 2>/dev/null; then
    echo "scripts/contracts tell operators to whitelist 8654" >&2
    return 1
  fi
}

run_frontend_unit() {
  set -euo pipefail
  if [[ ! -x frontend-dapp/node_modules/.bin/vitest ]]; then
    bash scripts/with-node.sh --cwd frontend-dapp -- npm ci
  fi
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/CreateTokenPage.test.tsx \
    src/pages/ManageTokenPage.test.tsx \
    src/utils/communityTaxMigrate.test.ts
}

run_626() { make verify-issue-626; }
run_592() { make verify-issue-592; }
run_593() { make verify-issue-593; }
run_594() { make verify-issue-594; }

run_c5_pins() {
  set -euo pipefail
  # shellcheck source=scripts/lib/lcd-smart-query.sh
  source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"
  # shellcheck source=scripts/lib/localterra-host-curl.sh
  source "$REPO_ROOT/scripts/lib/localterra-host-curl.sh"
  local raw ids cfg
  if [[ -z "$FACTORY_C5" || "$FACTORY_C5" != terra1* ]]; then
    echo "FAIL: columbus-5 factory address unset (VERIFY628_FACTORY)" >&2
    return 1
  fi
  raw="$(lcd_smart_query_raw "$LCD_C5" "$FACTORY_C5" '{"get_whitelisted_code_ids":{}}')"
  ids="$(lcd_decode_smart_data "$raw")"
  echo "$ids" | jq -e '.code_ids | index(6036) != null' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(8266) != null' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(10184) != null' >/dev/null
  echo "$ids" | jq -e '.code_ids | (index(11626) != null or index(11630) != null)' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(8654) == null' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(3) == null' >/dev/null
  echo "$ids" | jq -e '.code_ids | index(11619) == null' >/dev/null
  raw="$(localterra_host_curl "${LCD_C5%/}/cosmwasm/wasm/v1/contract/${LAUNCHER_C5}" 2>/dev/null \
    || curl -fsS --max-time 30 "${LCD_C5%/}/cosmwasm/wasm/v1/contract/${LAUNCHER_C5}")"
  # Canonical instance stayed terra126pr5…; wasm moved 11622 → 11632 (#635).
  echo "$raw" | jq -e '
    (.contract_info.code_id|tonumber) == 11622
    or (.contract_info.code_id|tonumber) == 11632
  ' >/dev/null
  raw="$(lcd_smart_query_raw "$LCD_C5" "$LAUNCHER_C5" '{"get_config":{}}')"
  cfg="$(lcd_decode_smart_data "$raw")"
  echo "$cfg" | jq -e '
    (.token_code_id|tonumber) == 11630
    or (.token_code_id|tonumber) == 11626
  ' >/dev/null
  echo "$cfg" | jq -e '(.token_code_id|tonumber) != 11619' >/dev/null
  echo "columbus-5 whitelist=$(echo "$ids" | jq -c '.code_ids') launcher token=$(echo "$cfg" | jq -r '.token_code_id')"
}

run_c5_6036_cw2() {
  set -euo pipefail
  local list addr key raw decoded name
  list="$(curl -fsS --max-time 30 \
    "${LCD_C5%/}/cosmwasm/wasm/v1/code/6036/contracts?pagination.limit=5")"
  addr="$(echo "$list" | jq -r '.contracts[0] // .contract_addresses[0] // empty')"
  [[ "$addr" == terra1* ]] || {
    echo "FAIL: no live 6036 instance from LCD" >&2
    echo "$list" >&2
    return 1
  }
  key="$(printf 'contract_info' | base64 -w0 2>/dev/null || printf 'contract_info' | base64 | tr -d '\n')"
  raw="$(curl -fsS --max-time 30 \
    "${LCD_C5%/}/cosmwasm/wasm/v1/contract/${addr}/raw/${key}")"
  decoded="$(echo "$raw" | jq -r '.data // empty' | base64 -d 2>/dev/null || true)"
  name="$(printf '%s' "$decoded" | jq -r '.contract // empty' 2>/dev/null || true)"
  if [[ -z "$name" ]]; then
    name="$(printf '%s' "$decoded")"
  fi
  echo "6036 instance $addr cw2=$name"
  if printf '%s' "$name" | grep -qiE 'terraswap-token'; then
    echo "M628-5: live 6036 cw2 is terraswap-token — page-go / chain-revert; do not append from #628"
    return 0
  fi
  printf '%s' "$name" | grep -qE 'crates.io:cw20-base' || {
    echo "FAIL: live 6036 cw2 is neither cw20-base nor terraswap-token: $name" >&2
    return 1
  }
  echo "M628-5: live 6036 cw2 is crates.io:cw20-base (S3 adopt-go)"
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
  printf '%s' "$js" | grep -qE 'VITE_COMMUNITY_TAX_CODE_ID:"(11626|11630)"'
  if printf '%s' "$js" | grep -qE 'VITE_COMMUNITY_TAX_CODE_ID:"11619"'; then
    echo "Coolify still bakes 11619 as the adopt target (M628-2 / M628-4)" >&2
    return 1
  fi
  printf '%s' "$js" | grep -qF "$LAUNCHER_C5"
  printf '%s' "$js" | grep -qE '/token/migrate'
  echo "dex.cl8y.com bakes listed tax pin (11626 or 11630) + migrate route"
}

run_live_indexer() {
  set -euo pipefail
  local body
  body="$(curl -fsS --max-time 20 "${INDEXER_URL%/}/api/v1/community-tokens")"
  echo "$body" | jq -e '.configured == true and (.code_id == 11626 or .code_id == 11630)' >/dev/null
  echo "$body" | jq -e '.code_id != 11619' >/dev/null
  echo "indexer community-tokens configured=true code_id=$(echo "$body" | jq -r '.code_id')"
}

run_live_lt() {
  set -euo pipefail
  ./scripts/qa/localterra-628-migrate-leftover.sh
}

echo ""
echo "── first pass ──"
run_step "docs: Q14 M628-1–M628-8 + skill + AGENTS crosslinks" run_docs
run_step "source: allowlist 8654 + Create Token code-id-free + P11 gate" run_source
run_step "frontend: Create / Manage / migrate classify" run_frontend_unit

if [[ "${VERIFY628_SKIP_CHILDREN:-}" == "1" ]]; then
  skip "children 626/592/593/594 (VERIFY628_SKIP_CHILDREN=1)"
else
  run_step "child: make verify-issue-626" run_626
  run_step "child: make verify-issue-592" run_592
  run_step "child: make verify-issue-593" run_593
  run_step "child: make verify-issue-594" run_594
fi

if [[ "${VERIFY628_SKIP_LIVE:-}" == "1" ]]; then
  skip "columbus-5 / Coolify leftovers (VERIFY628_SKIP_LIVE=1)"
else
  if run_c5_pins >/tmp/cl8y-628-c5-pins.out 2>/tmp/cl8y-628-c5-pins.err; then
    cat /tmp/cl8y-628-c5-pins.out
    ok "M628-2/3: columbus-5 whitelist + launcher pin"
  else
    cat /tmp/cl8y-628-c5-pins.err >&2 || true
    if require_live; then
      bad "M628-2/3: columbus-5 whitelist + launcher pin"
    else
      skip "columbus-5 factory/launcher (LCD unreachable)"
    fi
  fi
  if run_c5_6036_cw2 >/tmp/cl8y-628-cw2.out 2>/tmp/cl8y-628-cw2.err; then
    cat /tmp/cl8y-628-cw2.out
    ok "M628-5: live 6036 cw2 recorded"
  else
    cat /tmp/cl8y-628-cw2.err >&2 || true
    if require_live; then
      bad "M628-5: live 6036 cw2 recorded"
    else
      skip "live 6036 cw2 (LCD unreachable)"
    fi
  fi
  if run_live_dapp >/tmp/cl8y-628-dapp.out 2>/tmp/cl8y-628-dapp.err; then
    cat /tmp/cl8y-628-dapp.out
    ok "M628-4: dex.cl8y.com bakes 11626/11630"
  else
    cat /tmp/cl8y-628-dapp.err >&2 || true
    if require_live; then
      bad "M628-4: dex.cl8y.com bakes 11626/11630"
    else
      skip "dex.cl8y.com Coolify bake (unreachable or stale)"
    fi
  fi
  if run_live_indexer >/tmp/cl8y-628-idx.out 2>/tmp/cl8y-628-idx.err; then
    cat /tmp/cl8y-628-idx.out
    ok "M628-4: indexer catalog single-id 11626/11630"
  else
    cat /tmp/cl8y-628-idx.err >&2 || true
    if require_live; then
      bad "M628-4: indexer catalog single-id 11626/11630"
    else
      skip "indexer.dex.cl8y.com catalog (unreachable or stale)"
    fi
  fi
fi

HAS_LT=1
if timeout 20 make -s has-localterra >/dev/null 2>&1; then
  HAS_LT=0
fi
if [[ "${VERIFY628_SKIP_CHAIN:-}" == "1" ]]; then
  skip "LocalTerra P3/P7/P11 (VERIFY628_SKIP_CHAIN=1)"
elif [[ "$HAS_LT" -eq 0 ]] \
  && { [[ -f frontend-dapp/.env.local ]] \
    || [[ -f /home/boilnokbr/repos/cl8y-dex-terraclassic/frontend-dapp/.env.local ]]; }; then
  run_step "M628-6: LocalTerra P3/P7/P11 leftover" run_live_lt
elif require_live || [[ "${VERIFY628_REQUIRE_CHAIN:-}" == "1" ]]; then
  bad "LocalTerra required (VERIFY628_REQUIRE_LIVE/CHAIN) — make setup-cloud-localterra"
else
  skip "LocalTerra P3/P7/P11 (make has-localterra + tax pins)"
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> GitLab #628 verification passed"
