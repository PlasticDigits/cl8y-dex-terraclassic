#!/usr/bin/env bash
# GitLab #379 — sweep max_maker_fills on route/solve/best (LocalTerra / deploy env).
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=scripts/lib/localterra-host-curl.sh
source "$ROOT/scripts/lib/localterra-host-curl.sh"

INDEXER_URL="${INDEXER_URL:-http://127.0.0.1:3001}"
ENV_LOCAL="$ROOT/frontend-dapp/.env.local"

if [[ ! -f "$ENV_LOCAL" ]]; then
  echo "Missing $ENV_LOCAL — run make setup-cloud-localterra or make deploy-local" >&2
  exit 1
fi

# shellcheck disable=SC1090
source "$ENV_LOCAL"

TOKEN_IN="${BENCH_TOKEN_IN:-${VITE_USTC_CW20:-}}"
TOKEN_OUT="${BENCH_TOKEN_OUT:-${VITE_LUNC_CW20:-}}"
AMOUNT_IN="${BENCH_AMOUNT_IN:-1000000}"

if [[ -z "$TOKEN_IN" || -z "$TOKEN_OUT" ]]; then
  echo "Set BENCH_TOKEN_IN / BENCH_TOKEN_OUT or deploy .env.local CW20 addresses" >&2
  exit 1
fi

SAMPLES="${BENCH_SAMPLES:-5}"
VALUES=(1 8 30 100 4294967295)

echo "Indexer: $INDEXER_URL"
echo "token_in=$TOKEN_IN token_out=$TOKEN_OUT amount_in=$AMOUNT_IN samples=$SAMPLES"
echo "max_maker_fills | p50_ms | http"
echo "----------------|--------|----"

for mmf in "${VALUES[@]}"; do
  times=()
  status=0
  for ((i = 0; i < SAMPLES; i++)); do
    url="${INDEXER_URL}/api/v1/route/solve/best?token_in=${TOKEN_IN}&token_out=${TOKEN_OUT}&amount_in=${AMOUNT_IN}&max_maker_fills=${mmf}"
    start=$(date +%s%3N)
    if resp=$(curl -sf -o /dev/null -w '%{http_code}' "$url" 2>/dev/null); then
      status=$resp
    else
      status="${resp:-000}"
    fi
    end=$(date +%s%3N)
    times+=($((end - start)))
  done
  IFS=$'\n' sorted=($(sort -n <<<"${times[*]}"))
  unset IFS
  mid=$((SAMPLES / 2))
  p50="${sorted[$mid]}"
  printf '%15s | %6s | %s\n' "$mmf" "$p50" "$status"
done
