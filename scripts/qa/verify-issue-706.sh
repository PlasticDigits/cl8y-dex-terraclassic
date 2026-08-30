#!/usr/bin/env bash
# Automated verification for GitLab #706 — stale Vite lazy chunks after Coolify deploy.
#
# Proves (unit + nginx greps + docs; optional docker headers + live curl + Playwright):
#   1. Classifier + reloadOnceOnStaleChunk + LazyRoute / ErrorBoundary (L706-1–L706-6).
#   2. nginx hashed 200 immutable; HTML no-cache; 404 JS no-store (L706-7).
#   3. Playbook + frontend.md + AGENTS.md (L706-8).
#   4. No service worker; LazyRoute still used in App.tsx (no un-split).
#
# VERIFY706_SKIP_E2E=1 — skip Playwright even if it could run.
# VERIFY706_SKIP_NGINX=1 — skip docker nginx header smoke.
# VERIFY706_SKIP_LIVE=1 — skip Coolify curl even if reachable.
# VERIFY706_REQUIRE_E2E=1 — FAIL when Playwright cannot run.
# VERIFY706_REQUIRE_NGINX=1 — FAIL when docker nginx smoke cannot run.
# VERIFY706_REQUIRE_LIVE=1 or VERIFY706_IID=706 — FAIL when Coolify curl cannot run.
#
# Refs: skills/AGENTS_FRONTEND_LAZY_CHUNK_LOAD.md, docs/frontend.md § Lazy route chunks
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }
skip() { RESULTS+=("SKIP  $1"); echo "  [SKIP] $1"; }

require_live() {
  [[ "${VERIFY706_REQUIRE_LIVE:-}" == "1" || "${VERIFY706_IID:-}" == "706" ]]
}

require_nginx() {
  [[ "${VERIFY706_REQUIRE_NGINX:-}" == "1" ]]
}

require_e2e() {
  [[ "${VERIFY706_REQUIRE_E2E:-}" == "1" ]]
}

run_step() {
  local label="$1"
  shift
  echo ""
  echo "[$label]"
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
echo "  GitLab #706 — stale lazy-chunk reload after Coolify deploy"
echo "════════════════════════════════════════════════════════════════"

export PATH="/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
unset PLAYWRIGHT_WEB_PORT || true
unset PLAYWRIGHT_BASE_URL || true
unset VITE_PLAYWRIGHT_E2E || true
unset CI || true

DAPP_URL="${VERIFY706_DAPP_URL:-https://dex.cl8y.com}"
DAPP_URL="${DAPP_URL%/}"

docker_cmd() {
  if command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1; then
    docker "$@"
    return
  fi
  if command -v sg >/dev/null 2>&1 && sg docker -c 'docker info' >/dev/null 2>&1; then
    sg docker -c "docker $*"
    return
  fi
  return 1
}

header_cc() {
  echo "$1" | tr -d '\r' | grep -i '^cache-control:' | head -1 | sed 's/^[Cc]ache-[Cc]ontrol:[[:space:]]*//' || true
}

run_step "frontend: chunkLoadError + LazyRoute + ErrorBoundary + humanize" \
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/utils/__tests__/chunkLoadError.test.ts \
    src/components/common/__tests__/LazyRoute.test.tsx \
    src/components/common/__tests__/ErrorBoundary.test.tsx \
    src/utils/__tests__/humanizeUserFacingError.test.ts

run_step "code: reloadOnceOnStaleChunk + sessionStorage key + same-origin reload" \
  grep -qE 'STALE_CHUNK_RELOAD_STORAGE_KEY = .cl8y-dex-stale-chunk-reload.' \
    frontend-dapp/src/utils/chunkLoadError.ts && \
  grep -qE 'export function reloadOnceOnStaleChunk' frontend-dapp/src/utils/chunkLoadError.ts && \
  grep -qE 'window.location.reload' frontend-dapp/src/utils/chunkLoadError.ts && \
  grep -qE 'staleChunkReloadIo' frontend-dapp/src/utils/chunkLoadError.ts && \
  grep -qE 'clearStaleChunkReloadGuard' \
    frontend-dapp/src/components/common/RouteContentReadyMarker.tsx && \
  grep -qE 'wouldAutoReloadOnStaleChunk' frontend-dapp/src/components/common/ErrorBoundary.tsx && \
  grep -qE 'route-error-reload-app' frontend-dapp/src/components/common/ErrorBoundary.tsx && \
  grep -qE 'stale-chunk-updating' frontend-dapp/src/components/common/ErrorBoundary.tsx && \
  bash -c '! grep -nE "window.location.href = .*(error|module|referrer)" \
    frontend-dapp/src/utils/chunkLoadError.ts frontend-dapp/src/components/common/ErrorBoundary.tsx'

run_step "code: App.tsx still LazyRoute (no eager page bundle); no service worker" \
  grep -qE "LazyRoute loader=\{\(\) => import\('\./pages/PoolPage'\)" frontend-dapp/src/App.tsx && \
  grep -qE "LazyRoute loader=\{\(\) => import\('\./pages/ChartsPage'\)" frontend-dapp/src/App.tsx && \
  grep -qE "LazyRoute loader=\{\(\) => import\('\./pages/TradePage'\)" frontend-dapp/src/App.tsx && \
  grep -qE "LazyRoute loader=\{\(\) => import\('\./pages/ProtocolPage'\)" frontend-dapp/src/App.tsx && \
  grep -qE "LazyRoute loader=\{\(\) => import\('\./pages/CreateTokenPage'\)" frontend-dapp/src/App.tsx && \
  bash -c '! grep -R -qiE "workbox|serviceWorker|navigator.serviceWorker" \
    frontend-dapp/src frontend-dapp/vite.config.ts docker/frontend' && \
  bash -c '! grep -nE "unsafe-eval" docker/frontend/nginx.conf frontend-dapp/src/utils/chunkLoadError.ts'

run_step "code: nginx hashed 200 immutable; HTML no-cache; 404 JS no-store" \
  grep -qE 'location @hashed_asset_miss' docker/frontend/nginx.conf && \
  grep -qE 'try_files \$uri @hashed_asset_miss' docker/frontend/nginx.conf && \
  grep -qE 'Cache-Control "public, max-age=604800, immutable"' docker/frontend/nginx.conf && \
  grep -qE 'Cache-Control "no-store" always' docker/frontend/nginx.conf && \
  grep -qE 'Cache-Control "no-cache, must-revalidate"' docker/frontend/nginx.conf && \
  grep -qE 'X-Frame-Options DENY' docker/frontend/nginx.conf && \
  grep -qE 'GitLab #706' docker/frontend/nginx.conf && \
  bash -c '! grep -A6 "location @hashed_asset_miss" docker/frontend/nginx.conf | grep -q immutable'

run_step "docs: frontend.md stale Coolify hash + #172 offline Try Again" \
  grep -qE 'stale Coolify' docs/frontend.md && \
  grep -qE 'make verify-issue-706' docs/frontend.md && \
  grep -qE 'L706-1' docs/frontend.md && \
  grep -qE 'AGENTS_FRONTEND_LAZY_CHUNK_LOAD' docs/frontend.md && \
  grep -qE '#172' docs/frontend.md

run_step "docs: skill L706-1–L706-8 + AGENTS.md playbook" \
  grep -qE '\*\*L706-1' skills/AGENTS_FRONTEND_LAZY_CHUNK_LOAD.md && \
  grep -qE '\*\*L706-8' skills/AGENTS_FRONTEND_LAZY_CHUNK_LOAD.md && \
  grep -qE 'make verify-issue-706' skills/AGENTS_FRONTEND_LAZY_CHUNK_LOAD.md && \
  grep -qE 'AGENTS_FRONTEND_LAZY_CHUNK_LOAD' AGENTS.md && \
  grep -qE 'verify-issue-706' AGENTS.md && \
  grep -qE '#706' skills/AGENTS_FRONTEND_USER_ERRORS.md && \
  grep -qE '#706' skills/AGENTS_FRONTEND_SHELL_NAV.md && \
  grep -qE '#706' docs/runbooks/rollback-decision.md && \
  grep -qE '#706' skills/AGENTS_ROLLBACK_DECISION.md

echo ""
echo "[nginx docker header smoke]"
if [[ "${VERIFY706_SKIP_NGINX:-}" == "1" ]]; then
  skip "nginx docker header smoke (VERIFY706_SKIP_NGINX=1)"
elif ! docker_cmd info >/dev/null 2>&1; then
  if require_nginx; then
    bad "nginx docker header smoke (docker unavailable)"
  else
    skip "nginx docker header smoke (docker unavailable)"
  fi
else
  tmp=
  cid=
  cleanup_nginx() {
    [[ -n "${cid:-}" ]] && docker_cmd rm -f "$cid" >/dev/null 2>&1 || true
    [[ -n "${tmp:-}" ]] && rm -rf "$tmp" || true
  }
  trap cleanup_nginx EXIT
  tmp="$(mktemp -d)"
  mkdir -p "$tmp/html/assets"
  printf '<!doctype html><title>ok</title>\n' > "$tmp/html/index.html"
  printf 'console.log(1)\n' > "$tmp/html/assets/index-abc.js"
  host_port="$((18080 + RANDOM % 200))"
  cid="$(docker_cmd run -d --rm \
    -p "127.0.0.1:${host_port}:80" \
    -v "$tmp/html:/usr/share/nginx/html:ro" \
    -v "$REPO_ROOT/docker/frontend/nginx.conf:/etc/nginx/conf.d/default.conf:ro" \
    nginx:1.27-alpine)" || cid=""
  if [[ -z "$cid" ]]; then
    if require_nginx; then
      bad "nginx docker header smoke (container start failed)"
    else
      skip "nginx docker header smoke (container start failed)"
    fi
  else
    smoke_ready=0
    for _ in 1 2 3 4 5 6 7 8; do
      if curl -sI --max-time 2 "http://127.0.0.1:${host_port}/" | grep -qE '^HTTP/'; then
        smoke_ready=1
        break
      fi
      sleep 0.4
    done
    smoke_ok=1
    if [[ "$smoke_ready" -ne 1 ]]; then
      smoke_ok=0
      html_i=""; pool_i=""; js_i=""; miss_i=""
    else
      html_i="$(curl -sI --max-time 5 "http://127.0.0.1:${host_port}/" || true)"
      pool_i="$(curl -sI --max-time 5 "http://127.0.0.1:${host_port}/pool" || true)"
      js_i="$(curl -sI --max-time 5 "http://127.0.0.1:${host_port}/assets/index-abc.js" || true)"
      miss_i="$(curl -sI --max-time 5 "http://127.0.0.1:${host_port}/assets/PoolPage-does-not-exist.js" || true)"
    fi
    html_cc="$(header_cc "$html_i")"
    pool_cc="$(header_cc "$pool_i")"
    js_cc="$(header_cc "$js_i")"
    miss_cc="$(header_cc "$miss_i")"
    echo "  / Cache-Control: ${html_cc}"
    echo "  /pool Cache-Control: ${pool_cc}"
    echo "  hashed JS Cache-Control: ${js_cc}"
    echo "  missing JS Cache-Control: ${miss_cc}"
    echo "$html_i" | grep -qE '^HTTP/.* 200' || smoke_ok=0
    echo "$pool_i" | grep -qE '^HTTP/.* 200' || smoke_ok=0
    echo "$js_i" | grep -qE '^HTTP/.* 200' || smoke_ok=0
    echo "$miss_i" | grep -qE '^HTTP/.* 404' || smoke_ok=0
    [[ "$html_cc" == *no-cache* ]] || smoke_ok=0
    [[ "$html_cc" != *immutable* ]] || smoke_ok=0
    [[ "$pool_cc" == *no-cache* ]] || smoke_ok=0
    [[ "$pool_cc" != *immutable* ]] || smoke_ok=0
    [[ "$js_cc" == *immutable* ]] || smoke_ok=0
    [[ "$miss_cc" != *immutable* ]] || smoke_ok=0
    [[ "$miss_cc" == *no-store* ]] || smoke_ok=0
    if [[ "$smoke_ok" -eq 1 ]]; then
      ok "nginx docker header smoke (HTML / hashed JS / missing JS)"
    else
      bad "nginx docker header smoke (header mismatch)"
    fi
  fi
  cleanup_nginx
  trap - EXIT
fi

echo ""
echo "[live Coolify cache headers]"
if [[ "${VERIFY706_SKIP_LIVE:-}" == "1" ]]; then
  skip "live Coolify curl (VERIFY706_SKIP_LIVE=1)"
else
  live_html="$(curl -sI --max-time 8 "${DAPP_URL}/" || true)"
  if [[ -z "$live_html" ]] || ! echo "$live_html" | grep -qE '^HTTP/'; then
    if require_live; then
      bad "live Coolify curl (${DAPP_URL} unreachable)"
    else
      skip "live Coolify curl (${DAPP_URL} unreachable)"
    fi
  else
    live_ok=1
    live_pool="$(curl -sI --max-time 8 "${DAPP_URL}/pool" || true)"
    live_miss="$(curl -sI --max-time 8 "${DAPP_URL}/assets/PoolPage-does-not-exist.js" || true)"
    html_cc="$(header_cc "$live_html")"
    pool_cc="$(header_cc "$live_pool")"
    miss_cc="$(header_cc "$live_miss")"
    echo "  ${DAPP_URL}/ Cache-Control: ${html_cc}"
    echo "  /pool Cache-Control: ${pool_cc}"
    echo "  missing JS: $(echo "$live_miss" | tr -d '\r' | head -1) Cache-Control: ${miss_cc}"
    if [[ "$html_cc" == *immutable* ]]; then live_ok=0; fi
    if [[ "$pool_cc" == *immutable* ]]; then live_ok=0; fi
    echo "$live_miss" | grep -qE '^HTTP/.* 404' || live_ok=0
    if [[ "$miss_cc" == *immutable* ]]; then live_ok=0; fi
    js_href="$(curl -s --max-time 8 "${DAPP_URL}/" | grep -oE '/assets/index-[^"]+\.js' | head -1 || true)"
    if [[ -n "$js_href" ]]; then
      live_js="$(curl -sI --max-time 8 "${DAPP_URL}${js_href}" || true)"
      js_cc="$(header_cc "$live_js")"
      echo "  ${js_href} Cache-Control: ${js_cc}"
      [[ "$js_cc" == *immutable* ]] || live_ok=0
    fi
    if [[ "$live_ok" -eq 1 ]]; then
      ok "live Coolify cache headers (${DAPP_URL})"
    elif require_live; then
      bad "live Coolify cache headers (C7 mismatch — leftover until frontend rebuild)"
    else
      skip "live Coolify cache headers (C7 leftover until Coolify rebuild with this nginx.conf)"
    fi
  fi
fi

echo ""
echo "[Playwright stale-chunk e2e]"
if [[ "${VERIFY706_SKIP_E2E:-}" == "1" ]]; then
  skip "Playwright stale-chunk e2e (VERIFY706_SKIP_E2E=1)"
elif [[ ! -d frontend-dapp/node_modules ]]; then
  if require_e2e; then
    bad "Playwright stale-chunk e2e (no frontend-dapp/node_modules)"
  else
    skip "Playwright stale-chunk e2e (no frontend-dapp/node_modules)"
  fi
else
  run_step "Playwright e2e/stale-chunk-reload-706.spec.ts (5 workers)" \
    env PLAYWRIGHT_SKIP_CHAIN=1 REQUIRE_LOCALTERRA=0 \
    bash scripts/with-node.sh --cwd frontend-dapp -- \
      ./node_modules/.bin/playwright test --project=e2e-smoke e2e/stale-chunk-reload-706.spec.ts
fi

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
