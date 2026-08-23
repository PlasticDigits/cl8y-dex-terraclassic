#!/usr/bin/env bash
# Automated verification for GitLab #594 — community tax indexer catalog.
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
echo "  GitLab #594 — community tax indexer catalog"
echo "════════════════════════════════════════════════════════════════"

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
fi

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_step "docs: invariants + skill + testing + AGENTS.md" \
  bash -c 'grep -q "I594-1" skills/AGENTS_INDEXER_COMMUNITY_TOKENS.md && \
    grep -q "Community tax catalog (GitLab #594)" docs/indexer-invariants.md && \
    grep -q "verify-issue-594" docs/testing.md && \
    grep -q "AGENTS_INDEXER_COMMUNITY_TOKENS" AGENTS.md'

run_step "code: no request-path LCD in list handler" \
  bash -c '! grep -qE "get_contract_info|query_contract" indexer/src/api/community_tokens.rs'

run_step "indexer integration: api_community_tokens" \
  bash -c 'cd indexer && cargo test --test api_community_tokens -- --test-threads=1 --quiet'

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> GitLab #594 verification passed"
