#!/usr/bin/env bash
# Unit checks for verify-deploy-config assertion helpers (no Docker / LCD required).
# GitLab #441
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

_fail() {
  echo "FAIL: $*" >&2
  exit 1
}

# Factory config fixture (base64 of {"governance":"terra1gov","treasury":"terra1treasury","default_fee_bps":180})
_factory_cfg_body='{"data":"eyJnb3Zlcm5hbmNlIjoidGVycmExZ292IiwidHJlYXN1cnkiOiJ0ZXJyYTF0cmVhc3VyeSIsImRlZmF1bHRfZmVlX2JwcyI6MTgwfQ=="}'
cfg="$(source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"; lcd_decode_smart_data "$_factory_cfg_body")"
[ "$(echo "$cfg" | jq -r '.governance')" = "terra1gov" ] || _fail "decode governance"
[ "$(echo "$cfg" | jq -r '.default_fee_bps')" = "180" ] || _fail "decode default_fee_bps"

# Whitelist fixture
_wl_body='{"data":{"code_ids":[42,43],"next":null}}'
wl="$(source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"; lcd_decode_smart_data "$_wl_body")"
[ "$(echo "$wl" | jq '[.code_ids[]] | length')" = "2" ] || _fail "whitelist count"

# Trusted router fixture
_tr_body='{"data":{"is_trusted":true}}'
tr="$(source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"; lcd_decode_smart_data "$_tr_body")"
[ "$(echo "$tr" | jq -r '.is_trusted')" = "true" ] || _fail "is_trusted"

# Blacklist clean wallet fixture — jq `false // true` is true (false is falsy); use type check
_bl_body='{"data":{"wallet_blacklisted":false,"blocked":false,"blacklisted_tokens":[],"pair_blacklisted":false}}'
bl="$(source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"; lcd_decode_smart_data "$_bl_body")"
wl_bl="$(echo "$bl" | jq -r 'if (.wallet_blacklisted | type) == "boolean" then (.wallet_blacklisted | tostring) else "true" end')"
[ "$wl_bl" = "false" ] || _fail "wallet_blacklisted jq parse (got $wl_bl)"
blk="$(echo "$bl" | jq -r 'if (.blocked | type) == "boolean" then (.blocked | tostring) else "true" end')"
[ "$blk" = "false" ] || _fail "blocked jq parse (got $blk)"

# is_trusted_router JSON uses addr (not router) per fee-discount QueryMsg
msg="$(jq -nc --arg addr 'terra1router' '{is_trusted_router:{addr:$addr}}')"
[ "$(echo "$msg" | jq -r '.is_trusted_router.addr')" = "terra1router" ] || _fail "trusted router msg shape"

echo "OK: test-verify-deploy-config.sh (#441 fixture checks)"
