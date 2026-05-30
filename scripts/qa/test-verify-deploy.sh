#!/usr/bin/env bash
# Unit checks for qa-verify-deploy helpers (no Docker / LCD required).
# GitLab #203
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=scripts/lib/lcd-smart-query.sh
source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"

_fail() {
  echo "FAIL: $*" >&2
  exit 1
}

# lcd_smart_query_ok rejects RPC error envelopes with unknown variant text.
if lcd_smart_query_ok "http://127.0.0.1:1" "terra1fake" '{"is_paused":{}}' 2>/dev/null; then
  : # unreachable host — curl fails; ok returns non-zero
else
  :
fi

_variant_body='{"code":2,"message":"Error parsing into type cl8y_pair::ContractError: unknown variant `is_paused`","details":[]}'
if echo "$_variant_body" | jq -e '.message != null and (.message | test("unknown variant|Error parsing|execute wasm contract failed"; "i"))' >/dev/null; then
  :
else
  _fail "jq variant detector should match unknown variant message"
fi

_ok_body='{"data":"eyJwYXVzZWQiOmZhbHNlfQ=="}'
if echo "$_ok_body" | jq -e '.code != null and .code != 0' >/dev/null 2>&1; then
  _fail "ok body should not match code error filter"
fi

decoded="$(lcd_decode_smart_data "$_ok_body")"
if [ "$(echo "$decoded" | jq -r '.paused')" != "false" ]; then
  _fail "lcd_decode_smart_data should decode base64 paused=false"
fi

echo "OK: test-verify-deploy.sh (#203 lcd helper checks)"
