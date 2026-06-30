#!/usr/bin/env python3
"""Verify SEC-D10 governance key rotation docs (GitLab #408)."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RUNBOOK = ROOT / "docs/runbooks/governance-key-rotation.md"
LAUNCH = ROOT / "docs/runbooks/launch-checklist.md"
WASM = ROOT / "docs/runbooks/wasm-admin-migration.md"
SKILL = ROOT / "skills/AGENTS_GOVERNANCE_KEY_ROTATION.md"
SCRIPT = ROOT / "scripts/rehearse-governance-key-rotation.sh"
MAKEFILE = ROOT / "Makefile"


def main() -> int:
    errors: list[str] = []

    required_files = (RUNBOOK, SKILL, SCRIPT)
    for path in required_files:
        if not path.is_file():
            errors.append(f"missing {path.relative_to(ROOT)}")

    if RUNBOOK.is_file():
        text = RUNBOOK.read_text()
        for marker in (
            "SEC-D10",
            "#408",
            "DEX-P2-026",
            "set-contract-admin",
            "update_config",
            "governance",
            "multisign",
            "rehearse-governance-key-rotation.sh",
            "## 2. Rotate the wasm contract-admin",
            "## 3. Rotate the factory governance pointer",
            "## 4. Post-rotation verification",
        ):
            if marker not in text:
                errors.append(f"{RUNBOOK.relative_to(ROOT)} missing {marker!r}")

    if LAUNCH.is_file():
        launch = LAUNCH.read_text()
        if "governance-key-rotation.md" not in launch:
            errors.append("launch-checklist.md must link governance-key-rotation.md")
        if "SEC-D10" not in launch:
            errors.append("launch-checklist.md must reference SEC-D10")
        if "verify-issue-408" not in launch:
            errors.append("launch-checklist.md must reference make verify-issue-408")

    if WASM.is_file():
        wasm = WASM.read_text()
        if "governance-key-rotation.md" not in wasm:
            errors.append("wasm-admin-migration.md must link governance-key-rotation.md")

    if MAKEFILE.is_file():
        mk = MAKEFILE.read_text()
        if "verify-issue-408" not in mk:
            errors.append("Makefile must define verify-issue-408")
        if "check-governance-key-rotation-docs" not in mk:
            errors.append("Makefile must define check-governance-key-rotation-docs")
        if "rehearse-governance-key-rotation" not in mk:
            errors.append("Makefile must define rehearse-governance-key-rotation")

    if errors:
        for err in errors:
            print(f"ERROR: {err}", file=sys.stderr)
        return 1

    print("OK: governance key rotation docs (SEC-D10)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
