#!/usr/bin/env bash
# Automated verification for GitLab #573 — post-merge stack !368–!377.
#
# Proves (docs + logo + child verifies):
#   1. Q6 / M573-1–M573-8 documented and crosslinked.
#   2. Simplified C+8 favicon wired (not full-scene /logo.png).
#   3. Coolify production env rules (mainnet, no SHOW_TEST_TOKENS, no faucet).
#   4. Child make verify-issue-{557,560,561,562,563,564,565,566,567}.
#
# Optional: VERIFY573_SKIP_CHILDREN=1 for docs/logo only.
# Playwright children use PLAYWRIGHT_WEB_PORT (default 3173) so worktrees do not steal :3000.
#
# Refs: skills/AGENTS_POST_MERGE_STACK.md, docs/qa-invariants.md § Q6
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }

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
echo "  GitLab #573 — post-merge stack !368–!377"
echo "════════════════════════════════════════════════════════════════"

export PLAYWRIGHT_WEB_PORT="${PLAYWRIGHT_WEB_PORT:-3173}"

run_step "docs: Q6 M573-1–M573-8 + skill + AGENTS crosslinks" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_POST_MERGE_STACK.md
    grep -qE "\*\*M573-1\*\*" docs/qa-invariants.md
    grep -qE "\*\*M573-8\*\*" docs/qa-invariants.md
    grep -qE "post-merge-stack-573" docs/qa-invariants.md
    grep -qE "\*\*M573-1" skills/AGENTS_POST_MERGE_STACK.md
    grep -qE "make verify-issue-573" skills/AGENTS_POST_MERGE_STACK.md
    grep -qE "AGENTS_POST_MERGE_STACK" AGENTS.md
    grep -qE "verify-issue-573" AGENTS.md
    grep -qE "verify-issue-573" Makefile
    grep -qE "#573" docs/testing.md
    grep -qE "AGENTS_POST_MERGE_STACK" docs/README.md
  '

run_step "docs: Coolify production env (M573-2)" \
  bash -c '
    set -euo pipefail
    grep -qE "Do not set \`VITE_FAUCET_ADDRESS\` on production" docs/runbooks/mainnet-soft-launch.md
    grep -qE "VITE_SHOW_TEST_TOKENS" docs/runbooks/mainnet-soft-launch.md
    grep -qE "#573" docs/runbooks/mainnet-soft-launch.md
    grep -qE "ARG VITE_NETWORK=mainnet" docker/frontend/Dockerfile
    grep -qE "ARG VITE_SHOW_TEST_TOKENS=" docker/frontend/Dockerfile
    grep -qE "ARG VITE_FAUCET_ADDRESS=" docker/frontend/Dockerfile
  '

run_step "logo: simplified C+8 favicon (M573-5 / !368)" \
  bash -c '
    set -euo pipefail
    test -f frontend-dapp/public/favicon-16.png
    test -f frontend-dapp/public/favicon-32.png
    test -f frontend-dapp/public/logo-simplified-variant.png
    grep -qE "favicon-32.png" frontend-dapp/index.html
    grep -qE "favicon-16.png" frontend-dapp/index.html
    grep -qE "logo-simplified-variant" docs/design-system.md
    ! grep -qE "href=\"/logo.png\"" frontend-dapp/index.html
  '

run_step "child skills point at #573 stack" \
  bash -c '
    set -euo pipefail
    grep -qE "#573" skills/AGENTS_FRONTEND_TAPE_AMOUNTS.md
    grep -qE "#573" skills/AGENTS_FRONTEND_HUB_PNL.md
    grep -qE "#573" skills/AGENTS_FRONTEND_TRADE_PAGE_LAYOUT.md
    grep -qE "#573" skills/AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md
    grep -qE "#573" skills/AGENTS_FRONTEND_TRADE_TICKET_HEADING.md
    grep -qE "#573" skills/AGENTS_FRONTEND_CHARTS_PAIR_STATS.md
    grep -qE "#573" skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md
    grep -qE "#573" skills/AGENTS_FRONTEND_KEPLR_LEDGER.md
  '

run_step "P1 spec exists: LocalTerra Swap pay still lists EMBER (M573-4)" \
  bash -c '
    set -euo pipefail
    test -f frontend-dapp/e2e/retail-test-tokens-562.spec.ts
    grep -qE "pay picker lists every factory token including gems" frontend-dapp/e2e/retail-test-tokens-562.spec.ts
    grep -qE "retail-test-tokens-562" scripts/qa/verify-issue-562.sh
  '

if [[ "${VERIFY573_SKIP_CHILDREN:-0}" = "1" ]]; then
  echo ""
  echo "[children] SKIP — VERIFY573_SKIP_CHILDREN=1"
  RESULTS+=("SKIP  child verify-issue-* (VERIFY573_SKIP_CHILDREN=1)")
else
  for n in 557 560 561 562 563 564 565 566 567; do
    run_step "child: make verify-issue-${n}" make "verify-issue-${n}"
  done
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
echo "==> GitLab #573 verification passed"
