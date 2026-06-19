#!/usr/bin/env python3
"""Verify IBC-hooks deploy runbook gate (SEC-D02, GitLab #407)."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RUNBOOK = ROOT / "docs/runbooks/launch-checklist.md"
DEPLOY_GUIDE = ROOT / "docs/deployment-guide.md"
SECURITY_MODEL = ROOT / "docs/security-model.md"
SKILL = ROOT / "skills/AGENTS_IBC_HOOKS_DEPLOY.md"
VERIFY_SCRIPT = ROOT / "scripts/verify-no-ibc-hooks-in-contracts.sh"


def fail(msg: str) -> None:
    print(f"ERROR: {msg}", file=sys.stderr)
    sys.exit(1)


def main() -> int:
    if not RUNBOOK.is_file():
        fail(f"missing {RUNBOOK.relative_to(ROOT)}")

    runbook = RUNBOOK.read_text()
    required_runbook_markers: tuple[str, ...] = (
        "SEC-D02",
        "IBC-hooks chain exposure",
        "terrad version --long",
        "verify-no-ibc-hooks-in-contracts",
        "chain upgrade",
        "new contract modules",
        "ibc_receive",
        "ibc_ack",
        "ibc_timeout",
        "verify-issue-407",
    )
    for marker in required_runbook_markers:
        if marker not in runbook:
            fail(f"{RUNBOOK.relative_to(ROOT)} missing required content: {marker!r}")

    if "Phase 0" not in runbook or runbook.find("SEC-D02") > runbook.find("## Phase 1"):
        fail("SEC-D02 gate must appear in Phase 0 before Phase 1")

    if not DEPLOY_GUIDE.is_file():
        fail(f"missing {DEPLOY_GUIDE.relative_to(ROOT)}")

    deploy_text = DEPLOY_GUIDE.read_text()
    if "SEC-D02" not in deploy_text:
        fail("docs/deployment-guide.md Post-Deployment Checklist must reference SEC-D02")
    if "verify-no-ibc-hooks-in-contracts" not in deploy_text:
        fail("deployment-guide must reference make verify-no-ibc-hooks-in-contracts")

    if not SECURITY_MODEL.is_file():
        fail(f"missing {SECURITY_MODEL.relative_to(ROOT)}")

    security_text = SECURITY_MODEL.read_text()
    if "### IBC hooks chain dependency (SEC-D02)" not in security_text:
        fail("security-model must define IBC hooks SEC-D02 section")
    if "ibc_receive" not in security_text:
        fail("security-model must document absence of ibc_receive entry points")

    if not SKILL.is_file():
        fail(f"missing agent skill {SKILL.relative_to(ROOT)}")

    if not VERIFY_SCRIPT.is_file():
        fail(f"missing {VERIFY_SCRIPT.relative_to(ROOT)}")

    print(
        "OK: IBC-hooks deploy gate (SEC-D02) documented in launch runbook, "
        "deployment guide, security-model, and agent skill"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
