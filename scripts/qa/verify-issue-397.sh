#!/usr/bin/env bash
# Verification for GitLab #397 — governance emergency controls multisig rehearsal (SEC-B09).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
SKIP=0
declare -a RESULTS=()

ok()   { RESULTS+=("PASS  $1"); PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad()  { RESULTS+=("FAIL  $1"); FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }
skip() { RESULTS+=("SKIP  $1"); SKIP=$((SKIP + 1)); echo "  [SKIP] $1"; }

echo "════════════════════════════════════════════════════════════════"
echo "  GitLab #397 — governance emergency rehearsal (SEC-B09)"
echo "════════════════════════════════════════════════════════════════"

echo ""
echo "[1] Doc invariants (runbook, template, skill, cross-links)..."
if python3 scripts/check_governance_emergency_rehearsal_docs.py; then
  ok "check-governance-emergency-rehearsal-docs"
else
  bad "check-governance-emergency-rehearsal-docs"
fi

echo ""
echo "[2] Rehearsal script present and executable..."
if [[ -x scripts/rehearse-governance-emergency-controls.sh ]]; then
  ok "rehearse-governance-emergency-controls.sh executable"
else
  bad "rehearse-governance-emergency-controls.sh missing or not executable"
fi

echo ""
echo "[3] Agent skill AGENTS_GOVERNANCE_EMERGENCY_REHEARSAL.md..."
if [[ -f skills/AGENTS_GOVERNANCE_EMERGENCY_REHEARSAL.md ]]; then
  ok "agent skill present"
else
  bad "agent skill missing"
fi

CONTAINER_NAME="$(docker compose ps -q localterra 2>/dev/null | head -1 || true)"
if [[ -z "$CONTAINER_NAME" ]]; then
  CONTAINER_NAME="$(sg docker -c 'docker compose ps -q localterra' 2>/dev/null | head -1 || true)"
fi
IDX_ENV="$REPO_ROOT/indexer/.env"
FACTORY=""
if [[ -f "$IDX_ENV" ]]; then
  FACTORY="$(sed -n 's/^FACTORY_ADDRESS=//p' "$IDX_ENV" | head -1)"
fi

echo ""
echo "[4] LocalTerra multisig rehearsal (pause, blacklist, unpause, unblacklist)..."
if [[ -z "$CONTAINER_NAME" || -z "$FACTORY" ]]; then
  skip "live LocalTerra rehearsal (chain not up or deploy env missing)"
else
  if ./scripts/rehearse-governance-emergency-controls.sh --output /tmp/sec-b09-verify-397.md; then
    ok "LocalTerra multisig rehearsal completed"
    if grep -q "SetPairPaused paused=true" /tmp/sec-b09-verify-397.md \
      && grep -q "BlacklistWallet" /tmp/sec-b09-verify-397.md \
      && grep -q "UnblacklistWallet" /tmp/sec-b09-verify-397.md; then
      ok "transcript includes all four emergency operations"
    else
      bad "transcript missing required operations"
    fi
  else
    bad "LocalTerra multisig rehearsal failed"
  fi
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL   SKIP: $SKIP"
echo "════════════════════════════════════════════════════════════════"

[[ "$FAIL" -eq 0 ]]
