#!/usr/bin/env bash
# Guards for scripts/upgrade-1246-alpha.sh — no chain.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
ok()  { PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }

run_syntax() {
  chmod +x scripts/upgrade-1246-alpha.sh
  bash -n scripts/upgrade-1246-alpha.sh
  rg -q 'UPGRADE1246_WHITELIST' scripts/upgrade-1246-alpha.sh
  rg -q '8654' scripts/upgrade-1246-alpha.sh
  rg -q 'WasmMsg::Migrate' scripts/upgrade-1246-alpha.sh
  rg -q 'register_listed_pair' scripts/upgrade-1246-alpha.sh
  rg -q 'refresh_pair_asset_code_ids' scripts/upgrade-1246-alpha.sh
  rg -q 'multisig-2of3-host-tx' scripts/upgrade-1246-alpha.sh
  rg -q 'terrad_host_ensure_keyring_pass' scripts/upgrade-1246-alpha.sh
  rg -q 'keep_listed' scripts/upgrade-1246-alpha.sh
}

run_dry() {
  DRY_RUN=1 UPGRADE1246_WHITELIST=1 UPGRADE1246_SKIP_LAUNCHER_CONFIG=1 \
    ./scripts/upgrade-1246-alpha.sh | tee /tmp/upgrade1246-dry.log
  rg -q 'AddWhitelistedCodeId' /tmp/upgrade1246-dry.log
  rg -q 'DRY_RUN skip: 2-of-3' /tmp/upgrade1246-dry.log
  rg -q 'CMM treasury ExecuteMsg has no WasmMsg::Migrate' /tmp/upgrade1246-dry.log
  rg -q 'OK token=' /tmp/upgrade1246-dry.log
}

run_syntax && ok "syntax + greps" || bad "syntax + greps"
run_dry && ok "DRY_RUN whitelist path" || bad "DRY_RUN whitelist path"

echo ""
echo "#1246 alpha script: $PASS passed, $FAIL failed"
[[ "$FAIL" -eq 0 ]]
