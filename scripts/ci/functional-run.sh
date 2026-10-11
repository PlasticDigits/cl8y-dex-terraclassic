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
    # A prebuilt canvas download can time out; its native fallback must still
    # have Cairo/Pango and compiler prerequisites in the pinned fresh job.
    apt-get update
    apt-get install -y --no-install-recommends build-essential pkg-config python3 \
      libcairo2-dev libpango1.0-dev libjpeg-dev libgif-dev librsvg2-dev fonts-dejavu-core
    npm ci --no-audit --no-fund
    # Exercise the fallback on every pinned worker even when npm ci obtained a
    # prebuilt binding. Remove only canvas output; retain the locked dependency
    # tree and prove fresh compilation before loading its native functionality.
    rm -rf node_modules/canvas/build
    npm_config_build_from_source=true npm_package_config_node_gyp_jobs=2 \
      npm rebuild canvas --no-audit --no-fund --foreground-scripts
    [[ -s node_modules/canvas/build/Release/canvas.node ]] || fail 'canvas native binding missing'
    [[ -n "$(find node_modules/canvas/build/Release/obj.target -type f -name '*.o' -size +0c -print -quit)" ]] || fail 'canvas native compile objects missing'
    node <<'CANVAS_NATIVE_PROBE'
const assert = require('node:assert/strict');
const { createCanvas } = require('canvas');
const canvas = createCanvas(64, 32);
const context = canvas.getContext('2d');
context.fillStyle = '#1f7fbf';
context.fillRect(0, 0, 64, 32);
assert.deepEqual(Array.from(context.getImageData(1, 1, 1, 1).data), [31, 127, 191, 255]);
context.clearRect(0, 0, 64, 32);
context.fillStyle = 'black';
context.font = '16px "DejaVu Sans"';
const metrics = context.measureText('QA');
assert(Number.isFinite(metrics.width) && metrics.width > 0);
context.fillText('QA', 2, 20);
assert(context.getImageData(0, 0, 64, 32).data.some((value, index) => index % 4 === 3 && value > 0));
console.log('PASS: fresh native canvas RGBA and Pango rendering');
CANVAS_NATIVE_PROBE
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
    [[ "${TEST_DATABASE_ADMIN_URL:-}" == 'postgres://dex_indexer_qa:ci-qa-disposable@postgres:5432/dex_indexer_test' ]] || fail 'isolated QA control database not configured'
    [[ ! -e indexer/.env && ! -e indexer/.env.qa ]] || fail 'checkout dotenv overrides test database'
    ready=false
    for ((attempt=0; attempt<30; attempt++)); do
      if pg_isready -h postgres -U cl8y_ci -d dex_indexer_ci_test >/dev/null 2>&1; then ready=true; break; fi
      sleep 2
    done
    [[ "$ready" == true ]] || fail 'PostgreSQL unavailable after 60 seconds'
    # Bootstrap occurs only on the fixed ephemeral service as its administrator.
    PGPASSWORD=cl8y_ci psql -h postgres -U cl8y_ci -d dex_indexer_ci_test -v ON_ERROR_STOP=1 <<'SQL'
CREATE ROLE dex_indexer_qa LOGIN CREATEDB NOSUPERUSER NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS PASSWORD 'ci-qa-disposable';
CREATE ROLE dex_indexer_runtime NOLOGIN NOCREATEDB NOSUPERUSER NOCREATEROLE NOINHERIT NOREPLICATION NOBYPASSRLS;
CREATE DATABASE dex_indexer_test;
COMMENT ON DATABASE dex_indexer_test IS 'cl8y-dex:isolated-qa:v1';
REVOKE ALL ON DATABASE dex_indexer_test FROM PUBLIC;
GRANT CONNECT ON DATABASE dex_indexer_test TO dex_indexer_qa;
REVOKE CONNECT ON DATABASE dex_indexer_ci_test FROM PUBLIC;
SQL
    cd indexer
    cargo test --locked --jobs 2 \
      --test security --test api_hooks --test api_route_tax_funding1381 \
      --test indexer_ingestion_hardening --test database_isolation -- --test-threads=1
    ;;
esac

printf '%s %s %s PASS\n' "$(cat "$state/run")" "$revision" "$job" > "$state/$job"
printf 'PASS: %s for %s\n' "$job" "$revision"
