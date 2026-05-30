#!/usr/bin/env bash
# Shared helpers: find/stop processes listening on the indexer HTTP port (127.0.0.1).
# Used by test-e2e-indexer-outage.sh so local QA indexers (.indexer-qa.pid) are stopped
# before outage Playwright, not only the script's INDEXER_PID_FILE (GitLab #219).
set -euo pipefail

# shellcheck disable=SC2034
INDEXER_PORT_LIB_SOURCED=1

indexer_port_from_url() {
  local url="${1:-http://127.0.0.1:3001}"
  url="${url%/}"
  if [[ "$url" =~ :([0-9]+)$ ]]; then
    echo "${BASH_REMATCH[1]}"
    return 0
  fi
  if [[ "$url" =~ :([0-9]+)/ ]]; then
    echo "${BASH_REMATCH[1]}"
    return 0
  fi
  echo "3001"
}

indexer_pids_listening_on_port() {
  local port="$1"
  lsof -t -iTCP:"${port}" -sTCP:LISTEN 2>/dev/null | sort -u || true
}

indexer_stop_port_listeners() {
  local port="$1"
  local pid
  local pids
  pids="$(indexer_pids_listening_on_port "$port")"
  if [[ -z "$pids" ]]; then
    return 0
  fi
  for pid in $pids; do
    if kill -0 "$pid" 2>/dev/null; then
      kill -TERM "$pid" 2>/dev/null || true
    fi
  done
  sleep 1
  pids="$(indexer_pids_listening_on_port "$port")"
  if [[ -n "$pids" ]]; then
    for pid in $pids; do
      kill -KILL "$pid" 2>/dev/null || true
    done
    sleep 1
  fi
}

indexer_sync_pidfile_with_port() {
  local pid_file="$1"
  local port="$2"
  local pid
  pid="$(indexer_pids_listening_on_port "$port" | head -1 || true)"
  if [[ -n "$pid" ]]; then
    echo "$pid" >"$pid_file"
  else
    rm -f "$pid_file"
  fi
}

indexer_clear_stale_qa_pidfile_if_dead() {
  local repo_root="$1"
  local port="$2"
  local qa_pidfile="${repo_root}/.indexer-qa.pid"
  [[ -f "$qa_pidfile" ]] || return 0
  local pid
  pid="$(cat "$qa_pidfile" 2>/dev/null || true)"
  if [[ -z "$pid" ]] || ! kill -0 "$pid" 2>/dev/null; then
    rm -f "$qa_pidfile"
    return 0
  fi
  if ! echo "$(indexer_pids_listening_on_port "$port")" | grep -qx "$pid"; then
    rm -f "$qa_pidfile"
  fi
}
