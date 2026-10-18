#!/usr/bin/env python3
"""DEX #1544: root-only read-only collector inventory; never exports env values."""
import json
import os
from pathlib import Path
import re
import subprocess
import sys

KEYS = frozenset(("CL8Y_LOGGING_USERNAME", "CL8Y_LOGGING_PASSWORD", "LOKI_PUSH_URL", "CL8Y_LOGGING_ENDPOINT"))
OUTPUT = "/tmp/dex1544-collector-inventory.json"


def command(args):
    result = subprocess.run(args, capture_output=True, text=True, timeout=10)
    if result.returncode:
        raise ValueError("inventory_command_failed")
    if len(result.stdout) > 16 * 1024 * 1024:
        raise ValueError("inventory_bound")
    return result.stdout


def container_summary(containers):
    if not isinstance(containers, list) or len(containers) > 256:
        raise ValueError("container_bound")
    rows = []
    for item in containers:
        env = item.get("Config", {}).get("Env", []) or []
        keys = sorted({row.split("=", 1)[0] for row in env if row.split("=", 1)[0] in KEYS})
        if not keys:
            continue
        service = item.get("Config", {}).get("Labels", {}).get("com.docker.compose.service", "unknown")
        if not re.fullmatch(r"[a-zA-Z0-9_-]{1,64}", service):
            service = "unknown"
        rows.append({"compose_service": service, "credential_keys": keys,
                     "running": item.get("State", {}).get("Running") is True})
    return rows


def inventory():
    if os.geteuid() != 0:
        raise ValueError("operator_sudo_required")
    files = []
    for path in ("/etc/default/alloy", "/etc/cl8y-logs/ingest.env", "/etc/cl8y-logs/run.env"):
        source = Path(path)
        if not source.is_file() or source.is_symlink():
            files.append({"path": path, "present": False})
            continue
        body = source.read_text(encoding="utf-8")
        if len(body) > 65_536:
            raise ValueError("file_bound")
        keys = sorted({match[1] for match in re.finditer(r"(?m)^\s*(?:export\s+)?([A-Z_]+)=", body) if match[1] in KEYS})
        files.append({"path": path, "present": True, "credential_keys": keys,
                      "mode": oct(source.stat().st_mode & 0o777)})
    ids = command(["docker", "ps", "-aq"]).split()
    if len(ids) > 256 or any(not re.fullmatch(r"[0-9a-f]{12,64}", value) for value in ids):
        raise ValueError("container_inventory_bound")
    containers = json.loads(command(["docker", "inspect", *ids])) if ids else []
    unit = subprocess.run(["systemctl", "is-active", "alloy"], capture_output=True, text=True, timeout=5)
    data = {"read_only": True, "scope": "local host collector metadata only; no environment values or logs",
            "alloy_active": unit.stdout.strip() == "active", "files": files,
            "container_consumers": container_summary(containers)}
    descriptor = os.open(OUTPUT, os.O_WRONLY | os.O_CREAT | os.O_TRUNC | os.O_NOFOLLOW, 0o600)
    with os.fdopen(descriptor, "w") as output:
        os.fchmod(output.fileno(), 0o600)
        sudo_uid, sudo_gid = int(os.environ.get("SUDO_UID", "0")), int(os.environ.get("SUDO_GID", "0"))
        if sudo_uid > 0:
            os.fchown(output.fileno(), sudo_uid, sudo_gid)
        json.dump(data, output, indent=2)
    print("PASS: sanitized collector metadata saved to " + OUTPUT)


if __name__ == "__main__":
    try:
        inventory()
    except Exception:
        print("FAIL: collector inventory unavailable; no credentials or raw errors printed.", file=sys.stderr)
        raise SystemExit(1)
