#!/usr/bin/env bash
# Banner when QA_FRESH_VOLUMES is set (LocalTerra + Postgres volumes will be removed).
set -euo pipefail

if [ -t 1 ] && [ -z "${NO_COLOR:-}" ]; then
  _B=$'\033[91;1m'
  _RST=$'\033[0m'
else
  _B=''
  _RST=''
fi

printf '%b\n' "${_B}┏━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┓${_RST}"
printf '%b\n' "${_B}┃  QA_FRESH_VOLUMES: wiping LocalTerra + Postgres Docker volumes            ┃${_RST}"
printf '%b\n' "${_B}┃  (localterra-data, postgres-data) — chain + DB state reset               ┃${_RST}"
printf '%b\n' "${_B}┗━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━┛${_RST}"
echo ""
