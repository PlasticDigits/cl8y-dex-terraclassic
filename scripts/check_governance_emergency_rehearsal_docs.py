#!/usr/bin/env python3
"""Verify SEC-B09 governance emergency rehearsal docs (GitLab #397)."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RUNBOOK = ROOT / "docs/runbooks/governance-emergency-rehearsal.md"
TEMPLATE = ROOT / "docs/templates/governance-emergency-rehearsal-evidence.md"
LAUNCH = ROOT / "docs/runbooks/launch-checklist.md"
SECURITY = ROOT / "docs/security-model.md"
SKILL = ROOT / "skills/AGENTS_GOVERNANCE_EMERGENCY_REHEARSAL.md"
SCRIPT = ROOT / "scripts/rehearse-governance-emergency-controls.sh"
MAKEFILE = ROOT / "Makefile"


def main() -> int:
    errors: list[str] = []

    required_files = (RUNBOOK, TEMPLATE, SKILL, SCRIPT)
    for path in required_files:
        if not path.is_file():
            errors.append(f"missing {path.relative_to(ROOT)}")

    if RUNBOOK.is_file():
        text = RUNBOOK.read_text()
        for marker in (
            "SEC-B09",
            "SetPairPaused",
            "BlacklistWallet",
            "UnblacklistWallet",
            "multisign",
            "rehearse-governance-emergency-controls.sh",
            "#397",
        ):
            if marker not in text:
                errors.append(f"{RUNBOOK.relative_to(ROOT)} missing {marker!r}")

    if TEMPLATE.is_file():
        tmpl = TEMPLATE.read_text()
        for marker in ("SetPairPaused", "BlacklistWallet", "UnblacklistWallet", "Tx hash"):
            if marker not in tmpl:
                errors.append(f"{TEMPLATE.relative_to(ROOT)} missing {marker!r}")

    if LAUNCH.is_file():
        launch = LAUNCH.read_text()
        if "governance-emergency-rehearsal.md" not in launch:
            errors.append("launch-checklist.md must link governance-emergency-rehearsal.md")
        if "SEC-B09" not in launch:
            errors.append("launch-checklist.md must reference SEC-B09")
        if "verify-issue-397" not in launch:
            errors.append("launch-checklist.md must reference make verify-issue-397")

    if SECURITY.is_file():
        sec = SECURITY.read_text()
        if "governance-emergency-rehearsal.md" not in sec:
            errors.append("security-model.md must link governance-emergency-rehearsal.md")

    if MAKEFILE.is_file():
        mk = MAKEFILE.read_text()
        if "verify-issue-397" not in mk:
            errors.append("Makefile must define verify-issue-397")
        if "rehearse-governance-emergency" not in mk:
            errors.append("Makefile must define rehearse-governance-emergency")

    if errors:
        for err in errors:
            print(f"ERROR: {err}", file=sys.stderr)
        return 1

    print("OK: governance emergency rehearsal docs (SEC-B09)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
