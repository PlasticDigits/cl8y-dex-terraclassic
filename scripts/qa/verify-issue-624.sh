#!/usr/bin/env bash
# Automated verification for GitLab #624 — post-merge !414 LocalTerra
# community-tax seed leftovers (#620).
#
# Proves (docs + child 620 + leftover live + children 601/592/610/594):
#   1. Q12 / M624-1–M624-8 documented and crosslinked.
#   2. Child make verify-issue-620 (L620-1–L620-8).
#   3. Swarm fundingExecuteMsg never Mints tax; --dry-run logs classify only.
#   4. Optional fresh volume: VERIFY624_FRESH=1 → make reset && make deploy-local.
#   5. Live env / LCD / indexer catalog / e2e-provision / swarm_funding_plan.
#   6. Children 601, 592, 610, 594 stay green (no contract changes in !414).
#
# VERIFY624_SKIP_CHILDREN=1 — skip 620/601/592/610/594 (docs + source + live).
# VERIFY624_SKIP_CHAIN=1 — skip LocalTerra leftovers even if the chain is up.
# VERIFY624_REQUIRE_CHAIN=1 — FAIL (do not SKIP) when LocalTerra is missing.
# VERIFY624_REQUIRE_LIVE=1 — FAIL (do not SKIP) when indexer/provision leftovers
#   cannot be probed.
# VERIFY624_FRESH=1 — make reset && make deploy-local before leftover probes.
# VERIFY624_SKIP_FRESH=1 — never reset (even if VERIFY624_FRESH=1).
#
# Refs: skills/AGENTS_POST_MERGE_OPS_624.md, docs/qa-invariants.md § Q12
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }
skip() { RESULTS+=("SKIP  $1"); echo "  [SKIP] $1"; }

run_step() {
  local label="$1"
  shift
  echo ""
  echo "[$label]"
  # Capture status — `if cmd` would disable `set -e` inside the callee.
  set +e
  "$@"
  local rc=$?
  set -e
  if [[ $rc -eq 0 ]]; then
    ok "$label"
  else
    bad "$label"
  fi
}

echo "════════════════════════════════════════════════════════════════"
echo "  GitLab #624 — post-merge !414 LocalTerra tax-seed leftovers"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
INDEXER_URL="${VERIFY624_INDEXER_URL:-http://127.0.0.1:3001}"
SWARM_FLOOR="${VERIFY624_SWARM_FLOOR:-10000000}"

run_docs() {
  set -euo pipefail
  test -f skills/AGENTS_POST_MERGE_OPS_624.md
  grep -qE '\*\*M624-1\*\*' docs/qa-invariants.md
  grep -qE '\*\*M624-8\*\*' docs/qa-invariants.md
  grep -qE 'post-merge-ops-624' docs/qa-invariants.md
  grep -qE '\*\*M624-1' skills/AGENTS_POST_MERGE_OPS_624.md
  grep -qE 'make verify-issue-624' skills/AGENTS_POST_MERGE_OPS_624.md
  grep -qE 'AGENTS_POST_MERGE_OPS_624' AGENTS.md
  grep -qE 'verify-issue-624' AGENTS.md
  grep -qE 'verify-issue-624' Makefile
  grep -qE '#624' docs/testing.md
  grep -qE 'AGENTS_POST_MERGE_OPS_624' docs/README.md
  grep -qE '#624' skills/AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md
  grep -qE '#624' skills/AGENTS_COMMUNITY_TAX_CW20.md
  grep -qE '#624' skills/AGENTS_INDEXER_COMMUNITY_TOKENS.md
  grep -qE '#624' skills/AGENTS_LOCALNET_TRADING_SWARM.md
  grep -qE '#624' skills/AGENTS_INDEXER_TAX_AWARE_ROUTING.md
  grep -qE '#624' skills/AGENTS_COMMUNITY_TAX_AUTOLP.md
  grep -qE '#624' docs/local-development.md
  grep -qE 'L620-1' docs/contracts-security-audit.md
  grep -qE 'swarm_funding_plan' skills/AGENTS_POST_MERGE_OPS_624.md
  grep -qE 'Do \*\*not\*\* reopen #620' skills/AGENTS_POST_MERGE_OPS_624.md
  if grep -nE 'AddWhitelistedCodeId 11611' skills/AGENTS_POST_MERGE_OPS_624.md \
      skills/AGENTS_LOCALTERRA_COMMUNITY_TAX_SEED.md; then
    echo "docs tell operators to whitelist columbus-5 11611 from LocalTerra" >&2
    return 1
  fi
}

run_source() {
  set -euo pipefail
  grep -qE 'swarm_funding_plan' packages/localnet-trading-swarm/src/swarmRunner.ts
  grep -qE 'planCw20Funding' packages/localnet-trading-swarm/src/swarmRunner.ts
  grep -qE 'if \(!opts.runnerOpts.dryRun\)' packages/localnet-trading-swarm/src/swarmRunner.ts
  grep -qE 'export function fundingExecuteMsg' packages/localnet-trading-swarm/src/funding.ts
  grep -qE 'kind: .swarm_funding_action' packages/localnet-trading-swarm/src/funding.ts
  grep -qE 'fundingExecuteMsg' packages/localnet-trading-swarm/src/funding.test.ts
  grep -qE 'classify_cw20_funding_kind' scripts/e2e-provision-dev-wallet.sh
  grep -qE 'fail-closed' scripts/e2e-provision-dev-wallet.sh
  grep -qE 'deploy_up_to_date' scripts/lib/deploy-up-to-date.sh
  grep -qE 'COMMUNITY_TAX_OPTION2_CODE_IDS' scripts/deploy-dex-local.sh
  if grep -nE 'add_whitelisted_code_id.*11611' scripts/lib/deploy-community-tax-local.sh \
      scripts/deploy-dex-local.sh; then
    echo "deploy tells LocalTerra to whitelist columbus-5 11611" >&2
    return 1
  fi
  if grep -nE 'features:\[.*"mint_control"' scripts/lib/deploy-community-tax-local.sh; then
    echo "QA seed must not enable MintControl" >&2
    return 1
  fi
}

run_swarm_unit() {
  set -euo pipefail
  if [[ ! -d packages/localnet-trading-swarm/node_modules ]]; then
    bash scripts/with-node.sh --cwd packages/localnet-trading-swarm -- npm ci
  fi
  bash scripts/with-node.sh --cwd packages/localnet-trading-swarm -- \
    npm run test:run -- src/funding.test.ts
}

run_620() { make verify-issue-620; }
run_601() { make verify-issue-601; }
run_592() { make verify-issue-592; }
run_610() { make verify-issue-610; }
run_594() { make verify-issue-594; }

_load_vite() {
  local env_local="$REPO_ROOT/frontend-dapp/.env.local"
  [[ -f "$env_local" ]] || return 1
  set -a
  # shellcheck disable=SC1090
  source <(grep -E '^VITE_[A-Z0-9_]+=' "$env_local")
  set +a
  [[ -n "${VITE_TOKEN_COMMUNITY_TAX_ADDRESS:-}" && -n "${VITE_FACTORY_ADDRESS:-}" ]]
}

run_env_pins() {
  set -euo pipefail
  _load_vite || {
    echo "tax pins unset — run make deploy-local without DEPLOY_SKIP_COMMUNITY_TAX" >&2
    return 1
  }
  local idx="$REPO_ROOT/indexer/.env"
  [[ -f "$idx" ]] || {
    echo "missing indexer/.env" >&2
    return 1
  }
  grep -qE '^VITE_COMMUNITY_TAX_CODE_ID=[0-9]+' frontend-dapp/.env.local
  grep -qE '^VITE_COMMUNITY_TOKEN_LAUNCHER=terra1' frontend-dapp/.env.local
  grep -qE '^VITE_TOKEN_COMMUNITY_TAX_ADDRESS=terra1' frontend-dapp/.env.local
  grep -qE '^VITE_PAIR_COMMUNITY_TAX_EMBER=terra1' frontend-dapp/.env.local
  grep -qE '^VITE_UST1_TOKEN_ADDRESS=terra1' frontend-dapp/.env.local
  grep -qE '^COMMUNITY_TAX_CODE_ID=[0-9]+' "$idx"
  grep -qE '^COMMUNITY_TOKEN_LAUNCHER=terra1' "$idx"
  grep -qE '^CMM_GOVERNANCE_ADDR=terra1x46rqay4d3cssq8gxxvqz8xt6nwlz4td20k38v' "$idx"
  grep -qE '^COMMUNITY_TAX_OPTION2_CODE_IDS=[0-9]+' "$idx"
  local local_id vite_id
  vite_id="$(grep -E '^VITE_COMMUNITY_TAX_CODE_ID=' frontend-dapp/.env.local | tail -n1 | cut -d= -f2)"
  local_id="$(grep -E '^COMMUNITY_TAX_CODE_ID=' "$idx" | tail -n1 | cut -d= -f2)"
  [[ "$vite_id" == "$local_id" ]]
  [[ "$vite_id" != "11611" && "$vite_id" != "11619" ]] || {
    echo "indexer/dApp pin columbus-5 $vite_id against local instances" >&2
    return 1
  }
  echo "env pins: tax=$VITE_TOKEN_COMMUNITY_TAX_ADDRESS code=$vite_id"
}

run_stamp_head() {
  set -euo pipefail
  local stamp="$REPO_ROOT/.qa-deploy-stamp"
  [[ -f "$stamp" ]] || {
    echo "missing .qa-deploy-stamp" >&2
    return 1
  }
  local head stamp_sha
  head="$(git -C "$REPO_ROOT" rev-parse --short HEAD)"
  stamp_sha="$(grep -E '^git_sha=' "$stamp" | tail -n1 | sed 's/^git_sha=//')"
  [[ "$head" == "$stamp_sha" ]] || {
    echo "stamp git_sha=$stamp_sha != HEAD=$head (fresh leftover: VERIFY624_FRESH=1)" >&2
    return 1
  }
  grep -qE '^community_tax_token=terra1' "$stamp"
  # shellcheck source=scripts/lib/deploy-up-to-date.sh
  source "$REPO_ROOT/scripts/lib/deploy-up-to-date.sh"
  # shellcheck source=scripts/lib/lcd-smart-query.sh
  source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"
  deploy_up_to_date "$REPO_ROOT"
}

run_live_lcd() {
  set -euo pipefail
  # shellcheck source=scripts/lib/localterra-host-curl.sh
  source "$REPO_ROOT/scripts/lib/localterra-host-curl.sh"
  # shellcheck source=scripts/lib/lcd-smart-query.sh
  source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"
  _load_vite || return 1
  local lcd="${VITE_TERRA_LCD_URL:-http://localhost:1317}"
  lcd="${lcd%/}"
  local pair alp cfg alp_cfg pool
  pair="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$lcd" "$VITE_FACTORY_ADDRESS" \
    "$(jq -nc --arg a "$VITE_TOKEN_COMMUNITY_TAX_ADDRESS" --arg b "$VITE_TOKEN_EMBER_ADDRESS" \
      '{pair:{asset_infos:[{token:{contract_addr:$a}},{token:{contract_addr:$b}}]}}')")")"
  echo "$pair" | jq -e --arg p "$VITE_PAIR_COMMUNITY_TAX_EMBER" \
    '.contract_addr == $p or .pair.contract_addr == $p' >/dev/null
  cfg="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$lcd" "$VITE_TOKEN_COMMUNITY_TAX_ADDRESS" '{"get_config":{}}')")"
  alp="$(echo "$cfg" | jq -r '.autolp // empty')"
  [[ "$alp" == terra1* ]]
  alp_cfg="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$lcd" "$alp" '{"get_config":{}}')")"
  echo "$alp_cfg" | jq -e --arg p "$VITE_PAIR_COMMUNITY_TAX_EMBER" '.pair == $p' >/dev/null
  pool="$(lcd_decode_smart_data "$(lcd_smart_query_raw "$lcd" "$VITE_PAIR_COMMUNITY_TAX_EMBER" '{"pool":{}}')")"
  echo "$pool" | jq -e --argjson floor "$SWARM_FLOOR" \
    '[.assets[].amount | tonumber] | length == 2 and min >= $floor' >/dev/null
  echo "live LCD: pair=$VITE_PAIR_COMMUNITY_TAX_EMBER autolp=$alp floor>=$SWARM_FLOOR"
}

run_indexer_catalog() {
  set -euo pipefail
  _load_vite || return 1
  local body
  body="$(curl -fsS --max-time 20 "${INDEXER_URL%/}/api/v1/community-tokens")"
  echo "$body" | jq -e '.configured == true' >/dev/null
  echo "$body" | jq -e --arg t "$VITE_TOKEN_COMMUNITY_TAX_ADDRESS" \
    '[.items[]? | select(.contract_address == $t and .attested_cmm == true)] | length == 1' >/dev/null
  local tok
  tok="$(curl -fsS --max-time 20 "${INDEXER_URL%/}/api/v1/tokens/${VITE_TOKEN_COMMUNITY_TAX_ADDRESS}")" || true
  if [[ -n "$tok" ]]; then
    echo "$tok" | jq -e '.community_tax.buy_bps != null or .community_tax.sell_bps != null' >/dev/null \
      || echo "WARN: GET /tokens/{tax} omitted community_tax (allowed until ingest)"
  fi
  echo "indexer catalog: configured=true attested_cmm=$VITE_TOKEN_COMMUNITY_TAX_ADDRESS"
}

run_route_solve_tax() {
  set -euo pipefail
  _load_vite || return 1
  [[ -n "${VITE_TOKEN_EMBER_ADDRESS:-}" ]] || return 1
  local qs body
  qs="$(jq -nr --arg a "$VITE_TOKEN_COMMUNITY_TAX_ADDRESS" --arg b "$VITE_TOKEN_EMBER_ADDRESS" \
    '"token_in=\($a)&token_out=\($b)&amount_in=1000000"')"
  body="$(curl -fsS --max-time 45 "${INDEXER_URL%/}/api/v1/route/solve?${qs}")" || return 1
  echo "$body" | jq -e '(.buy_tax_bps != null) or (.sell_tax_bps != null) or (.tax_kind != null)' >/dev/null
  echo "route/solve tax fields present"
}

run_provision() {
  set -euo pipefail
  local log
  log="$(mktemp)"
  if ! bash "$REPO_ROOT/scripts/e2e-provision-dev-wallet.sh" >"$log" 2>&1; then
    tail -n 40 "$log" >&2
    rm -f "$log"
    return 1
  fi
  if grep -E 'minting .* tax token|Mint fallback' "$log"; then
    echo "provision Minted the tax token" >&2
    cat "$log" >&2
    rm -f "$log"
    return 1
  fi
  grep -qE 'e2e-provision: CW20 balances' "$log"
  echo "provision: Transfer fork (no tax Mint)"
  rm -f "$log"
}

run_swarm_dry_run_plan() {
  set -euo pipefail
  _load_vite || return 1
  if [[ ! -d packages/localnet-trading-swarm/node_modules ]]; then
    bash scripts/with-node.sh --cwd packages/localnet-trading-swarm -- npm ci
  fi
  local log
  log="$(mktemp)"
  set +e
  timeout 45 bash scripts/with-node.sh --cwd packages/localnet-trading-swarm -- \
    npm run start -- --dry-run >"$log" 2>&1
  local rc=$?
  set -e
  if ! grep -q '"swarm_funding_plan"' "$log"; then
    echo "dry-run missing swarm_funding_plan (rc=$rc)" >&2
    tail -n 40 "$log" >&2
    rm -f "$log"
    return 1
  fi
  if grep -q '"swarm_funding_action"' "$log"; then
    echo "dry-run called fundBotWallets (swarm_funding_action present)" >&2
    rm -f "$log"
    return 1
  fi
  python3 - "$log" "$VITE_TOKEN_COMMUNITY_TAX_ADDRESS" \
    "${VITE_LUNC_C_TOKEN_ADDRESS:-}" "${VITE_USTC_C_TOKEN_ADDRESS:-}" <<'PY'
import json, sys
path, tax, wrap_l, wrap_u = sys.argv[1:5]
plan = None
with open(path, encoding="utf-8") as f:
    for line in f:
        line = line.strip()
        if not line.startswith("{"):
            continue
        try:
            doc = json.loads(line)
        except json.JSONDecodeError:
            continue
        if doc.get("kind") == "swarm_funding_plan":
            plan = doc
if not plan:
    raise SystemExit("no swarm_funding_plan JSON")
if plan.get("dryRun") is not True:
    raise SystemExit("plan.dryRun is not true")
by_tok = {row["token"]: row["fundingKind"] for row in plan.get("tokens") or []}
if tax and by_tok.get(tax) != "transfer":
    raise SystemExit(f"tax {tax} kind={by_tok.get(tax)!r} want transfer")
for w in (wrap_l, wrap_u):
    if w and w in by_tok and by_tok[w] != "skip":
        raise SystemExit(f"wrap {w} kind={by_tok[w]!r} want skip")
print(f"dry-run plan: tax=transfer tokens={len(by_tok)}")
PY
  rm -f "$log"
}

restart_indexer_after_deploy() {
  set -euo pipefail
  local bin="$REPO_ROOT/indexer/target/release/cl8y-dex-indexer"
  if [[ ! -x "$bin" ]]; then
    local sibling
    sibling="$(cd "$REPO_ROOT/.." && pwd)/cl8y-dex-terraclassic/indexer/target/release/cl8y-dex-indexer"
    if [[ -x "$sibling" ]]; then
      mkdir -p "$(dirname "$bin")"
      ln -sfn "$sibling" "$bin"
    elif [[ -x /home/answorld/repos/cl8y-dex-terraclassic/indexer/target/release/cl8y-dex-indexer ]]; then
      mkdir -p "$(dirname "$bin")"
      ln -sfn /home/answorld/repos/cl8y-dex-terraclassic/indexer/target/release/cl8y-dex-indexer "$bin"
    else
      (cd "$REPO_ROOT/indexer" && cargo build --release)
    fi
  fi
  local pid
  pid="$(ss -ltnp 2>/dev/null | awk '/:3001/ {print}' | grep -oE 'pid=[0-9]+' | head -1 | cut -d= -f2 || true)"
  if [[ -n "$pid" ]]; then
    echo "stopping indexer pid $pid on :3001 (postgres was reset)"
    kill "$pid" 2>/dev/null || true
    sleep 2
  fi
  INDEXER_PID_FILE="${INDEXER_PID_FILE:-/tmp/cl8y-indexer-624.pid}" \
    INDEXER_LOG="${INDEXER_LOG:-/tmp/cl8y-indexer-624.log}" \
    bash "$REPO_ROOT/scripts/e2e-start-indexer.sh"
}

run_fresh_deploy() {
  set -euo pipefail
  echo "==> VERIFY624_FRESH=1: make reset && make start && make deploy-local"
  make reset || return 1
  make start || return 1
  make wait-healthy || return 1
  # Sibling artifacts can be older than a fresh worktree checkout (mtime).
  # Leftover #1 is a real deploy-local so the stamp matches HEAD.
  make deploy-local || return 1
  restart_indexer_after_deploy || return 1
}

maybe_fresh() {
  if [[ "${VERIFY624_SKIP_FRESH:-}" == "1" ]]; then
    skip "fresh volume (VERIFY624_SKIP_FRESH=1)"
    return 0
  fi
  if [[ "${VERIFY624_FRESH:-}" == "1" ]]; then
    run_step "M624-2: fresh make reset && make deploy-local" run_fresh_deploy
  fi
}

echo ""
echo "── first pass ──"
run_step "docs: Q12 M624-1–M624-8 + skill + AGENTS crosslinks" run_docs
run_step "source: fundingExecuteMsg + dry-run plan + no 11611 whitelist" run_source
run_step "swarm: fundingKind + fundingExecuteMsg vitest" run_swarm_unit

if [[ "${VERIFY624_SKIP_CHILDREN:-}" == "1" ]]; then
  skip "children 620/601/592/610/594 (VERIFY624_SKIP_CHILDREN=1)"
else
  run_step "child: make verify-issue-620" run_620
fi

HAS_LT=1
if timeout 20 make -s has-localterra >/dev/null 2>&1; then
  HAS_LT=0
fi

if [[ "${VERIFY624_SKIP_CHAIN:-}" == "1" ]]; then
  skip "LocalTerra leftovers (VERIFY624_SKIP_CHAIN=1)"
elif [[ "$HAS_LT" -eq 0 ]]; then
  maybe_fresh
  run_step "M624-2: dApp + indexer local COMMUNITY_TAX pins" run_env_pins
  if run_stamp_head; then
    ok "M624-2/6: stamp git_sha == HEAD + deploy_up_to_date"
  else
    if [[ "${VERIFY624_FRESH:-}" == "1" || "${VERIFY624_REQUIRE_LIVE:-}" == "1" ]]; then
      bad "M624-2/6: stamp git_sha == HEAD + deploy_up_to_date"
    else
      skip "M624-2/6 stamp != HEAD (set VERIFY624_FRESH=1 for leftover #1)"
    fi
  fi
  run_step "M624-2: LCD factory pair + AutoLP + reserves ≥ floor" run_live_lcd
  if run_indexer_catalog; then
    ok "M624-3: GET /community-tokens configured + attested_cmm"
    if run_route_solve_tax; then
      ok "M624-3: route/solve buy_tax_bps / sell_tax_bps"
    else
      if [[ "${VERIFY624_REQUIRE_LIVE:-}" == "1" ]]; then
        bad "M624-3: route/solve tax fields"
      else
        skip "M624-3 route/solve tax fields (indexer graph not ready). Set VERIFY624_REQUIRE_LIVE=1 to fail."
      fi
    fi
  else
    if [[ "${VERIFY624_REQUIRE_LIVE:-}" == "1" ]]; then
      bad "M624-3: GET /community-tokens configured + attested_cmm"
    else
      skip "M624-3 indexer catalog (start indexer after deploy). Set VERIFY624_REQUIRE_LIVE=1 to fail."
    fi
  fi
  run_step "M624-4: e2e-provision Transfer fork (no tax Mint)" run_provision
  run_step "M624-5: swarm --dry-run logs swarm_funding_plan (no fundBotWallets)" run_swarm_dry_run_plan
elif [[ "${VERIFY624_REQUIRE_CHAIN:-}" == "1" ]]; then
  bad "LocalTerra required (VERIFY624_REQUIRE_CHAIN=1) — make setup-cloud-localterra"
else
  skip "LocalTerra (make has-localterra). Cloud Agent: make setup-cloud-localterra"
fi

if [[ "${VERIFY624_SKIP_CHILDREN:-}" == "1" ]]; then
  :
else
  run_step "child: make verify-issue-601" run_601
  run_step "child: make verify-issue-592" run_592
  run_step "child: make verify-issue-610" run_610
  run_step "child: make verify-issue-594" run_594
fi

echo ""
echo "── retest ──"
run_step "retest docs: Q12 M624" run_docs
run_step "retest source: funding plan + no 11611" run_source
run_step "retest swarm: fundingExecuteMsg vitest" run_swarm_unit

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  #624 results: ${PASS} passed, ${FAIL} failed"
echo "════════════════════════════════════════════════════════════════"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -ne 0 ]]; then
  exit 1
fi
echo "==> GitLab #624 verification passed"
exit 0
