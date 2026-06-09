#!/usr/bin/env bash
# Shared Node / Rust / apt toolchain helpers for Cursor Cloud Agent VMs.
# shellcheck shell=bash

# nvm.sh references optional vars; disable nounset while invoking nvm.
cloud_agent_nvm() {
  set +u
  local rc=0
  "$@" || rc=$?
  set -u
  return "$rc"
}

cloud_agent_nvmrc_version() {
  local repo_root="${1:-}"
  if [[ -z "$repo_root" ]]; then
    echo "cloud_agent_nvmrc_version: repo root required" >&2
    return 1
  fi
  tr -d '[:space:]' <"${repo_root}/.nvmrc"
}

cloud_agent_load_nvm() {
  export NVM_DIR="${NVM_DIR:-$HOME/.nvm}"
  if [[ -s "$NVM_DIR/nvm.sh" ]]; then
    # shellcheck source=/dev/null
    . "$NVM_DIR/nvm.sh"
    return 0
  elif command -v brew >/dev/null 2>&1; then
    local brew_nvm
    brew_nvm="$(brew --prefix nvm 2>/dev/null || true)"
    if [[ -n "$brew_nvm" && -s "$brew_nvm/nvm.sh" ]]; then
      # shellcheck source=/dev/null
      . "$brew_nvm/nvm.sh"
      return 0
    fi
  fi
  echo "[cloud-agent-toolchain] installing nvm…"
  curl -fsSL https://raw.githubusercontent.com/nvm-sh/nvm/v0.40.1/install.sh | bash
  if [[ -s "$NVM_DIR/nvm.sh" ]]; then
    # shellcheck source=/dev/null
    . "$NVM_DIR/nvm.sh"
    return 0
  fi
  echo "[cloud-agent-toolchain] nvm not found at ${NVM_DIR}/nvm.sh" >&2
  return 1
}

# Cloud VMs ship /exec-daemon/node (v22) on PATH before nvm; strip it so nvm wins.
cloud_agent_strip_exec_daemon_from_path() {
  local path_in="${1:-$PATH}"
  local part out=""
  local IFS=':'
  read -r -a parts <<<"$path_in"
  for part in "${parts[@]}"; do
    [[ -z "$part" || "$part" == "/exec-daemon" ]] && continue
    if [[ -n "$out" ]]; then
      out="${out}:"
    fi
    out="${out}${part}"
  done
  printf '%s' "$out"
}

cloud_agent_node_bin_dir() {
  local repo_root="${1:-}"
  local ver bin
  cloud_agent_load_nvm || return 1
  if [[ -n "$repo_root" ]]; then
    ver="$(cloud_agent_nvmrc_version "$repo_root")"
    bin="$(cloud_agent_nvm nvm which "$ver" 2>/dev/null || true)"
    if [[ -z "$bin" || ! -x "$bin" ]]; then
      cloud_agent_nvm nvm install "$ver" >/dev/null 2>&1
      cloud_agent_nvm nvm use "$ver" >/dev/null 2>&1
      bin="$(cloud_agent_nvm nvm which "$ver" 2>/dev/null || cloud_agent_nvm nvm which current 2>/dev/null || true)"
    fi
  else
    bin="$(cloud_agent_nvm nvm which current 2>/dev/null || true)"
  fi
  if [[ -z "$bin" || ! -x "$bin" ]]; then
    echo "[cloud-agent-toolchain] could not resolve nvm node binary" >&2
    return 1
  fi
  dirname "$bin"
}

cloud_agent_prepend_node_path() {
  local repo_root="${1:-}"
  local bin_dir cleaned
  bin_dir="$(cloud_agent_node_bin_dir "$repo_root")" || return 1
  cleaned="$(cloud_agent_strip_exec_daemon_from_path "$PATH")"
  export PATH="${bin_dir}:${cleaned}"
}

cloud_agent_ensure_node() {
  local repo_root="${1:?cloud_agent_ensure_node: repo root required}"
  local ver got
  cloud_agent_load_nvm || return 1
  ver="$(cloud_agent_nvmrc_version "$repo_root")"
  echo "[cloud-agent-toolchain] ensuring Node ${ver} (nvm)…"
  cloud_agent_nvm nvm install "$ver" >/dev/null 2>&1
  cloud_agent_nvm nvm alias default "$ver" >/dev/null 2>&1 || true
  cloud_agent_nvm nvm use "$ver" >/dev/null 2>&1
  cloud_agent_prepend_node_path "$repo_root"
  got="$(node -v 2>/dev/null || true)"
  got="${got#v}"
  if [[ "$got" != "$ver" && "$got" != "${ver}"* ]]; then
    echo "[cloud-agent-toolchain] ERROR: node v${got:-?} on PATH; want v${ver} from .nvmrc" >&2
    echo "[cloud-agent-toolchain]        node: $(command -v node 2>/dev/null || echo missing)" >&2
    return 1
  fi
  echo "[cloud-agent-toolchain] node: $(command -v node) ($(node -v))"
  return 0
}

cloud_agent_node_path_export() {
  local repo_root="${1:?cloud_agent_node_path_export: repo root required}"
  local bin_dir
  bin_dir="$(cloud_agent_node_bin_dir "$repo_root")" || return 1
  printf 'export PATH="%s:%s"\n' "$bin_dir" "$(cloud_agent_strip_exec_daemon_from_path "$PATH")"
}

cloud_agent_ensure_apt_packages() {
  local missing=()
  dpkg -s curl >/dev/null 2>&1 || missing+=(curl)
  dpkg -s libssl-dev >/dev/null 2>&1 || missing+=(libssl-dev)
  dpkg -s pkg-config >/dev/null 2>&1 || missing+=(pkg-config)
  if [[ ${#missing[@]} -eq 0 ]]; then
    return 0
  fi
  echo "[cloud-agent-toolchain] installing apt packages: ${missing[*]}…"
  sudo DEBIAN_FRONTEND=noninteractive apt-get update -qq
  sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq "${missing[@]}"
}

cloud_agent_ensure_rustup() {
  if command -v rustup >/dev/null 2>&1; then
    echo "[cloud-agent-toolchain] updating rustup stable toolchain…"
    rustup update stable >/dev/null || {
      echo "[cloud-agent-toolchain] WARNING: rustup update failed; using installed toolchain" >&2
    }
    return 0
  fi
  echo "[cloud-agent-toolchain] installing rustup (stable)…"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable --no-modify-path
}

cloud_agent_ensure_rust() {
  local cargo_ver major_minor
  cloud_agent_ensure_apt_packages
  cloud_agent_ensure_rustup
  export PATH="${HOME}/.cargo/bin:/usr/local/cargo/bin:${PATH}"
  if command -v rustup >/dev/null 2>&1; then
    echo "[cloud-agent-toolchain] rustup default stable…"
    rustup default stable >/dev/null
  fi
  if ! command -v cargo >/dev/null 2>&1; then
    echo "[cloud-agent-toolchain] ERROR: cargo not found after rustup install" >&2
    return 1
  fi
  cargo_ver="$(cargo --version | awk '{print $2}')"
  if [[ "$(printf '%s\n' "1.96.0" "$cargo_ver" | sort -V | head -1)" != "1.96.0" ]]; then
    echo "[cloud-agent-toolchain] ERROR: cargo ${cargo_ver} too old; need 1.96+ for edition2024 deps" >&2
    return 1
  fi
  echo "[cloud-agent-toolchain] rust: $(rustc --version) / $(cargo --version)"
  return 0
}

cloud_agent_ensure_frontend_deps() {
  local repo_root="${1:?cloud_agent_ensure_frontend_deps: repo root required}"
  local fe="$repo_root/frontend-dapp"
  if [[ -x "$fe/node_modules/.bin/vitest" ]]; then
    return 0
  fi
  echo "[cloud-agent-toolchain] installing frontend deps (npm ci)…"
  cloud_agent_ensure_node "$repo_root"
  bash "$repo_root/scripts/with-node.sh" --cwd frontend-dapp -- npm ci
}

cloud_agent_ensure_vm_toolchain() {
  local repo_root="${1:?cloud_agent_ensure_vm_toolchain: repo root required}"
  cloud_agent_ensure_node "$repo_root"
  cloud_agent_ensure_rust
}
