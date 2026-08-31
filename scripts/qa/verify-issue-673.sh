#!/usr/bin/env bash
# Automated verification for GitLab #673 — post-merge !437–!458 leftover stack.
#
# Proves (docs + children 655–672 + leftover live + optional Playwright):
#   1. Q15 / M673-1–M673-8 documented and crosslinked.
#   2. Children make verify-issue-{655..672}.
#   3. Source: no charts-overview census; one pair_liquidity_usd.rs; no tmp-558 track.
#   4. Coolify leftovers (SKIP unless reachable).
#   5. Optional LocalTerra leftover e2e-smoke (5 workers).
#
# VERIFY673_SKIP_CHILDREN=1 — docs + source + live (no 18 children).
# VERIFY673_SKIP_LIVE=1 — skip Coolify even if reachable.
# VERIFY673_SKIP_CHAIN=1 — skip leftover Playwright even if the chain is up.
# VERIFY673_REQUIRE_CHAIN=1 — FAIL (do not SKIP) when LocalTerra / Playwright missing.
# VERIFY673_REQUIRE_LIVE=1 or VERIFY673_IID=673 — FAIL (do not SKIP) when Coolify
#   leftovers cannot run.
# VERIFY673_LEFTOVER_E2E=1 — run leftover Playwright even when children already did.
#
# Refs: skills/AGENTS_POST_MERGE_OPS_673.md, docs/qa-invariants.md § Q15
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
  [[ "${VERIFY673_REQUIRE_LIVE:-}" == "1" || "${VERIFY673_IID:-}" == "673" ]]
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
echo "  GitLab #673 — post-merge !437–!458 leftover verify"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
# Do not export PLAYWRIGHT_WEB_PORT here — children default to :3173 (in indexer CORS).
# A leaked dedicated port (e.g. 31673) makes getPairs fail CORS and /pool hides the table.
unset PLAYWRIGHT_WEB_PORT || true
unset PLAYWRIGHT_BASE_URL || true
unset VITE_PLAYWRIGHT_E2E || true
unset CI || true

DAPP_URL="${VERIFY673_DAPP_URL:-https://dex.cl8y.com}"
INDEXER_URL="${VERIFY673_INDEXER_URL:-https://indexer.dex.cl8y.com}"
INDEXER_URL="${INDEXER_URL%/}"
DAPP_URL="${DAPP_URL%/}"

CHILDREN=(655 656 657 658 659 660 661 662 663 664 665 666 667 668 669 670 671 672)

CHILD_SKILLS=(
  skills/AGENTS_INDEXER_PAIR_LIQUIDITY_USD.md
  skills/AGENTS_FRONTEND_TRADER_IDENTITY.md
  skills/AGENTS_FRONTEND_TRADER_LEADERBOARD.md
  skills/AGENTS_FRONTEND_CLICKWRAP.md
  skills/AGENTS_FRONTEND_SWAP_DIRECTION_SEAM.md
  skills/AGENTS_FRONTEND_POOL_MANAGE_IA.md
  skills/AGENTS_FRONTEND_POOL_PROVIDE_LABELS.md
  skills/AGENTS_FRONTEND_POOL_CREATED.md
  skills/AGENTS_FRONTEND_PRODUCT_LINKS.md
  skills/AGENTS_FRONTEND_TRADE_IDENTITY_LP.md
  skills/AGENTS_FRONTEND_SHARE_LINK.md
  skills/AGENTS_FRONTEND_CHARTS_PAIR_SCOPED.md
  skills/AGENTS_FRONTEND_PROTOCOL_STATS.md
  skills/AGENTS_FRONTEND_CREATE_TOKEN_LAYOUT.md
  skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md
  skills/AGENTS_FRONTEND_WALLET_CHIP.md
  skills/AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md
)

clickwrap_is_stub() {
  local dist="frontend-dapp/node_modules/@plasticdigits/cl8y-clickwrap/dist/index.js"
  [[ -f "$dist" ]] && grep -q 'getSignatureStatus' "$dist" && return 1
  return 0
}

ensure_clickwrap_package() {
  if [[ ! -f frontend-dapp/node_modules/@plasticdigits/cl8y-clickwrap/index.js ]] \
     || clickwrap_is_stub; then
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
  test -f skills/AGENTS_POST_MERGE_OPS_673.md
  grep -qE '\*\*M673-1\*\*' docs/qa-invariants.md
  grep -qE '\*\*M673-8\*\*' docs/qa-invariants.md
  grep -qE 'post-merge-ops-673' docs/qa-invariants.md
  grep -qE '\*\*M673-1' skills/AGENTS_POST_MERGE_OPS_673.md
  grep -qE 'make verify-issue-673' skills/AGENTS_POST_MERGE_OPS_673.md
  grep -qE 'AGENTS_POST_MERGE_OPS_673' AGENTS.md
  grep -qE 'verify-issue-673' AGENTS.md
  grep -qE 'verify-issue-673' Makefile
  grep -qE '#673' docs/testing.md
  grep -qE 'AGENTS_POST_MERGE_OPS_673' docs/README.md
  grep -qE 'verify-issue-673' docs/local-development.md
  grep -qE 'M673-1' docs/contracts-security-audit.md
  grep -qE 'verify-issue-673' scripts/qa/README.md
  grep -qE '#673' docs/indexer-invariants.md
  grep -qE 'Do \*\*not\*\* reopen' skills/AGENTS_POST_MERGE_OPS_673.md
  grep -qE 'tmp-558' skills/AGENTS_POST_MERGE_OPS_673.md
  grep -qE 'charts-overview' skills/AGENTS_POST_MERGE_OPS_673.md
  grep -qE 'pass-through stub' skills/AGENTS_FRONTEND_CLICKWRAP.md
  grep -qE 'unset PLAYWRIGHT_WEB_PORT' scripts/qa/verify-issue-673.sh
  grep -qE 'PLAYWRIGHT_WEB_PORT' skills/AGENTS_POST_MERGE_OPS_673.md
  local f
  for f in "${CHILD_SKILLS[@]}"; do
    grep -qE 'AGENTS_POST_MERGE_OPS_673|#673' "$f"
  done
}

run_source() {
  set -euo pipefail
  test -f indexer/migrations/20260826150000_pair_liquidity_usd.sql
  test -f indexer/migrations/20260826180000_protocol_volume_hourly_monthly.sql
  test -f indexer/src/db/queries/pair_liquidity_usd.rs
  if [[ -e indexer/src/db/queries/pair_liquidity.rs ]]; then
    echo "Do not restore a second pair_liquidity.rs (M673-6)" >&2
    return 1
  fi
  if grep -nE 'charts-overview-pairs|charts-overview-tokens|charts-overview-volume-usd' \
       frontend-dapp/src/pages/ChartsPage.tsx; then
    echo "Do not restore Charts DEX-census tiles (M673-6)" >&2
    return 1
  fi
  grep -q 'charts-pair-24h-stats' frontend-dapp/src/pages/ChartsPage.tsx
  grep -q 'PAIR_SCOPED_SORTS' indexer/src/api/traders.rs
  grep -q 'parse_volume_grain' indexer/src/db/queries/protocol_volume.rs
  grep -q 'pool-manage-tab-provide' frontend-dapp/src/components/pool/PoolAdvancedManage.tsx
  grep -q 'liquidity_usd' indexer/src/api/pairs.rs
  grep -q 'created_at' indexer/src/api/pairs.rs
  if git ls-files --error-unmatch 'scripts/tmp-558-*' >/dev/null 2>&1; then
    echo "Do not commit scripts/tmp-558-* (M673-6)" >&2
    git ls-files 'scripts/tmp-558-*' >&2
    return 1
  fi
  bash -n scripts/qa/verify-issue-673.sh
}

indexer_get() {
  local path="$1"
  curl -sS -o "$2" -w "%{http_code}" --max-time 25 "${INDEXER_URL}${path}"
}

run_live_indexer() {
  set -euo pipefail
  local code tmp
  tmp="$(mktemp)"
  code="$(indexer_get '/api/v1/pairs?limit=1' "$tmp")"
  [[ "$code" == "200" ]] || { echo "pairs HTTP $code" >&2; cat "$tmp" >&2; return 1; }
  python3 - "$tmp" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
items = d.get("items") or []
assert items, "empty pair list"
assert "created_at" in items[0], "pairs missing created_at (#662)"
print("pairs[0] keys include created_at; liquidity_usd", "liquidity_usd" in items[0])
PY

  code="$(indexer_get '/api/v1/pairs?sort=liquidity_usd&limit=1' "$tmp")"
  [[ "$code" == "200" ]] || { echo "sort=liquidity_usd HTTP $code" >&2; cat "$tmp" >&2; return 1; }
  python3 - "$tmp" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
items = d.get("items") or []
assert items, "empty liquidity_usd sort"
assert "liquidity_usd" in items[0], "stamped sort row missing liquidity_usd (#655)"
print("liquidity_usd", items[0]["liquidity_usd"])
PY

  code="$(indexer_get '/api/v1/pairs?sort=not_a_sort' "$tmp")"
  [[ "$code" == "400" ]] || { echo "invalid sort HTTP $code (want 400)" >&2; return 1; }
  grep -q liquidity_usd "$tmp"

  code="$(indexer_get '/api/v1/traders/leaderboard?pair=terra1notapairxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx' "$tmp")"
  [[ "$code" == "404" ]] || { echo "unknown pair HTTP $code (want 404)" >&2; cat "$tmp" >&2; return 1; }

  code="$(indexer_get '/api/v1/traders/leaderboard?pair=terra1notapairxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxxx&sort=best_trade_pnl' "$tmp")"
  [[ "$code" == "400" ]] || { echo "pair+best_trade_pnl HTTP $code (want 400)" >&2; cat "$tmp" >&2; return 1; }

  code="$(indexer_get '/api/v1/traders/leaderboard?limit=1' "$tmp")"
  [[ "$code" == "200" ]] || { echo "unscoped leaderboard HTTP $code" >&2; return 1; }

  for grain_qs in 'grain=hourly&limit=24' 'grain=daily&limit=14' 'grain=monthly&limit=6'; do
    code="$(indexer_get "/api/v1/protocol/volume/daily?${grain_qs}" "$tmp")"
    [[ "$code" == "200" ]] || { echo "volume ${grain_qs} HTTP $code" >&2; cat "$tmp" >&2; return 1; }
    python3 - "$tmp" "$grain_qs" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
want = sys.argv[2].split("&")[0].split("=")[1]
assert d.get("grain") == want, d
assert "series" in d
print("grain", want, "points", len(d["series"]))
PY
  done
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

def chunk(name):
    mm = re.search(rf"assets/({re.escape(name)}-[A-Za-z0-9_.-]+\.js)", index)
    assert mm, f"missing {name} chunk"
    return urllib.request.urlopen(base + "/" + mm.group(0), timeout=30).read().decode("utf-8", "replace")

charts = chunk("ChartsPage")
pool = chunk("PoolPage")
protocol = chunk("ProtocolPage")
swap = chunk("SwapPage")
for banned in ("charts-overview-pairs", "charts-overview-tokens", "charts-overview-volume-usd"):
    assert banned not in charts, banned
assert "charts-pair-24h-stats" in charts
assert "pool-manage-tab-provide" in pool
assert "pool-sort-lp-usd" in pool or "v2 LP" in pool
assert "protocol-volume-grain" in protocol
assert "swap-direction-seam" in swap
print("live frontend chunks: charts pair-scoped, pool four-tab+LP, protocol grain, swap seam")
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
  # :3173 is already on LocalTerra indexer CORS (deploy-dex-local / #625).
  local port="${PLAYWRIGHT_WEB_PORT:-3173}"
  CI=1 PLAYWRIGHT_WEB_PORT="$port" \
    PLAYWRIGHT_BASE_URL="http://127.0.0.1:${port}" \
    bash scripts/with-node.sh --cwd frontend-dapp -- \
    ./node_modules/.bin/playwright test --project=e2e-smoke --workers=5 \
      e2e/pool-manage-660.spec.ts \
      e2e/footer-product-links-663.spec.ts \
      e2e/price-chart-smoke.spec.ts \
      e2e/trader-page.spec.ts
}

bootstrap_worktree

run_step "docs: Q15 M673-1–M673-8 + skill + AGENTS crosslinks" run_docs
run_step "source: no census restore; one liquidity stamp; no tmp-558" run_source

if [[ "${VERIFY673_SKIP_CHILDREN:-0}" == "1" ]]; then
  echo ""
  echo "[children] SKIP — VERIFY673_SKIP_CHILDREN=1"
  skip "child verify-issue-655..672 (VERIFY673_SKIP_CHILDREN=1)"
else
  for n in "${CHILDREN[@]}"; do
    run_step "child: make verify-issue-${n}" make "verify-issue-${n}"
  done
fi

if [[ "${VERIFY673_SKIP_LIVE:-0}" == "1" ]]; then
  skip "Coolify leftover live (VERIFY673_SKIP_LIVE=1)"
else
  if curl -fsS --max-time 15 "${INDEXER_URL}/health" >/dev/null 2>&1 \
     || curl -fsS --max-time 15 "${INDEXER_URL}/api/v1/pairs?limit=1" >/dev/null 2>&1; then
    run_step "live: indexer pairs / leaderboard / volume grains" run_live_indexer
  else
    if require_live; then
      bad "Coolify indexer leftover (VERIFY673_REQUIRE_LIVE=1) — ${INDEXER_URL} unreachable"
    else
      skip "Coolify indexer leftover (${INDEXER_URL} unreachable)"
    fi
  fi
  if curl -fsS --max-time 15 "${DAPP_URL}/" >/dev/null 2>&1; then
    run_step "live: Coolify frontend chunks (8af5563c+ markers)" run_live_frontend
  else
    if require_live; then
      bad "Coolify frontend leftover (VERIFY673_REQUIRE_LIVE=1) — ${DAPP_URL} unreachable"
    else
      skip "Coolify frontend leftover (${DAPP_URL} unreachable)"
    fi
  fi
fi

want_leftover_e2e() {
  [[ "${VERIFY673_LEFTOVER_E2E:-0}" == "1" || "${VERIFY673_SKIP_CHILDREN:-0}" == "1" ]]
}

if [[ "${VERIFY673_SKIP_CHAIN:-0}" == "1" ]]; then
  skip "leftover Playwright (VERIFY673_SKIP_CHAIN=1)"
elif ! want_leftover_e2e; then
  skip "leftover Playwright (children already ran 660/663/657 smoke; set VERIFY673_LEFTOVER_E2E=1)"
elif has_chain; then
  run_step "playwright leftover: 660/663/charts/trader (5 workers)" run_leftover_e2e
else
  if [[ "${VERIFY673_REQUIRE_CHAIN:-0}" == "1" ]]; then
    bad "LocalTerra required (VERIFY673_REQUIRE_CHAIN=1) — make setup-cloud-localterra"
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
echo "==> GitLab #673 verification passed"
