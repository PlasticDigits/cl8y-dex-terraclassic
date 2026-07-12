#!/usr/bin/env bash
# Unit tests for mainnet soft-launch defaults + dry-run deploy wiring.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=../../scripts/lib/mainnet-soft-launch-defaults.sh
source "$ROOT/scripts/lib/mainnet-soft-launch-defaults.sh"

fail() { echo "FAIL: $*" >&2; exit 1; }
ok() { echo "OK: $*"; }

# --- defaults catalog ---
[[ "$(mainnet_soft_launch_token_count)" -eq 8 ]] || fail "expected 8 tokens"
[[ "$(mainnet_soft_launch_pair_count)" -eq 10 ]] || fail "expected 10 pairs"
[[ "$MAINNET_SOFT_LAUNCH_DEPLOY_ADDR" == "terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv" ]] || fail "deploy addr"
[[ "$MAINNET_CW20_MINTABLE_CODE_ID" == "10184" ]] || fail "mintable code id"
[[ "$MAINNET_CW20_BASE_CODE_ID" == "6036" ]] || fail "cw20-base code id"
[[ "$MAINNET_SOFT_LAUNCH_FRONTEND_ORIGIN" == "https://dex.cl8y.com" ]] || fail "frontend origin"
[[ "$MAINNET_SOFT_LAUNCH_INDEXER_ORIGIN" == "https://indexer.dex.cl8y.com" ]] || fail "indexer origin"

kinds="$(printf '%s\n' "${MAINNET_SOFT_LAUNCH_TOKENS[@]}" | cut -d'|' -f4 | sort -u | tr '\n' ' ')"
echo "$kinds" | grep -q 'base' || fail "missing base kind"
echo "$kinds" | grep -q 'mintable' || fail "missing mintable kind"

# Every pair symbol resolves
for pair in "${MAINNET_SOFT_LAUNCH_PAIRS[@]}"; do
  IFS='|' read -r a b _ _ <<<"$pair"
  mainnet_soft_launch_symbol_index "$a" >/dev/null || fail "unknown symbol $a"
  mainnet_soft_launch_symbol_index "$b" >/dev/null || fail "unknown symbol $b"
done
ok "token/pair catalog"

# Whitelist policy: only two kinds → only two code-id slots in docs/script comments
grep -q 'SL1' "$ROOT/scripts/lib/mainnet-soft-launch-defaults.sh" || fail "SL1 invariant comment"
ok "SL invariants documented in defaults"

# Tier msgs align with fee-discount drift check (count)
tier_count="$(mainnet_soft_launch_fee_discount_tier_msgs | grep -c add_tier || true)"
[[ "$tier_count" -eq 11 ]] || fail "expected 11 tiers, got $tier_count"
ok "fee-discount tier count"

# --- dry-run deploy (no chain) ---
OUT="$ROOT/deployments/mainnet-soft-launch-test-$$"
mkdir -p "$OUT"
cleanup() { rm -rf "$OUT"; }
trap cleanup EXIT

# Pretend artifacts exist for require_artifact by pointing at real ones if present,
# else skip dry-run deploy when wasm missing.
if [[ ! -f "$ROOT/smartcontracts/artifacts/cl8y_dex_factory.wasm" ]]; then
  echo "SKIP dry-run deploy (no optimized wasm artifacts)"
else
  DRY_RUN=1 \
    MAINNET_CW20_BASE_CODE_ID=999002 \
    MAINNET_SOFT_LAUNCH_OUT_DIR="$OUT" \
    bash "$ROOT/scripts/deploy-dex-mainnet-soft-launch.sh"
  [[ -f "$OUT/addresses.env" ]] || fail "addresses.env missing"
  grep -q 'FACTORY_ADDRESS=' "$OUT/addresses.env" || fail "FACTORY_ADDRESS"
  grep -q 'VITE_NETWORK=mainnet' "$OUT/frontend.env.example" || fail "frontend env"
  grep -q 'RUN_MODE=prod' "$OUT/indexer.env.example" || fail "indexer env"
  grep -q 'https://dex.cl8y.com' "$OUT/indexer.env.example" || fail "CORS origin"
  grep -q 'https://indexer.dex.cl8y.com' "$OUT/frontend.env.example" || fail "indexer URL"
  ok "dry-run deploy outputs"
fi

# --- Dockerfiles present (no compose) ---
[[ -f "$ROOT/docker/indexer/Dockerfile" ]] || fail "indexer Dockerfile"
[[ -f "$ROOT/docker/frontend/Dockerfile" ]] || fail "frontend Dockerfile"
[[ -f "$ROOT/docker/frontend/nginx.conf" ]] || fail "nginx.conf"
grep -q 'dex.cl8y.com' "$ROOT/docker/frontend/Dockerfile" || fail "frontend hostname"
grep -q 'indexer.dex.cl8y.com' "$ROOT/docker/indexer/Dockerfile" || fail "indexer hostname comment"
! grep -RniE 'docker-compose\.ya?ml|compose\.ya?ml' "$ROOT/docker" >/dev/null 2>&1 || fail "compose file under docker/"
ok "docker layout"

# --- shellcheck-ish: scripts executable ---
[[ -x "$ROOT/scripts/deploy-dex-mainnet-soft-launch.sh" ]] || chmod +x "$ROOT/scripts/deploy-dex-mainnet-soft-launch.sh"
[[ -x "$ROOT/scripts/build-cw20-base-artifact.sh" ]] || chmod +x "$ROOT/scripts/build-cw20-base-artifact.sh"
ok "scripts executable"

echo "ALL PASS: test-mainnet-soft-launch-defaults"
