#!/usr/bin/env bash
# Host access to LocalTerra RPC/LCD with docker exec fallback.
#
# On some Linux hosts, curl to published 127.0.0.1:26657 / :1317 connects but hangs
# (docker userland-proxy + IPv6 listeners). In-container curl works; scripts use that
# fallback automatically. The browser/frontend still uses published host ports.
#
# See docs/local-development.md § LocalTerra host ports; skills/AGENTS_QA_DEPLOY_VERIFY.md
# shellcheck shell=bash

LOCALTERRA_CURL_CONNECT_TIMEOUT="${LOCALTERRA_CURL_CONNECT_TIMEOUT:-2}"
LOCALTERRA_CURL_MAX_TIME="${LOCALTERRA_CURL_MAX_TIME:-8}"

# Resolve running localterra container id (empty if compose stack down).
localterra_container_id() {
  local repo_root="${1:-}"
  if [ -z "$repo_root" ]; then
    repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
  fi
  docker compose -f "$repo_root/docker-compose.yml" ps -q localterra 2>/dev/null | head -1
}

# curl inside the localterra container (RPC/LCD listen on 127.0.0.1 there).
localterra_exec_curl() {
  local cid="$1"
  shift
  docker exec "$cid" curl -sf \
    --connect-timeout "$LOCALTERRA_CURL_CONNECT_TIMEOUT" \
    --max-time "$LOCALTERRA_CURL_MAX_TIME" \
    "$@"
}

# Map host-facing URL to in-container loopback (same ports inside the image).
localterra_in_container_url() {
  local url="$1"
  local port
  case "$url" in
    *:26657* | *:26659*) port=26657 ;;
    *:1317* | *:1319*) port=1317 ;;
    *) printf '%s' "$url"; return 0 ;;
  esac
  printf '%s' "$(echo "$url" | sed -E "s#//[^/:]+:[0-9]+#//127.0.0.1:${port}#")"
}

# Try host curl, then docker exec. Prints response body; returns curl exit code.
localterra_host_curl() {
  local url="$1"
  local out cid inner
  out="$(curl -sf \
    --connect-timeout "$LOCALTERRA_CURL_CONNECT_TIMEOUT" \
    --max-time "$LOCALTERRA_CURL_MAX_TIME" \
    "$url" 2>/dev/null || true)"
  if [ -n "$out" ]; then
    printf '%s' "$out"
    return 0
  fi
  cid="$(localterra_container_id)"
  if [ -z "$cid" ]; then
    return 1
  fi
  inner="$(localterra_in_container_url "$url")"
  localterra_exec_curl "$cid" "$inner"
}

# GET path on LCD base URL (e.g. base=http://127.0.0.1:1317 path=/cosmos/...).
localterra_lcd_curl() {
  local lcd_base="$1"
  local path="$2"
  lcd_base="${lcd_base%/}"
  path="${path#/}"
  localterra_host_curl "${lcd_base}/${path}"
}

# RPC /status reachable (host or exec fallback).
localterra_rpc_status_ok() {
  local rpc_url="${1:-http://127.0.0.1:26657}"
  rpc_url="${rpc_url%/}/status"
  localterra_host_curl "$rpc_url" >/dev/null 2>&1
}
