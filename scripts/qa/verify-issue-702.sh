#!/usr/bin/env bash
# Automated verification for GitLab #702 — post-merge !476 leftover (#693).
#
# Proves (docs + children 693, 563, 653 + leftover live + optional Playwright):
#   1. Q19 / M702-1–M702-8 documented and crosslinked.
#   2. Children make verify-issue-{693,563,653}.
#   3. Source: Market default + compact text tabs + Advanced slippage.
#   4. Coolify leftovers (SKIP unless reachable).
#   5. Optional LocalTerra leftover Playwright (5 workers).
#
# VERIFY702_SKIP_CHILDREN=1 — docs + source + live (no children).
# VERIFY702_SKIP_LIVE=1 — skip Coolify even if reachable.
# VERIFY702_SKIP_CHAIN=1 — skip leftover Playwright even if the chain is up.
# VERIFY702_REQUIRE_CHAIN=1 — FAIL when LocalTerra / Playwright missing.
# VERIFY702_REQUIRE_LIVE=1 or VERIFY702_IID=702 — FAIL when Coolify leftovers cannot run.
# VERIFY702_LEFTOVER_E2E=1 — run leftover Playwright even when children already ran.
#
# Refs: skills/AGENTS_POST_MERGE_OPS_702.md, docs/qa-invariants.md § Q19
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
  [[ "${VERIFY702_REQUIRE_LIVE:-}" == "1" || "${VERIFY702_IID:-}" == "702" ]]
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
echo "  GitLab #702 — post-merge !476 leftover verify (#693 /trade flatten)"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
unset PLAYWRIGHT_WEB_PORT || true
unset PLAYWRIGHT_BASE_URL || true
unset VITE_PLAYWRIGHT_E2E || true
unset CI || true

DAPP_URL="${VERIFY702_DAPP_URL:-https://dex.cl8y.com}"
DAPP_URL="${DAPP_URL%/}"

CHILDREN=(693 563 653)

CHILD_SKILLS=(
  skills/AGENTS_FRONTEND_TRADE_TICKET_FLATTEN.md
  skills/AGENTS_FRONTEND_TRADE_TICKET_HEADING.md
  skills/AGENTS_FRONTEND_CHROME_NESTING.md
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
    if [[ ! -f frontend-dapp/.env.local && -f "$main_root/frontend-dapp/.env.local" ]]; then
      cp "$main_root/frontend-dapp/.env.local" frontend-dapp/.env.local
      echo "[bootstrap] copied frontend-dapp/.env.local from primary checkout"
    fi
  fi
  ensure_clickwrap_package
}

run_docs() {
  set -euo pipefail
  test -f skills/AGENTS_POST_MERGE_OPS_702.md
  grep -qE '\*\*M702-1\*\*' docs/qa-invariants.md
  grep -qE '\*\*M702-8\*\*' docs/qa-invariants.md
  grep -qE 'post-merge-ops-702' docs/qa-invariants.md
  grep -qE '\*\*Q19\*\*' docs/qa-invariants.md
  grep -qE '\*\*M702-1' skills/AGENTS_POST_MERGE_OPS_702.md
  grep -qE 'make verify-issue-702' skills/AGENTS_POST_MERGE_OPS_702.md
  grep -qE 'AGENTS_POST_MERGE_OPS_702' AGENTS.md
  grep -qE 'verify-issue-702' AGENTS.md
  grep -qE 'verify-issue-702' Makefile
  grep -qE '#702' docs/testing.md
  grep -qE 'AGENTS_POST_MERGE_OPS_702' docs/README.md
  grep -qE 'verify-issue-702' docs/local-development.md
  grep -qE 'M702-1' docs/contracts-security-audit.md
  grep -qE 'verify-issue-702' scripts/qa/README.md
  grep -qE 'Do \*\*not\*\* reopen' skills/AGENTS_POST_MERGE_OPS_702.md
  grep -qE '10.2.19' skills/AGENTS_POST_MERGE_OPS_702.md
  grep -qE 'unset PLAYWRIGHT_WEB_PORT' scripts/qa/verify-issue-702.sh
  grep -qE 'openTradeLimitAdvanced' frontend-dapp/e2e/trade-page-responsive.spec.ts
  grep -qE 'boxVisibleInScrollport' frontend-dapp/e2e/trade-page-responsive.spec.ts
  local f
  for f in "${CHILD_SKILLS[@]}"; do
    grep -qE 'AGENTS_POST_MERGE_OPS_702|#702' "$f"
  done
  grep -qE 'AGENTS_POST_MERGE_OPS_702|#702' skills/AGENTS_FRONTEND_SLIPPAGE_PRESET_ALIGN.md
  grep -qE 'AGENTS_POST_MERGE_OPS_702|#702' skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md
}

run_source() {
  set -euo pipefail
  grep -qF "useState<'limit' | 'market'>('market')" \
    frontend-dapp/src/components/trade/TradeOrderTicket.tsx
  grep -qE 'trade-order-text-tab' frontend-dapp/src/components/trade/TradeOrderTicket.tsx
  grep -qE 'trade-order-tab-market' frontend-dapp/src/components/trade/TradeOrderTicket.tsx
  grep -qE 'trade-order-mode-docs' frontend-dapp/src/components/trade/TradeOrderTicket.tsx
  grep -qE 'useTokenHeadingWash' frontend-dapp/src/components/trade/TradeOrderTicket.tsx
  if grep -nE 'TicketSection' frontend-dapp/src/components/trade/TradeOrderTicket.tsx; then
    echo "Trade ticket must not use TicketSection (M702-3)" >&2
    return 1
  fi
  grep -qE 'SlippageProtectionPresets' frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx
  grep -qF 'showDeviationChrome={false}' frontend-dapp/src/components/trade/TradeOrderTicket.tsx
  grep -qE 'quoteCw20ViaRouteSolve' frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx
  if grep -nE 'trade-market-hybrid-toggle' frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx; then
    echo "Do not add a hybrid-off control (M702-5 / H596)" >&2
    return 1
  fi
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
want = re.compile(r"(TradePage|TradeOrder|TradeMarket)", re.I)
blob = index
for name in assets:
    if want.search(name):
        blob += fetch("/assets/" + name, 90)

assert "trade-order-text-tab" in blob, "missing compact text tabs"
assert "trade-order-tab-market" in blob, "missing Market tab testid"
assert "trade-order-mode-docs" in blob, "missing mode+Docs line"
assert "rgba(251, 146, 60" not in blob, "leftover orange wash"
print("live frontend: /trade flatten markers (Market tabs + mode docs, no orange wash)")
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
    e2e/trade-page-responsive.spec.ts
}

bootstrap_worktree

run_step "docs: Q19 M702-1–M702-8 + skill + AGENTS crosslinks" run_docs
run_step "source: Market default + compact tabs + no TicketSection" run_source

if [[ "${VERIFY702_SKIP_CHILDREN:-0}" == "1" ]]; then
  echo ""
  echo "[children] SKIP — VERIFY702_SKIP_CHILDREN=1"
  skip "child verify-issue-693,563,653 (VERIFY702_SKIP_CHILDREN=1)"
else
  for n in "${CHILDREN[@]}"; do
    run_step "child: make verify-issue-${n}" make "verify-issue-${n}"
  done
fi

if [[ "${VERIFY702_SKIP_LIVE:-0}" == "1" ]]; then
  skip "Coolify leftover live (VERIFY702_SKIP_LIVE=1)"
else
  if curl -fsS --max-time 15 "${DAPP_URL}/" >/dev/null 2>&1; then
    run_step "live: Coolify frontend /trade flatten markers" run_live_frontend
  else
    if require_live; then
      bad "Coolify frontend leftover (VERIFY702_REQUIRE_LIVE=1) — ${DAPP_URL} unreachable"
    else
      skip "Coolify frontend leftover (${DAPP_URL} unreachable)"
    fi
  fi
fi

want_leftover_e2e() {
  [[ "${VERIFY702_LEFTOVER_E2E:-0}" == "1" || "${VERIFY702_SKIP_CHILDREN:-0}" == "1" ]]
}

if [[ "${VERIFY702_SKIP_CHAIN:-0}" == "1" ]]; then
  skip "leftover Playwright (VERIFY702_SKIP_CHAIN=1)"
elif ! want_leftover_e2e; then
  skip "leftover Playwright (children already ran 693/563/653; set VERIFY702_LEFTOVER_E2E=1)"
elif has_chain; then
  run_step "playwright leftover: trade-page-responsive (5 workers)" run_leftover_e2e
else
  if [[ "${VERIFY702_REQUIRE_CHAIN:-0}" == "1" ]]; then
    bad "LocalTerra required (VERIFY702_REQUIRE_CHAIN=1) — make setup-cloud-localterra"
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
echo "==> GitLab #702 verification passed"
