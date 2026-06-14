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
echo "== gitleaks: clean tracked tree must pass =="
"${ROOT}/scripts/ci/gitleaks-scan-tracked.sh"
echo "OK: clean tracked tree passed"

echo ""
echo "== gitleaks: force-tracked node_modules secret must fail =="
NM_REPO="$(mktemp -d)"
ROOT_NM_REPO="$(mktemp -d)"
cleanup_nm_repos() {
  rm -rf "$NM_REPO" "$ROOT_NM_REPO"
}
trap cleanup_nm_repos EXIT

cp "$CONFIG" "${NM_REPO}/.gitleaks.toml"
printf '%s\n' 'node_modules/' >"${NM_REPO}/.gitignore"
git -C "$NM_REPO" init -q
git -C "$NM_REPO" config user.email "gitleaks-test@example.com"
git -C "$NM_REPO" config user.name "gitleaks-test"
mkdir -p "${NM_REPO}/frontend-dapp/node_modules/evil"
printf '%s\n' "ghp_$(openssl rand -hex 20)" >"${NM_REPO}/frontend-dapp/node_modules/evil/leak.txt"
git -C "$NM_REPO" add .gitignore .gitleaks.toml
git -C "$NM_REPO" add -f frontend-dapp/node_modules/evil/leak.txt
git -C "$NM_REPO" commit -q -m "gitleaks node_modules bypass regression"

set +e
GITLEAKS_SCAN_ROOT="$NM_REPO" GITLEAKS_CONFIG="${NM_REPO}/.gitleaks.toml" \
  "${ROOT}/scripts/ci/gitleaks-scan-tracked.sh" --verbose
nm_rc=$?
set -e
if [[ "$nm_rc" -eq 0 ]]; then
  echo "FAIL: gitleaks did not detect force-tracked secret under node_modules/"
  exit 1
fi
echo "OK: force-tracked node_modules secret rejected (exit ${nm_rc})"

echo ""
echo "== gitleaks: force-tracked root node_modules secret must fail =="
cp "$CONFIG" "${ROOT_NM_REPO}/.gitleaks.toml"
printf '%s\n' 'node_modules/' >"${ROOT_NM_REPO}/.gitignore"
git -C "$ROOT_NM_REPO" init -q
git -C "$ROOT_NM_REPO" config user.email "gitleaks-test@example.com"
git -C "$ROOT_NM_REPO" config user.name "gitleaks-test"
mkdir -p "${ROOT_NM_REPO}/node_modules/evil"
printf '%s\n' "ghp_$(openssl rand -hex 20)" >"${ROOT_NM_REPO}/node_modules/evil/leak.txt"
git -C "$ROOT_NM_REPO" add .gitignore .gitleaks.toml
git -C "$ROOT_NM_REPO" add -f node_modules/evil/leak.txt
git -C "$ROOT_NM_REPO" commit -q -m "gitleaks root node_modules bypass regression"

set +e
GITLEAKS_SCAN_ROOT="$ROOT_NM_REPO" GITLEAKS_CONFIG="${ROOT_NM_REPO}/.gitleaks.toml" \
  "${ROOT}/scripts/ci/gitleaks-scan-tracked.sh" --verbose
root_nm_rc=$?
set -e
if [[ "$root_nm_rc" -eq 0 ]]; then
  echo "FAIL: gitleaks did not detect force-tracked secret under root node_modules/"
  exit 1
fi
echo "OK: force-tracked root node_modules secret rejected (exit ${root_nm_rc})"
