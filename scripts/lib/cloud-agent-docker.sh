#!/usr/bin/env bash
# Shared Docker helpers for Cursor Cloud Agent setup scripts.
# shellcheck shell=bash

: "${CLOUD_AGENT_DOCKER_SOCK:=/var/run/docker.sock}"
: "${CLOUD_AGENT_DOCKERD_LOG:=/tmp/cl8y-dockerd.log}"
: "${CLOUD_AGENT_DOCKER_DAEMON_JSON:=/etc/docker/daemon.json}"

cloud_agent_need_sudo() {
  [[ "$(id -u)" -eq 0 ]] || command -v sudo >/dev/null 2>&1
}

cloud_agent_run_as_root() {
  if [[ "$(id -u)" -eq 0 ]]; then
    "$@"
  else
    sudo "$@"
  fi
}

cloud_agent_docker_cmd() {
  local timeout_s="${CLOUD_AGENT_DOCKER_TIMEOUT_S:-15}"
  if groups 2>/dev/null | grep -qw docker; then
    timeout "$timeout_s" docker "$@"
    return $?
  fi
  timeout "$timeout_s" sg docker -c "docker $(printf '%q ' "$@")"
}

cloud_agent_docker_daemon_responds() {
  cloud_agent_docker_cmd info >/dev/null 2>&1
}

cloud_agent_docker_run_hello() {
  local attempt
  for attempt in 1 2 3; do
    cloud_agent_docker_cmd run --rm hello-world >/dev/null 2>&1 && return 0
    [[ "$attempt" -lt 3 ]] && sleep 2
  done
  return 1
}

cloud_agent_docker_run_hello_err() {
  cloud_agent_docker_cmd run --rm hello-world 2>&1
}

cloud_agent_docker_hello_err_storage_related() {
  local err="${1:-}"
  [[ -n "$err" ]] || return 1
  echo "$err" | grep -qiE 'storage-driver|fuse-overlayfs|overlay.*mount|OCI runtime create|failed to (start|create|register).*(container|layer|shim)'
}

cloud_agent_fix_docker_socket_permissions() {
  if [[ ! -e "${CLOUD_AGENT_DOCKER_SOCK}" ]]; then
    return 1
  fi
  if cloud_agent_docker_daemon_responds && cloud_agent_docker_run_hello; then
    return 0
  fi
  if getent group docker >/dev/null 2>&1 && [[ -n "${USER:-}" ]]; then
    cloud_agent_run_as_root usermod -aG docker "${USER}" 2>/dev/null || true
  fi
  cloud_agent_run_as_root chmod 666 "${CLOUD_AGENT_DOCKER_SOCK}" 2>/dev/null || true
  cloud_agent_docker_daemon_responds && cloud_agent_docker_run_hello
}

cloud_agent_write_docker_storage_driver() {
  local driver="$1"
  cloud_agent_run_as_root mkdir -p /etc/docker
  printf '%s\n' '{' "  \"storage-driver\": \"${driver}\"" '}' \
    | cloud_agent_run_as_root tee "${CLOUD_AGENT_DOCKER_DAEMON_JSON}" >/dev/null
  echo "[cloud-agent] Docker storage-driver: ${driver}"
}

cloud_agent_install_docker_packages() {
  if command -v docker >/dev/null 2>&1; then
    return 0
  fi
  if ! cloud_agent_need_sudo; then
    echo "[cloud-agent] docker missing and no sudo." >&2
    return 1
  fi
  echo "[cloud-agent] installing Docker CE…"
  cloud_agent_run_as_root install -m 0755 -d /etc/apt/keyrings
  if [[ ! -f /etc/apt/keyrings/docker.gpg ]]; then
    curl --retry 3 --retry-delay 5 -fsSL https://download.docker.com/linux/ubuntu/gpg \
      | cloud_agent_run_as_root gpg --dearmor -o /etc/apt/keyrings/docker.gpg
    cloud_agent_run_as_root chmod a+r /etc/apt/keyrings/docker.gpg
  fi
  if [[ ! -f /etc/apt/sources.list.d/docker.list ]]; then
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu $(. /etc/os-release && echo "${VERSION_CODENAME}") stable" \
      | cloud_agent_run_as_root tee /etc/apt/sources.list.d/docker.list >/dev/null
  fi
  cloud_agent_run_as_root apt-get update -qq
  cloud_agent_run_as_root DEBIAN_FRONTEND=noninteractive apt-get install -y \
    docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin \
    fuse-overlayfs iptables
  cloud_agent_run_as_root update-alternatives --set iptables /usr/sbin/iptables-legacy 2>/dev/null || true
  cloud_agent_run_as_root update-alternatives --set ip6tables /usr/sbin/ip6tables-legacy 2>/dev/null || true
}

cloud_agent_start_dockerd_if_needed() {
  if cloud_agent_docker_daemon_responds; then
    return 0
  fi
  cloud_agent_fix_docker_socket_permissions && cloud_agent_docker_daemon_responds && return 0
  if ! command -v dockerd >/dev/null 2>&1; then
    return 1
  fi
  echo "[cloud-agent] starting dockerd (systemd unavailable on some Cloud VMs)…"
  if ! pgrep -x dockerd >/dev/null 2>&1; then
    cloud_agent_run_as_root nohup dockerd >"${CLOUD_AGENT_DOCKERD_LOG}" 2>&1 &
  fi
  for _ in $(seq 1 30); do
    sleep 1
    cloud_agent_fix_docker_socket_permissions || true
    cloud_agent_docker_daemon_responds && return 0
  done
  echo "[cloud-agent] dockerd failed to start (see ${CLOUD_AGENT_DOCKERD_LOG})." >&2
  return 1
}

cloud_agent_install_docker() {
  cloud_agent_install_docker_packages || true
  if ! command -v docker >/dev/null 2>&1; then
    echo "[cloud-agent] docker CLI not available — LocalTerra stack will be skipped." >&2
    return 1
  fi

  cloud_agent_fix_docker_socket_permissions || true
  local tried_fuse_overlayfs=false
  if ! cloud_agent_docker_daemon_responds; then
    cloud_agent_write_docker_storage_driver "fuse-overlayfs"
    tried_fuse_overlayfs=true
    cloud_agent_start_dockerd_if_needed || true
    cloud_agent_fix_docker_socket_permissions || true
  fi

  if cloud_agent_docker_daemon_responds && ! cloud_agent_docker_run_hello; then
    local hello_err
    hello_err="$(cloud_agent_docker_run_hello_err || true)"
    if [[ "$tried_fuse_overlayfs" == "true" ]] || cloud_agent_docker_hello_err_storage_related "$hello_err"; then
      echo "[cloud-agent] Docker hello-world failed with fuse-overlayfs — trying vfs…"
      cloud_agent_run_as_root pkill -x dockerd 2>/dev/null || true
      sleep 2
      cloud_agent_write_docker_storage_driver "vfs"
      cloud_agent_start_dockerd_if_needed || true
      cloud_agent_fix_docker_socket_permissions || true
    fi
  fi

  if cloud_agent_docker_daemon_responds && cloud_agent_docker_run_hello; then
    echo "[cloud-agent] Docker OK"
    return 0
  fi

  echo "[cloud-agent] Docker not usable for agent user — LocalTerra stack will be skipped." >&2
  return 1
}

cloud_agent_tmux_cmd() {
  local TMUX_CONF="${TMUX_CONF:-/exec-daemon/tmux.portal.conf}"
  if [[ -f "$TMUX_CONF" ]]; then
    tmux -f "$TMUX_CONF" "$@"
  else
    tmux "$@"
  fi
}

cloud_agent_ensure_dockerd() {
  if cloud_agent_docker_daemon_responds; then
    return 0
  fi
  echo "[cloud-agent] starting dockerd…"
  if ! pgrep -x dockerd >/dev/null 2>&1; then
    if cloud_agent_tmux_cmd has-session -t dockerd 2>/dev/null; then
      cloud_agent_tmux_cmd kill-session -t dockerd 2>/dev/null || true
    fi
    if [[ -f /exec-daemon/tmux.portal.conf ]]; then
      cloud_agent_tmux_cmd new-session -d -s dockerd "sudo dockerd > /tmp/dockerd.log 2>&1"
    else
      cloud_agent_run_as_root nohup dockerd >"${CLOUD_AGENT_DOCKERD_LOG}" 2>&1 &
    fi
  fi
  for _ in $(seq 1 60); do
    if cloud_agent_docker_daemon_responds; then
      echo "[cloud-agent] dockerd ready"
      return 0
    fi
    sleep 2
  done
  echo "[cloud-agent] ERROR: dockerd did not become ready. See ${CLOUD_AGENT_DOCKERD_LOG} or /tmp/dockerd.log" >&2
  return 1
}

cloud_agent_docker_compose() {
  if groups 2>/dev/null | grep -qw docker; then
    docker compose "$@"
  else
    sg docker -c "docker compose $(printf '%q ' "$@")"
  fi
}
