#!/usr/bin/env bash
# Automated verification for GitLab #629 — Keplr CW20 recognition pack.
#
# Proves (unit + pack schema + docs; no LocalTerra / no GitHub PR):
#   K629-1  chain dir is columbus (not terra / phoenix)
#   K629-2  six permanent CW20s; gems / ALPHA / USTRIX / SpaceUSD excluded
#   K629-3  CL8Y/USTR 18 decimals; tokenlist CL8Y is 18
#   K629-4  schema + coinGeckoId only on CL8Y; no price/oracle fields
#   K629-5  filename = address.json; logos exist under tokenlist/images
#   K629-6  vFDUSD address + 6 decimals
#   K629-7  Job 2 decision documented (ceramicliberty-com + no invented price)
#   K629-8  USTR already_registered; skill + AGENTS.md + verify target
#
# Refs: skills/AGENTS_KEPLR_CW20_REGISTRY.md
#       docs/listings/keplr-contract-registry/README.md
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }

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
echo "  GitLab #629 — Keplr CW20 contract-registry pack"
echo "════════════════════════════════════════════════════════════════"

run_step "pack: schema, pins, logos, tokenlist CL8Y decimals" \
  python3 scripts/qa/keplr_cw20_registry_validate.py

run_frontend() {
  if [[ ! -x frontend-dapp/node_modules/.bin/vitest ]]; then
    bash scripts/with-node.sh --cwd frontend-dapp -- npm ci
  fi
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/keplrCw20Registry.test.ts
}

run_step "frontend: catalog builder matches on-disk JSON" \
  run_frontend

run_step "export: drop-in tree omits already-registered USTR" \
  bash -c '
    tmp="$(mktemp -d)"
    trap "rm -rf \"$tmp\"" EXIT
    ./scripts/qa/export-keplr-cw20-pack.sh "$tmp"
    test -f "$tmp/cosmos/columbus/tokens/terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3.json"
    test ! -f "$tmp/cosmos/columbus/tokens/terra1vy3kc0swag2rhn7jz6n72jp0l2ns0p6r6ez5grxq5uhj2rvs97fqfsetxv.json"
    test -f "$tmp/images/columbus/CL8Y.png"
    test ! -f "$tmp/images/columbus/USTR.png"
  '

run_step "docs: listing pack invariants K629-1–K629-8" \
  grep -qE '\*\*K629-1\*\*' docs/listings/keplr-contract-registry/README.md && \
  grep -qE '\*\*K629-8\*\*' docs/listings/keplr-contract-registry/README.md && \
  grep -qE 'cosmos/columbus' docs/listings/keplr-contract-registry/README.md && \
  grep -qE 'ceramicliberty-com' docs/listings/keplr-contract-registry/README.md && \
  grep -qE 'already listed|already_registered|already registered' \
    docs/listings/keplr-contract-registry/README.md && \
  grep -qE 'make verify-issue-629' docs/listings/keplr-contract-registry/README.md

run_step "docs: integrators + README + testing + QA note" \
  grep -qE 'keplr-cw20-recognition-gitlab-629' docs/integrators.md && \
  grep -qE 'AGENTS_KEPLR_CW20_REGISTRY' docs/integrators.md && \
  grep -qE 'keplr-contract-registry' docs/README.md && \
  grep -qE 'verify-issue-629' docs/testing.md && \
  grep -qE 'K629-1' docs/qa/issue-629/README.md && \
  grep -qE 'ceramicliberty-com' docs/CG_CMC_COMPLIANCE.md

run_step "docs: skill + AGENTS.md playbook #629" \
  grep -qE '\*\*K629-1' skills/AGENTS_KEPLR_CW20_REGISTRY.md && \
  grep -qE '\*\*K629-8' skills/AGENTS_KEPLR_CW20_REGISTRY.md && \
  grep -qE 'make verify-issue-629' skills/AGENTS_KEPLR_CW20_REGISTRY.md && \
  grep -qE 'AGENTS_KEPLR_CW20_REGISTRY' AGENTS.md && \
  grep -qE 'verify-issue-629' AGENTS.md && \
  grep -qE 'AGENTS_KEPLR_CW20_REGISTRY' skills/AGENTS_FRONTEND_KEPLR_LEDGER.md

echo ""
echo "────────────────────────────────────────────────────────────────"
echo "  $PASS passed, $FAIL failed"
echo "────────────────────────────────────────────────────────────────"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
