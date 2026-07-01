#!/usr/bin/env python3
"""Verify FACTORY_ADDRESS non-empty guard is documented for SEC-I02 (GitLab #451)."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LAUNCH_RUNBOOK = ROOT / "docs/runbooks/launch-checklist.md"
DEPLOYMENT_GUIDE = ROOT / "docs/deployment-guide.md"
OPERATOR_SECRETS = ROOT / "docs/operator-secrets.md"
INDEXER_INVARIANTS = ROOT / "docs/indexer-invariants.md"
QA_INVARIANTS = ROOT / "docs/qa-invariants.md"
SKILL = ROOT / "skills/AGENTS_FACTORY_ADDRESS_GUARD.md"
VERIFY_DEPLOY = ROOT / "scripts/qa/verify-deploy.sh"

REQUIRED_MARKERS: tuple[tuple[Path, str], ...] = (
    (LAUNCH_RUNBOOK, "SEC-I02"),
    (LAUNCH_RUNBOOK, "Phase 0 — Preconditions"),
    (LAUNCH_RUNBOOK, "FACTORY_ADDRESS"),
    (DEPLOYMENT_GUIDE, "SEC-I02"),
    (OPERATOR_SECRETS, "EmptyFactoryAddress"),
    (INDEXER_INVARIANTS, "EmptyFactoryAddress"),
    (INDEXER_INVARIANTS, "#451"),
    (QA_INVARIANTS, "Q5"),
    (SKILL, "SEC-I02"),
    (SKILL, "EmptyFactoryAddress"),
)


def main() -> int:
    errors: list[str] = []

    if not VERIFY_DEPLOY.is_file():
        errors.append(f"missing {VERIFY_DEPLOY.relative_to(ROOT)}")

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
        "OK: FACTORY_ADDRESS non-empty guard (SEC-I02) documented in launch runbook, "
        "operator secrets, indexer/qa invariants, and agent skill"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
