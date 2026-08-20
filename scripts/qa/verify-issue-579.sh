#!/usr/bin/env bash
# Verification for GitLab #579: CoinGecko oracle descriptive User-Agent (403 vs 429).
#
# Proves (unit + docs; no live CoinGecko, no Postgres):
#   1. Oracle reqwest client sets ORACLE_USER_AGENT (indexer name + repo URL).
#   2. Wiremock CG mocks require that User-Agent header.
#   3. CoinGecko 429 → RateLimited; 403 User-Agent body → MissingUserAgent (not 429).
#   4. Skill/runbook/invariants document X7 and 403 vs 429.
#
# Refs: skills/AGENTS_INDEXER_EXTERNAL_ORACLE.md
#       docs/runbooks/indexer-external-oracle.md
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
echo "  GitLab #579 — CoinGecko oracle User-Agent (X7)"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"

run_docs() {
  set -euo pipefail
  test -f docs/runbooks/indexer-external-oracle.md
  test -f skills/AGENTS_INDEXER_EXTERNAL_ORACLE.md
  grep -q 'ORACLE_USER_AGENT' indexer/src/indexer/oracle.rs
  grep -q 'cl8y-dex-indexer/' indexer/src/indexer/oracle.rs
  grep -q 'https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic' indexer/src/indexer/oracle.rs
  grep -q 'CARGO_PKG_VERSION' indexer/src/indexer/oracle.rs
  grep -q '.user_agent(ORACLE_USER_AGENT)' indexer/src/indexer/oracle.rs
  grep -q 'MissingUserAgent' indexer/src/indexer/oracle.rs
  grep -q 'header("user-agent", ORACLE_USER_AGENT)' indexer/src/indexer/oracle.rs
  grep -q 'fetch_coingecko_maps_403_user_agent_not_rate_limited' indexer/src/indexer/oracle.rs
  grep -q 'fetch_coingecko_maps_429_to_rate_limited' indexer/src/indexer/oracle.rs
  grep -q 'CoinGecko HTTP 403: User-Agent required' indexer/src/indexer/oracle.rs
  # No browser impersonation / UA rotation on the production const.
  ! grep -A6 'pub const ORACLE_USER_AGENT' indexer/src/indexer/oracle.rs | grep -qiE 'mozilla|chrome|firefox|keplr'
  ! grep -qE 'rotate.*[Uu]ser-[Aa]gent|[Uu]ser-[Aa]gent.*rotat' indexer/src/indexer/oracle.rs
  grep -qE '\*\*X7\*\*' skills/AGENTS_INDEXER_EXTERNAL_ORACLE.md
  grep -q 'User-Agent' skills/AGENTS_INDEXER_EXTERNAL_ORACLE.md
  grep -q 'MissingUserAgent' skills/AGENTS_INDEXER_EXTERNAL_ORACLE.md
  grep -q 'RateLimited' skills/AGENTS_INDEXER_EXTERNAL_ORACLE.md
  grep -q 'User-Agent' docs/runbooks/indexer-external-oracle.md
  grep -q '403' docs/runbooks/indexer-external-oracle.md
  grep -q '429' docs/runbooks/indexer-external-oracle.md
  grep -q 'CoinGecko User-Agent (#579)' docs/indexer-invariants.md
  grep -q 'verify-issue-579' AGENTS.md
  grep -q 'AGENTS_INDEXER_EXTERNAL_ORACLE' AGENTS.md
}

run_oracle_lib() {
  (cd indexer && cargo test --lib oracle -- --quiet)
}

echo ""
echo "── first pass ──"
run_step "docs: UA const + 403 vs 429 + skill/runbook/invariants" run_docs
run_step "indexer lib: oracle User-Agent + CoinGecko 403/429" run_oracle_lib

echo ""
echo "── retest ──"
run_step "retest docs: UA const + 403 vs 429 + skill/runbook/invariants" run_docs
run_step "retest indexer lib: oracle User-Agent + CoinGecko 403/429" run_oracle_lib

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  #579 results: ${PASS} passed, ${FAIL} failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
