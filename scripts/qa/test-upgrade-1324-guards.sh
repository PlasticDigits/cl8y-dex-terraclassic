#!/usr/bin/env bash
# Guards for scripts/upgrade-1324-pair-twap.sh — no chain, no keyring.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
ok()  { PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }

run_syntax() {
  chmod +x scripts/upgrade-1324-pair-twap.sh
  bash -n scripts/upgrade-1324-pair-twap.sh
  rg -q "PAIR_VERSION:-1.18.0" scripts/upgrade-1324-pair-twap.sh
  rg -q "wasm migrate \"\$pair\" \"\$PAIR_CODE\" '\\{\\}'" scripts/upgrade-1324-pair-twap.sh
  rg -q 'multisig-2of3-host-tx' scripts/upgrade-1324-pair-twap.sh
  rg -q 'cl8ydeploy' scripts/upgrade-1324-pair-twap.sh
  rg -q 'update_config:\{pair_code_id:\$id\}' scripts/upgrade-1324-pair-twap.sh
  rg -q 'observations\[\$obs_index\] changed' scripts/upgrade-1324-pair-twap.sh
  rg -q 'Factory wasm is not stored' scripts/upgrade-1324-pair-twap.sh
  rg -q 'upgrade582_cw2_version "\$pair"' scripts/upgrade-1324-pair-twap.sh
  rg -q 'factory pair_code_id' scripts/upgrade-1324-pair-twap.sh
  rg -q 'seconds_ago.*\[0,60\]' scripts/upgrade-1324-pair-twap.sh
  rg -q 'observations_stored' scripts/upgrade-1324-pair-twap.sh
  # Factory address must never be the migrate target.
  if rg -n 'wasm migrate "\$FACTORY"' scripts/upgrade-1324-pair-twap.sh; then
    echo "factory migrate must not be in the #1324 script" >&2
    return 1
  fi
}

run_stale_wasm_refused() {
  local tmp
  tmp="$(mktemp)"
  printf 'cl8y-dex-pair\x001.17.0' >"$tmp"
  set +e
  DRY_RUN=1 UPGRADE1324_PAIR_WASM="$tmp" ./scripts/upgrade-1324-pair-twap.sh >"$tmp.out" 2>"$tmp.err"
  local st=$?
  set -e
  [[ "$st" -ne 0 ]]
  rg -q '1.18.0' "$tmp.err"
  rm -f "$tmp" "$tmp.out" "$tmp.err"
}

run_dry() {
  local tmp
  tmp="$(mktemp)"
  printf 'cl8y-dex-pair1.18.0' >"$tmp"
  DRY_RUN=1 UPGRADE1324_PAIR_WASM="$tmp" ./scripts/upgrade-1324-pair-twap.sh | tee /tmp/upgrade1324-dry.log
  rg -q "cw2 1.18.0" /tmp/upgrade1324-dry.log
  rg -q "DRY_RUN skip: 2-of-3 wasm migrate" /tmp/upgrade1324-dry.log
  rg -q '^OK$' /tmp/upgrade1324-dry.log
  rm -f "$tmp"
}

run_syntax && ok "syntax + migrate '{}'" || bad "syntax + migrate '{}'"
run_stale_wasm_refused && ok "stale 1.17.0 wasm refused" || bad "stale 1.17.0 wasm refused"
run_dry && ok "DRY_RUN 1.18.0 wasm" || bad "DRY_RUN 1.18.0 wasm"

echo ""
echo "#1324 pair migrate script: $PASS passed, $FAIL failed"
[[ "$FAIL" -eq 0 ]]
