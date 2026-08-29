#!/usr/bin/env bash
# Automated verification for GitLab #698 — post-merge !474/!475 leftover (#694 #695).
#
# Proves (docs + children 694, 695 + leftover live + optional Playwright):
#   1. Q17 / M698-1–M698-8 documented and crosslinked.
#   2. Children make verify-issue-{694,695}.
#   3. Source: MAX_GT_EVENT_ROWS, lcd_heavy membership, production VITE_DEV_MODE reject.
#   4. Coolify leftovers (SKIP unless reachable).
#   5. Optional LocalTerra leftover e2e-smoke (5 workers).
#
# VERIFY698_SKIP_CHILDREN=1 — docs + source + live (no children).
# VERIFY698_SKIP_LIVE=1 — skip Coolify even if reachable.
# VERIFY698_SKIP_CHAIN=1 — skip leftover Playwright even if the chain is up.
# VERIFY698_REQUIRE_CHAIN=1 — FAIL (do not SKIP) when LocalTerra / Playwright missing.
# VERIFY698_REQUIRE_LIVE=1 or VERIFY698_IID=698|699|700 — FAIL when Coolify leftovers cannot run.
# VERIFY698_LEFTOVER_E2E=1 — run leftover Playwright even when children already ran.
#
# Aliases: make verify-issue-699 / verify-issue-700 run this script (#699/#700 are dups).
#
# Refs: skills/AGENTS_POST_MERGE_OPS_698.md, docs/qa-invariants.md § Q17
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
  [[ "${VERIFY698_REQUIRE_LIVE:-}" == "1" \
    || "${VERIFY698_IID:-}" == "698" \
    || "${VERIFY698_IID:-}" == "699" \
    || "${VERIFY698_IID:-}" == "700" ]]
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
echo "  GitLab #698 — post-merge !474/!475 leftover verify (#694 #695)"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
unset PLAYWRIGHT_WEB_PORT || true
unset PLAYWRIGHT_BASE_URL || true
unset VITE_PLAYWRIGHT_E2E || true
unset CI || true

DAPP_URL="${VERIFY698_DAPP_URL:-https://dex.cl8y.com}"
INDEXER_URL="${VERIFY698_INDEXER_URL:-https://indexer.dex.cl8y.com}"
INDEXER_URL="${INDEXER_URL%/}"
DAPP_URL="${DAPP_URL%/}"

CHILDREN=(694 695)

CHILD_SKILLS=(
  skills/AGENTS_INDEXER_API4_PER_REQUEST.md
  skills/AGENTS_FRONTEND_DEV_MODE_GUARD.md
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
  test -f skills/AGENTS_POST_MERGE_OPS_698.md
  grep -qE '\*\*M698-1\*\*' docs/qa-invariants.md
  grep -qE '\*\*M698-8\*\*' docs/qa-invariants.md
  grep -qE 'post-merge-ops-698' docs/qa-invariants.md
  grep -qE '\*\*Q17\*\*' docs/qa-invariants.md
  grep -qE '\*\*M698-1' skills/AGENTS_POST_MERGE_OPS_698.md
  grep -qE 'make verify-issue-698' skills/AGENTS_POST_MERGE_OPS_698.md
  grep -qE 'AGENTS_POST_MERGE_OPS_698' AGENTS.md
  grep -qE 'verify-issue-698' AGENTS.md
  grep -qE 'verify-issue-698' Makefile
  grep -qE '#698' docs/testing.md
  grep -qE 'AGENTS_POST_MERGE_OPS_698' docs/README.md
  grep -qE 'verify-issue-698' docs/local-development.md
  grep -qE 'M698-1' docs/contracts-security-audit.md
  grep -qE 'verify-issue-698' scripts/qa/README.md
  grep -qE 'verify-issue-695' scripts/qa/README.md
  grep -qE '#698' docs/indexer-invariants.md
  grep -qE 'Do \*\*not\*\* reopen' skills/AGENTS_POST_MERGE_OPS_698.md
  grep -qE '#699|#700' skills/AGENTS_POST_MERGE_OPS_698.md
  grep -qE 'INTERNAL_GROK46_1787908099' skills/AGENTS_POST_MERGE_OPS_698.md
  grep -qE 'unset PLAYWRIGHT_WEB_PORT' scripts/qa/verify-issue-698.sh
  grep -qE 'linked node_modules clickwrap is a stub' scripts/qa/verify-issue-698.sh
  local f
  for f in "${CHILD_SKILLS[@]}"; do
    grep -qE 'AGENTS_POST_MERGE_OPS_698|#698' "$f"
  done
  grep -qE 'AGENTS_POST_MERGE_OPS_698|#698' skills/AGENTS_INDEXER_ROUTE_SOLVE_PROGRESS.md
  grep -qE 'Closed \(prod leftover #698' audits/INTERNAL_GROK46_1787908099.md
}

run_source() {
  set -euo pipefail
  grep -qE 'MAX_GT_EVENT_ROWS' indexer/src/api/gt.rs
  grep -qE 'event count exceeds 5000' indexer/src/api/gt.rs
  grep -qE 'MAX_BLACKLIST_TOKENS' indexer/src/api/compliance.rs
  grep -qE 'DISCOUNT_BPS_CACHE_TTL' indexer/src/api/route_solver.rs
  if grep -qE 'FROM pair_reserves|JOIN pair_reserves' indexer/src/api/gt.rs; then
    echo "/gt/events GET must not SELECT pair_reserves (M698-3 / A694-2)" >&2
    return 1
  fi
  python3 - <<'PY'
from pathlib import Path
text = Path("indexer/src/api/mod.rs").read_text()
start = text.find("let lcd_heavy_router = Router::new()")
end = text.find("apply_rate_limit_layer(lcd_heavy_router")
block = text[start:end]
assert "/api/v1/route/solve/progress" in block, "progress not on lcd_heavy_router"
assert "/api/v1/compliance/blacklist-check" in block, "blacklist-check not on lcd_heavy_router"
print("router membership ok")
PY
  grep -qE "mode === 'production' && env.VITE_DEV_MODE === 'true'" frontend-dapp/vite.config.ts
  grep -qE 'VITE_DEV_MODE=true is not allowed for production vite builds' frontend-dapp/vite.config.ts
  grep -qE 'progressPollTraderParams' frontend-dapp/src/utils/routeSolveProgress.ts
  grep -qE 'SIM_QUOTE_PROGRESS_MAX_BACKOFF_MS' frontend-dapp/src/utils/routeSolveProgress.ts
  grep -qE 'knownDiscountBps' frontend-dapp/src/hooks/useRouteSolveProgress.ts
}

indexer_get() {
  local path="$1"
  curl -sS -o "$2" -w "%{http_code}" --max-time 25 "${INDEXER_URL}${path}"
}

run_live_indexer() {
  set -euo pipefail
  local code tmp tokens
  tmp="$(mktemp)"

  tokens="$(python3 -c 'print(",".join(["terra1x"]*17))')"
  code="$(indexer_get "/api/v1/compliance/blacklist-check?tokens=${tokens}" "$tmp")"
  [[ "$code" == "400" ]] || { echo "blacklist 17 tokens HTTP $code (want 400)" >&2; cat "$tmp" >&2; return 1; }
  grep -qE 'exceeds max 16' "$tmp"

  code="$(indexer_get '/api/v1/route/solve/progress?token_in=uluna&token_out=uusd&amount_in=1' "$tmp")"
  [[ "$code" == "200" ]] || { echo "progress HTTP $code" >&2; cat "$tmp" >&2; return 1; }
  python3 - "$tmp" <<'PY'
import json, sys
d = json.load(open(sys.argv[1]))
assert "stage" in d, d
print("progress stage", d.get("stage"))
PY

  code="$(indexer_get '/gt/latest-block' "$tmp")"
  [[ "$code" == "200" ]] || { echo "gt/latest-block HTTP $code" >&2; cat "$tmp" >&2; return 1; }
  python3 - "$INDEXER_URL" "$tmp" <<'PY'
import json, sys, urllib.request
base = sys.argv[1].rstrip("/")
lb = json.load(open(sys.argv[2]))
h = int((lb.get("block") or lb).get("blockNumber") or (lb.get("block") or {}).get("blockNumber"))
lo = max(1, h - 1999)
url = f"{base}/gt/events?fromBlock={lo}&toBlock={h}"
req = urllib.request.Request(url)
with urllib.request.urlopen(req, timeout=45) as r:
    raw = r.read()
    code = r.status
print("gt/events HTTP", code, "bytes", len(raw), "window", lo, h)
assert code == 200 or code == 400, code
assert len(raw) < 5_000_000, f"multi-MB GT payload {len(raw)}"
body = raw.decode("utf-8", "replace")
if code == 400:
    assert "event count exceeds 5000" in body or "exceeds" in body.lower(), body[:400]
    print("gt over-cap 400")
else:
    d = json.loads(body)
    events = d.get("events") if isinstance(d, dict) else d
    n = len(events) if isinstance(events, list) else 0
    print("gt events", n)
    assert n <= 5000, f"uncapped GT window returned {n} events"
    assert "pair_reserves" not in body[:2000]
print("gt leftover: bounded window")
PY
  rm -f "$tmp"
}

run_live_frontend() {
  set -euo pipefail
  python3 - "$DAPP_URL" <<'PY'
import re, sys, subprocess

base = sys.argv[1].rstrip("/")

def fetch(path, timeout=120):
    return subprocess.check_output(
        ["curl", "-sS", "--max-time", str(timeout), base + path],
        timeout=timeout + 10,
    ).decode("utf-8", "replace")

html = fetch("/", 25)
m = re.search(r"assets/(index-[A-Za-z0-9_.-]+\.js)", html)
assert m, "no index JS"
index_name = m.group(1)
index = fetch("/assets/" + index_name, 30)
assets = sorted(set(re.findall(r"assets/([A-Za-z0-9_.-]+\.js)", index)))
want = re.compile(r"(TradePage|SwapPage|wallet-terra|PoolPage)", re.I)
blob = index
for name in assets:
    if want.search(name):
        blob += fetch("/assets/" + name, 120)

assert "Simulated Wallet" not in blob, "production bundle must not ship Simulated Wallet (FE-01 / M698-4)"
assert "VITE_DEV_MODE=true" not in blob
# Flatten/API4 tip (e6ddbf1d+) is a proxy that Coolify rebuilt after !474/!475.
assert "trade-order-text-tab" in blob or "trade-order-tab-market" in blob, "missing #693 markers on rebuilt tip"
print("live frontend: no Simulated Wallet; trade flatten markers present")
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
    e2e/swap.spec.ts
}

bootstrap_worktree

run_step "docs: Q17 M698-1–M698-8 + skill + AGENTS crosslinks" run_docs
run_step "source: API4 caps + lcd_heavy + production VITE_DEV_MODE reject" run_source

if [[ "${VERIFY698_SKIP_CHILDREN:-0}" == "1" ]]; then
  echo ""
  echo "[children] SKIP — VERIFY698_SKIP_CHILDREN=1"
  skip "child verify-issue-694,695 (VERIFY698_SKIP_CHILDREN=1)"
else
  for n in "${CHILDREN[@]}"; do
    run_step "child: make verify-issue-${n}" make "verify-issue-${n}"
  done
fi

if [[ "${VERIFY698_SKIP_LIVE:-0}" == "1" ]]; then
  skip "Coolify leftover live (VERIFY698_SKIP_LIVE=1)"
else
  if curl -fsS --max-time 15 "${INDEXER_URL}/health" >/dev/null 2>&1 \
     || curl -fsS --max-time 15 "${INDEXER_URL}/api/v1/hub-prices" >/dev/null 2>&1; then
    run_step "live: indexer blacklist 400 + progress + bounded /gt/events" run_live_indexer
  else
    if require_live; then
      bad "Coolify indexer leftover (VERIFY698_REQUIRE_LIVE=1) — ${INDEXER_URL} unreachable"
    else
      skip "Coolify indexer leftover (${INDEXER_URL} unreachable)"
    fi
  fi
  if curl -fsS --max-time 15 "${DAPP_URL}/" >/dev/null 2>&1; then
    run_step "live: Coolify frontend FE-01 (no Simulated Wallet)" run_live_frontend
  else
    if require_live; then
      bad "Coolify frontend leftover (VERIFY698_REQUIRE_LIVE=1) — ${DAPP_URL} unreachable"
    else
      skip "Coolify frontend leftover (${DAPP_URL} unreachable)"
    fi
  fi
fi

want_leftover_e2e() {
  [[ "${VERIFY698_LEFTOVER_E2E:-0}" == "1" || "${VERIFY698_SKIP_CHILDREN:-0}" == "1" ]]
}

if [[ "${VERIFY698_SKIP_CHAIN:-0}" == "1" ]]; then
  skip "leftover Playwright (VERIFY698_SKIP_CHAIN=1)"
elif ! want_leftover_e2e; then
  skip "leftover Playwright (children already ran 694/695; set VERIFY698_LEFTOVER_E2E=1)"
elif has_chain; then
  run_step "playwright leftover: swap smoke (5 workers)" run_leftover_e2e
else
  if [[ "${VERIFY698_REQUIRE_CHAIN:-0}" == "1" ]]; then
    bad "LocalTerra required (VERIFY698_REQUIRE_CHAIN=1) — make setup-cloud-localterra"
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
echo "==> GitLab #698 verification passed"
