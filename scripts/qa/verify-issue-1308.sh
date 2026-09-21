#!/usr/bin/env bash
# Forgejo #1308 — Lunc Dash WalletConnect payload query on dex.cl8y.com (mobile pairing).
#
# Unit + child 519/554 pre-check. Production device AC1–AC4 (wallet lists dex.cl8y.com)
# closes via QA after Coolify — not this script.
#
# VERIFY1308_DEVICE_COMPLETE=1 / VERIFY1308_IID=1308 MUST FAIL.
#
# Refs: skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md, skills/AGENTS_OPS_LUNCDASH_VERIFY.md
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

device_complete_requested() {
  [[ "${VERIFY1308_IID:-}" == "1308" || "${VERIFY1308_DEVICE_COMPLETE:-}" == "1" ]]
}

echo "════════════════════════════════════════════════════════════════"
echo "  Forgejo #1308 — Lunc Dash WC payload query (pre-check)"
echo "════════════════════════════════════════════════════════════════"
echo "  Device AC on production dex.cl8y.com closes #1308 after deploy."
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: Lunc Dash payload query + pairing modal + hook" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/walletConnectPairing.test.ts \
    src/components/wallet/__tests__/WalletConnectPairingModal.test.ts \
    src/services/terraclassic/__tests__/walletConnectPairingHook.test.ts \
    src/services/terraclassic/__tests__/cosmesPatch127.test.ts'

run_step "skill: AGENTS_FRONTEND_WALLETCONNECT_MOBILE #1308 payload key" \
  bash -c '
    set -euo pipefail
    grep -qE "#1308" skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md
    grep -qE "parseLuncDashDeepLinkPayload" skills/AGENTS_FRONTEND_WALLETCONNECT_MOBILE.md
    grep -qE "buildLuncDashDeepLink" frontend-dapp/src/utils/walletConnectPairing.ts
    grep -qE "parseLuncDashDeepLinkPayload" frontend-dapp/src/utils/walletConnectPairing.ts
    grep -qF "payload=" frontend-dapp/src/utils/walletConnectPairing.ts
    grep -qF "encodeURIComponent(uri)" frontend-dapp/src/utils/walletConnectPairing.ts
  '

run_step "docs: frontend.md Lunc Dash payload + verify-issue-1308" \
  bash -c '
    set -euo pipefail
    grep -qE "verify-issue-1308" docs/frontend.md
    grep -qE "#1308" docs/frontend.md
    grep -qE "verify-issue-1308" docs/testing.md
  '

if [[ "${VERIFY1308_SKIP_CHILDREN:-}" == "1" ]]; then
  echo ""
  echo "[child verify-issue-519 / verify-issue-554] skipped (VERIFY1308_SKIP_CHILDREN=1)"
  ok "child 519/554 (skipped)"
else
  run_step "child: verify-issue-519" ./scripts/qa/verify-issue-519.sh
  run_step "child: verify-issue-554" ./scripts/qa/verify-issue-554.sh
fi

if device_complete_requested; then
  echo ""
  echo "[device-complete] VERIFY1308_DEVICE_COMPLETE / VERIFY1308_IID=1308 must not close #1308 via make"
  bad "device-complete guard (expected FAIL)"
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  Summary: $PASS passed, $FAIL failed"
echo "════════════════════════════════════════════════════════════════"
for row in "${RESULTS[@]}"; do echo "  $row"; done

if [[ $FAIL -gt 0 ]]; then
  exit 1
fi
exit 0
