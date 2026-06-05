#!/usr/bin/env bash
# Provision LocalTerra + deploy + frontend/indexer env for Cursor Cloud Agent VMs.
#
# Idempotent: skips build/deploy when artifacts and .env.local already match the stamp.
# Writes frontend-dapp/.env.local and frontend-dapp/.env.development (via deploy-dex-local.sh).
#
# Usage (repo root):
#   ./scripts/setup-cloud-agent-localterra.sh              # full: infra + build + deploy
#   ./scripts/setup-cloud-agent-localterra.sh --skip-build # infra + deploy only (artifacts exist)
#   ./scripts/setup-cloud-agent-localterra.sh --infra-only # docker up + wait-healthy only
#   ./scripts/setup-cloud-agent-localterra.sh --postgres-only # Postgres + indexer/.env only (#335)
#
# After success:
#   tmux attach -t indexer-dev    # if started with --start-indexer (default)
#   make dev                    # Vite on http://127.0.0.1:5173
#
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

SKIP_BUILD=0
INFRA_ONLY=0
POSTGRES_ONLY=0
FRESH_VOLUMES=0
START_INDEXER=1
START_FRONTEND=0

while [[ $# -gt 0 ]]; do
  case "$1" in
    --skip-build) SKIP_BUILD=1; shift ;;
    --infra-only) INFRA_ONLY=1; shift ;;
    --postgres-only) POSTGRES_ONLY=1; shift ;;
    --fresh) FRESH_VOLUMES=1; shift ;;
    --no-indexer) START_INDEXER=0; shift ;;
    --start-frontend) START_FRONTEND=1; shift ;;
    -h | --help)
      sed -n '2,20p' "$0"
      exit 0
      ;;
    *)
      echo "[setup-cloud-localterra] unknown arg: $1" >&2
      exit 1
      ;;
  esac
done

ENV_LOCAL="$REPO_ROOT/frontend-dapp/.env.local"
STAMP="$REPO_ROOT/.qa-deploy-stamp"
TMUX_CONF="${TMUX_CONF:-/exec-daemon/tmux.portal.conf}"
tmux_cmd() {
  if [[ -f "$TMUX_CONF" ]]; then
    tmux -f "$TMUX_CONF" "$@"
  else
    tmux "$@"
  fi
}

_run_make() {
  if groups 2>/dev/null | grep -qw docker; then
    make "$@"
  else
    sg docker -c "make $*"
  fi
}

_ensure_dockerd() {
  if sg docker -c 'docker info' >/dev/null 2>&1; then
    return 0
  fi
  echo "[setup-cloud-localterra] starting dockerd in tmux session 'dockerd'…"
  if ! tmux_cmd has-session -t dockerd 2>/dev/null; then
    tmux_cmd new-session -d -s dockerd "sudo dockerd > /tmp/dockerd.log 2>&1"
  fi
  for _ in $(seq 1 60); do
    if sg docker -c 'docker info' >/dev/null 2>&1; then
      echo "[setup-cloud-localterra] dockerd ready"
      return 0
    fi
    sleep 2
  done
  echo "[setup-cloud-localterra] ERROR: dockerd did not become ready. See /tmp/dockerd.log" >&2
  exit 1
}

_ensure_node() {
  export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
  if [[ -s "$NVM_DIR/nvm.sh" ]]; then
    # shellcheck source=/dev/null
    . "$NVM_DIR/nvm.sh"
    nvm install >/dev/null 2>&1 || true
    nvm use --silent
  fi
  local want got
  want="$(tr -d '[:space:]' <"$REPO_ROOT/.nvmrc")"
  got="$(node -v 2>/dev/null | sed 's/^v//' || true)"
  if [[ "$got" != "$want" ]]; then
    echo "[setup-cloud-localterra] WARNING: node v${got:-?} on PATH; want v${want} from .nvmrc" >&2
    echo "[setup-cloud-localterra]          prepend: export PATH=\"\$HOME/.nvm/versions/node/v${want}/bin:\$PATH\"" >&2
  fi
}

_ensure_rust() {
  if ! command -v cargo >/dev/null 2>&1; then
    echo "[setup-cloud-localterra] ERROR: cargo not found. Install rustup stable (1.96+) and libssl-dev." >&2
    exit 1
  fi
}

# shellcheck source=scripts/lib/deploy-up-to-date.sh
source "$REPO_ROOT/scripts/lib/deploy-up-to-date.sh"

_start_indexer_tmux() {
  local session=indexer-dev restart="${1:-0}"
  if tmux_cmd has-session -t "$session" 2>/dev/null; then
    if [[ "$restart" -eq 1 ]]; then
      echo "[setup-cloud-localterra] restarting tmux session '$session' after deploy…"
      tmux_cmd kill-session -t "$session"
    else
      echo "[setup-cloud-localterra] tmux session '$session' already exists (restart: tmux kill-session -t $session)"
      return 0
    fi
  fi
  tmux_cmd new-session -d -s "$session" -c "$REPO_ROOT/indexer" \
    "export PATH=\"/usr/local/cargo/bin:\$HOME/.cargo/bin:\$PATH\"; cargo run --release; exec bash -l"
  echo "[setup-cloud-localterra] indexer starting in tmux '$session' (port 3001)"
}

_start_frontend_tmux() {
  local session=frontend-dev
  if tmux_cmd has-session -t "$session" 2>/dev/null; then
    echo "[setup-cloud-localterra] tmux session '$session' already exists"
    return 0
  fi
  tmux_cmd new-session -d -s "$session" -c "$REPO_ROOT" \
    "export PATH=\"\$HOME/.nvm/versions/node/$(tr -d '[:space:]' <.nvmrc)/bin:\$PATH\"; ./scripts/dev-frontend-local.sh; exec bash -l"
  echo "[setup-cloud-localterra] frontend starting in tmux '$session' (http://127.0.0.1:5173)"
}

_wait_indexer() {
  for _ in $(seq 1 90); do
    if curl -sf "http://127.0.0.1:3001/health" >/dev/null 2>&1 \
      || curl -sf "http://127.0.0.1:3001/" >/dev/null 2>&1; then
      echo "[setup-cloud-localterra] indexer HTTP ready"
      return 0
    fi
    sleep 2
  done
  echo "[setup-cloud-localterra] WARNING: indexer not responding on :3001 yet (check: tmux attach -t indexer-dev)" >&2
}

echo "[setup-cloud-localterra] repo: $REPO_ROOT"

if [[ "$POSTGRES_ONLY" -eq 1 ]]; then
  if [[ "$FRESH_VOLUMES" -eq 1 ]]; then
    _ensure_dockerd
    echo "[setup-cloud-localterra] --fresh: wiping docker volumes…"
    _run_make reset
  fi
  exec "$REPO_ROOT/scripts/setup-cloud-agent-indexer-postgres.sh"
fi

_ensure_dockerd
_ensure_node

if [[ "$FRESH_VOLUMES" -eq 1 ]]; then
  echo "[setup-cloud-localterra] --fresh: wiping docker volumes…"
  _run_make reset
fi

echo "[setup-cloud-localterra] starting infra (localterra + postgres)…"
_run_make start
_run_make wait-healthy

if [[ "$INFRA_ONLY" -eq 1 ]]; then
  echo "[setup-cloud-localterra] --infra-only: done."
  exit 0
fi

DEPLOY_RAN=0
if [[ "$FRESH_VOLUMES" -eq 0 ]] && deploy_up_to_date "$REPO_ROOT"; then
  echo "[setup-cloud-localterra] deploy stamp matches HEAD and artifacts present — skipping build/deploy"
else
  if [[ "$SKIP_BUILD" -eq 0 ]]; then
    echo "[setup-cloud-localterra] building optimized wasm (Docker; first run ~10–15 min)…"
    _run_make build-optimized
  else
    echo "[setup-cloud-localterra] --skip-build: skipping make build-optimized"
  fi

  echo "[setup-cloud-localterra] deploying contracts + writing .env.local / indexer/.env…"
  if groups 2>/dev/null | grep -qw docker; then
    bash scripts/deploy-dex-local.sh
  else
    sg docker -c "bash scripts/deploy-dex-local.sh"
  fi
  DEPLOY_RAN=1
fi

[[ -f "$ENV_LOCAL" ]] || {
  echo "[setup-cloud-localterra] ERROR: $ENV_LOCAL missing after deploy" >&2
  exit 1
}

echo "[setup-cloud-localterra] OK"
echo "  frontend-dapp/.env.local       ($(grep -c '^VITE_' "$ENV_LOCAL" || true) VITE_* keys)"
echo "  frontend-dapp/.env.development (Simulated Wallet mnemonic)"
echo "  indexer/.env"
if [[ -f "$STAMP" ]]; then
  echo "  deploy stamp: $(cat "$STAMP")"
fi

if [[ "$START_INDEXER" -eq 1 ]]; then
  _ensure_rust
  _start_indexer_tmux "$DEPLOY_RAN"
  _wait_indexer
fi

if [[ "$START_FRONTEND" -eq 1 ]]; then
  _start_frontend_tmux
  for _ in $(seq 1 60); do
    if curl -sf "http://127.0.0.1:5173/" >/dev/null 2>&1; then
      echo "[setup-cloud-localterra] frontend ready: http://127.0.0.1:5173"
      break
    fi
    sleep 2
  done
fi

echo ""
echo "Next:"
echo "  make dev"
echo "  google-chrome --no-sandbox --disable-dev-shm-usage --disable-gpu http://127.0.0.1:5173/limits"
