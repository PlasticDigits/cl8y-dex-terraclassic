#!/usr/bin/env bash
# Focused first-pass/retest checks with explicit unavailable-check accounting.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
PASS=0 FAIL=0 SKIP=0
run() {
  local label="$1"; shift
  printf '\n[%s]\n' "$label"
  if "$@"; then PASS=$((PASS+1)); else FAIL=$((FAIL+1)); fi
}
skip() { SKIP=$((SKIP+1)); printf '\n[SKIP %s] %s\n' "$1" "$2"; }
tests() { cargo test --locked --manifest-path smartcontracts/Cargo.toml -p bridge-debt -p bridge-debt-token -- --test-threads=1; }
shell_checks() {
  bash -n scripts/deploy-usdt-ust1-lp.sh scripts/qa/verify-issue-1393.sh scripts/qa/build-issue-1393-wasm.sh || return 1
  python3 scripts/qa/test-deploy-usdt-ust1-lp1393.py || return 1
  python3 scripts/qa/check-issue-1393-docs.py
}
chain() {
  python3 scripts/qa/localterra-bridge-debt1393.py \
    --container "${VERIFY_ISSUE_1393_CONTAINER:-cl8y-dex-terraclassic-localterra-1}" \
    --artifacts "${VERIFY_ISSUE_1393_ARTIFACTS:-smartcontracts/artifacts}" \
    --evidence-file "${VERIFY_ISSUE_1393_CHAIN_EVIDENCE:-/tmp/issue1393-localterra}-${1}.json" \
    --code-cache "${VERIFY_ISSUE_1393_CODE_CACHE:-/tmp/issue1393-localterra-codes.json}"
}
for pass in first retest; do
  if [[ -n "${VERIFY_ISSUE_1393_SKIP_TESTS_REASON:-}" ]]; then skip "$pass focused tests" "$VERIFY_ISSUE_1393_SKIP_TESTS_REASON"
  else run "$pass focused tests" tests; fi
  if [[ -n "${VERIFY_ISSUE_1393_SKIP_SCRIPTS_REASON:-}" ]]; then skip "$pass script integration and docs" "$VERIFY_ISSUE_1393_SKIP_SCRIPTS_REASON"
  else run "$pass script integration and docs" shell_checks; fi
done
if [[ -n "${VERIFY_ISSUE_1393_SKIP_STATIC_REASON:-}" ]]; then skip "clippy/formatting" "$VERIFY_ISSUE_1393_SKIP_STATIC_REASON"
else
  run "touched-crate formatting" cargo fmt --manifest-path smartcontracts/Cargo.toml -p bridge-debt -p bridge-debt-token --check
  run "touched-crate clippy" cargo clippy --locked --manifest-path smartcontracts/Cargo.toml -p bridge-debt -p bridge-debt-token --all-targets -- -D warnings
fi
if [[ -n "${VERIFY_ISSUE_1393_SKIP_FULL_REASON:-}" ]]; then skip "full contract suite" "$VERIFY_ISSUE_1393_SKIP_FULL_REASON"
else run "full contract suite" make test-contracts; fi
if [[ -n "${VERIFY_ISSUE_1393_SKIP_CHAIN_REASON:-}" ]]; then skip "optimized LocalTerra smoke" "$VERIFY_ISSUE_1393_SKIP_CHAIN_REASON"
else
  for pass in first retest; do run "$pass optimized LocalTerra smoke" chain "$pass"; done
fi
printf '\nIssue #1393: PASS=%s FAIL=%s SKIP=%s\n' "$PASS" "$FAIL" "$SKIP"
if [[ "$FAIL" != 0 ]]; then echo 'FAIL'; exit 1
elif [[ "$SKIP" != 0 ]]; then echo 'PASS with skips (executed checks passed; skipped scope unverified)'
else echo 'PASS'; fi
