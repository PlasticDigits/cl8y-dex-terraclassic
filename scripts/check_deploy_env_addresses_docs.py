#!/usr/bin/env python3
"""Verify env/chain address cross-check is documented for SEC-H04 (GitLab #442)."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
LAUNCH_RUNBOOK = ROOT / "docs/runbooks/launch-checklist.md"
DEPLOYMENT_GUIDE = ROOT / "docs/deployment-guide.md"
TEMPLATE = ROOT / "docs/templates/deploy-trace.md"
QA_INVARIANTS = ROOT / "docs/qa-invariants.md"
SKILL = ROOT / "skills/AGENTS_DEPLOY_ENV_ADDRESSES_VERIFY.md"
SCRIPT = ROOT / "scripts/qa/verify-env-addresses.sh"

REQUIRED_MARKERS: tuple[tuple[Path, str], ...] = (
    (LAUNCH_RUNBOOK, "verify-env-addresses.sh"),
    (LAUNCH_RUNBOOK, "Phase 4 — Off-chain stack"),
    (LAUNCH_RUNBOOK, "SEC-H04"),
    (DEPLOYMENT_GUIDE, "verify-env-addresses.sh"),
    (TEMPLATE, "verify-env-addresses.sh"),
    (TEMPLATE, "SEC-H04"),
    (QA_INVARIANTS, "Q4"),
    (QA_INVARIANTS, "verify-env-addresses"),
    (SKILL, "SEC-H04"),
    (SKILL, "verify-env-addresses.sh"),
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
        "OK: env address cross-check (SEC-H04) documented in launch runbook, "
        "deployment guide, deploy trace template, and qa-invariants"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
