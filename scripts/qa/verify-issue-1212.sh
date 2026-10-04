#!/usr/bin/env bash
# #1212: source policy + real nginx positive, sibling, asset and missing-file checks.
# VERIFY1212_DIST_DIR=/absolute/dist tests a built Vite dist instead of source fixtures.
# VERIFY1212_REQUIRE_NGINX=1 fails if Docker is unavailable; otherwise report SKIP.
# VERIFY1212_LIVE_URL=https://dex.cl8y.com opts into strict post-deploy HTTP checks.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"
CHECKER="$REPO_ROOT/scripts/qa/check-crawl-files.py"
TMP_DIR=""
CONTAINER_ID=""
SKIPS=0

docker_cmd() {
    if [[ "${DOCKER_GROUP:-0}" == 1 ]]; then
        local quoted
        printf -v quoted '%q ' docker "$@"
        sg docker -c "$quoted"
    else
        docker "$@"
    fi
}

cleanup() {
    if [[ -n "$CONTAINER_ID" ]]; then
        docker_cmd rm -f "$CONTAINER_ID" >/dev/null 2>&1 || true
    fi
    if [[ -n "$TMP_DIR" ]]; then
        rm -rf "$TMP_DIR"
    fi
}
trap cleanup EXIT

python3 "$CHECKER"

if command -v docker >/dev/null 2>&1 && docker info >/dev/null 2>&1; then
    DOCKER_GROUP=0
elif command -v sg >/dev/null 2>&1 && sg docker -c 'docker info' >/dev/null 2>&1; then
    DOCKER_GROUP=1
else
    if [[ "${VERIFY1212_REQUIRE_NGINX:-0}" == 1 ]]; then
        echo 'FAIL nginx: Docker unavailable' >&2
        exit 1
    fi
    echo 'SKIP nginx runtime: Docker unavailable; source checks passed'
    SKIPS=$((SKIPS + 1))
    DOCKER_GROUP="unavailable"
fi

if [[ "$DOCKER_GROUP" != unavailable ]]; then
    TMP_DIR="$(mktemp -d)"
    mkdir -p "$TMP_DIR/html/assets"
    if [[ -n "${VERIFY1212_DIST_DIR:-}" ]]; then
        test -f "$VERIFY1212_DIST_DIR/index.html"
        cp -a "$VERIFY1212_DIST_DIR/." "$TMP_DIR/html/"
    else
        cp frontend-dapp/public/{robots.txt,sitemap.xml,og-image.png} "$TMP_DIR/html/"
        printf '<!doctype html><title>QA 1212</title><script src="/assets/index-qa1212.js"></script><link rel="stylesheet" href="/assets/index-qa1212.css">\n' > "$TMP_DIR/html/index.html"
        printf 'console.log("qa1212");\n' > "$TMP_DIR/html/assets/index-qa1212.js"
        printf 'body { color: white; }\n' > "$TMP_DIR/html/assets/index-qa1212.css"
    fi
    # Match the runtime stage of docker/frontend/Dockerfile; no full app/chain needed.
    CONTAINER_ID="$(docker_cmd run --detach --rm \
        --publish 127.0.0.1::80 \
        --volume "$TMP_DIR/html:/usr/share/nginx/html:ro" \
        --volume "$REPO_ROOT/docker/frontend/nginx.conf:/etc/nginx/conf.d/default.conf:ro" \
        nginx:1.27-alpine)"
    docker_cmd exec "$CONTAINER_ID" nginx -t
    PORT="$(docker_cmd port "$CONTAINER_ID" 80/tcp)"
    BASE_URL="http://$PORT"
    READY=0
    for _ in {1..20}; do
        if curl --silent --fail --max-time 2 "$BASE_URL/" >/dev/null; then
            READY=1
            break
        fi
        sleep 0.2
    done
    if [[ "$READY" != 1 ]]; then
        docker_cmd logs "$CONTAINER_ID" >&2
        echo 'FAIL nginx: container HTTP endpoint did not become ready' >&2
        exit 1
    fi
    python3 "$CHECKER" --base-url "$BASE_URL"
    # Delete only disposable copies; never mutate public/, dist, or a running deploy.
    mv "$TMP_DIR/html/robots.txt" "$TMP_DIR/robots.txt"
    mv "$TMP_DIR/html/sitemap.xml" "$TMP_DIR/sitemap.xml"
    python3 "$CHECKER" --base-url "$BASE_URL" --missing-files
fi

if [[ -n "${VERIFY1212_LIVE_URL:-}" ]]; then
    python3 "$CHECKER" --base-url "$VERIFY1212_LIVE_URL"
else
    echo 'SKIP production: set VERIFY1212_LIVE_URL after deploying this revision'
    SKIPS=$((SKIPS + 1))
fi
echo "PASS #1212 verification ($SKIPS skipped checks; skips are not live acceptance evidence)"
