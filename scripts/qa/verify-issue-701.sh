#!/usr/bin/env bash
# Automated verification for GitLab #701 — post-merge !477 leftover (#692).
#
# Proves (docs + child 692 + leftover live + optional Playwright):
#   1. Q18 / M701-1–M701-8 documented and crosslinked.
#   2. Child make verify-issue-692.
#   3. Source: pair_volume_24h.volume_usd migration + list JOIN.
#   4. Coolify leftovers (SKIP unless reachable).
#
# VERIFY701_SKIP_CHILDREN=1 — docs + source + live (no child).
# VERIFY701_SKIP_LIVE=1 — skip Coolify even if reachable.
# VERIFY701_SKIP_CHAIN=1 — skip leftover Playwright even if the chain is up.
# VERIFY701_REQUIRE_CHAIN=1 — FAIL when LocalTerra / Playwright missing.
# VERIFY701_REQUIRE_LIVE=1 or VERIFY701_IID=701 — FAIL when Coolify leftovers cannot run.
# VERIFY701_LEFTOVER_E2E=1 — run leftover Playwright even when the child already ran it.
#
# Refs: skills/AGENTS_POST_MERGE_OPS_701.md, docs/qa-invariants.md § Q18
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
  [[ "${VERIFY701_REQUIRE_LIVE:-}" == "1" || "${VERIFY701_IID:-}" == "701" ]]
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
echo "  GitLab #701 — post-merge !477 leftover verify (#692 Vol USD)"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
unset PLAYWRIGHT_WEB_PORT || true
unset PLAYWRIGHT_BASE_URL || true
unset VITE_PLAYWRIGHT_E2E || true
unset CI || true

DAPP_URL="${VERIFY701_DAPP_URL:-https://dex.cl8y.com}"
INDEXER_URL="${VERIFY701_INDEXER_URL:-https://indexer.dex.cl8y.com}"
INDEXER_URL="${INDEXER_URL%/}"
DAPP_URL="${DAPP_URL%/}"

CHILD_SKILLS=(
  skills/AGENTS_INDEXER_PAIR_VOLUME_USD.md
  skills/AGENTS_FRONTEND_POOL_TABLE.md
)

clickwrap_is_stub() {
  local dist="frontend-dapp/node_modules/@plasticdigits/cl8y-clickwrap/dist/index.js"
  [[ -f "$dist" ]] && grep -q 'getSignatureStatus' "$dist" && return 1
  return 0
}

ensure_clickwrap_package() {
  if [[ -L frontend-dapp/node_modules ]]; then
    if clickwrap_is_stub; then
      echo "[bootstrap] linked node_modules clickwrap is a stub — reinstall on the primary checkout, not here" >&2
      return 1
    fi
    return 0
  fi
  if clickwrap_is_stub; then
    echo "[bootstrap] installing @plasticdigits/cl8y-clickwrap from GitLab npm (not a pass-through stub)"
    bash scripts/with-node.sh --cwd frontend-dapp -- npm install @plasticdigits/cl8y-clickwrap@0.1.0 --no-save
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
  test -f skills/AGENTS_POST_MERGE_OPS_701.md
  grep -qE '\*\*M701-1\*\*' docs/qa-invariants.md
  grep -qE '\*\*M701-8\*\*' docs/qa-invariants.md
  grep -qE 'post-merge-ops-701' docs/qa-invariants.md
  grep -qE '\*\*Q18\*\*' docs/qa-invariants.md
  grep -qE '\*\*M701-1' skills/AGENTS_POST_MERGE_OPS_701.md
  grep -qE 'make verify-issue-701' skills/AGENTS_POST_MERGE_OPS_701.md
  grep -qE 'AGENTS_POST_MERGE_OPS_701' AGENTS.md
  grep -qE 'verify-issue-701' AGENTS.md
  grep -qE 'verify-issue-701' Makefile
  grep -qE '#701' docs/testing.md
  grep -qE 'AGENTS_POST_MERGE_OPS_701' docs/README.md
  grep -qE 'verify-issue-701' docs/local-development.md
  grep -qE 'M701-1' docs/contracts-security-audit.md
  grep -qE 'verify-issue-701' scripts/qa/README.md
  grep -qE '#701' docs/indexer-invariants.md
  grep -qE 'Do \*\*not\*\* reopen' skills/AGENTS_POST_MERGE_OPS_701.md
  grep -qE '20260829120000_pair_volume_24h_usd' skills/AGENTS_POST_MERGE_OPS_701.md
  grep -qE 'unset PLAYWRIGHT_WEB_PORT' scripts/qa/verify-issue-701.sh
  local f
  for f in "${CHILD_SKILLS[@]}"; do
    grep -qE 'AGENTS_POST_MERGE_OPS_701|#701' "$f"
  done
}

run_source() {
  set -euo pipefail
  test -f indexer/migrations/20260829120000_pair_volume_24h_usd.sql
  grep -q 'volume_usd' indexer/migrations/20260829120000_pair_volume_24h_usd.sql
  grep -qE 'volume_usd_24h' indexer/src/api/pairs.rs
  grep -qE 'PairListSort::VolumeUsd24h' indexer/src/db/queries/pairs.rs
  grep -qE 'pv.volume_usd AS volume_usd_24h' indexer/src/db/queries/pairs.rs
  grep -qE 'formatPairListVolumeUsd' frontend-dapp/src/components/pool/PoolPairsTable.tsx
  grep -qE 'volume_usd_24h' frontend-dapp/src/utils/poolListQuery.ts
  grep -qE 'pool-row-vol' frontend-dapp/src/components/pool/PoolPairsTable.tsx
  if grep -nE 'formatQuoteVolume24h' frontend-dapp/src/components/pool/PoolPairsTable.tsx \
       frontend-dapp/src/components/pool/PoolAdvancedManage.tsx 2>/dev/null; then
    echo "Pool Vol/Manage must not format quote-token volume (M701-4)" >&2
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

  code="$(indexer_get '/api/v1/pairs?limit=5&sort=volume_usd_24h&order=desc' "$tmp")"
  [[ "$code" == "200" ]] || { echo "pairs volume_usd_24h HTTP $code" >&2; cat "$tmp" >&2; return 1; }

  code="$(indexer_get '/api/v1/pairs?sort=volume_usd' "$tmp.bad")"
  [[ "$code" == "400" ]] || { echo "sort=volume_usd HTTP $code (want 400)" >&2; cat "$tmp.bad" >&2; return 1; }

  python3 - "$INDEXER_URL" "$tmp" <<'PY'
import json, sys, urllib.request
from decimal import Decimal

base = sys.argv[1].rstrip("/")
d = json.load(open(sys.argv[2]))
items = d if isinstance(d, list) else (d.get("items") or d.get("pairs") or [])
assert items, "empty pair list"
assert any("volume_usd_24h" in (i or {}) for i in items), "missing volume_usd_24h field"

priced = [i for i in items if i.get("volume_usd_24h") not in (None, "")]
assert priced, "no priced volume_usd_24h after leftover migrate (refresh_pair_volumes?)"
row = priced[0]
addr = row["pair_address"]
list_usd = Decimal(str(row["volume_usd_24h"]))

with urllib.request.urlopen(f"{base}/api/v1/pairs/{addr}/stats", timeout=25) as r:
    stats = json.load(r)
stats_usd = Decimal(str(stats.get("volume_usd") or "0"))
# Same stamp source; allow tiny JSON rounding.
delta = abs(list_usd - stats_usd)
print(addr, "list", list_usd, "stats", stats_usd, "delta", delta)
if list_usd == 0 and stats_usd == 0:
    print("both zero (idle priced pair)")
else:
    # Relative 1% or 1e-6 abs — leftover lag is ~5 min; same refresh should match.
    lim = max(Decimal("0.000001"), abs(stats_usd) * Decimal("0.01"))
    assert delta <= lim, f"list vs stats USD mismatch {delta} > {lim}"
print("pairs leftover: volume_usd_24h present + list≈stats + sort=volume_usd 400")
PY
  rm -f "$tmp" "$tmp.bad"
}

run_live_frontend() {
  set -euo pipefail
  python3 - "$DAPP_URL" <<'PY'
import re, sys, subprocess

base = sys.argv[1].rstrip("/")

def fetch(path, timeout=60):
    return subprocess.check_output(
        ["curl", "-sS", "--max-time", str(timeout), base + path],
        timeout=timeout + 10,
    ).decode("utf-8", "replace")

html = fetch("/", 25)
m = re.search(r"assets/(index-[A-Za-z0-9_.-]+\.js)", html)
assert m, "no index JS"
index = fetch("/assets/" + m.group(1), 30)
assets = sorted(set(re.findall(r"assets/([A-Za-z0-9_.-]+\.js)", index)))
want = re.compile(r"(PoolPage|chartsOverviewStats|pairCatalog)", re.I)
blob = index
for name in assets:
    if want.search(name):
        blob += fetch("/assets/" + name, 90)

assert "pool-row-vol" in blob, "missing /pool Vol cell testid"
assert "volume_usd_24h" in blob, "missing volume_usd_24h sort/field"
assert "pool-sort-vol" in blob, "missing Vol header sort"
print("live frontend: /pool Vol USD markers")
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
  export PLAYWRIGHT_SKIP_CHAIN=1
  export PLAYWRIGHT_WEB_PORT="$port"
  export PLAYWRIGHT_BASE_URL="http://127.0.0.1:${port}"
  bash scripts/with-node.sh --cwd frontend-dapp -- \
    ./node_modules/.bin/playwright test --project=e2e-smoke --workers=5 \
    e2e/pool-table-547.spec.ts
}

bootstrap_worktree

run_step "docs: Q18 M701-1–M701-8 + skill + AGENTS crosslinks" run_docs
run_step "source: volume_usd migration + /pool Vol USD cell" run_source

if [[ "${VERIFY701_SKIP_CHILDREN:-0}" == "1" ]]; then
  echo ""
  echo "[children] SKIP — VERIFY701_SKIP_CHILDREN=1"
  skip "child verify-issue-692 (VERIFY701_SKIP_CHILDREN=1)"
else
  run_step "child: make verify-issue-692" make verify-issue-692
fi

if [[ "${VERIFY701_SKIP_LIVE:-0}" == "1" ]]; then
  skip "Coolify leftover live (VERIFY701_SKIP_LIVE=1)"
else
  if curl -fsS --max-time 15 "${INDEXER_URL}/health" >/dev/null 2>&1 \
     || curl -fsS --max-time 15 "${INDEXER_URL}/api/v1/hub-prices" >/dev/null 2>&1; then
    run_step "live: indexer volume_usd_24h list≈stats + invalid sort 400" run_live_indexer
  else
    if require_live; then
      bad "Coolify indexer leftover (VERIFY701_REQUIRE_LIVE=1) — ${INDEXER_URL} unreachable"
    else
      skip "Coolify indexer leftover (${INDEXER_URL} unreachable)"
    fi
  fi
  if curl -fsS --max-time 15 "${DAPP_URL}/" >/dev/null 2>&1; then
    run_step "live: Coolify frontend /pool Vol USD markers" run_live_frontend
  else
    if require_live; then
      bad "Coolify frontend leftover (VERIFY701_REQUIRE_LIVE=1) — ${DAPP_URL} unreachable"
    else
      skip "Coolify frontend leftover (${DAPP_URL} unreachable)"
    fi
  fi
fi

want_leftover_e2e() {
  [[ "${VERIFY701_LEFTOVER_E2E:-0}" == "1" || "${VERIFY701_SKIP_CHILDREN:-0}" == "1" ]]
}

if [[ "${VERIFY701_SKIP_CHAIN:-0}" == "1" ]]; then
  skip "leftover Playwright (VERIFY701_SKIP_CHAIN=1)"
elif ! want_leftover_e2e; then
  skip "leftover Playwright (child 692 already ran pool-table; set VERIFY701_LEFTOVER_E2E=1)"
elif [[ -x frontend-dapp/node_modules/.bin/playwright ]]; then
  run_step "playwright leftover: pool-table Vol USD (5 workers)" run_leftover_e2e
else
  if [[ "${VERIFY701_REQUIRE_CHAIN:-0}" == "1" ]]; then
    bad "Playwright required (VERIFY701_REQUIRE_CHAIN=1)"
  else
    skip "leftover Playwright (no Playwright install)"
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
echo "==> GitLab #701 verification passed"
