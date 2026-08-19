#!/usr/bin/env bash
# Automated verification for GitLab #567 — Keplr + Ledger Nano signing stall.
#
# Proves (unit + docs; no physical Ledger / no chain required):
#   1. Amino vs signDirect: Ledger / useAmino → signAmino; software Keplr → signDirect.
#   2. Pre-sign Keplr experimentalSuggestChain is best-effort (suggest reject does not fail).
#   3. Signing-phase Ledger hint vs software t=0; recovering copy unchanged (#359).
#   4. Sign-stall copy ≠ TERRA_TX_BROADCAST_TIMEOUT_MESSAGE; late signature does not broadcast.
#   5. Docs/skills K567-1–K567-8 crosslinked; make verify-issue-567.
#
# Refs: skills/AGENTS_FRONTEND_KEPLR_LEDGER.md,
#       docs/frontend.md § Keplr + Ledger signing
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
echo "  GitLab #567 — Keplr + Ledger Nano signing stall"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: amino vs direct + Keplr prepare + sign-stall" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/services/terraclassic/__tests__/terraWalletSignTxRaw.amino.test.ts \
    src/services/terraclassic/__tests__/keplrExtensionConfig.test.ts \
    src/services/terraclassic/__tests__/terraBroadcastKeplrLedger.test.ts'

run_step "frontend: signing hint + stall copy + timeout late-settle" \
  bash -c 'bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/terraBroadcastUi.test.ts \
    src/utils/__tests__/terraTxTimeout.keplrLedger.test.ts \
    src/utils/__tests__/withPromiseTimeout.test.ts \
    src/components/ui/__tests__/TerraBroadcastPendingLink.test.tsx'

run_step "code: exported walletUsesAmino + Keplr prepare hook" \
  grep -qE 'export function walletUsesAmino' frontend-dapp/src/services/terraclassic/terraWalletSignTxRaw.ts && \
  grep -qE 'prepareKeplrExtensionForTerraClassicSign' frontend-dapp/src/services/terraclassic/terraBroadcast.ts && \
  grep -qE 'walletIsNanoLedger' frontend-dapp/src/services/terraclassic/terraWalletSignTxRaw.ts && \
  grep -qE 'TERRA_TX_SIGN_TIMEOUT_MS' frontend-dapp/src/services/terraclassic/terraBroadcast.ts

run_step "code: sign-stall copy is not #173 broadcast timeout" \
  python3 - <<'PY'
from pathlib import Path
text = Path("frontend-dapp/src/utils/terraTxTimeout.ts").read_text()
if "TERRA_TX_SIGN_STALL_LEDGER_MESSAGE" not in text or "TERRA_TX_SIGN_STALL_KEPLR_MESSAGE" not in text:
    raise SystemExit("missing sign-stall message constants")
if "TERRA_TX_SIGN_TIMEOUT_MS" not in text:
    raise SystemExit("missing TERRA_TX_SIGN_TIMEOUT_MS")
# Stall copy must not reuse broadcast timeout wording.
import re
m = re.search(r"TERRA_TX_SIGN_STALL_LEDGER_MESSAGE\s*=\s*'([^']+)'", text)
k = re.search(r"TERRA_TX_SIGN_STALL_KEPLR_MESSAGE\s*=\s*'([^']+)'", text)
b = re.search(r"TERRA_TX_BROADCAST_TIMEOUT_MESSAGE\s*=\s*'([^']+)'", text)
if not (m and k and b):
    raise SystemExit("could not parse timeout message strings")
for label, msg in (("ledger", m.group(1)), ("keplr", k.group(1))):
    if msg == b.group(1):
        raise SystemExit(f"{label} stall copy equals broadcast timeout")
    if "check your connection" in msg.lower():
        raise SystemExit(f"{label} stall copy mentions check your connection")
    if "330" in msg or "118" in msg:
        raise SystemExit(f"{label} stall copy mentions coin types")
print("sign-stall copy distinct from #173")
PY

run_step "docs: frontend.md Keplr Ledger invariants K567-1–K567-8" \
  grep -qE 'keplr-ledger-signing' docs/frontend.md && \
  grep -qE '\*\*K567-1\*\*' docs/frontend.md && \
  grep -qE '\*\*K567-8\*\*' docs/frontend.md && \
  grep -qE 'make verify-issue-567' docs/frontend.md

run_step "docs: QA matrix Keplr+Ledger Nano columbus-5" \
  grep -qE 'Keplr \+ Ledger Nano' docs/qa-onboarding.md && \
  grep -qE 'columbus-5' docs/qa-onboarding.md && \
  grep -qE '#567' docs/qa-onboarding.md

run_step "docs: FAQ recovery + QA_TEMPLATE Ledger rows" \
  grep -qE 'keplr-ledger-signing-stall' docs/user-incident-faq.md && \
  grep -qE 'Terra Classic \(LUNA\)' docs/user-incident-faq.md && \
  grep -qE 'Keplr \+ Ledger Nano' QA_TEMPLATE.md

run_step "skill: AGENTS_FRONTEND_KEPLR_LEDGER K567 + verify" \
  grep -qE '\*\*K567-1' skills/AGENTS_FRONTEND_KEPLR_LEDGER.md && \
  grep -qE 'make verify-issue-567' skills/AGENTS_FRONTEND_KEPLR_LEDGER.md && \
  grep -qE 'walletUsesAmino' skills/AGENTS_FRONTEND_KEPLR_LEDGER.md

run_step "skill: timeout + Station + swap-summary crosslinks #567" \
  grep -qE 'AGENTS_FRONTEND_KEPLR_LEDGER|#567' skills/AGENTS_FRONTEND_TX_BROADCAST_TIMEOUT.md && \
  grep -qE 'AGENTS_FRONTEND_KEPLR_LEDGER|#567' skills/AGENTS_FRONTEND_STATION_SIGNING.md && \
  grep -qE 'AGENTS_FRONTEND_KEPLR_LEDGER|#567' skills/AGENTS_FRONTEND_SWAP_SIGNING_CONFIRMATION.md

run_step "AGENTS.md playbook link #567" \
  grep -qE 'AGENTS_FRONTEND_KEPLR_LEDGER' AGENTS.md && \
  grep -qE 'verify-issue-567' AGENTS.md

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
