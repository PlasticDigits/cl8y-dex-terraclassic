#!/usr/bin/env python3
"""Verify SEC-B10 admin-key custody docs (GitLab #398)."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RUNBOOK = ROOT / "docs/runbooks/key-custody.md"
LAUNCH = ROOT / "docs/runbooks/launch-checklist.md"
SECURITY = ROOT / "docs/security-model.md"
SKILL = ROOT / "skills/AGENTS_KEY_CUSTODY.md"
MAKEFILE = ROOT / "Makefile"


def main() -> int:
    errors: list[str] = []

    required_files = (RUNBOOK, SKILL)
    for path in required_files:
        if not path.is_file():
            errors.append(f"missing {path.relative_to(ROOT)}")

    if RUNBOOK.is_file():
        text = RUNBOOK.read_text()
        for marker in (
            "SEC-B10",
            "#398",
            "single EOA",
            "threshold",
            "rotation",
            "backup signer",
            "## 1. Multisig type and threshold",
            "## 2. Signer roster",
            "## 3. Backup signer and escalation",
            "## 4. Key rotation",
            "update_admin",
            "UpdateConfig",
        ):
            if marker not in text:
                errors.append(f"{RUNBOOK.relative_to(ROOT)} missing {marker!r}")

    if LAUNCH.is_file():
        launch = LAUNCH.read_text()
        if "key-custody.md" not in launch:
            errors.append("launch-checklist.md must link key-custody.md")
        if "SEC-B10" not in launch:
            errors.append("launch-checklist.md must reference SEC-B10")
        if "verify-issue-398" not in launch:
            errors.append("launch-checklist.md must reference make verify-issue-398")

    if SECURITY.is_file():
        sec = SECURITY.read_text()
        if "key-custody.md" not in sec:
            errors.append("security-model.md must link key-custody.md")

    if MAKEFILE.is_file():
        mk = MAKEFILE.read_text()
        if "verify-issue-398" not in mk:
            errors.append("Makefile must define verify-issue-398")
        if "check-key-custody-docs" not in mk:
            errors.append("Makefile must define check-key-custody-docs")

    if errors:
        for err in errors:
            print(f"ERROR: {err}", file=sys.stderr)
        return 1

    print("OK: key custody docs (SEC-B10)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
