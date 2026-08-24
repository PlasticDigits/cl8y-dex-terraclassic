#!/usr/bin/env bash
# Launch swap workers (5 types × 5 replicas = 25), five limit-order makers, three
# provide_liquidity workers (#293), and one tax-aware worker (#621) unless
# SWARM_TAX_WORKERS=0. Gem swap/LP workers exclude the community-tax token.
# Runs bootstrap-swarm-liquidity once first unless BOTS_SKIP_BOOTSTRAP=1.
#
# Usage (from repo root):
#   ./scripts/bots/launch-swarm.sh
#   BOTS_MEAN_INTERVAL_SEC=60 ./scripts/bots/launch-swarm.sh
#   BOTS_DRY_RUN=1 ./scripts/bots/launch-swarm.sh   # log only
#   BOTS_SKIP_BOOTSTRAP=1 ./scripts/bots/launch-swarm.sh
#
# Stop: ./scripts/bots/stop-swarm.sh
# Requires: make start + make deploy-local, Python 3, docker.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
SWARM_PY="$REPO_ROOT/scripts/bots/swarm.py"
BOOTSTRAP_SH="$REPO_ROOT/scripts/bots/bootstrap-swarm-liquidity.sh"
PREFLIGHT_SH="$REPO_ROOT/scripts/bots/preflight-test1-uluna.sh"
RUNDIR="$REPO_ROOT/scripts/bots/run"
LOGDIR="$RUNDIR/logs"
PIDFILE="$RUNDIR/pids.txt"

mkdir -p "$LOGDIR"
: >"$PIDFILE"

BASE_MEAN="${BOTS_MEAN_INTERVAL_SEC:-45}"
LIMIT_MEAN="${BOTS_LIMIT_MEAN_INTERVAL_SEC:-120}"
LP_MEAN="${BOTS_LP_MEAN_INTERVAL_SEC:-90}"
TAX_MEAN="${BOTS_TAX_MEAN_INTERVAL_SEC:-40}"
DRY="${BOTS_DRY_RUN:-0}"
SKIP_BOOTSTRAP="${BOTS_SKIP_BOOTSTRAP:-0}"
TAX_WORKERS="${SWARM_TAX_WORKERS:-1}"

echo "Preflight: test1 gas balance…"
python3 "$SWARM_PY" --preflight-gas

SWAP_TYPES=(offer0 offer1 heavy light directed)

chmod +x "$PREFLIGHT_SH"
"$PREFLIGHT_SH"

if [[ "$SKIP_BOOTSTRAP" != "1" ]]; then
  echo "Running bootstrap-swarm-liquidity (set BOTS_SKIP_BOOTSTRAP=1 to skip)…"
  chmod +x "$BOOTSTRAP_SH"
  BOTS_DRY_RUN="$DRY" "$BOOTSTRAP_SH"
else
  echo "Skipping bootstrap-swarm-liquidity (BOTS_SKIP_BOOTSTRAP=1)."
fi

echo "Launching ${#SWAP_TYPES[@]} swap types × 5 replicas → $((${#SWAP_TYPES[@]} * 5)) processes"
echo "  plus 5 limit-order workers (mean ${LIMIT_MEAN}s)"
echo "  plus 3 provide_liquidity workers (mean ${LP_MEAN}s)"
if [[ "$TAX_WORKERS" != "0" && "$TAX_WORKERS" != "false" ]]; then
  echo "  plus 1 tax-aware worker (mean ${TAX_MEAN}s) — set SWARM_TAX_WORKERS=0 to skip"
else
  echo "  tax workers off (SWARM_TAX_WORKERS=${TAX_WORKERS}); gem workers still exclude the tax token"
fi
echo "  swap base mean interval: ${BASE_MEAN}s  dry_run: ${DRY}"
echo "  logs: $LOGDIR  pids: $PIDFILE"

# Start tax-0 first so leftover #625 soak sees extra-debit / hybrid skip
# before 33 gem workers share test1 and storm the account sequence.
if [[ "$TAX_WORKERS" != "0" && "$TAX_WORKERS" != "false" ]]; then
  log="$LOGDIR/tax-0.log"
  (
    cd "$REPO_ROOT"
    export BOTS_TAX_MEAN_INTERVAL_SEC="$TAX_MEAN"
    export BOTS_MEAN_INTERVAL_SEC="$TAX_MEAN"
    export BOTS_DRY_RUN="$DRY"
    exec python3 "$SWARM_PY" --worker tax 0
  ) >>"$log" 2>&1 &
  echo $! >>"$PIDFILE"
  echo "  started tax-0 pid=$! mean=${TAX_MEAN}s -> $log"
fi

for t in "${SWAP_TYPES[@]}"; do
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

for i in 0 1 2 3 4; do
  mean="$(python3 -c "print(round(float('${LIMIT_MEAN}') * (0.72 + int('${i}') * 0.07), 2))")"
  amt_mult="$(python3 -c "print(round(0.55 + int('${i}') * 0.1, 3))")"
  log="$LOGDIR/limit-${i}.log"
  (
    cd "$REPO_ROOT"
    export BOTS_LIMIT_MEAN_INTERVAL_SEC="$mean"
    export BOTS_MEAN_INTERVAL_SEC="$mean"
    export BOTS_WORKER_AMOUNT_MULT="$amt_mult"
    export BOTS_DRY_RUN="$DRY"
    exec python3 "$SWARM_PY" --worker limit "$i"
  ) >>"$log" 2>&1 &
  echo $! >>"$PIDFILE"
  echo "  started limit-${i} pid=$! mean=${mean}s amt_mult=${amt_mult} -> $log"
done

for i in 0 1 2; do
  mean="$(python3 -c "print(round(float('${LP_MEAN}') * (0.8 + int('${i}') * 0.12), 2))")"
  log="$LOGDIR/lp-${i}.log"
  (
    cd "$REPO_ROOT"
    export BOTS_LP_MEAN_INTERVAL_SEC="$mean"
    export BOTS_DRY_RUN="$DRY"
    exec python3 "$SWARM_PY" --worker lp "$i"
  ) >>"$log" 2>&1 &
  echo $! >>"$PIDFILE"
  echo "  started lp-${i} pid=$! mean=${mean}s -> $log"
done

echo "Done. $(wc -l <"$PIDFILE") PIDs recorded. Stop with: $REPO_ROOT/scripts/bots/stop-swarm.sh"
