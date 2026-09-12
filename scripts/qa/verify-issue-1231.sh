#!/usr/bin/env bash
# Automated verification for git.cl8y.com #1231 — Observe query
# Decimal::checked_from_ratio skip after #465 execute skip.
#
# Layers (no LocalTerra required):
#   1. pair oracle_overflow (#465 execute) + oracle_observe (#1231 query)
#   2. cl8y-dex-tests oracle (existing Observe JSON / interpolation)
#   3. Docs/skills O1231 skip-extrapolate (not clamp, not from_ratio)
#   4. Grep: Observe path has no Decimal::from_ratio
#   5. Retest pair oracle modules
#
# Refs: skills/AGENTS_TWAP_OBSERVE_RATIO.md, git.cl8y.com #1231
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
echo "  #1231 — Observe query checked_from_ratio skip (O1231)"
echo "════════════════════════════════════════════════════════════════"

export PATH="${HOME}/.cargo/bin:/usr/local/cargo/bin:${PATH}"

run_step "pair: oracle_overflow (#465 execute skip stays green)" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-pair oracle_overflow -- --nocapture'

run_step "pair: oracle_observe (extreme ratio skip + balanced extrapolate)" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-pair oracle_observe -- --nocapture'

run_step "multitest: cl8y-dex-tests oracle Observe JSON / interpolation" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-tests oracle -- --test-threads=1'

PAIR_CONTRACT="smartcontracts/contracts/pair/src/contract.rs"

run_step "pair: Observe path uses checked_from_ratio (no from_ratio nearby)" \
  python3 - <<'PY'
import pathlib, sys
p = pathlib.Path("smartcontracts/contracts/pair/src/contract.rs")
text = p.read_text()
fn = "fn oracle_observe_single"
i = text.find(fn)
if i < 0:
    print("oracle_observe_single not found", file=sys.stderr)
    sys.exit(1)
# Next top-level fn after this helper (instantiate is the following section).
j = text.find("\nfn instantiate(", i + len(fn))
if j < 0:
    j = text.find("\npub fn instantiate(", i + len(fn))
chunk = text[i:j if j > 0 else i + 8000]
if "Decimal::from_ratio(" in chunk:
    print("Decimal::from_ratio still in oracle_observe_single", file=sys.stderr)
    sys.exit(1)
if "Decimal::checked_from_ratio" not in chunk:
    print("checked_from_ratio missing in oracle_observe_single", file=sys.stderr)
    sys.exit(1)
if "price_a_cumulative, latest_obs.price_b_cumulative" not in chunk:
    print("skip-extrapolate last-cumulative return missing", file=sys.stderr)
    sys.exit(1)
print("oracle_observe_single: checked_from_ratio + skip last cumulatives")
PY

run_step "pair: oracle_update still checked_from_ratio (#465)" \
  grep -qE 'Decimal::checked_from_ratio\(reserve_b, reserve_a\)' "$PAIR_CONTRACT"

run_step "docs: twap-oracle #1231 skip-extrapolate" \
  grep -qE '#1231' docs/twap-oracle.md

run_step "docs: contracts-security-audit O1231" \
  grep -qE 'O1231' docs/contracts-security-audit.md

run_step "skill: AGENTS_TWAP_OBSERVE_RATIO O1231-1 skip" \
  grep -qE 'O1231-1' skills/AGENTS_TWAP_OBSERVE_RATIO.md

run_step "skill: no clamp-to-MAX as the chosen policy" \
  grep -qE 'Do not clamp to .Decimal::MAX' skills/AGENTS_TWAP_OBSERVE_RATIO.md

run_step "AGENTS.md verify-issue-1231" \
  grep -qE 'verify-issue-1231' AGENTS.md

run_step "docs: testing.md #1231 row" \
  grep -qE 'verify-issue-1231' docs/testing.md

run_step "docs: integrators.md skip-extrapolate" \
  grep -qE '#1231' docs/integrators.md

echo ""
echo "── retest (pair oracle modules) ──"
run_step "retest: pair oracle_overflow" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-pair oracle_overflow -- --nocapture'
run_step "retest: pair oracle_observe" \
  bash -c 'cd smartcontracts && cargo test -p cl8y-dex-pair oracle_observe -- --nocapture'

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
