#!/usr/bin/env bash
# git.cl8y.com #1224 and #1322 — TWAP price×dt and cumulative add are Uint256.
# A stored u128 decimal string zero-extends. Sums past 2^128 stay the full
# integer. #465 / #1231 ratio skip is unchanged.
#
# make verify-issue-1224 and make verify-issue-1322 both run this script.
#
# Layers (no LocalTerra):
#   1. dex-common oracle helpers
#   2. pair oracle_overflow (#465 + #1224) and oracle_u256 (#1322)
#   3. pair oracle_observe (#1231 still green)
#   4. multitest oracle module
#   5. Charts decimal-string helper
#   6. Docs / skill O1322, no "handled gracefully with errors"
#   7. Retest the pair oracle modules
#
# Refs: skills/AGENTS_TWAP_CUMULATIVE_U256.md
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
echo "  #1224 / #1322 — TWAP cumulative Uint256 (O1322)"
echo "════════════════════════════════════════════════════════════════"

export PATH="${HOME}/.cargo/bin:/usr/local/cargo/bin:${PATH}"

run_step "dex-common: oracle helpers (wide product + legacy json)" \
  bash -c 'cd smartcontracts && cargo test -p dex-common --lib oracle -- --nocapture'

run_step "pair: oracle_overflow (#465 skip + #1224 wide product)" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-pair oracle_overflow -- --nocapture'

run_step "pair: oracle_u256 (near-max sum, interpolation, zero-extend)" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-pair oracle_u256 -- --nocapture'

run_step "pair: oracle_observe (#1231 skip stays green)" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-pair oracle_observe -- --nocapture'

run_step "multitest: cl8y-dex-tests oracle" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-tests oracle -- --test-threads=1'

run_step "frontend: computeTwapPriceDecimalString across 2^128" \
  bash -c './scripts/with-node.sh --cwd frontend-dapp -- npm exec -- vitest run src/services/terraclassic/__tests__/oracle.test.ts'

run_step "pair: cumulatives are Uint256 and adds are checked" \
  python3 - <<'PY'
import pathlib, sys
oracle = pathlib.Path("smartcontracts/packages/dex-common/src/oracle.rs").read_text()
pair = pathlib.Path("smartcontracts/contracts/pair/src/contract.rs").read_text()
if "pub price_a_cumulative: Uint256" not in oracle:
    print("Observation cumulative is not Uint256", file=sys.stderr)
    sys.exit(1)
if "wrapping_add" in oracle or "wrapping_mul" in oracle:
    print("oracle.rs uses wrapping arithmetic", file=sys.stderr)
    sys.exit(1)
start = pair.find("fn oracle_update(")
end = pair.find("fn oracle_observe_single(")
chunk = pair[start:end]
if "wrapping_add" in chunk or "wrapping_mul" in chunk:
    print("oracle_update wraps", file=sys.stderr)
    sys.exit(1)
if "add_price_cumulative" not in chunk:
    print("oracle_update missing checked cumulative add", file=sys.stderr)
    sys.exit(1)
# Production call sites: swap, provide, withdraw. Tests are extra.
prod = pair.split("mod spot_linear_spread_tests")[0]
calls = prod.count("oracle_update(")
# definition + 3 execute sites
if calls != 4:
    print(f"expected 4 oracle_update( in production, found {calls}", file=sys.stderr)
    sys.exit(1)
print("Uint256 cumulative, checked add, three execute call sites")
PY

run_step "docs: twap-oracle describes Uint256, not an error brick" \
  python3 - <<'PY'
import pathlib, sys
text = pathlib.Path("docs/twap-oracle.md").read_text()
if "handled gracefully with errors" in text:
    print("stale overflow-as-error sentence", file=sys.stderr)
    sys.exit(1)
for needle in ("Uint256", "#1224", "#1322", "O1322"):
    if needle not in text:
        print(f"missing {needle}", file=sys.stderr)
        sys.exit(1)
print("twap-oracle.md O1322")
PY

run_step "docs: contracts-security-audit O1322 and O1231 both present" \
  python3 - <<'PY'
import pathlib, sys
text = pathlib.Path("docs/contracts-security-audit.md").read_text()
if "O1322" not in text or "O1231" not in text:
    print("audit matrix missing O1322 or O1231", file=sys.stderr)
    sys.exit(1)
print("audit matrix")
PY

run_step "docs: live-pair migrate follow-ups cross-linked" \
  python3 - <<'PY'
import pathlib, sys
paths = (
    "docs/twap-oracle.md",
    "docs/integrators.md",
    "docs/contracts-security-audit.md",
    "docs/testing.md",
    "skills/AGENTS_TWAP_CUMULATIVE_U256.md",
    "AGENTS.md",
)
for name in paths:
    text = pathlib.Path(name).read_text()
    for issue in ("#1324", "#1232"):
        if issue not in text:
            print(f"{name} missing {issue} rollout/preflight link", file=sys.stderr)
            sys.exit(1)
print("#1324 live-pair migrate and #1232 preflight backfill linked")
PY

run_step "skill: AGENTS_TWAP_CUMULATIVE_U256 O1322-1" \
  grep -q 'O1322-1' skills/AGENTS_TWAP_CUMULATIVE_U256.md

run_step "skill: ratio skip still points at O1231" \
  grep -q 'O1231-1' skills/AGENTS_TWAP_OBSERVE_RATIO.md

run_step "AGENTS.md verify-issue-1322" \
  grep -q 'verify-issue-1322' AGENTS.md

run_step "docs: testing.md #1322 row" \
  grep -q 'verify-issue-1322' docs/testing.md

run_step "pair cw2 is 1.18.0" \
  grep -q 'const CONTRACT_VERSION: &str = "1.18.0"' smartcontracts/contracts/pair/src/contract.rs

echo ""
echo "── retest (pair oracle modules) ──"
run_step "retest: pair oracle_overflow" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-pair oracle_overflow -- --nocapture'
run_step "retest: pair oracle_u256" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-pair oracle_u256 -- --nocapture'
run_step "retest: dex-common oracle" \
  bash -c 'cd smartcontracts && cargo test -p dex-common --lib oracle -- --nocapture'

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  Results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
