#!/usr/bin/env bash
# Verification for GitLab #584 — order-enforcing F6 upgrade script + freeze runbook.
#
# Refs: scripts/upgrade-582-code-id-pin.sh
#       docs/runbooks/cw20-code-id-ops.md
#       skills/AGENTS_CW20_CODE_ID_PIN.md
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS+1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL+1)); echo "  [FAIL] $1" >&2; }

run_step() {
  local label="$1"
  shift
  echo ""
  echo "[$label]"
  if "$@"; then
    ok "$label"
  else
    bad "$label"
  fi
}

echo "════════════════════════════════════════════════════════════════"
echo "  GitLab #584 — F6 upgrade-582 ops + ContractInfo probe + runbook"
echo "════════════════════════════════════════════════════════════════"

DUMMY_FACTORY="terra1ejpgvv7g3hj0u6fpcnxhflqp84g0w3cnaskqkg5733ygwlmf963sfchsea"

run_syntax() {
  set -euo pipefail
  test -x scripts/upgrade-582-code-id-pin.sh || chmod +x scripts/upgrade-582-code-id-pin.sh
  test -x scripts/qa/probe-columbus5-contract-info.sh || chmod +x scripts/qa/probe-columbus5-contract-info.sh
  test -x scripts/qa/test-upgrade-582-pagination.sh || chmod +x scripts/qa/test-upgrade-582-pagination.sh
  test -x scripts/qa/test-upgrade-582-refresh-events.sh || chmod +x scripts/qa/test-upgrade-582-refresh-events.sh
  test -x scripts/lib/upgrade-582-code-id-pin.sh || chmod +x scripts/lib/upgrade-582-code-id-pin.sh
  bash -n scripts/upgrade-582-code-id-pin.sh
  bash -n scripts/lib/upgrade-582-code-id-pin.sh
  bash -n scripts/qa/probe-columbus5-contract-info.sh
  bash -n scripts/qa/test-upgrade-582-pagination.sh
  bash -n scripts/qa/test-upgrade-582-refresh-events.sh
  rg -q 'set -euo pipefail' scripts/upgrade-582-code-id-pin.sh
  rg -q 'terrad-host.sh' scripts/upgrade-582-code-id-pin.sh
  rg -q 'lcd-smart-query.sh' scripts/upgrade-582-code-id-pin.sh
}

run_greps() {
  set -euo pipefail
  # Never the #514 silent-truncation pattern.
  if rg -n 'limit":60|"limit": 60|limit: 60' scripts/upgrade-582-code-id-pin.sh scripts/lib/upgrade-582-code-id-pin.sh; then
    echo "upgrade-582 must not use limit 60" >&2
    return 1
  fi
  rg -q 'get_pair_count' scripts/upgrade-582-code-id-pin.sh
  rg -q 'GetPairCount' scripts/upgrade-582-code-id-pin.sh scripts/lib/upgrade-582-code-id-pin.sh
  rg -q 'is_code_id_whitelisted' scripts/upgrade-582-code-id-pin.sh scripts/lib/upgrade-582-code-id-pin.sh
  rg -q 'get_asset_code_ids' scripts/upgrade-582-code-id-pin.sh
  rg -q 'GetAssetCodeIds' scripts/upgrade-582-code-id-pin.sh
  rg -q '/cosmwasm/wasm/v1/contract/' scripts/upgrade-582-code-id-pin.sh scripts/lib/upgrade-582-code-id-pin.sh
  rg -q 'PAIR_MIGRATE_BEGIN' scripts/upgrade-582-code-id-pin.sh
  rg -q 'UPDATE_CONFIG_BEGIN' scripts/upgrade-582-code-id-pin.sh
  rg -q 'update_config' scripts/upgrade-582-code-id-pin.sh
  rg -q 'pair_code_id' scripts/upgrade-582-code-id-pin.sh
  rg -q 'already code_id' scripts/upgrade-582-code-id-pin.sh
  rg -q 'upgrade582_refresh_batch_cursor' scripts/upgrade-582-code-id-pin.sh
  rg -q 'upgrade582_whitelist_bool' scripts/upgrade-582-code-id-pin.sh
  rg -q 'LCD flake' scripts/upgrade-582-code-id-pin.sh
  rg -q 'start_after' scripts/lib/upgrade-582-code-id-pin.sh
  rg -q 'ungated' scripts/upgrade-582-code-id-pin.sh
  # SKIP_PAIR_MIGRATE cannot skip smoke.
  rg -q 'smoke will still hard-fail' scripts/upgrade-582-code-id-pin.sh
  rg -q 'VITE_FACTORY_ADDRESS|VITE_LCD_URL' scripts/upgrade-582-code-id-pin.sh
}

run_docs() {
  set -euo pipefail
  rg -q 'SetPairPaused' docs/runbooks/cw20-code-id-ops.md
  rg -q 'RefreshPairAssetCodeIds' docs/runbooks/cw20-code-id-ops.md
  rg -q 'start_after' docs/runbooks/cw20-code-id-ops.md
  rg -q 'unfreeze' docs/runbooks/cw20-code-id-ops.md
  rg -q '10184' docs/runbooks/cw20-code-id-ops.md
  rg -q 'GetAssetCodeIds' docs/runbooks/cw20-code-id-ops.md
  rg -q 'ContractInfo' docs/runbooks/cw20-code-id-ops.md
  rg -q 'upgrade-582-code-id-pin' docs/runbooks/cw20-code-id-ops.md
  rg -q 'upgrade-582-code-id-pin' skills/AGENTS_CW20_CODE_ID_PIN.md
  rg -q 'upgrade-582-code-id-pin' AGENTS.md
  rg -q 'upgrade-582-code-id-pin' docs/testing.md
  rg -q 'upgrade-582-code-id-pin' docs/runbooks/launch-checklist.md
  rg -q 'BLOCK' docs/runbooks/launch-checklist.md
  rg -q 'upgrade-582-code-id-pin' docs/runbooks/wasm-admin-migration.md
  rg -q 'SetPairPaused' docs/runbooks/emergency-commands.md
  rg -q 'upgrade-582-code-id-pin' docs/runbooks/emergency-commands.md
  rg -q 'ContractInfo' docs/templates/deploy-trace.md
  rg -q 'GetPairCount' docs/templates/deploy-trace.md
  rg -q 'keep' docs/runbooks/cw20-code-id-ops.md
  rg -q 'maximal freeze' docs/runbooks/cw20-code-id-ops.md docs/runbooks/cw20-whitelist-policy.md
  rg -q 'pair_code_id' docs/runbooks/cw20-code-id-ops.md
  rg -q 'LCD flake' docs/runbooks/cw20-code-id-ops.md skills/AGENTS_CW20_CODE_ID_PIN.md
  rg -q '584' docs/testing.md
  rg -q 'verify-issue-584' Makefile
  rg -q 'pair_code_id' docs/runbooks/cw20-code-id-ops.md
  rg -q 'LCD flake' skills/AGENTS_CW20_CODE_ID_PIN.md docs/runbooks/cw20-code-id-ops.md
}

run_dry_run_ok() {
  set -euo pipefail
  DRY_RUN=1 UPGRADE582_SKIP_STORE=1 UPGRADE582_SKIP_CONTRACT_INFO_PROBE=1 \
    UPGRADE582_FACTORY_CODE_ID=10 UPGRADE582_PAIR_CODE_ID=11 \
    UPGRADE582_FORCE_FACTORY_VERSION=1.9.0 \
    UPGRADE582_FACTORY_ADDRESS="$DUMMY_FACTORY" \
    ./scripts/upgrade-582-code-id-pin.sh | tee /tmp/upgrade582-dry-ok.log
  rg -q 'factory 1.9.0 then pairs 1.15.0' /tmp/upgrade582-dry-ok.log
  rg -q 'PAIR_MIGRATE_BEGIN' /tmp/upgrade582-dry-ok.log
  rg -q 'IsCodeIdWhitelisted' /tmp/upgrade582-dry-ok.log
  rg -q 'UPDATE_CONFIG_BEGIN' /tmp/upgrade582-dry-ok.log
}

run_dry_run_refuse_old_factory() {
  set -euo pipefail
  set +e
  DRY_RUN=1 UPGRADE582_SKIP_STORE=1 UPGRADE582_SKIP_CONTRACT_INFO_PROBE=1 \
    UPGRADE582_FACTORY_CODE_ID=10 UPGRADE582_PAIR_CODE_ID=11 \
    UPGRADE582_FORCE_FACTORY_VERSION=1.8.0 \
    UPGRADE582_FACTORY_ADDRESS="$DUMMY_FACTORY" \
    ./scripts/upgrade-582-code-id-pin.sh > /tmp/upgrade582-dry-old.log 2>&1
  rc=$?
  set -e
  [[ "$rc" -ne 0 ]] || {
    echo "expected non-zero when factory cw2 < 1.9.0" >&2
    cat /tmp/upgrade582-dry-old.log >&2
    return 1
  }
  if rg -q 'PAIR_MIGRATE_BEGIN' /tmp/upgrade582-dry-old.log; then
    echo "pair migrate logged before factory version assert failed" >&2
    cat /tmp/upgrade582-dry-old.log >&2
    return 1
  fi
  rg -q 'refusing pair' /tmp/upgrade582-dry-old.log
}

run_probe_fail() {
  set -euo pipefail
  set +e
  DRY_RUN=1 UPGRADE582_SKIP_STORE=1 UPGRADE582_PROBE_ONLY=1 \
    UPGRADE582_SKIP_CONTRACT_INFO_PROBE=0 \
    LCD_URL="http://127.0.0.1:9" \
    UPGRADE582_FACTORY_ADDRESS="$DUMMY_FACTORY" \
    LOCALTERRA_CURL_CONNECT_TIMEOUT=1 LOCALTERRA_CURL_MAX_TIME=2 \
    ./scripts/upgrade-582-code-id-pin.sh > /tmp/upgrade582-probe-fail.log 2>&1
  rc=$?
  set -e
  [[ "$rc" -ne 0 ]] || {
    echo "expected probe fail against dead LCD" >&2
    cat /tmp/upgrade582-probe-fail.log >&2
    return 1
  }
  rg -qi 'ContractInfo' /tmp/upgrade582-probe-fail.log
}

run_pagination() {
  bash scripts/qa/test-upgrade-582-pagination.sh
}

run_refresh_events() {
  bash scripts/qa/test-upgrade-582-refresh-events.sh
}

run_dry_run_refuse_whitelist() {
  set -euo pipefail
  set +e
  DRY_RUN=1 UPGRADE582_SKIP_STORE=1 UPGRADE582_SKIP_CONTRACT_INFO_PROBE=1 \
    UPGRADE582_FACTORY_CODE_ID=10 UPGRADE582_PAIR_CODE_ID=11 \
    UPGRADE582_FORCE_FACTORY_VERSION=1.9.0 \
    UPGRADE582_FORCE_WHITELIST_JSON='{}' \
    UPGRADE582_FACTORY_ADDRESS="$DUMMY_FACTORY" \
    ./scripts/upgrade-582-code-id-pin.sh > /tmp/upgrade582-dry-wl.log 2>&1
  rc=$?
  set -e
  [[ "$rc" -ne 0 ]] || {
    echo "expected non-zero when IsCodeIdWhitelisted is unparseable" >&2
    cat /tmp/upgrade582-dry-wl.log >&2
    return 1
  }
  if rg -q 'PAIR_MIGRATE_BEGIN' /tmp/upgrade582-dry-wl.log; then
    echo "pair migrate logged before whitelist assert failed" >&2
    cat /tmp/upgrade582-dry-wl.log >&2
    return 1
  fi
  rg -q 'IsCodeIdWhitelisted' /tmp/upgrade582-dry-wl.log
}

run_dry_run_refresh_has_more() {
  set -euo pipefail
  DRY_RUN=1 UPGRADE582_SKIP_STORE=1 UPGRADE582_SKIP_CONTRACT_INFO_PROBE=1 \
    UPGRADE582_FACTORY_CODE_ID=10 UPGRADE582_PAIR_CODE_ID=11 \
    UPGRADE582_FORCE_FACTORY_VERSION=1.9.0 \
    UPGRADE582_REFRESH=1 \
    UPGRADE582_FACTORY_ADDRESS="$DUMMY_FACTORY" \
    ./scripts/upgrade-582-code-id-pin.sh | tee /tmp/upgrade582-dry-refresh.log
  rg -q 'has_more=false' /tmp/upgrade582-dry-refresh.log
  if rg -q 'DRY_RUN refresh skipped' /tmp/upgrade582-dry-refresh.log; then
    echo "refresh loop must parse has_more, not skip" >&2
    return 1
  fi
}

run_skip_store_requires_ids() {
  set -euo pipefail
  set +e
  DRY_RUN=1 UPGRADE582_SKIP_STORE=1 UPGRADE582_SKIP_CONTRACT_INFO_PROBE=1 \
    UPGRADE582_FORCE_FACTORY_VERSION=1.9.0 \
    UPGRADE582_FACTORY_ADDRESS="$DUMMY_FACTORY" \
    ./scripts/upgrade-582-code-id-pin.sh > /tmp/upgrade582-skip-store.log 2>&1
  rc=$?
  set -e
  [[ "$rc" -ne 0 ]] || {
    echo "SKIP_STORE without code ids must fail" >&2
    return 1
  }
}

echo ""
echo "── first pass ──"
run_step "syntax: bash -n + helpers executable" run_syntax
run_step "greps: pagination, GetPairCount, whitelist query, ContractInfo, no limit 60" run_greps
run_step "docs: runbook + skill + launch BLOCK + deploy-trace" run_docs
run_step "pagination mock: 31 and 61 pairs (two pages + short third)" run_pagination
run_step "refresh events: has_more + next_start_after (sdk53 + legacy)" run_refresh_events
run_step "DRY_RUN happy: factory 1.9.0 then PAIR_MIGRATE_BEGIN" run_dry_run_ok
run_step "DRY_RUN refuse: factory 1.8.0 exits before PAIR_MIGRATE_BEGIN" run_dry_run_refuse_old_factory
run_step "DRY_RUN refuse: unparseable IsCodeIdWhitelisted before PAIR_MIGRATE_BEGIN" run_dry_run_refuse_whitelist
run_step "DRY_RUN refresh: parses wasm has_more=false" run_dry_run_refresh_has_more
run_step "ContractInfo probe fail-closed on dead LCD" run_probe_fail
run_step "SKIP_STORE still requires code ids" run_skip_store_requires_ids

echo ""
echo "── retest ──"
run_step "retest syntax" run_syntax
run_step "retest DRY_RUN happy" run_dry_run_ok
run_step "retest DRY_RUN refuse factory < 1.9.0" run_dry_run_refuse_old_factory
run_step "retest DRY_RUN refuse unparseable whitelist" run_dry_run_refuse_whitelist
run_step "retest pagination mock" run_pagination
run_step "retest refresh events" run_refresh_events

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  #584 results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
echo "OK"
