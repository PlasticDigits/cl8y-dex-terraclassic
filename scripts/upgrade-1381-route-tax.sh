#!/usr/bin/env bash
# Default: read-only plan. --execute deploys; --verify probes; --finalize restores pauses.
set -euo pipefail
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
exec python3 "$SCRIPT_DIR/lib/route_tax_rollout_cli.py" "$@"
