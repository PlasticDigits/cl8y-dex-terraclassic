#!/usr/bin/env bash
# Refresh certificate authentication, then perform the one local-custody handoff.
set -euo pipefail
RELEASE_OPS_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
# Public runtime routing only; the prepared manifest keeps its original URLs.
export TERRAD_RUNTIME_READ_NODE="${TERRAD_RUNTIME_READ_NODE:-https://rpc.terra-classic.hexxagon.io:443}"
export TERRAD_RUNTIME_READ_LCD="${TERRAD_RUNTIME_READ_LCD:-https://terra-classic-lcd.publicnode.com}"
export TERRAD_RUNTIME_EXEC_NODE="${TERRAD_RUNTIME_EXEC_NODE:-https://rpc.terra-classic.hexxagon.io:443}"
source "$HOME/.config/openbao/env.sh"
bao login -no-print -method=cert -path=workstation-cert name=answorld-pop-os
if [[ $# -eq 0 ]]; then
  set -- --run
fi
exec bao-exec --only COOLIFY_WIRE_TOKEN,COOLIFY_URL,FORGEJO_PLASTICDIGTS_PAT -- \
  python3 "$RELEASE_OPS_ROOT/scripts/ops/2026-10-06/complete-custody.py" "$@"
