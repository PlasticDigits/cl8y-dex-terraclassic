#!/usr/bin/env bash
# GitLab #584 — parse RefreshPairAssetCodeIdsBatch wasm has_more / next_start_after.
# Does not assume a single page (issue notes: optional refresh loop never parsed events).
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
# shellcheck source=../lib/upgrade-582-code-id-pin.sh
source "$REPO_ROOT/scripts/lib/upgrade-582-code-id-pin.sh"

die() { echo "FAIL: $*" >&2; exit 1; }

SDK53='{"events":[{"type":"wasm","attributes":[{"key":"action","value":"refresh_pair_asset_code_ids_batch"},{"key":"has_more","value":"true"},{"key":"next_start_after","value":"29"}]}]}'
LEGACY='{"logs":[{"events":[{"type":"wasm","attributes":[{"key":"has_more","value":"false"}]}]}]}'
MISSING='{"events":[{"type":"wasm","attributes":[{"key":"action","value":"refresh_pair_asset_code_ids_batch"}]}]}'

echo "== SDK 0.53 .events has_more=true next_start_after=29 =="
cur="$(upgrade582_refresh_batch_cursor "$SDK53")" || die "sdk53 parse failed"
[[ "$cur" == $'true\t29' ]] || die "sdk53 cursor=$cur"
echo "  ok"

echo "== legacy .logs has_more=false =="
cur="$(upgrade582_refresh_batch_cursor "$LEGACY")" || die "legacy parse failed"
[[ "$cur" == $'false\t' ]] || die "legacy cursor=$cur"
echo "  ok"

echo "== missing has_more fails closed =="
if upgrade582_refresh_batch_cursor "$MISSING" >/dev/null 2>&1; then
  die "missing has_more must fail"
fi
echo "  ok"

echo "OK refresh events"
