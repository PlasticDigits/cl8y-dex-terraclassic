#!/usr/bin/env bash
# Verification for Forgejo #1257 — USTR→USDT mixed 18/6 hop honesty + Expert no-waive ≥99%.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

# Gate-impl VMs may lack rustup / nvm / libssl-dev until first verify.
# Prefer host cargo + rustup. apt is skipped when the FS is read-only (gate-impl sandbox);
# OPENSSL_DIR / PKG_CONFIG_PATH in $HOME then satisfy openssl-sys.
# shellcheck source=scripts/lib/cloud-agent-toolchain.sh
source "$REPO_ROOT/scripts/lib/cloud-agent-toolchain.sh"
mkdir -p "${NVM_DIR:-$HOME/.nvm}"
if cloud_agent_ensure_apt_packages 2>/dev/null; then
  :
else
  echo "[verify-issue-1257] apt packages unavailable; using rustup + optional HOME openssl"
fi
cloud_agent_ensure_rustup
export PATH="${HOME}/.cargo/bin:/usr/local/cargo/bin:${PATH}"
# Read-only apt: headers extracted to $HOME/opt-openssl (see agent sandbox).
if [[ -f "${HOME}/.openssl-local.env" ]]; then
  # shellcheck disable=SC1091
  source "${HOME}/.openssl-local.env"
fi
cloud_agent_ensure_frontend_deps "$REPO_ROOT"

PASS=0
FAIL=0

ok()  { PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }

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
echo "  Forgejo #1257 — USTR→USDT mixed 18/6 hop honesty"
echo "════════════════════════════════════════════════════════════════"

export PATH="${HOME}/.cargo/bin:/usr/local/cargo/bin:${PATH}"

run_step "indexer: hop sim mixed 18/6 + implausible + cache buckets" \
  bash -c 'cd indexer && cargo test --lib hop_sim_implausible -- --test-threads=1 && \
    cargo test --lib mixed_18 -- --test-threads=1 && \
    cargo test --lib mixed_6_to_18 -- --test-threads=1 && \
    cargo test --lib scale_mismatch_18dec -- --test-threads=1 && \
    cargo test --lib wide_k_18dec -- --test-threads=1 && \
    cargo test --lib per_unit_size_curve -- --test-threads=1 && \
    cargo test --lib amount_cache_key_18dec -- --test-threads=1'

run_step "frontend: quote scale + Swap 30%/99% Expert" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/swapQuoteAmountScale.test.ts \
    src/utils/swapRouteSlippage.test.ts \
    src/utils/__tests__/tokenRegistry.test.ts \
    src/utils/__tests__/formatAmount.test.ts \
    src/pages/SwapPage.test.tsx'

run_step "code: hop skip, Expert no-waive ≥99%, USDT 18-dec pin" \
  bash -c '
    set -euo pipefail
    grep -q "hop_sim_implausible" indexer/src/api/db_orderbook_sim.rs
    grep -q "ImplausibleHop" indexer/src/api/db_orderbook_sim.rs
    grep -q "ceil_div_wide" indexer/src/api/db_orderbook_sim.rs
    grep -q "ImplausibleHop => HybridSimError::PathUnusable" indexer/src/api/hybrid_route_opt.rs
    grep -q "swapRouteSlippageBlocksSubmit" frontend-dapp/src/pages/SwapPage.tsx
    grep -q "isTheaterRouteQuote" frontend-dapp/src/pages/SwapPage.tsx
    grep -q "swap-theater-quote-blocked" frontend-dapp/src/pages/SwapPage.tsx
    grep -q "swapAmountDecimals" frontend-dapp/src/components/trade/TradeMarketOrderPanel.tsx
    grep -q "USDT_CW20_ADDRESS" frontend-dapp/src/utils/tokenRegistry.ts
    grep -q "terra1z0xe7t5ymmltg4vju8tghkq0pewy4et548ta23nlu9zxtl950uyqkv8mv4" frontend-dapp/src/utils/tokenRegistry.ts
  '

run_step "docs: Q1257 + skill + testing.md" \
  bash -c '
    set -euo pipefail
    grep -q "ustr-usdt-quote-scale" docs/frontend.md
    grep -qE "\*\*Q1257-1\*\*" docs/frontend.md
    grep -qE "\*\*Q1257-6\*\*" docs/frontend.md
    grep -q "Mixed 18/6 hop honesty" docs/indexer-invariants.md
    grep -q "1 USTR" docs/route-solver.md
    grep -qE "cannot\\*\\* waive|cannot waive" docs/swap-max-spread-ux.md
    grep -q "verify-issue-1257" docs/testing.md
    grep -q "Q1257-1" skills/AGENTS_FRONTEND_SWAP_USTR_USDT_SCALE.md
    grep -q "verify-issue-1257" AGENTS.md
  '

run_step "skill: AGENTS_FRONTEND_SWAP_USTR_USDT_SCALE exists" \
  test -f skills/AGENTS_FRONTEND_SWAP_USTR_USDT_SCALE.md

run_step "route-solver constant drift" \
  python3 scripts/check_route_solver_docs.py

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  $PASS passed, $FAIL failed"
echo "════════════════════════════════════════════════════════════════"

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
echo "OK"
