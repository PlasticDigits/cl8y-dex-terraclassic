#!/usr/bin/env bash
# Implementation verification with truthful unavailable-check accounting.
set -euo pipefail
cd "$(dirname "${BASH_SOURCE[0]}")/../.."
pass=0 fail=0 skip=0
run() {
  local label="$1"; shift
  if "$@"; then printf 'PASS: %s\n' "$label"; pass=$((pass+1))
  else printf 'FAIL: %s\n' "$label"; fail=$((fail+1)); fi
}
for attempt in first retest; do
  run "$attempt functional failure/completeness/cache tests" python3 scripts/ci/functional-test.py
  run "$attempt build contamination/provenance/cleanup tests" python3 scripts/ci/test-build-inputs.py
  run "$attempt immutable image declarations" python3 scripts/ci/verify-image-pins.py
done
run 'shell syntax' bash -n scripts/ci/functional-run.sh scripts/ci/functional-localterra.sh \
  scripts/ci/build-indexer-release.sh smartcontracts/scripts/optimize.sh
run 'diff whitespace' git diff --check
if [[ -n "${VERIFY_ISSUE_1400_SKIP_POLICY_REASON:-}" ]]; then
  printf 'SKIP: signed policy image/tests: %s\n' "$VERIFY_ISSUE_1400_SKIP_POLICY_REASON"
  skip=$((skip+1))
else
  run 'signed policy tests/image' timeout --signal=TERM --kill-after=30s 30m \
    docker build -f scripts/ci/trusted-policy/Dockerfile -t cl8y-dex-ci-policy:verify-1400 .
fi
printf 'Source suites, hosted red/green PR enforcement, independent release rebuild and scheduled LocalTerra evidence: see docs/qa/issue-1400/verification.md; this command alone does not attest them.\n'
printf 'Results: %s PASS, %s FAIL, %s SKIP\n' "$pass" "$fail" "$skip"
if (( fail > 0 )); then exit 1; fi
if (( skip > 0 )); then echo 'PASS (available checks; skips recorded)'; else echo 'PASS (implementation checks)'; fi
