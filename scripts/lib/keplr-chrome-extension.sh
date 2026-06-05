#!/usr/bin/env bash
# Keplr Chrome extension download + profile install helpers for Cloud Agent VMs.
# Sourced by scripts/setup-browser-cloud-agent.sh (not meant to run standalone).
set -euo pipefail

KEPLR_EXTENSION_ID="${KEPLR_EXTENSION_ID:-dmkamcknogkgcdfhhbddcghachkejeap}"
KEPLR_CWS_UPDATE_URL="${KEPLR_CWS_UPDATE_URL:-https://clients2.google.com/service/update2/crx?response=redirect&prodversion=131.0&acceptformat=crx2,crx3&x=id%3D${KEPLR_EXTENSION_ID}%26uc}"
KEPLR_CACHE_DIR="${KEPLR_CACHE_DIR:-/tmp/keplr-extension}"
KEPLR_CRX_CACHE="${KEPLR_CRX_CACHE:-/tmp/keplr-extension.crx}"

# Return 0 when $1 looks like a complete unpacked Keplr CRX (not manifest-only cache).
keplr_extension_dir_ready() {
  local dir="$1"
  [[ -d "$dir" ]] || return 1
  python3 - "$dir" <<'PY'
import json
import sys
from pathlib import Path

root = Path(sys.argv[1])
manifest_path = root / "manifest.json"
if not manifest_path.is_file():
    raise SystemExit(1)
try:
    manifest = json.loads(manifest_path.read_text())
except json.JSONDecodeError:
    raise SystemExit(1)
if not isinstance(manifest.get("version"), str) or not manifest.get("version"):
    raise SystemExit(1)
if not isinstance(manifest.get("name"), str) or not manifest.get("name"):
    raise SystemExit(1)
# A real CRX unpack has many files; manifest-only trees are incomplete caches.
if len([p for p in root.rglob("*") if p.is_file()]) < 3:
    raise SystemExit(1)
PY
}

# Download CRX from the Chrome Web Store (cached at KEPLR_CRX_CACHE).
keplr_download_crx() {
  if [[ -f "$KEPLR_CRX_CACHE" ]] && file -b "$KEPLR_CRX_CACHE" | grep -qi 'chrome extension'; then
    return 0
  fi
  echo "[setup-browser] downloading Keplr extension (${KEPLR_EXTENSION_ID})…" >&2
  curl -fsSL -o "$KEPLR_CRX_CACHE" "$KEPLR_CWS_UPDATE_URL"
  if ! file -b "$KEPLR_CRX_CACHE" | grep -qi 'chrome extension'; then
    echo "[setup-browser] downloaded file is not a Chrome extension: ${KEPLR_CRX_CACHE}" >&2
    rm -f "$KEPLR_CRX_CACHE"
    return 1
  fi
}

# Unpack CRX into KEPLR_CACHE_DIR (idempotent when manifest already present).
keplr_unpack_crx_to_cache() {
  if keplr_extension_dir_ready "$KEPLR_CACHE_DIR"; then
    return 0
  fi
  keplr_download_crx
  rm -rf "$KEPLR_CACHE_DIR"
  mkdir -p "$KEPLR_CACHE_DIR"
  python3 - "$KEPLR_CRX_CACHE" "$KEPLR_CACHE_DIR" <<'PY'
import io
import struct
import sys
import zipfile
from pathlib import Path

crx_path = Path(sys.argv[1])
dest = Path(sys.argv[2])
data = crx_path.read_bytes()
offset = 0
if data[:4] == b"Cr24":
    version = struct.unpack_from("<I", data, 4)[0]
    offset = 8
    if version == 2:
        pubkey_len, sig_len = struct.unpack_from("<II", data, offset)
        offset += 8 + pubkey_len + sig_len
    elif version == 3:
        header_size = struct.unpack_from("<I", data, offset)[0]
        offset += 4 + header_size
payload = data[offset:]
with zipfile.ZipFile(io.BytesIO(payload)) as zf:
    zf.extractall(dest)
if not (dest / "manifest.json").is_file():
    raise SystemExit("manifest.json missing after CRX unpack")
PY
  if ! keplr_extension_dir_ready "$KEPLR_CACHE_DIR"; then
    echo "[setup-browser] unpacked extension cache failed validation: ${KEPLR_CACHE_DIR}" >&2
    rm -rf "$KEPLR_CACHE_DIR"
    return 1
  fi
}

# Install unpacked extension into a Chrome profile and register it in Preferences.
# Args: chrome_profile_dir (e.g. ~/.config/google-chrome/Default)
keplr_install_into_chrome_profile() {
  local chrome_profile="$1"
  keplr_unpack_crx_to_cache

  python3 - "$KEPLR_CACHE_DIR" "$chrome_profile" "$KEPLR_EXTENSION_ID" <<'PY'
import json
import shutil
import sys
from pathlib import Path

src = Path(sys.argv[1])
profile = Path(sys.argv[2])
ext_id = sys.argv[3]

manifest = json.loads((src / "manifest.json").read_text())
version_dir = f"{manifest['version']}_0"
dest = profile / "Extensions" / ext_id / version_dir
prefs_path = profile / "Preferences"

dest.parent.mkdir(parents=True, exist_ok=True)
if dest.is_dir():
    shutil.rmtree(dest)
shutil.copytree(src, dest)

prefs: dict = {}
if prefs_path.is_file():
    prefs = json.loads(prefs_path.read_text())

settings = prefs.setdefault("extensions", {}).setdefault("settings", {})
settings[ext_id] = {
    "active_permissions": {
        "api": manifest.get("permissions", []),
        "explicit_host": manifest.get("host_permissions", []),
    },
    "creation_flags": 1,
    "from_webstore": True,
    "granted_permissions": {
        "api": manifest.get("permissions", []),
        "explicit_host": manifest.get("host_permissions", []),
    },
    "incognito": "split",
    "location": 1,
    "manifest": manifest,
    "path": f"{ext_id}/{version_dir}",
    "state": 1,
    "was_installed_by_default": False,
    "was_installed_by_oem": False,
    "install_time": settings.get(ext_id, {}).get("install_time", "0"),
    "update_time": "0",
}
prefs_path.parent.mkdir(parents=True, exist_ok=True)
prefs_path.write_text(json.dumps(prefs))
sys.stdout.write(version_dir)
PY
}
