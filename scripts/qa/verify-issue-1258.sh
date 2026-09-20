#!/usr/bin/env bash
# Verification for Forgejo #1258: registry USDT quote USD pin (Charts Price / 24h OHLC).
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
echo "  Forgejo #1258 — USDT-quoted Charts Price (USD) catalog pin"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

PIN='terra1z0xe7t5ymmltg4vju8tghkq0pewy4et548ta23nlu9zxtl950uyqkv8mv4'

run_step "docs: invariants + skill + AGENTS crosslinks" \
  bash -c "
    set -euo pipefail
    grep -q 'USDT' docs/indexer-invariants.md
    grep -q '#1258' docs/indexer-invariants.md
    grep -q 'USDT_CW20_ADDRESS' docs/indexer-invariants.md
    grep -q 'not.*Peg1' docs/indexer-invariants.md
    grep -q '1258' skills/AGENTS_INDEXER_PAIR_PRICE_USD.md
    grep -q 'quote_usd_kind(\"USDT\")' skills/AGENTS_INDEXER_PAIR_PRICE_USD.md
    grep -q 'QuoteUsdKind::Usdt' skills/AGENTS_INDEXER_PAIR_PRICE_USD.md
    grep -q 'verify-issue-1258' AGENTS.md
    grep -q 'AGENTS_INDEXER_PAIR_PRICE_USD' AGENTS.md
    grep -q '1258' docs/frontend.md
    grep -q 'USDT' skills/AGENTS_INDEXER_CANDLE_USD_MARK.md
    test -f indexer/migrations/20260916120000_usdt_quote_usd_null_backfill.sql
    test -f indexer/src/db/queries/usdt_quote_usd.rs
  "

run_step "source: identity pin, not Peg1 / hub / CEX FDUSD / symbol arm" \
  bash -c "
    set -euo pipefail
    grep -qF '${PIN}' indexer/src/config.rs
    grep -q 'DEFAULT_USDT_CW20_ADDRESS' indexer/src/config.rs
    grep -q 'USDT_CW20_ADDRESS' indexer/src/config.rs
    grep -q 'QuoteUsdKind::Usdt' indexer/src/indexer/pair_price_usd.rs
    grep -q 'is_pinned_usdt_cw20' indexer/src/indexer/pair_price_usd.rs
    grep -q 'quote_usd_kind_for_identity' indexer/src/indexer/pair_price_usd.rs
    if grep -nE '\"USDT\" => Some\\(QuoteUsdKind::' indexer/src/indexer/pair_price_usd.rs | grep -v Usdt; then
      echo 'bare USDT symbol arm must not price' >&2
      exit 1
    fi
    # Symbol-only quote_usd_kind must stay None for USDT (tested in rust; grep the match arms).
    awk '/fn quote_usd_kind/,/^}/' indexer/src/indexer/pair_price_usd.rs | grep -q 'USDT' && {
      echo 'quote_usd_kind must not match ticker USDT' >&2
      exit 1
    }
    grep -q 'QuoteUsdKind::Usdt => Some(BigDecimal::from(1))' indexer/src/indexer/pair_price_usd.rs
    if grep -nE 'QuoteUsdKind::Peg1 => Some\\(BigDecimal::from\\(1\\)\\)' indexer/src/indexer/pair_price_usd.rs; then
      echo 'Peg1 must not hardcode \$1 (USDT is Usdt, not Peg1)' >&2
      exit 1
    fi
    if grep -n 'economic_usd_per_human' indexer/src/indexer/pair_price_usd.rs | grep -v economic; then
      :
    fi
    ! grep -q 'OracleTicker::Vfdusd' indexer/src/indexer/pair_price_usd.rs
    grep -q 'price_usd_for_human_quote_per_base_pinned' indexer/src/indexer/parser.rs
    grep -q 'usdt_cw20_address' indexer/src/indexer/parser.rs
    grep -q 'backfill_null_usdt_quote_usd' indexer/src/main.rs
    grep -q 'price_usd IS NULL' indexer/src/db/queries/usdt_quote_usd.rs
    grep -q 'price_usd IS NULL' indexer/migrations/20260916120000_usdt_quote_usd_null_backfill.sql
    grep -q '${PIN}' indexer/migrations/20260916120000_usdt_quote_usd_null_backfill.sql
    grep -q 'USDT_CW20_ADDRESS' indexer/.env.example
    grep -q 'REGISTRY_USDT_CW20_ADDRESS' frontend-dapp/src/utils/tokenRegistry.ts
    grep -q 'isPinnedUsdtCw20' frontend-dapp/src/utils/pairPriceUsd.ts
    if grep -nE \"case 'USDT':\" frontend-dapp/src/utils/pairPriceUsd.ts; then
      echo 'classifyQuoteSymbol must not map ticker USDT without the pin' >&2
      exit 1
    fi
    grep -q \"kind === 'usdt'\" frontend-dapp/src/utils/pairPriceUsd.ts
  "

run_step "indexer lib: pair_price_usd (USDT pin / spoof / 6/18)" \
  bash -c 'cd indexer && cargo test --lib pair_price_usd -- --quiet'

run_step "frontend: pairPriceUsd + chartsPairStats USDT OHLC" \
  bash -c '
    set -euo pipefail
    VITEST_ARGS=(src/utils/__tests__/pairPriceUsd.test.ts src/utils/__tests__/chartsPairStats.test.ts)
    FILTER="USDT|formatPairStatsUsdOhlc|resolveTapeLastPriceUsd|resolveDisplayPairStatsUsdOhlc|classifyQuoteSymbol"
    if bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run "${VITEST_ARGS[@]}" -t "$FILTER"; then
      exit 0
    fi
    echo "with-node.sh failed; falling back to PATH node $(command -v node) $(node -v 2>/dev/null || true)"
    cd frontend-dapp
    npx vitest run "${VITEST_ARGS[@]}"
  '

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
