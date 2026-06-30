#!/usr/bin/env bash
# Unit checks for verify-env-addresses LCD response parsing (no Docker / LCD required).
# GitLab #442
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

_fail() {
  echo "FAIL: $*" >&2
  exit 1
}

# Router config fixture — factory must match env FACTORY_ADDRESS
_router_body='{"data":"eyJmYWN0b3J5IjoidGVycmExZmFjdG9yeSIsIndyYXBfbWFwcGVyIjpudWxsfQ=="}'
router_cfg="$(source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"; lcd_decode_smart_data "$_router_body")"
[ "$(echo "$router_cfg" | jq -r '.factory')" = "terra1factory" ] || _fail "router factory decode"

# Fee-discount config fixture
_fd_body='{"data":"eyJnb3Zlcm5hbmNlIjoidGVycmExZmRnb3YiLCJjbDh5X3Rva2VuIjoidGVycmExY2w4eSJ9"}'
fd_cfg="$(source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"; lcd_decode_smart_data "$_fd_body")"
[ "$(echo "$fd_cfg" | jq -r '.governance')" = "terra1fdgov" ] || _fail "fee-discount governance decode"

# Factory config fixture
_factory_body='{"data":"eyJnb3Zlcm5hbmNlIjoidGVycmExZ292IiwidHJlYXN1cnkiOiJ0ZXJyYTF0cmVhc3VyeSIsImRlZmF1bHRfZmVlX2JwcyI6MTgwfQ=="}'
factory_cfg="$(source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"; lcd_decode_smart_data "$_factory_body")"
[ "$(echo "$factory_cfg" | jq -r '.governance')" = "terra1gov" ] || _fail "factory governance decode"

# Env read helper shape (sed pattern used by verify-env-addresses.sh)
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT
cat > "$tmpdir/.env" <<'EOF'
FACTORY_ADDRESS=terra1idxfactory
ROUTER_ADDRESS=terra1idxrouter
FEE_DISCOUNT_ADDRESS=terra1idxfd
EOF
read_env_var() {
  local file="$1"
  local key="$2"
  sed -n "s/^${key}=//p" "$file" 2>/dev/null | head -1
}
[ "$(read_env_var "$tmpdir/.env" FACTORY_ADDRESS)" = "terra1idxfactory" ] || _fail "read_env_var FACTORY"

echo "OK: test-verify-env-addresses.sh (#442 fixture checks)"
