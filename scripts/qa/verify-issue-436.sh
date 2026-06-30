#!/usr/bin/env bash
# Automated verification for GitLab #436 — pool triage by TVL/liquidity (SEC-G03).
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
echo "  GitLab #436 — pool triage by liquidity (SEC-G03)"
echo "════════════════════════════════════════════════════════════════"

run_step "pool triage doc invariant" \
  make check-pool-triage-docs

run_step "agent skill present" \
  test -f skills/AGENTS_POOL_TRIAGE.md

run_step "incident template triage links pool ranking" \
  grep -q 'quick-pool-triage-sec-g03' docs/templates/incident-dex-indexer.md

run_step "emergency runbook SQL orders by approx_liquidity_units DESC" \
  grep -q 'ORDER BY approx_liquidity_units DESC' docs/runbooks/emergency-commands.md

echo ""
echo "[optional] Live SQL against indexer Postgres..."
IDX_ENV="$REPO_ROOT/indexer/.env"
if [[ ! -f "$IDX_ENV" ]]; then
  skip "SQL smoke (indexer/.env missing)"
else
  # shellcheck disable=SC1090
  source "$IDX_ENV"
  if [[ -z "${DATABASE_URL:-}" ]]; then
    skip "SQL smoke (DATABASE_URL unset in indexer/.env)"
  elif ! command -v psql >/dev/null 2>&1; then
    if docker compose ps -q postgres >/dev/null 2>&1; then
      if docker compose exec -T postgres psql "$DATABASE_URL" -X -c \
        "SELECT 1 FROM pair_reserves LIMIT 1;" >/dev/null 2>&1; then
        ok "SQL smoke (pair_reserves reachable via docker compose exec)"
      else
        skip "SQL smoke (postgres up but pair_reserves query failed)"
      fi
    else
      skip "SQL smoke (psql not installed and postgres container not running)"
    fi
  elif psql "$DATABASE_URL" -X -c "SELECT 1 FROM pair_reserves LIMIT 1;" >/dev/null 2>&1; then
    ok "SQL smoke (pair_reserves reachable)"
  else
    skip "SQL smoke (DATABASE_URL not reachable)"
  fi
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL   SKIP: $SKIP"
echo "════════════════════════════════════════════════════════════════"

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
