#!/usr/bin/env bash
# Automated verification for GitLab #508 — UST1 secondary AMM pair (or Path B waiver).
#
# Layers:
#   1. Doc / skill / invariant cross-links (U1–U7) + PRODUCT_WAIVER
#   2. Soft-launch catalog exclusion (U6)
#   3. Scripts present + bash -n
#   4. Frontend unit tests (copy guardrails U1)
#   5. Mainnet dry-run preflight (token code ids; inventory may WARN)
#   6. Optional LocalTerra fixture: VERIFY508_LOCAL=1
#   7. Optional mainnet presence: VERIFY508_MAINNET=1 (fails if pair missing — Path A only)
#
# Refs: docs/runbooks/ust1-secondary-amm-pair.md, skills/AGENTS_UST1_SECONDARY_AMM.md
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
echo "  GitLab #508 — UST1 secondary AMM pair"
echo "════════════════════════════════════════════════════════════════"

run_step "runbook + skill + PRODUCT_WAIVER exist" \
  bash -c 'test -f docs/runbooks/ust1-secondary-amm-pair.md \
    && test -f skills/AGENTS_UST1_SECONDARY_AMM.md \
    && test -f deployments/ust1-secondary-pair/PRODUCT_WAIVER.md \
    && grep -q "Revisit trigger" deployments/ust1-secondary-pair/PRODUCT_WAIVER.md \
    && grep -q "#508" deployments/ust1-secondary-pair/PRODUCT_WAIVER.md \
    && grep -q "#502" deployments/ust1-secondary-pair/PRODUCT_WAIVER.md'

run_step "invariants U1–U7 in runbook + defaults" \
  bash -c 'for id in U1 U2 U3 U4 U5 U6 U7; do
      grep -q "\*\*${id}\*\*" docs/runbooks/ust1-secondary-amm-pair.md || exit 1
      grep -q "$id" scripts/lib/ust1-secondary-pair-defaults.sh || exit 1
    done'

run_step "soft-launch defaults do not list UST1 (U6)" \
  bash -c '! grep -Eiq "UST1|vFDUSD|cUSTC" scripts/lib/mainnet-soft-launch-defaults.sh'

run_step "AGENTS.md + soft-launch + deployment-guide crosslink #508" \
  bash -c 'grep -q "AGENTS_UST1_SECONDARY_AMM" AGENTS.md \
    && grep -q "ust1-secondary-amm-pair" docs/runbooks/mainnet-soft-launch.md \
    && grep -q "#508" docs/deployment-guide.md \
    && grep -q "PRODUCT_WAIVER\|ust1-secondary-pair" docs/runbooks/mainnet-soft-launch.md'

run_step "scripts executable + shell syntax" \
  bash -c 'chmod +x scripts/add-ust1-secondary-pair.sh scripts/seed-ust1-secondary-pair-local.sh \
    scripts/lib/ust1-secondary-pair-defaults.sh scripts/qa/verify-issue-508.sh \
    && bash -n scripts/add-ust1-secondary-pair.sh \
    && bash -n scripts/seed-ust1-secondary-pair-local.sh \
    && bash -n scripts/lib/ust1-secondary-pair-defaults.sh'

run_step "Path A guards: symbol match + unseeded gate + trade path shape" \
  bash -c 'grep -q "norm_sym" scripts/add-ust1-secondary-pair.sh \
    && grep -q "UST1_SEC_ALLOW_UNSEEDED" scripts/add-ust1-secondary-pair.sh \
    && grep -q "UST1_SEC_ALLOW_DISCOUNT_FAIL" scripts/add-ust1-secondary-pair.sh \
    && grep -q "VITE_UST1_SECONDARY_PAIR_ADDRESS" scripts/seed-ust1-secondary-pair-local.sh \
    && ! grep -q "market=secondary" frontend-dapp/src/utils/ust1SecondaryMarket.ts \
    && grep -q "/trade/" frontend-dapp/src/utils/ust1SecondaryMarket.ts'

run_step "frontend unit: ust1SecondaryMarket + CreatePair notice" \
  bash -c '
    ROOT="'"$REPO_ROOT"'"
    SIBLING="$(dirname "$ROOT")/cl8y-dex-terraclassic/frontend-dapp/node_modules"
    if [[ ! -x "$ROOT/frontend-dapp/node_modules/.bin/vitest" ]]; then
      if [[ -x "$SIBLING/.bin/vitest" ]]; then
        ln -sfn "$SIBLING" "$ROOT/frontend-dapp/node_modules"
      else
        bash "$ROOT/scripts/with-node.sh" --cwd frontend-dapp -- npm ci --silent
      fi
    fi
    bash "$ROOT/scripts/with-node.sh" --cwd frontend-dapp -- npm test -- --run \
      src/utils/__tests__/ust1SecondaryMarket.test.ts \
      src/pages/CreatePairPage.test.tsx
  '

run_step "mainnet dry-run preflight (LCD token/code-id checks)" \
  bash -c 'DRY_RUN=1 ./scripts/add-ust1-secondary-pair.sh'

if [[ "${VERIFY508_LOCAL:-0}" == "1" ]]; then
  echo ""
  echo "[LocalTerra seed fixture (VERIFY508_LOCAL=1)]"
  set +e
  ./scripts/seed-ust1-secondary-pair-local.sh
  SEED_ST=$?
  set -e
  if [[ "$SEED_ST" -eq 2 ]]; then
    echo "  [SKIP] LocalTerra factory missing/stale .env.local — redeploy required (not a #508 code fail)"
    ok "LocalTerra fixture skipped (stale/missing deploy)"
  elif [[ "$SEED_ST" -eq 0 ]] \
    && test -f deployments/ust1-secondary-pair/local-addresses.env \
    && grep -q "LOCAL_UST1_VFDUSD_PAIR_ADDRESS=terra1" deployments/ust1-secondary-pair/local-addresses.env; then
    # shellcheck disable=SC1091
    source deployments/ust1-secondary-pair/local-addresses.env
    if [[ "${LOCAL_PAIR_TOTAL_SHARE:-0}" != "0" ]]; then
      ok "LocalTerra seed fixture (VERIFY508_LOCAL=1)"
    else
      bad "LocalTerra seed fixture (empty pool)"
    fi
  else
    bad "LocalTerra seed fixture (VERIFY508_LOCAL=1)"
  fi
else
  echo ""
  echo "[skip] LocalTerra fixture (set VERIFY508_LOCAL=1 to enable)"
  ok "LocalTerra fixture skipped (optional)"
fi

if [[ "${VERIFY508_MAINNET:-0}" == "1" ]]; then
  run_step "mainnet pair presence (VERIFY508_MAINNET=1 — Path A only)" \
    bash -c '
      # shellcheck source=scripts/lib/ust1-secondary-pair-defaults.sh
      source scripts/lib/ust1-secondary-pair-defaults.sh
      # shellcheck source=scripts/lib/lcd-smart-query.sh
      source scripts/lib/lcd-smart-query.sh
      LCD="${UST1_SEC_LCD_URL:-https://terra-classic-lcd.publicnode.com}"
      Q="$(ust1_sec_quote_address)"
      MSG="$(jq -nc --arg a "$UST1_SEC_UST1_ADDRESS" --arg b "$Q" \
        "{pair:{asset_infos:[{token:{contract_addr:\$a}},{token:{contract_addr:\$b}}]}}")"
      RAW="$(lcd_smart_query_raw "$LCD" "$UST1_SEC_FACTORY_ADDRESS" "$MSG")"
      echo "$RAW" | jq -e ".data" >/dev/null
    '
else
  echo ""
  echo "[skip] mainnet pair presence (Path B waiver active; set VERIFY508_MAINNET=1 after Path A)"
  ok "mainnet pair presence skipped (optional Path A)"
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
