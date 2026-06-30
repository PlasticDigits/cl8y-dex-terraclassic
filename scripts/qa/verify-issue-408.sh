#!/usr/bin/env bash
# Verification for GitLab #408 — governance key rotation cookbook + rehearsal (SEC-D10).
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
echo "  GitLab #408 — governance key rotation (SEC-D10)"
echo "════════════════════════════════════════════════════════════════"

echo ""
echo "[1] Doc invariants (runbook, skill, script, cross-links)..."
if python3 scripts/check_governance_key_rotation_docs.py; then
  ok "check-governance-key-rotation-docs"
else
  bad "check-governance-key-rotation-docs"
fi

echo ""
echo "[2] Rehearsal script present and executable..."
if [[ -x scripts/rehearse-governance-key-rotation.sh ]]; then
  ok "rehearse-governance-key-rotation.sh executable"
else
  bad "rehearse-governance-key-rotation.sh missing or not executable"
fi

echo ""
echo "[3] Agent skill AGENTS_GOVERNANCE_KEY_ROTATION.md..."
if [[ -f skills/AGENTS_GOVERNANCE_KEY_ROTATION.md ]]; then
  ok "agent skill present"
else
  bad "agent skill missing"
fi

echo ""
echo "[4] DEX-P2-026 marked shipped in backlog..."
if grep -q "DEX-P2-026" docs/reviews/20260409T030009Z/ISSUE_BACKLOG.md \
  && grep -A6 "DEX-P2-026" docs/reviews/20260409T030009Z/ISSUE_BACKLOG.md | grep -qiE 'shipped|done|governance-key-rotation'; then
  ok "DEX-P2-026 backlog entry closed with runbook link"
else
  bad "DEX-P2-026 backlog entry not marked shipped"
fi

CONTAINER_NAME="$(docker compose ps -q localterra 2>/dev/null | head -1 || true)"
if [[ -z "$CONTAINER_NAME" ]]; then
  CONTAINER_NAME="$(sg docker -c 'docker compose ps -q localterra' 2>/dev/null | head -1 || true)"
fi
FACTORY="$(sed -n 's/^FACTORY_ADDRESS=//p' indexer/.env 2>/dev/null | head -1)"

echo ""
echo "[5] LocalTerra wasm-admin rotation round-trip (rehearsal)..."
if [[ -z "$CONTAINER_NAME" || -z "$FACTORY" ]]; then
  skip "live rotation rehearsal (chain not up or deploy env missing)"
else
  set +e
  ./scripts/rehearse-governance-key-rotation.sh --output /tmp/sec-d10-verify-408.md
  rc=$?
  set -e
  if [[ "$rc" -eq 0 ]]; then
    if grep -q "round-trip restored" /tmp/sec-d10-verify-408.md \
      && grep -q "set-contract-admin (multisig) -> original" /tmp/sec-d10-verify-408.md; then
      ok "LocalTerra rotation round-trip completed and restored"
    else
      bad "rotation transcript missing expected operations"
    fi
  elif [[ "$rc" -eq 2 ]]; then
    skip "live rotation (factory admin not in local keyring — safe round-trip not possible here)"
  else
    bad "LocalTerra rotation rehearsal failed (exit $rc)"
  fi
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL   SKIP: $SKIP"
echo "════════════════════════════════════════════════════════════════"

[[ "$FAIL" -eq 0 ]]
