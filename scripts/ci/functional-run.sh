#!/usr/bin/env bash
# Required Woodpecker functional matrix (#1400). The trusted config extension
# embeds this script; PR copies are never the authority for required job policy.
set -euo pipefail

root="${CI_WORKSPACE:-$PWD}"
cd "$root"
job="${1:?usage: functional-run.sh JOB}"
state="$root/.ci-functional"
required=(gitleaks contracts frontend indexer-lib indexer-postgres)
revision="${CI_COMMIT_SHA:-$(git rev-parse HEAD)}"

fail() { printf 'functional gate: %s\n' "$*" >&2; exit 1; }

case "$job" in
  init)
    # The externally served policy remains authoritative, but its source
    # mirrors must remain buildable. Deletion cannot become missing-job success.
    for asset in .woodpecker.yaml .gitleaks.toml scripts/ci/functional-run.sh scripts/ci/gitleaks-scan-tracked.sh; do
      [[ -f "$root/$asset" && ! -L "$root/$asset" ]] || fail "missing policy source: $asset"
    done
    # Checkout contents cannot supply stale success receipts.
    rm -rf "$state"
    mkdir -m 0700 "$state"
    printf '%s\n' "$revision" > "$state/revision"
    od -An -N32 -tx1 /dev/urandom | tr -d ' \n' > "$state/run"
    exit 0
    ;;
  aggregate)
    [[ -f "$state/run" && -f "$state/revision" ]] || fail 'initialization missing'
    [[ "$(cat "$state/revision")" == "$revision" ]] || fail 'revision mismatch'
    for name in "${required[@]}"; do
      [[ -f "$state/$name" && ! -L "$state/$name" ]] || fail "missing job: $name"
      expected="$(cat "$state/run") $revision $name PASS"
      [[ "$(cat "$state/$name")" == "$expected" ]] || fail "invalid receipt: $name"
    done
    printf 'PASS: all mandatory functional jobs completed for %s\n' "$revision"
    exit 0
    ;;
  gitleaks|contracts|frontend|indexer-lib|indexer-postgres) ;;
  *) fail "unknown job: $job" ;;
esac

[[ -f "$state/run" && -f "$state/revision" ]] || fail 'initialization missing'
[[ "$(cat "$state/revision")" == "$revision" ]] || fail 'revision mismatch'
rm -f "$state/$job"
# No executable target/cache is restored, shared with another pipeline, or
# eligible for publishing. Cargo and npm state live outside the checkout.
scratch="$(mktemp -d "${TMPDIR:-/tmp}/cl8y-functional.XXXXXXXX")"
trap 'rm -rf "$scratch"' EXIT
export CARGO_HOME="$scratch/cargo" CARGO_TARGET_DIR="$scratch/target"
export CARGO_BUILD_JOBS=2 CARGO_INCREMENTAL=0
# Omit debug symbols to bound cold-link memory/IO. Debug assertions remain on.
export CARGO_PROFILE_DEV_DEBUG=0 CARGO_PROFILE_TEST_DEBUG=0
export npm_config_cache="$scratch/npm" npm_config_audit=false
# Cargo homes are empty; rustup's installed toolchain remains available.
export PATH="${RUSTUP_HOME:-/usr/local/rustup}/toolchains/stable-x86_64-unknown-linux-gnu/bin:$PATH"

case "$job" in
  gitleaks)
    # The extension also embeds the scanner and its reviewed TOML config.
    GITLEAKS_SCAN_ROOT="$root" bash "${GITLEAKS_SCAN_SCRIPT:-scripts/ci/gitleaks-scan-tracked.sh}" --verbose --exit-code 1
    ;;
  contracts)
    cd smartcontracts
    cargo fmt --check
    cargo test --locked --workspace
    ;;
  frontend)
    cd frontend-dapp
    npm ci --no-audit --no-fund
    # Required tools are invoked directly: PR-owned npm script aliases cannot
    # replace a functional assertion/build with an unconditional success.
    ./node_modules/.bin/eslint .
    ./node_modules/.bin/vitest run --maxWorkers=2
    # Cold tsc build catches regressions not covered by Vitest or lint.
    rm -rf dist
    find . -maxdepth 1 -name '*.tsbuildinfo' -delete
    ./node_modules/.bin/tsc -b
    VITE_WC_PROJECT_ID=ci-build-typecheck VITE_DEV_MODE=false ./node_modules/.bin/vite build
    [[ -z "$(find dist -name '*.map' -print -quit)" ]] || fail 'production sourcemap found'
    ;;
  indexer-lib)
    cd indexer
    cargo test --locked --lib
    ;;
  indexer-postgres)
    # Only the dedicated disposable PostgreSQL service is accepted. No real
    # credentials, host DBs, or .env overrides belong in a required PR job.
    [[ "${TEST_DATABASE_URL:-}" == 'postgres://cl8y_ci:cl8y_ci@postgres:5432/dex_indexer_ci_test' ]] || fail 'isolated test database not configured'
    [[ ! -e indexer/.env ]] || fail 'checkout .env overrides test database'
    ready=false
    for ((attempt=0; attempt<30; attempt++)); do
      if pg_isready -h postgres -U cl8y_ci -d dex_indexer_ci_test >/dev/null 2>&1; then ready=true; break; fi
      sleep 2
    done
    [[ "$ready" == true ]] || fail 'PostgreSQL unavailable after 60 seconds'
    cd indexer
    cargo test --locked --jobs 2 \
      --test security --test api_hooks --test api_route_tax_funding1381 \
      --test indexer_ingestion_hardening -- --test-threads=1
    ;;
esac

printf '%s %s %s PASS\n' "$(cat "$state/run")" "$revision" "$job" > "$state/$job"
printf 'PASS: %s for %s\n' "$job" "$revision"
