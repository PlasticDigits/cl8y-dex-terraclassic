#!/usr/bin/env bash
# Refresh certificate authentication, then perform the one local-custody handoff.
set -euo pipefail
RELEASE_OPS_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
source "$HOME/.config/openbao/env.sh"
bao login -no-print -method=cert -path=workstation-cert name=answorld-pop-os
if [[ $# -eq 0 ]]; then
  set -- --run
fi
exec bao-exec --only COOLIFY_WIRE_TOKEN,COOLIFY_URL,FORGEJO_PLASTICDIGTS_PAT -- \
  python3 "$RELEASE_OPS_ROOT/scripts/ops/2026-10-06/complete-custody.py" "$@"
