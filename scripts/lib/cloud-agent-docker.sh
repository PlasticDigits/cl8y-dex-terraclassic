#!/usr/bin/env bash
# Shared Docker helpers for Cursor Cloud Agent setup scripts.
# shellcheck shell=bash

cloud_agent_tmux_cmd() {
  local TMUX_CONF="${TMUX_CONF:-/exec-daemon/tmux.portal.conf}"
  if [[ -f "$TMUX_CONF" ]]; then
    tmux -f "$TMUX_CONF" "$@"
  else
    tmux "$@"
  fi
}

cloud_agent_ensure_dockerd() {
  if groups 2>/dev/null | grep -qw docker && docker info >/dev/null 2>&1; then
    return 0
  fi
  if sg docker -c 'docker info' >/dev/null 2>&1; then
    return 0
  fi
  echo "[cloud-agent] starting dockerd in tmux session 'dockerd'…"
  if ! cloud_agent_tmux_cmd has-session -t dockerd 2>/dev/null; then
    cloud_agent_tmux_cmd new-session -d -s dockerd "sudo dockerd > /tmp/dockerd.log 2>&1"
  fi
  for _ in $(seq 1 60); do
    if sg docker -c 'docker info' >/dev/null 2>&1; then
      echo "[cloud-agent] dockerd ready"
      return 0
    fi
    sleep 2
  done
  echo "[cloud-agent] ERROR: dockerd did not become ready. See /tmp/dockerd.log" >&2
  return 1
}

cloud_agent_docker_compose() {
  if groups 2>/dev/null | grep -qw docker; then
    docker compose "$@"
  else
    sg docker -c "docker compose $(printf '%q ' "$@")"
  fi
}
