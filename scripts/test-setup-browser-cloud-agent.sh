#!/usr/bin/env bash
# Static checks for setup-browser-cloud-agent.sh (no live Chrome required).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

_fail() {
  echo "FAIL: $*" >&2
  exit 1
}

_assert_file_contains() {
  local file=$1 pattern=$2 msg=$3
  grep -q "$pattern" "$file" || _fail "$msg"
}

_assert_file_contains "$REPO_ROOT/scripts/setup-browser-cloud-agent.sh" \
  'keplr-chrome-extension.sh' \
  'setup-browser must source keplr-chrome-extension.sh'

_assert_file_contains "$REPO_ROOT/scripts/lib/keplr-chrome-extension.sh" \
  'dmkamcknogkgcdfhhbddcghachkejeap' \
  'keplr helper must pin Keplr extension id'

_assert_file_contains "$REPO_ROOT/scripts/lib/keplr-chrome-extension.sh" \
  'keplr_install_into_chrome_profile' \
  'keplr helper must define install function'

_assert_file_contains "$REPO_ROOT/AGENTS.md" \
  'setup-browser-cloud-agent.sh' \
  'AGENTS.md must document setup-browser-cloud-agent.sh'

# shellcheck source=scripts/lib/keplr-chrome-extension.sh
source "${REPO_ROOT}/scripts/lib/keplr-chrome-extension.sh"

_tmp="$(mktemp -d)"
trap 'rm -rf "$_tmp"' EXIT

# manifest-only dirs must not count as ready (Bugbot: partial cache readiness).
mkdir -p "${_tmp}/manifest-only"
printf '{"name":"Keplr","version":"0.0.0"}' >"${_tmp}/manifest-only/manifest.json"
if keplr_extension_dir_ready "${_tmp}/manifest-only"; then
  _fail 'manifest-only dir must not pass keplr_extension_dir_ready'
fi

# minimal valid unpacked tree (manifest + two payload files).
mkdir -p "${_tmp}/valid/background"
printf '{"name":"Keplr","version":"0.0.0"}' >"${_tmp}/valid/manifest.json"
echo 'console.log("bg");' >"${_tmp}/valid/background/index.js"
echo 'icon' >"${_tmp}/valid/icon.png"
if ! keplr_extension_dir_ready "${_tmp}/valid"; then
  _fail 'valid unpacked dir must pass keplr_extension_dir_ready'
fi

echo "OK: setup-browser static checks"
