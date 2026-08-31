#!/usr/bin/env bash
# Automated verification for GitLab #686 — post-merge !459–!468 leftover stack.
#
# Proves (docs + children 674–680, 683 + leftover live + optional Playwright):
#   1. Q16 / M686-1–M686-8 documented and crosslinked.
#   2. Children make verify-issue-{674,675,676,677,678,679,680,683}.
#   3. Source: NUMERIC(78,18) + economic_token_marks migrations; hub 4 cells;
#      no liquidity 30d chip; no hybrid-off leftover.
#   4. Coolify leftovers (SKIP unless reachable).
#   5. Optional LocalTerra leftover e2e-smoke (5 workers).
#
# VERIFY686_SKIP_CHILDREN=1 — docs + source + live (no 8 children).
# VERIFY686_SKIP_LIVE=1 — skip Coolify even if reachable.
# VERIFY686_SKIP_CHAIN=1 — skip leftover Playwright even if the chain is up.
# VERIFY686_REQUIRE_CHAIN=1 — FAIL (do not SKIP) when LocalTerra / Playwright missing.
# VERIFY686_REQUIRE_LIVE=1 or VERIFY686_IID=686 — FAIL (do not SKIP) when Coolify
#   leftovers cannot run.
# VERIFY686_LEFTOVER_E2E=1 — run leftover Playwright even when children already did.
# VERIFY686_TRADER — optional columbus-5 wallet for positions shape probe.
#
# Refs: skills/AGENTS_POST_MERGE_OPS_686.md, docs/qa-invariants.md § Q16
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
  [[ "${VERIFY686_REQUIRE_LIVE:-}" == "1" || "${VERIFY686_IID:-}" == "686" ]]
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
echo "  GitLab #686 — post-merge !459–!468 leftover verify"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
# Do not export PLAYWRIGHT_WEB_PORT here — children default to CORS-safe ports.
# A leaked dedicated port (e.g. 31686) can hide catalog fetches.
unset PLAYWRIGHT_WEB_PORT || true
unset PLAYWRIGHT_BASE_URL || true
unset VITE_PLAYWRIGHT_E2E || true
unset CI || true

DAPP_URL="${VERIFY686_DAPP_URL:-https://dex.cl8y.com}"
INDEXER_URL="${VERIFY686_INDEXER_URL:-https://indexer.dex.cl8y.com}"
INDEXER_URL="${INDEXER_URL%/}"
DAPP_URL="${DAPP_URL%/}"

CHILDREN=(674 675 676 677 678 679 680 683)

CHILD_SKILLS=(
  skills/AGENTS_FRONTEND_PORTFOLIO_TEST_PAIRS.md
  skills/AGENTS_FRONTEND_PORTFOLIO_UNREALIZED.md
  skills/AGENTS_INDEXER_TRADER_POSITIONS_DECIMALS.md
  skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
  skills/AGENTS_FRONTEND_SWAP_ACQUIRE_GUIDANCE.md
  skills/AGENTS_TERRACLASSIC_GAS.md
  skills/AGENTS_FRONTEND_CHARTS_UST1_HERO.md
  skills/AGENTS_INDEXER_ECONOMIC_FEE_USD.md
)

clickwrap_is_stub() {
  local dist="frontend-dapp/node_modules/@plasticdigits/cl8y-clickwrap/dist/index.js"
  [[ -f "$dist" ]] && grep -q 'getSignatureStatus' "$dist" && return 1
  return 0
}

ensure_clickwrap_package() {
  # Never `npm install` over a worktree symlink — npm replaces the link and
  # drops sibling packages (swarm vitest, Playwright). Primary checkout already
  # has the GitLab tarball when dist/index.js exports getSignatureStatus.
  if [[ -L frontend-dapp/node_modules ]]; then
    if clickwrap_is_stub; then
      echo "[bootstrap] linked node_modules clickwrap is a stub — reinstall on the primary checkout, not here" >&2
      return 1
    fi
    return 0
  fi
  if clickwrap_is_stub; then
    echo "[bootstrap] installing @plasticdigits/cl8y-clickwrap from GitLab npm (not a pass-through stub)"
    bash scripts/with-node.sh --cwd frontend-dapp -- npm install @plasticdigits/cl8y-clickwrap@0.1.1 --no-save
  fi
}

bootstrap_worktree() {
  local common main_root
  common="$(git rev-parse --git-common-dir)"
  main_root="$(cd "$common/.." && pwd)"
  if [[ "$main_root" != "$REPO_ROOT" ]]; then
    if [[ ! -x frontend-dapp/node_modules/.bin/vitest && -x "$main_root/frontend-dapp/node_modules/.bin/vitest" ]]; then
      ln -sfn "$main_root/frontend-dapp/node_modules" "$REPO_ROOT/frontend-dapp/node_modules"
      echo "[bootstrap] linked frontend-dapp/node_modules from primary checkout"
    fi
    if [[ ! -x packages/localnet-trading-swarm/node_modules/.bin/vitest \
         && -x "$main_root/packages/localnet-trading-swarm/node_modules/.bin/vitest" ]]; then
      ln -sfn "$main_root/packages/localnet-trading-swarm/node_modules" \
        "$REPO_ROOT/packages/localnet-trading-swarm/node_modules"
      echo "[bootstrap] linked packages/localnet-trading-swarm/node_modules from primary checkout"
    fi
    if [[ ! -f indexer/.env && -f "$main_root/indexer/.env" ]]; then
      cp "$main_root/indexer/.env" indexer/.env
      echo "[bootstrap] copied indexer/.env from primary checkout"
    fi
    if [[ ! -f frontend-dapp/.env.local && -f "$main_root/frontend-dapp/.env.local" ]]; then
      cp "$main_root/frontend-dapp/.env.local" frontend-dapp/.env.local
      echo "[bootstrap] copied frontend-dapp/.env.local from primary checkout"
    fi
  fi
  if [[ ! -f indexer/.env ]]; then
    echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
    make setup-indexer-postgres
  fi
  ensure_clickwrap_package
}

run_docs() {
  set -euo pipefail
  test -f skills/AGENTS_POST_MERGE_OPS_686.md
  grep -qE '\*\*M686-1\*\*' docs/qa-invariants.md
  grep -qE '\*\*M686-8\*\*' docs/qa-invariants.md
  grep -qE 'post-merge-ops-686' docs/qa-invariants.md
  grep -qE '\*\*Q16\*\*' docs/qa-invariants.md
  grep -qE '\*\*M686-1' skills/AGENTS_POST_MERGE_OPS_686.md
  grep -qE 'make verify-issue-686' skills/AGENTS_POST_MERGE_OPS_686.md
  grep -qE 'AGENTS_POST_MERGE_OPS_686' AGENTS.md
  grep -qE 'verify-issue-686' AGENTS.md
  grep -qE 'verify-issue-686' Makefile
  grep -qE '#686' docs/testing.md
  grep -qE 'AGENTS_POST_MERGE_OPS_686' docs/README.md
  grep -qE 'verify-issue-686' docs/local-development.md
  grep -qE 'M686-1' docs/contracts-security-audit.md
  grep -qE 'verify-issue-686' scripts/qa/README.md
  grep -qE '#686' docs/indexer-invariants.md
  grep -qE 'Do \*\*not\*\* reopen' skills/AGENTS_POST_MERGE_OPS_686.md
  grep -qE '#684' skills/AGENTS_POST_MERGE_OPS_686.md
  grep -qE 'HUB_CL8Y_ADDRESS' skills/AGENTS_POST_MERGE_OPS_686.md
  grep -qE 'NUMERIC\(78, 18\)|NUMERIC\(78,18\)' skills/AGENTS_POST_MERGE_OPS_686.md
  grep -qE 'unset PLAYWRIGHT_WEB_PORT' scripts/qa/verify-issue-686.sh
  grep -qE 'PLAYWRIGHT_WEB_PORT' skills/AGENTS_POST_MERGE_OPS_686.md
  grep -qE 'Do \*\*not\*\* turn hybrid off' skills/AGENTS_POST_MERGE_OPS_686.md
  grep -qE 'localnet-trading-swarm/node_modules' skills/AGENTS_POST_MERGE_OPS_686.md
  grep -qE 'linked node_modules clickwrap is a stub' scripts/qa/verify-issue-686.sh
  grep -qE 'bootstrap_swarm_worktree' scripts/qa/verify-issue-679.sh
  grep -qE 'free_tcp_port' scripts/qa/verify-issue-677.sh
  grep -qE '30677' skills/AGENTS_POST_MERGE_OPS_686.md
  local f
  for f in "${CHILD_SKILLS[@]}"; do
    grep -qE 'AGENTS_POST_MERGE_OPS_686|#686' "$f"
  done
}

run_source() {
  set -euo pipefail
  test -f indexer/migrations/20260827120000_trader_positions_numeric_78.sql
  test -f indexer/migrations/20260827140000_economic_token_marks.sql
  grep -q 'NUMERIC(78, 18)' indexer/migrations/20260827120000_trader_positions_numeric_78.sql
  grep -q 'economic_token_marks' indexer/migrations/20260827140000_economic_token_marks.sql
  grep -q 'HUB_CL8Y_ADDRESS' indexer/src/config.rs
  grep -q 'DEFAULT_HUB_CL8Y_ADDRESS' indexer/src/config.rs
  grep -q 'fn resolve_economic_marks' indexer/src/indexer/economic_usd.rs
  grep -q 'fn backfill_null_fee_usd' indexer/src/db/queries/protocol_fees.rs
  grep -q 'HUB_TICKERS: \[\&str; 4\] = \["custc", "lunc", "ust1", "ustr"\]' indexer/src/indexer/hub_usd.rs
  if grep -nE 'HUB_TICKERS: \[.*, "cl8y"' indexer/src/indexer/hub_usd.rs; then
    echo "Hub ticker allowlist must stay four cells (M686-3 / EFee-4)" >&2
    return 1
  fi
  grep -q 'portfolio-show-test-pairs' frontend-dapp/src/components/portfolio/PortfolioShowTestPairsToggle.tsx
  grep -q 'trader-position-unrealized' frontend-dapp/src/components/trader/TraderPositionsTable.tsx
  grep -q 'protocol-stat-liquidity-24h' frontend-dapp/src/components/protocol/ProtocolGlobalStats.tsx
  if grep -nE 'protocol-stat-liquidity-30d' frontend-dapp/src/components/protocol/ProtocolGlobalStats.tsx; then
    echo "Total liquidity must not render protocol-stat-liquidity-30d (M686-4)" >&2
    return 1
  fi
  grep -q 'parseChartsPriceQuery' frontend-dapp/src/utils/chartsPairRoute.ts
  grep -q 'cl8y-dex-charts-pair-invert:' frontend-dapp/src/utils/tradePairDisplayOrientation.ts
  grep -q 'swap-quote-only' frontend-dapp/src/pages/SwapPage.tsx
  grep -q 'MIXED_HYBRID_ROUTER_HEADROOM_GAS' frontend-dapp/src/utils/constants.ts
  if grep -nE 'VITE_HYBRID_DISABLED|hybridEnabled:\s*false' \
       frontend-dapp/src/pages/SwapPage.tsx \
       frontend-dapp/src/pages/TradePage.tsx 2>/dev/null; then
    echo "Do not turn hybrid off to fix Station auto-gas (M686-6 / H596)" >&2
    return 1
  fi
}

indexer_get() {
  local path="$1"
  curl -sS -o "$2" -w "%{http_code}" --max-time 25 "${INDEXER_URL}${path}"
}

run_live_indexer() {
  set -euo pipefail
  local code tmp
  tmp="$(mktemp)"

  code="$(indexer_get '/api/v1/hub-prices' "$tmp")"
  [[ "$code" == "200" ]] || { echo "hub-prices HTTP $code" >&2; cat "$tmp" >&2; return 1; }
  python3 - "$tmp" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
tickers = d.get("tickers") or list((d.get("prices") or {}).keys())
# Accept either explicit tickers list or prices object keys.
if not tickers and isinstance(d, dict):
    tickers = [k for k in ("custc", "lunc", "ust1", "ustr") if k in d]
allowed = {"custc", "lunc", "ust1", "ustr"}
extra = [t for t in tickers if str(t).lower() not in allowed and str(t).lower() != "updated_at"]
print("hub-prices keys", sorted({str(t).lower() for t in tickers}))
assert not extra, extra
PY

  code="$(indexer_get '/api/v1/hub-prices/cl8y' "$tmp")"
  [[ "$code" == "400" ]] || { echo "hub-prices/cl8y HTTP $code (want 400)" >&2; cat "$tmp" >&2; return 1; }

  code="$(indexer_get '/api/v1/protocol/fees?window=24h' "$tmp")"
  [[ "$code" == "200" ]] || { echo "protocol/fees HTTP $code" >&2; cat "$tmp" >&2; return 1; }
  python3 - "$tmp" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
assert "by_token" in d, d.keys()
rows = d.get("by_token") or []
cl8y = [r for r in rows if str(r.get("symbol") or "").upper().startswith("CL8Y")]
if cl8y:
    hit = cl8y[0]
    print("CL8Y mix", hit.get("symbol"), "human", hit.get("amount_human"), "usd", hit.get("amount_usd"))
    if hit.get("amount_human") and hit.get("amount_usd") in (None, ""):
        raise SystemExit("CL8Y-cb human present without amount_usd after leftover migrate")
else:
    print("CL8Y not in 24h token mix (qualifying pair / HUB_CL8Y_ADDRESS leftover may still be open)")
PY

  # Default columbus-5 wallet that traded UST1/USTR and CL8Y-cb/cUSTC (leftover #676).
  local addr="${VERIFY686_TRADER:-terra1e7s0h9ftxakwca5gxspyt4haeuaqxds6swr08ul3tsepq7el924sprrsrw}"
  python3 - "$INDEXER_URL" "$addr" <<'PY'
import json, sys, urllib.request

base = sys.argv[1].rstrip("/")
addr = sys.argv[2]
want = {
    "terra16vxrhpvpcucu05y0nr862vf9hnqeh274uaff4s7hz4n0ea74006qf5hgqy": "UST1/USTR",
    "terra1tz5vwrungh6drd9nt95qym3k892vs3as8nqmu7sg4ypek7wxvv4qm89upc": "CL8Y-cb/cUSTC",
}

def get(path):
    with urllib.request.urlopen(base + path, timeout=25) as r:
        return json.load(r)

pos = get(f"/api/v1/traders/{addr}/positions")
items = pos if isinstance(pos, list) else (pos.get("items") or pos.get("positions") or [])
blob = json.dumps(items)
assert "1e+" not in blob.lower(), "scientific notation in positions JSON (#676)"
by_pair = {p.get("pair_address"): p for p in items}
for pair, label in want.items():
    row = by_pair.get(pair)
    assert row, f"missing {label} position for leftover wallet"
    count = int(row.get("trade_count") or 0)
    trades = get(f"/api/v1/traders/{addr}/trades?pair={pair}&limit=200")
    trows = trades if isinstance(trades, list) else (trades.get("items") or trades.get("trades") or [])
    n = len(trows)
    print(label, "trade_count", count, "trades", n)
    assert count == n, f"{label} trade_count {count} != /trades {n}"
print("positions leftover: NUMERIC plain strings + trade_count match")
PY
  rm -f "$tmp"
}

run_live_frontend() {
  set -euo pipefail
  python3 - "$DAPP_URL" <<'PY'
import re, sys, urllib.request

base = sys.argv[1].rstrip("/")
html = urllib.request.urlopen(base + "/", timeout=25).read().decode("utf-8", "replace")
m = re.search(r"assets/(index-[A-Za-z0-9_.-]+\.js)", html)
assert m, "no index JS"
index = urllib.request.urlopen(base + "/assets/" + m.group(1), timeout=30).read().decode("utf-8", "replace")
assets = sorted(set(re.findall(r"assets/([A-Za-z0-9_.-]+\.js)", index)))
# Vite splits table/helpers off the page chunks (ShareLink / chartsPairRoute / PnlValue).
want = re.compile(
    r"(PortfolioPage|ChartsPage|ProtocolPage|SwapPage|TradePage|"
    r"ShareLink|PnlValue|chartsPairRoute|TraderPositions)",
    re.I,
)
blob = index
for name in assets:
    if want.search(name):
        blob += urllib.request.urlopen(base + "/assets/" + name, timeout=30).read().decode("utf-8", "replace")

assert "portfolio-show-test-pairs" in blob, "missing #674 Show test pairs"
assert "trader-position-unrealized" in blob, "missing #675 unrealized column"
assert "protocol-stat-liquidity-24h" in blob, "missing #677 24h liquidity chip"
assert "protocol-stat-liquidity-30d" not in blob, "do not restore liquidity 30d chip"
assert "swap-quote-only" in blob or "swap-acquire-guidance" in blob, "missing #678 Quote only"
assert (
    "parseChartsPriceQuery" in blob
    or "cl8y-dex-charts-pair-invert" in blob
    or "price=UST1" in blob
), "missing #680 Charts ?price= / invert prefix"
print("live frontend: portfolio hide+unrealized, protocol 24h liq, swap acquire, charts hero")
PY
}

has_chain() {
  timeout 20 make -s has-localterra >/dev/null 2>&1
}

run_leftover_e2e() {
  set -euo pipefail
  if [[ ! -x frontend-dapp/node_modules/.bin/playwright ]]; then
    echo "Playwright not installed" >&2
    return 1
  fi
  if [[ ! -f frontend-dapp/.env.local ]]; then
    echo "frontend-dapp/.env.local missing" >&2
    return 1
  fi
  local port="${PLAYWRIGHT_WEB_PORT:-3173}"
  local pids=""
  if command -v lsof >/dev/null 2>&1; then
    pids="$(lsof -ti ":$port" 2>/dev/null || true)"
  fi
  if [[ -n "${pids// /}" ]]; then
    echo "[bootstrap] freeing TCP :$port (stale Playwright Vite): $pids"
    # shellcheck disable=SC2086
    kill $pids 2>/dev/null || true
    sleep 1
  fi
  export CI=1
  export PLAYWRIGHT_WEB_PORT="$port"
  export PLAYWRIGHT_BASE_URL="http://127.0.0.1:${port}"
  bash scripts/with-node.sh --cwd frontend-dapp -- \
    ./node_modules/.bin/playwright test --project=e2e-smoke --workers=5 \
    e2e/portfolio.spec.ts e2e/protocol-page.spec.ts e2e/price-chart-smoke.spec.ts
}

bootstrap_worktree

run_step "docs: Q16 M686-1–M686-8 + skill + AGENTS crosslinks" run_docs
run_step "source: migrations + hub 4 cells + 24h liq + acquire + mixed hybrid" run_source

if [[ "${VERIFY686_SKIP_CHILDREN:-0}" == "1" ]]; then
  echo ""
  echo "[children] SKIP — VERIFY686_SKIP_CHILDREN=1"
  skip "child verify-issue-674..680,683 (VERIFY686_SKIP_CHILDREN=1)"
else
  for n in "${CHILDREN[@]}"; do
    run_step "child: make verify-issue-${n}" make "verify-issue-${n}"
  done
fi

if [[ "${VERIFY686_SKIP_LIVE:-0}" == "1" ]]; then
  skip "Coolify leftover live (VERIFY686_SKIP_LIVE=1)"
else
  if curl -fsS --max-time 15 "${INDEXER_URL}/health" >/dev/null 2>&1 \
     || curl -fsS --max-time 15 "${INDEXER_URL}/api/v1/hub-prices" >/dev/null 2>&1; then
    run_step "live: indexer hub-prices / protocol fees / optional positions" run_live_indexer
  else
    if require_live; then
      bad "Coolify indexer leftover (VERIFY686_REQUIRE_LIVE=1) — ${INDEXER_URL} unreachable"
    else
      skip "Coolify indexer leftover (${INDEXER_URL} unreachable)"
    fi
  fi
  if curl -fsS --max-time 15 "${DAPP_URL}/" >/dev/null 2>&1; then
    run_step "live: Coolify frontend chunks (36d64528+ markers)" run_live_frontend
  else
    if require_live; then
      bad "Coolify frontend leftover (VERIFY686_REQUIRE_LIVE=1) — ${DAPP_URL} unreachable"
    else
      skip "Coolify frontend leftover (${DAPP_URL} unreachable)"
    fi
  fi
fi

want_leftover_e2e() {
  [[ "${VERIFY686_LEFTOVER_E2E:-0}" == "1" || "${VERIFY686_SKIP_CHILDREN:-0}" == "1" ]]
}

if [[ "${VERIFY686_SKIP_CHAIN:-0}" == "1" ]]; then
  skip "leftover Playwright (VERIFY686_SKIP_CHAIN=1)"
elif ! want_leftover_e2e; then
  skip "leftover Playwright (children already ran 674/677 smoke; set VERIFY686_LEFTOVER_E2E=1)"
elif has_chain; then
  run_step "playwright leftover: 674/677/charts (5 workers)" run_leftover_e2e
else
  if [[ "${VERIFY686_REQUIRE_CHAIN:-0}" == "1" ]]; then
    bad "LocalTerra required (VERIFY686_REQUIRE_CHAIN=1) — make setup-cloud-localterra"
  else
    skip "leftover Playwright (make has-localterra). Cloud Agent: make setup-cloud-localterra"
  fi
fi

echo ""
echo "────────────────────────────────────────────────────────────────"
echo "  $PASS passed, $FAIL failed"
echo "────────────────────────────────────────────────────────────────"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
echo "==> GitLab #686 verification passed"
