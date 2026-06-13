#!/usr/bin/env bash
# Verify gitleaks custom rules catch fixtures and the clean tree passes (GitLab #380).
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
GITLEAKS_IMAGE="${GITLEAKS_IMAGE:-ghcr.io/gitleaks/gitleaks:v8.24.2}"
CONFIG="${ROOT}/.gitleaks.toml"
FIXTURE_CONFIG="${ROOT}/scripts/ci/gitleaks-fixture.toml"
FIXTURE="${ROOT}/scripts/ci/gitleaks-fixture"

run_gitleaks() {
  local config="$1"
  local source_dir="$2"
  shift 2
  if command -v gitleaks >/dev/null 2>&1; then
    gitleaks detect --source "$source_dir" --no-git --config "$config" --redact "$@"
  else
    docker run --rm -v "${ROOT}:${ROOT}" -w "$ROOT" --entrypoint gitleaks \
      "$GITLEAKS_IMAGE" detect --source "$source_dir" --no-git --config "$config" --redact "$@"
  fi
}

echo "== gitleaks: fixture must fail =="
set +e
run_gitleaks "$FIXTURE_CONFIG" "$FIXTURE" --verbose
fixture_rc=$?
set -e
if [[ "$fixture_rc" -eq 0 ]]; then
  echo "FAIL: gitleaks did not detect fixture secret in ${FIXTURE}"
  exit 1
fi
echo "OK: fixture rejected (exit ${fixture_rc})"

echo ""
echo "== gitleaks: clean working tree must pass =="
run_gitleaks "$CONFIG" "$ROOT"
echo "OK: clean tree passed"
