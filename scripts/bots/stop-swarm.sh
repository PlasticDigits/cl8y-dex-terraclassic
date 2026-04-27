#!/usr/bin/env bash
# Stop workers started by launch-swarm.sh (reads scripts/bots/run/pids.txt).
set -euo pipefail
REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
PIDFILE="$REPO_ROOT/scripts/bots/run/pids.txt"
if [[ ! -f "$PIDFILE" ]]; then
  echo "No pid file at $PIDFILE (nothing to stop)." >&2
  exit 0
fi
while read -r pid; do
  [[ -z "$pid" ]] && continue
  if kill -0 "$pid" 2>/dev/null; then
    kill "$pid" 2>/dev/null || true
    echo "stopped $pid"
  fi
done <"$PIDFILE"
rm -f "$PIDFILE"
echo "Swarm stop complete."
