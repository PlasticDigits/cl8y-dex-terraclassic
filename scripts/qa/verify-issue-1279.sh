#!/usr/bin/env bash
# Automated verification for Forgejo #1279 — leftover Lunc Dash WalletConnect path.
#
# Pre-check only (unit + docs + optional children 519/554/658). Device QA 1.5.1–1.5.11
# on a real Lunc Dash install closes leftover via ops-bot, not this script.
#
# VERIFY1279_IID=1279 / VERIFY1279_LEFTOVER_COMPLETE=1 MUST FAIL (L1279-7).
# VERIFY1279_SKIP_CHILDREN=1 skips child 519/554/658 (still runs Lunc Dash Vitest).
#
# Refs: skills/AGENTS_OPS_LUNCDASH_VERIFY.md, docs/qa-invariants.md Q20
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

leftover_complete_requested() {
  [[ "${VERIFY1279_IID:-}" == "1279" || "${VERIFY1279_LEFTOVER_COMPLETE:-}" == "1" ]]
}

echo "════════════════════════════════════════════════════════════════"
echo "  Forgejo #1279 — leftover Lunc Dash WalletConnect (pre-check)"
echo "════════════════════════════════════════════════════════════════"
echo "  Green make does not close leftover. Operator QA_TEMPLATE 1.5"
echo "  on a real Lunc Dash install + ops-bot closes #1279 (L1279-7)."
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: Lunc Dash always-WC + pairing + UA + legal hint + atomic post" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/components/wallet/__tests__/connectWalletOptions.test.ts \
    src/utils/__tests__/walletConnectPairing.test.ts \
    src/components/wallet/__tests__/WalletConnectPairingModal.test.tsx \
    src/utils/__tests__/detectWalletInAppBrowser.test.ts \
    src/utils/__tests__/legalKeplrInAppHint.test.ts \
    src/services/terraclassic/__tests__/terraWalletSignTxRaw.amino.test.ts'

run_step "skill: AGENTS_OPS_LUNCDASH_VERIFY L1279-1–L1279-8" \
  bash -c '
    set -euo pipefail
    test -f skills/AGENTS_OPS_LUNCDASH_VERIFY.md
    grep -qE "\*\*L1279-1" skills/AGENTS_OPS_LUNCDASH_VERIFY.md
    grep -qE "\*\*L1279-8" skills/AGENTS_OPS_LUNCDASH_VERIFY.md
    grep -qE "#1279" skills/AGENTS_OPS_LUNCDASH_VERIFY.md
    grep -qE "buildLuncDashDeepLink" skills/AGENTS_OPS_LUNCDASH_VERIFY.md
    grep -qE "VERIFY1279_LEFTOVER_COMPLETE" skills/AGENTS_OPS_LUNCDASH_VERIFY.md
  '

run_step "docs: Q20 + testing.md + frontend.md + QA 1.5 + onboarding matrix" \
  bash -c '
    set -euo pipefail
    grep -qE "Q20" docs/qa-invariants.md
    grep -qE "L1279-1" docs/qa-invariants.md
    grep -qE "L1279-8" docs/qa-invariants.md
    grep -qE "verify-issue-1279" docs/qa-invariants.md
    grep -qE "AGENTS_OPS_LUNCDASH_VERIFY" docs/qa-invariants.md
    grep -qE "verify-issue-1279" docs/testing.md
    grep -qE "L1279-1" docs/testing.md
    grep -qE "verify-issue-1279" docs/frontend.md
    grep -qE "1\.5\.1" QA_TEMPLATE.md
    grep -qE "1\.5\.11" QA_TEMPLATE.md
    grep -qE "Lunc Dash" docs/qa-onboarding.md
    grep -qE "1279" docs/qa-onboarding.md
  '

run_step "AGENTS.md leftover playbook #1279" \
  grep -qE "AGENTS_OPS_LUNCDASH_VERIFY|#1279" AGENTS.md && \
  grep -qE "verify-issue-1279" AGENTS.md

run_step "WC mobile + clickwrap leftover crosslink #1279" \
  grep -qE "AGENTS_OPS_LUNCDASH_VERIFY|#1279" skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md && \
  grep -qE "AGENTS_OPS_LUNCDASH_VERIFY|#1279" skills/AGENTS_FRONTEND_CLICKWRAP.md && \
  grep -qE "AGENTS_OPS_LUNCDASH_VERIFY|#1279" skills/AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md

run_step "code: LuncDash always WALLETCONNECT + luncdash scheme + atomic post + UA" \
  bash -c '
    set -euo pipefail
    grep -qE "buildLuncDashDeepLink" frontend-dapp/src/utils/walletConnectPairing.ts
    grep -qE "luncdash:" frontend-dapp/src/utils/walletConnectPairing.ts
    grep -qE "WalletName.LUNCDASH" frontend-dapp/src/components/wallet/connectWalletOptions.ts
    grep -qE "WalletType.WALLETCONNECT" frontend-dapp/src/components/wallet/connectWalletOptions.ts
    grep -qE "isAtomicWalletConnectPost" frontend-dapp/src/services/terraclassic/terraWalletSignTxRaw.ts
    grep -qF "LuncDash|LUNCDash|LUNC Dash" frontend-dapp/src/utils/detectWalletInAppBrowser.ts
    grep -qE "luncdash" frontend-dapp/src/utils/legalKeplrInAppHint.ts
    grep -qE "always WalletConnect" frontend-dapp/src/components/wallet/__tests__/connectWalletOptions.test.ts
    grep -qE "Lunc Dash in-app UA" frontend-dapp/src/utils/__tests__/detectWalletInAppBrowser.test.ts
    grep -qE "importOriginal" frontend-dapp/src/hooks/__tests__/useWallet.test.ts
  '

run_step "Makefile verify-issue-1279" \
  grep -qE "verify-issue-1279" Makefile

if [[ "${VERIFY1279_SKIP_CHILDREN:-}" == "1" ]]; then
  echo ""
  echo "[children 519/554/658 skipped (VERIFY1279_SKIP_CHILDREN=1)]"
  RESULTS+=("SKIP  children 519/554/658")
else
  run_step "child: verify-issue-519 (pairing)" \
    make verify-issue-519
  run_step "child: verify-issue-554 (pairing foreground + Connect options)" \
    make verify-issue-554
  run_step "child: verify-issue-658 (terms hint not Keplr-only)" \
    make verify-issue-658
fi

echo ""
echo "[leftover-complete gate (L1279-7)]"
if leftover_complete_requested; then
  bad "VERIFY1279_IID=1279 / LEFTOVER_COMPLETE=1 cannot close leftover (ops-bot QA 1.5)"
else
  ok "leftover-complete not requested (pre-check only)"
fi

echo ""
echo "[selftest: leftover-complete flags fail closed]"
if (
  VERIFY1279_IID=1279
  leftover_complete_requested
); then
  ok "bare VERIFY1279_IID=1279 is leftover-complete FAIL"
else
  bad "bare IID leftover-complete gate did not fail closed"
fi
if (
  unset VERIFY1279_IID
  VERIFY1279_LEFTOVER_COMPLETE=1
  leftover_complete_requested
); then
  ok "VERIFY1279_LEFTOVER_COMPLETE=1 is leftover-complete FAIL"
else
  bad "LEFTOVER_COMPLETE leftover-complete gate did not fail closed"
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
echo "  Pre-check only — do not close #1279 from this output."
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
exit 0
