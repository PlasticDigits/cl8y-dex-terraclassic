#!/usr/bin/env bash
# Install Google Chrome (when missing) and the Keplr wallet extension for Cloud Agent VMs.
#
# Cloud Agent manual QA and wallet flows expect Keplr in the default Chrome profile
# without agents hand-copying /tmp/keplr-extension on every session.
#
# Usage (from repo root):
#   ./scripts/setup-browser-cloud-agent.sh
#
# Optional env:
#   CHROME_USER_DATA_DIR  — Chrome profile parent (default: ~/.config/google-chrome)
#   CHROME_PROFILE        — profile name under user-data-dir (default: Default)
#   SKIP_CHROME_INSTALL=1 — do not apt-install google-chrome-stable when missing
#   KEPLR_CACHE_DIR       — unpacked extension cache (default: /tmp/keplr-extension)
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/lib/keplr-chrome-extension.sh
source "${REPO_ROOT}/scripts/lib/keplr-chrome-extension.sh"

CHROME_USER_DATA_DIR="${CHROME_USER_DATA_DIR:-${HOME}/.config/google-chrome}"
CHROME_PROFILE="${CHROME_PROFILE:-Default}"
CHROME_PROFILE_DIR="${CHROME_USER_DATA_DIR}/${CHROME_PROFILE}"

chrome_binary() {
  if command -v google-chrome >/dev/null 2>&1; then
    command -v google-chrome
    return 0
  fi
  if command -v google-chrome-stable >/dev/null 2>&1; then
    command -v google-chrome-stable
    return 0
  fi
  return 1
}

install_google_chrome() {
  if chrome_binary >/dev/null 2>&1; then
    return 0
  fi
  if [[ "${SKIP_CHROME_INSTALL:-0}" == "1" ]]; then
    echo "[setup-browser] google-chrome not found (SKIP_CHROME_INSTALL=1)." >&2
    return 1
  fi
  if ! command -v curl >/dev/null 2>&1 || ! command -v dpkg >/dev/null 2>&1; then
    echo "[setup-browser] curl and dpkg required to install google-chrome." >&2
    return 1
  fi

  local arch deb_url deb_path
  arch="$(uname -m)"
  case "$arch" in
    x86_64) deb_url="https://dl.google.com/linux/direct/google-chrome-stable_current_amd64.deb" ;;
    aarch64 | arm64) deb_url="https://dl.google.com/linux/direct/google-chrome-stable_current_arm64.deb" ;;
    *)
      echo "[setup-browser] unsupported architecture for Chrome install: ${arch}" >&2
      return 1
      ;;
  esac

  deb_path="/tmp/google-chrome-stable.deb"
  echo "[setup-browser] installing google-chrome-stable (${arch})…"
  curl -fsSL -o "$deb_path" "$deb_url"
  sudo dpkg -i "$deb_path" || sudo apt-get install -f -y
  rm -f "$deb_path"
}

install_google_chrome

chrome_bin="$(chrome_binary)" || {
  echo "[setup-browser] google-chrome is not available after install attempt." >&2
  exit 1
}

echo "[setup-browser] chrome: $("$chrome_bin" --version 2>/dev/null | head -1)"
echo "[setup-browser] profile: ${CHROME_PROFILE_DIR}"

installed_ver="$(keplr_install_into_chrome_profile "$CHROME_PROFILE_DIR")"
ext_path="${CHROME_PROFILE_DIR}/Extensions/${KEPLR_EXTENSION_ID}/${installed_ver}"

if ! keplr_extension_dir_ready "$ext_path"; then
  echo "[setup-browser] Keplr install verification failed: ${ext_path}" >&2
  exit 1
fi

echo "[setup-browser] Keplr ${installed_ver} → ${ext_path}"
echo "[setup-browser] OK — launch Chrome for wallet QA:"
echo "  google-chrome --no-sandbox --disable-dev-shm-usage --disable-gpu http://127.0.0.1:5173"
