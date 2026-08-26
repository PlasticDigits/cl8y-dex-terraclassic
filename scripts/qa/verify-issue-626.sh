#!/usr/bin/env bash
# Automated verification for GitLab #626 — free listed-template adopt + LP gate.
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
echo "  GitLab #626 — Migrate Token adopt + Terraport/GDEX LP gate"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
if [[ -e "$REPO_ROOT/smartcontracts/target" && ! -w "$REPO_ROOT/smartcontracts/target" ]]; then
  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/cl8y-626-sc-target}"
  mkdir -p "$CARGO_TARGET_DIR"
fi
if [[ -e "$REPO_ROOT/indexer/target" && ! -w "$REPO_ROOT/indexer/target" ]]; then
  export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-/tmp/cl8y-626-idx-target}"
  mkdir -p "$CARGO_TARGET_DIR"
fi

run_crates() {
  (cd smartcontracts && cargo test -p cl8y-community-tax-token --offline --lib adopt -- --test-threads=1)
}

run_indexer_lib() {
  (cd indexer && cargo test --lib community_tokens -- --test-threads=1 --quiet)
}

run_frontend() {
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/communityTaxMigrate.test.ts \
    src/utils/communityTaxMigrateCopy.test.ts \
    src/pages/MigrateTokenPage.test.tsx \
    src/pages/CreateTokenPage.test.tsx \
    src/components/common/navItems.test.ts
}

run_docs() {
  rg -q "migrate-adopt" skills/AGENTS_COMMUNITY_TAX_CW20.md
  rg -q "GetMigrateOrigin" skills/AGENTS_COMMUNITY_TAX_CW20.md
  rg -q "/token/migrate" skills/AGENTS_FRONTEND_CREATE_TOKEN.md
  rg -q "M626-1" skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md
  rg -q "8654" skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md
  rg -q "VITE_COMMUNITY_MIGRATE_CODE_IDS" skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md
  rg -q "tax_info" skills/AGENTS_FRONTEND_TOKEN_MIGRATE.md
  rg -q "S4 go" docs/contracts-terraclassic.md
  rg -q "migrate-adopt" docs/contracts-terraclassic.md
  rg -q "terra12u7kh" docs/contracts-terraclassic.md
  rg -q "terra1uxr6m55wxez5csnttz00893zur6pksn54nwlpye0c2pyuyyqp3qqknypyc" docs/contracts-terraclassic.md
  rg -q "terra1qz56v6p8ca3hh34wnj5yc3jykmw6jaaal0ukecscq8m9qqtgztnscs74n3" docs/contracts-terraclassic.md
  rg -q "GetMigrateOrigin" docs/contracts-terraclassic.md
  rg -q "/token/migrate" frontend-dapp/src/App.tsx
  rg -q "8654" frontend-dapp/src/utils/communityTaxMigrate.ts
  rg -q "GetMigrateOrigin" indexer/src/indexer/community_tokens.rs
  rg -q "verify-issue-626" docs/testing.md
  rg -q "verify-issue-626" AGENTS.md
  ! rg -n "AddWhitelistedCodeId 8654" smartcontracts/contracts scripts/upgrade*.sh
}

echo ""
echo "── first pass ──"
run_step "crates: adopt importer + multitest" run_crates
run_step "indexer: attest + parse migrate-adopt" run_indexer_lib
run_step "frontend: migrate page + classify + nav" run_frontend
run_step "docs: M626 + LP table + no 8654 whitelist" run_docs

echo ""
echo "── retest ──"
run_step "retest crates: adopt importer + multitest" run_crates
run_step "retest indexer: attest + parse migrate-adopt" run_indexer_lib
run_step "retest frontend: migrate page + classify + nav" run_frontend
run_step "retest docs: M626 + LP table + no 8654 whitelist" run_docs

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if (( FAIL > 0 )); then
  exit 1
fi
echo "==> GitLab #626 verification passed"
