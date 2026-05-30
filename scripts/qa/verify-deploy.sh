#!/usr/bin/env bash
# Post-deploy QA check: deployed pair exposes expected schema and deploy stamp matches HEAD.
# GitLab #203 — catches stale on-chain contracts when LocalTerra volumes are reused.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

STAMP_FILE="${QA_DEPLOY_STAMP_FILE:-$REPO_ROOT/.qa-deploy-stamp}"

set -a
if [ -f "$REPO_ROOT/.env" ]; then
  # shellcheck source=/dev/null
  source "$REPO_ROOT/.env"
fi
# shellcheck source=/dev/null
source "$REPO_ROOT/scripts/qa/qa-host.env"
set +a

if [ "${QA_SHARED_HOST:-}" = "1" ]; then
  export COMPOSE_FILE="${COMPOSE_FILE:-docker-compose.yml:docker-compose.qa-shared-host.yml}"
fi

LCD="${TERRA_LCD_URL:-http://127.0.0.1:${DEX_TERRA_LCD_PORT:-1317}}"
LCD="${LCD%/}"

# shellcheck source=scripts/lib/lcd-smart-query.sh
source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"

_fail_common_header() {
  local reason="$1"
  echo "" >&2
  echo "ERROR: QA deploy verification failed — ${reason}" >&2
  echo "" >&2
}

_fail_docs_footer() {
  echo "Docs: scripts/qa/README.md § Stale deployed contracts; skills/AGENTS_QA_DEPLOY_VERIFY.md" >&2
  exit 1
}

# Stale on-chain wasm / reused volumes — needs fresh volumes + redeploy.
fail_stale_contract() {
  _fail_common_header "$1"
  echo "Likely cause: reused LocalTerra/Postgres Docker volumes with contracts from an older tree." >&2
  echo "Symptoms: LCD 'unknown variant' on pair queries (e.g. is_paused, expired_limit_refund)." >&2
  echo "" >&2
  echo "Fix (wipe chain + DB state, then redeploy):" >&2
  echo "  make reset-qa" >&2
  echo "  # or: QA_FRESH_VOLUMES=1 make start-qa" >&2
  echo "  # or manually:" >&2
  echo "  docker compose down -v" >&2
  echo "  docker volume rm cl8y-dex-terraclassic_localterra-data cl8y-dex-terraclassic_postgres-data" >&2
  echo "  make start-qa" >&2
  echo "" >&2
  _fail_docs_footer
}

# Deploy stamp out of sync with working tree — redeploy only (volumes may be fine).
fail_stamp_mismatch() {
  local stamp_sha="$1"
  local head_sha="$2"
  _fail_common_header "deploy stamp git_sha=${stamp_sha} but HEAD=${head_sha} — redeploy after git pull"
  echo "Likely cause: git pull (or branch switch) without re-running deploy-local." >&2
  echo "On-chain schema probes passed; only the deploy stamp is stale." >&2
  echo "" >&2
  echo "Fix (redeploy from current HEAD — no volume wipe required unless schema probes also fail):" >&2
  echo "  make deploy-local && make qa-verify-deploy" >&2
  echo "  # or full QA path:" >&2
  echo "  make start-qa" >&2
  echo "" >&2
  _fail_docs_footer
}

read_env_var() {
  local file="$1"
  local key="$2"
  sed -n "s/^${key}=//p" "$file" | head -1
}

FACTORY=""
PAIR=""
if [ -f "$REPO_ROOT/indexer/.env" ]; then
  FACTORY="$(read_env_var "$REPO_ROOT/indexer/.env" FACTORY_ADDRESS)"
fi
if [ -f "$STAMP_FILE" ]; then
  # shellcheck disable=SC1090
  source "$STAMP_FILE"
  PAIR="${pair_address:-}"
  STAMP_FACTORY="${factory_address:-}"
  STAMP_GIT_SHA="${git_sha:-}"
  if [ -n "$STAMP_FACTORY" ] && [ -z "$FACTORY" ]; then
    FACTORY="$STAMP_FACTORY"
  fi
fi
if [ -z "$FACTORY" ] && [ -f "$REPO_ROOT/frontend-dapp/.env.local" ]; then
  FACTORY="$(read_env_var "$REPO_ROOT/frontend-dapp/.env.local" VITE_FACTORY_ADDRESS)"
fi

if [ -z "$FACTORY" ]; then
  echo "ERROR: FACTORY_ADDRESS not found (run make deploy-local first)." >&2
  exit 1
fi

_RPC_URL="${TERRA_RPC_URL:-http://127.0.0.1:${DEX_TERRA_RPC_PORT:-26657}}"
if ! localterra_rpc_status_ok "$_RPC_URL"; then
  echo "ERROR: LocalTerra RPC not reachable at ${_RPC_URL} (host or docker exec); start compose first." >&2
  exit 1
fi

if [ -z "$PAIR" ]; then
  echo "[qa-verify-deploy] Resolving first dual-CW20 pair from factory ${FACTORY}..."
  Q_PAIRS="$(lcd_b64_query '{"pairs":{"start_after":null,"limit":60}}')"
  RAW_PAIRS="$(localterra_lcd_curl "$LCD" "/cosmwasm/wasm/v1/contract/${FACTORY}/smart/${Q_PAIRS}")" \
    || fail_stale_contract "factory pairs query failed (factory=${FACTORY})"
  PAIRS_DOC="$(lcd_decode_smart_data "$RAW_PAIRS")"
  while IFS= read -r row; do
    [ -n "$row" ] || continue
    local_pair="$(echo "$row" | jq -r '.contract_addr')"
    t0="$(echo "$row" | jq -r '.asset_infos[0].token.contract_addr // empty')"
    t1="$(echo "$row" | jq -r '.asset_infos[1].token.contract_addr // empty')"
    if [[ "$t0" =~ ^terra1 && "$t1" =~ ^terra1 ]]; then
      PAIR="$local_pair"
      break
    fi
  done < <(echo "$PAIRS_DOC" | jq -c '.pairs[]? // empty')
fi

if [ -z "$PAIR" ]; then
  fail_stale_contract "no dual-CW20 pair found for factory ${FACTORY}"
fi

echo "[qa-verify-deploy] Pair ${PAIR} (factory ${FACTORY})"

# ── Schema probes (GitLab #120 / #203) ───────────────────────────────────
echo "[qa-verify-deploy] Query is_paused..."
if ! lcd_smart_query_ok "$LCD" "$PAIR" '{"is_paused":{}}'; then
  fail_stale_contract "pair ${PAIR} rejected is_paused (missing query variant — stale wasm?)"
fi
PAUSED_RAW="$(lcd_smart_query_raw "$LCD" "$PAIR" '{"is_paused":{}}')"
PAUSED_DOC="$(lcd_decode_smart_data "$PAUSED_RAW")"
if ! echo "$PAUSED_DOC" | jq -e 'has("paused")' >/dev/null 2>&1; then
  fail_stale_contract "pair ${PAIR} is_paused response missing paused field"
fi
echo "  is_paused.paused=$(echo "$PAUSED_DOC" | jq -r '.paused')"

echo "[qa-verify-deploy] Query expired_limit_refund (order_id=1, expect null when empty)..."
if ! lcd_smart_query_ok "$LCD" "$PAIR" '{"expired_limit_refund":{"order_id":1}}'; then
  fail_stale_contract "pair ${PAIR} rejected expired_limit_refund (missing query variant — stale wasm?)"
fi
echo "  expired_limit_refund: OK (variant accepted)"

# ── Deploy stamp vs HEAD ─────────────────────────────────────────────────
HEAD_SHA="$(git -C "$REPO_ROOT" rev-parse --short HEAD 2>/dev/null || true)"
if [ -f "$STAMP_FILE" ] && [ -n "${STAMP_GIT_SHA:-}" ] && [ -n "$HEAD_SHA" ]; then
  if [ "$STAMP_GIT_SHA" != "$HEAD_SHA" ]; then
    fail_stamp_mismatch "$STAMP_GIT_SHA" "$HEAD_SHA"
  fi
  echo "[qa-verify-deploy] Deploy stamp matches HEAD (${HEAD_SHA})"
elif [ ! -f "$STAMP_FILE" ]; then
  echo "[qa-verify-deploy] WARN: no ${STAMP_FILE#"$REPO_ROOT"/} (schema checks passed; run deploy-local to write stamp)"
else
  echo "[qa-verify-deploy] Deploy stamp present (git_sha=${STAMP_GIT_SHA:-?})"
fi

echo "[qa-verify-deploy] OK — deployed pair schema matches current tree."
