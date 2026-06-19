#!/usr/bin/env python3
"""Verify deploy trace recording is documented for SEC-D12 (GitLab #410)."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LAUNCH_RUNBOOK = ROOT / "docs/runbooks/launch-checklist.md"
MIGRATION_RUNBOOK = ROOT / "docs/runbooks/wasm-admin-migration.md"
DEPLOYMENT_GUIDE = ROOT / "docs/deployment-guide.md"
TEMPLATE = ROOT / "docs/templates/deploy-trace.md"
SKILL = ROOT / "skills/AGENTS_DEPLOY_TRACE.md"

REQUIRED_MARKERS: tuple[tuple[Path, str], ...] = (
    (LAUNCH_RUNBOOK, "Deploy trace (audit record)"),
    (LAUNCH_RUNBOOK, "git rev-parse HEAD"),
    (LAUNCH_RUNBOOK, "terrad version"),
    (LAUNCH_RUNBOOK, "wasm-checksums.txt"),
    (LAUNCH_RUNBOOK, "Contract code IDs"),
    (LAUNCH_RUNBOOK, "Post-deploy verification"),
    (LAUNCH_RUNBOOK, "deploy-trace.md"),
    (LAUNCH_RUNBOOK, "verify-issue-410"),
    (MIGRATION_RUNBOOK, "Deploy trace (SEC-D12)"),
    (MIGRATION_RUNBOOK, "git rev-parse HEAD"),
    (MIGRATION_RUNBOOK, "terrad version"),
    (MIGRATION_RUNBOOK, "wasm-checksums.txt"),
    (MIGRATION_RUNBOOK, "Contract code IDs"),
    (MIGRATION_RUNBOOK, "deploy-trace.md"),
    (DEPLOYMENT_GUIDE, "## Deploy trace (audit record)"),
    (DEPLOYMENT_GUIDE, "deploy-trace.md"),
    (TEMPLATE, "## Deploy trace —"),
    (TEMPLATE, "Git SHA"),
    (TEMPLATE, "Terra Classic chain version"),
    (TEMPLATE, "wasm-checksums.txt"),
    (TEMPLATE, "Contract code IDs"),
    (TEMPLATE, "Post-deploy verification"),
)


def main() -> int:
    errors: list[str] = []

    for path, marker in REQUIRED_MARKERS:
        if not path.is_file():
            errors.append(f"missing {path.relative_to(ROOT)}")
            continue
        if marker not in path.read_text():
            errors.append(f"{path.relative_to(ROOT)} missing required content: {marker!r}")

    if not SKILL.is_file():
        errors.append(f"missing {SKILL.relative_to(ROOT)}")

    if errors:
        for err in errors:
            print(f"ERROR: {err}", file=sys.stderr)
        return 1

    print(
        "OK: deploy trace recording (SEC-D12) documented in launch runbook, "
        "migration runbook, deployment guide, and template"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
