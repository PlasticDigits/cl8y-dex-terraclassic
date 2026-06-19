#!/usr/bin/env python3
"""Verify launch runbook contains SEC-A06 go/no-go gate (GitLab #391)."""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RUNBOOK = ROOT / "docs/runbooks/launch-checklist.md"
QA_TEMPLATE = ROOT / "QA_TEMPLATE.md"
SKILL = ROOT / "skills/AGENTS_LAUNCH_GO_NO_GO.md"


def main() -> int:
    if not RUNBOOK.is_file():
        print(f"ERROR: missing {RUNBOOK}", file=sys.stderr)
        return 1

    text = RUNBOOK.read_text()
    errors: list[str] = []

    required_headings = [
        "Phase 5 — Go / no-go decision",
        "### Decision outcomes",
        "### BLOCK — do not launch",
        "### PAUSE — delay launch",
        "### GO with accepted risk",
        "### Mandatory sign-off gate",
    ]
    for heading in required_headings:
        if heading not in text:
            errors.append(f"missing heading: {heading}")

    for decision in ("**BLOCK**", "**PAUSE**", "**GO**", "**GO with accepted risk**"):
        if decision not in text:
            errors.append(f"missing decision label: {decision}")

    p0_categories = (
        "Admin controls",
        "Value-flow invariants",
        "Deploy / runbook",
        "User visibility of risk",
    )
    for category in p0_categories:
        if category not in text:
            errors.append(f"BLOCK criteria missing P0 category: {category}")

    if "All P0" not in text and "all P0" not in text:
        errors.append("GO criteria must reference all P0 items closed")

    if "residual risk" not in text.lower():
        errors.append("GO with accepted risk must document residual risks")

    if "QA_TEMPLATE.md" not in text:
        errors.append("sign-off must reference QA_TEMPLATE.md")

    if "SIGN-OFF" not in text:
        errors.append("sign-off must reference QA_TEMPLATE SIGN-OFF section")

    if "Dev Lead" not in text or "Product Owner" not in text:
        errors.append("sign-off must embed QA_TEMPLATE role table (Dev Lead, Product Owner)")

    if "launch tracking issue" not in text:
        errors.append("sign-off must require posting on launch tracking issue")

    if "verify-issue-391" not in text:
        errors.append("runbook must reference make verify-issue-391")

    # Phase 5 must follow Phase 4 (final required gate)
    phase4 = text.find("## Phase 4 — Off-chain stack")
    phase5 = text.find("## Phase 5 — Go / no-go decision")
    rollback = text.find("## Rollback / incident")
    if phase4 < 0 or phase5 < 0 or rollback < 0:
        errors.append("missing Phase 4, Phase 5, or Rollback section")
    elif not (phase4 < phase5 < rollback):
        errors.append("Phase 5 must appear after Phase 4 and before Rollback / incident")

    if not QA_TEMPLATE.is_file():
        errors.append(f"missing {QA_TEMPLATE}")
    elif "## SIGN-OFF" not in QA_TEMPLATE.read_text():
        errors.append("QA_TEMPLATE.md must contain ## SIGN-OFF")

    if not SKILL.is_file():
        errors.append(f"missing agent skill {SKILL}")

    mandatory_gate = re.search(
        r"\*\*Mandatory gate:\*\*.*Phase 5.*required sign-off",
        text,
        re.DOTALL,
    )
    if not mandatory_gate:
        errors.append("runbook intro must declare Phase 5 mandatory gate before production mainnet")

    if errors:
        for err in errors:
            print(f"ERROR: {err}", file=sys.stderr)
        return 1

    print("OK: launch runbook go/no-go gate (SEC-A06) documented")
    return 0


if __name__ == "__main__":
    sys.exit(main())
