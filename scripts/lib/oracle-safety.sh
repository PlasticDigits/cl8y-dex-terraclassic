#!/usr/bin/env bash
# #1391 financial callers require fresh independent CEX quorum before preparation.
ORACLE_SAFETY_PY="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/oracle_safety.py"

oracle_safety_fetch() {
  local url="$1" ticker="$2" max_age="${3:-300}" raw
  raw="$(curl -fsS --connect-timeout 10 --max-time 20 "$url")" || return 1
  python3 "$ORACLE_SAFETY_PY" quote "$ticker" "$max_age" <<<"$raw"
}

oracle_safety_override() {
  if [[ "${DRY_RUN:-0}" != 1 ]]; then
    echo 'oracle safety: manual price overrides are dry-run only' >&2
    return 1
  fi
  python3 "$ORACLE_SAFETY_PY" override "$1"
}

oracle_safety_movement() {
  python3 "$ORACLE_SAFETY_PY" movement "$1" "$2"
}
