#!/usr/bin/env bash
# Reprint SSH tunnel + laptop workflow (same block as end of start-qa.sh).
# Run: ./scripts/qa/print-qa-tunnel-instructions.sh  or  make qa-tunnel-help
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

if [ -t 1 ] && [ -z "${NO_COLOR:-}" ]; then
  _B=$'\033[1m'
  _Y=$'\033[93m'
  _G=$'\033[92m'
  _W=$'\033[97m'
  _N=$'\033[0m'
  _ALERT=$'\033[1;93;41m'
  _SRV=$'\033[1;96m'
  _LAP=$'\033[1;95m'
else
  _B='' _Y='' _G='' _W='' _N='' _ALERT='' _SRV='' _LAP=''
fi

set -a
if [ -f "$REPO_ROOT/.env" ]; then
  # shellcheck source=/dev/null
  source "$REPO_ROOT/.env"
fi
# shellcheck source=/dev/null
source "$REPO_ROOT/scripts/qa/qa-host.env"
set +a

TERRA_RPC_PORT="${DEX_TERRA_RPC_PORT}"
TERRA_LCD_PORT="${DEX_TERRA_LCD_PORT}"
IDX_PORT="${API_PORT:-3001}"

if [ -n "${QA_SSH_HOST:-}" ]; then
  SSH_DEST="$(whoami)@${QA_SSH_HOST}"
else
  SSH_DEST="$(whoami)@$(hostname -f 2>/dev/null || hostname)"
fi
QA_SSH_PORT="${QA_SSH_PORT:-22}"
SSH_P_ARGS=""
SCP_P_ARGS=""
if [ "${QA_SSH_PORT}" != "22" ]; then
  SSH_P_ARGS="-p ${QA_SSH_PORT} "
  SCP_P_ARGS="-P ${QA_SSH_PORT} "
fi

printf '%b\n' "${_SRV}"
cat <<'EOF'
   __SERVER (QA host)__          ssh -L tunnels          __LAPTOP (your machine)__
          | )=============================================( |
          |'   copy-paste blocks below are NOT all on one host — read the tags   `|
EOF
printf '%b\n' "${_N}"

printf '%b\n' "${_SRV}  SERVER${_N} = this QA machine (where ${_G}make start-qa${_N} ran).  ${_LAP}LAPTOP${_N} = your local dev machine."
printf '%b\n' "${_Y}  Run Playwright/e2e on the ${_SRV}SERVER${_N} when they need the full stack on one host.${_N}"
printf '%b\n' "  Full doc: ${_G}scripts/qa/README.md${_N}"
echo ""
printf '%b\n' "${_W}  Optional in repo-root ${_G}.env${_N}: ${_G}QA_SSH_HOST${_N}, ${_G}QA_SSH_PORT${_N} (if not 22), ${_G}QA_SHARED_HOST${_N} (see README)."
printf '%b\n' "  SSH/scp user below is ${_G}$(whoami)${_N} (who ran start-qa on the server)."
echo ""

printf '%b\n' "${_SRV}${_B}  Step 1 — SERVER ONLY${_N}"
printf '%b\n' "${_SRV}         ${_G}make status${_N} — expect LocalTerra, Postgres, indexer healthy.${_N}"
printf '%b\n' "${_SRV}         Do not run Vite or the SSH tunnel ${_W}on the server${_N} for laptop QA.${_N}"
echo ""

printf '%b\n' "${_LAP}${_B}  Step 2 — LAPTOP ONLY${_N}"
printf '%b\n' "${_LAP}         SSH port forwards (keep this terminal open). Use 127.0.0.1 both sides.${_N}"
echo ""
printf '%b\n' "${_G}${_B}ssh -4 -N ${SSH_P_ARGS}\\${_N}"
printf '%b\n' "${_G}  -L 127.0.0.1:${TERRA_RPC_PORT}:127.0.0.1:${TERRA_RPC_PORT} \\${_N}"
printf '%b\n' "${_G}  -L 127.0.0.1:${TERRA_LCD_PORT}:127.0.0.1:${TERRA_LCD_PORT} \\${_N}"
printf '%b\n' "${_G}  -L 127.0.0.1:${IDX_PORT}:127.0.0.1:${IDX_PORT} \\${_N}"
printf '%b\n' "${_G}  ${SSH_DEST}${_N}"
echo ""

printf '%b\n' "${_LAP}${_B}  Step 3 — LAPTOP ONLY${_N}"
printf '%b\n' "${_LAP}         Copy ${_G}frontend-dapp/.env.local${_N} from the server into your clone.${_N}"
printf '%b\n' "    ${_G}scp ${SCP_P_ARGS}${SSH_DEST}:${REPO_ROOT}/frontend-dapp/.env.local frontend-dapp/.env.local${_N}"
printf '%b\n' "    Or after copy from another path: ${_G}./scripts/qa/write-frontend-env-local.sh /path/to/.env.local${_N}"
echo ""

printf '%b\n' "${_LAP}${_B}  Step 4 — LAPTOP ONLY${_N}"
printf '%b\n' "${_LAP}         Install deps and run Vite locally (${_Y}not${_N} tunneled)."
printf '%b\n' "    ${_G}cd frontend-dapp && npm ci && npm run dev${_N}"
printf '%b\n' "    Open the URL Vite prints (often ${_G}http://localhost:5173${_N})."
echo ""

printf '%b\n' "${_Y}${_B}"
cat <<'EOF'

      ___    ____  ____
     / _ \  |___ \ |___ \
    | |_| |   __) | __) |   SSH -L  = tunnel LocalTerra + indexer on laptop
    |  _  |  |__ < |__ <    Vite     = run locally (do NOT -L the dev server)
    |_| |_|  |___/ |___/
EOF
printf '%b\n' "${_N}"
printf '%b\n' "${_ALERT}  CHECK ABOVE STEPS. DO NOT TUNNEL VITE.${_N}"
