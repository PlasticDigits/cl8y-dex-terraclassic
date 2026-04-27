#!/usr/bin/env bash
# Delegates to the stdlib Python swarm (Poisson inter-arrival, 5 replicas × bot types).
# Prefer: `make swarm-local` from repo root.
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
exec python3 "$REPO_ROOT/scripts/bots/swarm.py" "$@"
