#!/usr/bin/env bash
# Launch 5 separate processes per bot type (25 workers), each with slightly different
# Poisson mean (BOTS_MEAN_INTERVAL_SEC) and amount multiplier (BOTS_WORKER_AMOUNT_MULT).
#
# Usage (from repo root):
#   ./scripts/bots/launch-swarm.sh
#   BOTS_MEAN_INTERVAL_SEC=60 ./scripts/bots/launch-swarm.sh
#   BOTS_DRY_RUN=1 ./scripts/bots/launch-swarm.sh   # log only
#
# Stop: ./scripts/bots/stop-swarm.sh
# Requires: make start + make deploy-local, Python 3, docker.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SWARM_PY="$REPO_ROOT/scripts/bots/swarm.py"
RUNDIR="$REPO_ROOT/scripts/bots/run"
LOGDIR="$RUNDIR/logs"
PIDFILE="$RUNDIR/pids.txt"

mkdir -p "$LOGDIR"
: >"$PIDFILE"

BASE_MEAN="${BOTS_MEAN_INTERVAL_SEC:-45}"
DRY="${BOTS_DRY_RUN:-0}"

BOT_TYPES=(offer0 offer1 heavy light directed)

echo "Launching ${#BOT_TYPES[@]} types × 5 replicas → $((${#BOT_TYPES[@]} * 5)) processes"
echo "  base mean interval: ${BASE_MEAN}s  dry_run: ${DRY}"
echo "  logs: $LOGDIR  pids: $PIDFILE"

for t in "${BOT_TYPES[@]}"; do
  for i in 0 1 2 3 4; do
    # Slightly different Poisson mean per replica (same curve for every type).
    mean="$(python3 -c "print(round(float('${BASE_MEAN}') * (0.62 + int('${i}') * 0.09), 2))")"
    # Slightly different size bias per replica (heavy/light bot logic still applies in-process).
    amt_mult="$(python3 -c "print(round(0.55 + int('${i}') * 0.1, 3))")"
    log="$LOGDIR/${t}-${i}.log"
    (
      cd "$REPO_ROOT"
      export BOTS_MEAN_INTERVAL_SEC="$mean"
      export BOTS_WORKER_AMOUNT_MULT="$amt_mult"
      export BOTS_DRY_RUN="$DRY"
      exec python3 "$SWARM_PY" --worker "$t" "$i"
    ) >>"$log" 2>&1 &
    echo $! >>"$PIDFILE"
    echo "  started ${t}-${i} pid=$! mean=${mean}s amt_mult=${amt_mult} -> $log"
  done
done

echo "Done. $(wc -l <"$PIDFILE") PIDs recorded. Stop with: $REPO_ROOT/scripts/bots/stop-swarm.sh"
