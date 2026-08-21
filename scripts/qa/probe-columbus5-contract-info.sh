#!/usr/bin/env bash
# GitLab #584 — columbus-5 (or any target LCD) read-only ContractInfo probe.
# Wraps upgrade-582-code-id-pin.sh UPGRADE582_PROBE_ONLY=1.
#
# Usage:
#   ./scripts/qa/probe-columbus5-contract-info.sh
#   LCD_URL=https://terra-classic-lcd.publicnode.com \
#     UPGRADE582_FACTORY_ADDRESS=terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea \
#     ./scripts/qa/probe-columbus5-contract-info.sh
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

export UPGRADE582_PROBE_ONLY=1
exec "$REPO_ROOT/scripts/upgrade-582-code-id-pin.sh"
