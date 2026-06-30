#!/usr/bin/env python3
"""Verify pre-deploy test evidence gate is documented for SEC-H08 (GitLab #444)."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LAUNCH_RUNBOOK = ROOT / "docs/runbooks/launch-checklist.md"
DEPLOYMENT_GUIDE = ROOT / "docs/deployment-guide.md"
TEMPLATE = ROOT / "docs/templates/deploy-trace.md"
QA_INVARIANTS = ROOT / "docs/qa-invariants.md"
SKILL = ROOT / "skills/AGENTS_TEST_EVIDENCE_GATE.md"

REQUIRED_MARKERS: tuple[tuple[Path, str], ...] = (
    (LAUNCH_RUNBOOK, "Test evidence gate (SEC-H08)"),
    (LAUNCH_RUNBOOK, "make test-contracts"),
    (LAUNCH_RUNBOOK, "make test-indexer-integration"),
    (LAUNCH_RUNBOOK, "make test-frontend"),
    (LAUNCH_RUNBOOK, "smoke-pool-swap"),
    (LAUNCH_RUNBOOK, "CI-built artifacts"),
    (LAUNCH_RUNBOOK, "verify-issue-444"),
    (DEPLOYMENT_GUIDE, "SEC-H08"),
    (DEPLOYMENT_GUIDE, "make test-contracts"),
    (DEPLOYMENT_GUIDE, "make test-indexer-integration"),
    (DEPLOYMENT_GUIDE, "make test-frontend"),
    (TEMPLATE, "Test results (pre-deploy evidence — SEC-H08)"),
    (TEMPLATE, "make test-contracts"),
    (TEMPLATE, "make test-indexer-integration"),
    (TEMPLATE, "make test-frontend"),
    (TEMPLATE, "CI-built artifacts"),
    (QA_INVARIANTS, "Q3"),
    (QA_INVARIANTS, "SEC-H08"),
    (SKILL, "SEC-H08"),
    (SKILL, "make test-contracts"),
    (SKILL, "make test-indexer-integration"),
    (SKILL, "make test-frontend"),
)


def main() -> int:
    errors: list[str] = []

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
        "OK: pre-deploy test evidence gate (SEC-H08) documented in launch runbook, "
        "deployment guide, deploy trace template, and qa-invariants"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
