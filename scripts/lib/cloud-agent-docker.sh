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

cloud_agent_docker_installed() {
  command -v docker >/dev/null 2>&1
}

cloud_agent_docker_group_exists() {
  getent group docker >/dev/null 2>&1
}

cloud_agent_in_docker_group() {
  groups 2>/dev/null | grep -qw docker
}

cloud_agent_run_docker() {
  if cloud_agent_in_docker_group && docker "$@" 2>/dev/null; then
    return $?
  fi
  if cloud_agent_docker_group_exists; then
    sg docker -c "docker $(printf '%q ' "$@")"
    return $?
  fi
  docker "$@"
}

cloud_agent_configure_docker_daemon() {
  local daemon_json="/etc/docker/daemon.json"
  local want_storage="fuse-overlayfs"
  if [[ -f "$daemon_json" ]] && grep -q "\"storage-driver\"[[:space:]]*:[[:space:]]*\"${want_storage}\"" "$daemon_json" 2>/dev/null; then
    return 0
  fi
  echo "[cloud-agent-docker] configuring ${daemon_json} (storage-driver=${want_storage})…"
  sudo mkdir -p /etc/docker
  if [[ -f "$daemon_json" ]]; then
    sudo cp "$daemon_json" "${daemon_json}.bak.$(date +%s)" 2>/dev/null || true
  fi
  printf '{\n  "storage-driver": "%s"\n}\n' "$want_storage" | sudo tee "$daemon_json" >/dev/null
}

cloud_agent_configure_iptables_legacy() {
  if [[ ! -x /usr/sbin/iptables-legacy ]]; then
    return 0
  fi
  if sudo update-alternatives --query iptables 2>/dev/null | grep -q 'Value: /usr/sbin/iptables-legacy'; then
    return 0
  fi
  echo "[cloud-agent-docker] switching to iptables-legacy…"
  sudo update-alternatives --set iptables /usr/sbin/iptables-legacy 2>/dev/null || true
  if [[ -x /usr/sbin/ip6tables-legacy ]]; then
    sudo update-alternatives --set ip6tables /usr/sbin/ip6tables-legacy 2>/dev/null || true
  fi
}

cloud_agent_install_docker() {
  if cloud_agent_docker_installed; then
    return 0
  fi
  if ! command -v curl >/dev/null 2>&1 || ! command -v apt-get >/dev/null 2>&1; then
    echo "[cloud-agent-docker] ERROR: curl and apt-get required to install Docker CE" >&2
    return 1
  fi

  local arch codename
  arch="$(dpkg --print-architecture)"
  # shellcheck disable=SC1091
  codename="$(. /etc/os-release && echo "${VERSION_CODENAME:-}")"
  if [[ -z "$codename" ]]; then
    echo "[cloud-agent-docker] ERROR: could not detect Ubuntu codename" >&2
    return 1
  fi

  echo "[cloud-agent-docker] installing Docker CE (${codename}/${arch})…"
  sudo DEBIAN_FRONTEND=noninteractive apt-get update -qq
  sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq \
    ca-certificates curl gnupg fuse-overlayfs iptables

  sudo install -m 0755 -d /etc/apt/keyrings
  curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo gpg --dearmor -o /etc/apt/keyrings/docker.gpg
  sudo chmod a+r /etc/apt/keyrings/docker.gpg
  echo "deb [arch=${arch} signed-by=/etc/apt/keyrings/docker.gpg] https://download.docker.com/linux/ubuntu ${codename} stable" \
    | sudo tee /etc/apt/sources.list.d/docker.list >/dev/null

  sudo DEBIAN_FRONTEND=noninteractive apt-get update -qq
  sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq \
    docker-ce docker-ce-cli containerd.io docker-buildx-plugin docker-compose-plugin

  cloud_agent_configure_iptables_legacy
  cloud_agent_configure_docker_daemon

  if ! getent group docker >/dev/null 2>&1; then
    sudo groupadd docker
  fi
  sudo usermod -aG docker "${USER:-ubuntu}" 2>/dev/null || true

  echo "[cloud-agent-docker] Docker CE installed: $(docker --version 2>/dev/null || echo unknown)"
}

cloud_agent_ensure_docker_access() {
  cloud_agent_install_docker || return 1
  if ! cloud_agent_docker_group_exists; then
    echo "[cloud-agent-docker] ERROR: docker group missing after install" >&2
    return 1
  fi
  if ! cloud_agent_in_docker_group; then
    sudo usermod -aG docker "${USER:-ubuntu}" 2>/dev/null || true
  fi
  return 0
}

cloud_agent_docker_reachable() {
  if cloud_agent_in_docker_group && docker info >/dev/null 2>&1; then
    return 0
  fi
  if cloud_agent_docker_group_exists && sg docker -c 'docker info' >/dev/null 2>&1; then
    return 0
  fi
  return 1
}

cloud_agent_ensure_dockerd() {
  cloud_agent_ensure_docker_access || return 1

  if cloud_agent_docker_reachable; then
    return 0
  fi

  echo "[cloud-agent-docker] starting dockerd in tmux session 'dockerd'…"
  if ! cloud_agent_tmux_cmd has-session -t dockerd 2>/dev/null; then
    cloud_agent_tmux_cmd new-session -d -s dockerd "sudo dockerd > /tmp/dockerd.log 2>&1"
  fi
  for _ in $(seq 1 60); do
    if cloud_agent_docker_reachable; then
      echo "[cloud-agent-docker] dockerd ready"
      return 0
    fi
    sleep 2
  done
  echo "[cloud-agent-docker] ERROR: dockerd did not become ready. See /tmp/dockerd.log" >&2
  return 1
}

cloud_agent_docker_compose() {
  if cloud_agent_in_docker_group; then
    docker compose "$@"
  elif cloud_agent_docker_group_exists; then
    sg docker -c "docker compose $(printf '%q ' "$@")"
  else
    docker compose "$@"
  fi
}
