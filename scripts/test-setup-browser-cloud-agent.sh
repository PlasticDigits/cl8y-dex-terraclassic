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

echo "OK: setup-browser static checks"
