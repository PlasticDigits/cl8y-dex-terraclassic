#!/usr/bin/env bash
# Verification for Forgejo #1205 — redacted UTC-day evidence JSON export.
# Requires Postgres (bootstraps indexer/.env when missing). A skipped cargo
# run is a failure: static greps alone do not prove the UNION decode or pages.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS+1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL+1)); echo "  [FAIL] $1" >&2; }

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
echo "  Forgejo #1205 — GET /api/v1/evidence/daily"
echo "════════════════════════════════════════════════════════════════"

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_step "evidence route in api router" \
  grep -q 'evidence/daily' indexer/src/api/mod.rs

run_step "evidence handler module" \
  test -f indexer/src/api/evidence.rs

run_step "evidence daily query module" \
  test -f indexer/src/db/queries/evidence_daily.rs

run_step "skill AGENTS_INDEXER_EVIDENCE_DAILY.md" \
  test -f skills/AGENTS_INDEXER_EVIDENCE_DAILY.md

run_step "invariants doc mentions evidence daily" \
  grep -q 'evidence/daily' docs/indexer-invariants.md

run_step "AGENTS.md lists verify-issue-1205" \
  grep -q 'verify-issue-1205' AGENTS.md

run_step "evidence is not on lcd_heavy_router and SQL skips pair_reserves" \
  bash -c '
    ! grep -q "pair_reserves" indexer/src/db/queries/evidence_daily.rs && \
    python3 - <<'"'"'PY'"'"'
from pathlib import Path
text = Path("indexer/src/api/mod.rs").read_text()
start = text.find("let lcd_heavy_router = Router::new()")
end = text.find("apply_rate_limit_layer(lcd_heavy_router")
block = text[start:end]
assert "/api/v1/evidence/daily" not in block, "evidence route is on lcd_heavy_router"
api = text[text.find("let api_router = Router::new()"):text.find(".merge(lcd_heavy_router)")]
assert "/api/v1/evidence/daily" in api, "evidence route missing from global api_router"
print("router membership ok")
PY
  '

if ! command -v cargo >/dev/null; then
  bad "integration tests api_evidence_daily (cargo missing)"
else
  run_step "integration tests api_evidence_daily" \
    bash -c 'cd indexer && cargo test --test api_evidence_daily -- --test-threads=1'
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '  PASS: %s  FAIL: %s\n' "$PASS" "$FAIL"
for r in "${RESULTS[@]}"; do echo "  $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
