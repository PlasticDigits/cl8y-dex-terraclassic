#!/usr/bin/env python3
"""Verify post-deploy config verification is documented for SEC-H03 (GitLab #441)."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LAUNCH_RUNBOOK = ROOT / "docs/runbooks/launch-checklist.md"
DEPLOYMENT_GUIDE = ROOT / "docs/deployment-guide.md"
TEMPLATE = ROOT / "docs/templates/deploy-trace.md"
QA_INVARIANTS = ROOT / "docs/qa-invariants.md"
SKILL = ROOT / "skills/AGENTS_DEPLOY_CONFIG_VERIFY.md"
SCRIPT = ROOT / "scripts/qa/verify-deploy-config.sh"

REQUIRED_MARKERS: tuple[tuple[Path, str], ...] = (
    (LAUNCH_RUNBOOK, "verify-deploy-config.sh"),
    (LAUNCH_RUNBOOK, "Phase 3 — Post-deploy verification"),
    (LAUNCH_RUNBOOK, "SEC-H03"),
    (DEPLOYMENT_GUIDE, "verify-deploy-config.sh"),
    (TEMPLATE, "verify-deploy-config.sh"),
    (TEMPLATE, "SEC-H03"),
    (QA_INVARIANTS, "Q2"),
    (QA_INVARIANTS, "verify-deploy-config"),
    (SKILL, "SEC-H03"),
    (SKILL, "verify-deploy-config.sh"),
)


def main() -> int:
    errors: list[str] = []

    if not SCRIPT.is_file():
        errors.append(f"missing {SCRIPT.relative_to(ROOT)}")

    for path, marker in REQUIRED_MARKERS:
        if not path.is_file():
            errors.append(f"missing {path.relative_to(ROOT)}")
            continue
        if marker not in path.read_text():
            errors.append(f"{path.relative_to(ROOT)} missing required content: {marker!r}")

    if errors:
        for err in errors:
            print(f"ERROR: {err}", file=sys.stderr)
        return 1

    print(
        "OK: post-deploy config verification (SEC-H03) documented in launch runbook, "
        "deployment guide, deploy trace template, and qa-invariants"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
