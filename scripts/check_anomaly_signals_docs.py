#!/usr/bin/env python3
"""Verify anomaly signals runbook covers SEC-G02 (#435) and is cross-linked."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RUNBOOK = ROOT / "docs/runbooks/anomaly-signals.md"
INCIDENT_TEMPLATE = ROOT / "docs/templates/incident-dex-indexer.md"
SECURITY_POSTURE = ROOT / "docs/security-posture.md"
BLACKLIST_RUNBOOK = ROOT / "docs/runbooks/blacklist-decision.md"
SKILL = ROOT / "skills/AGENTS_ANOMALY_SIGNALS.md"

REQUIRED_RUNBOOK_MARKERS: tuple[str, ...] = (
    "SEC-G02",
    "## Anomaly checklist",
    "**A1**",
    "**A2**",
    "**A3**",
    "**A4**",
    "**A5**",
    "> 15%",
    "≥ 3",
    "> 30%",
    "≥ 10",
    "> 20%",
    "Pair pause",
    "Investigate",
    "reserve consistency",
    "Rate limit review",
    "blacklist-decision.md",
)


def fail(msg: str) -> None:
    print(f"ERROR: {msg}", file=sys.stderr)
    sys.exit(1)


def main() -> int:
    if not RUNBOOK.is_file():
        fail(f"missing {RUNBOOK.relative_to(ROOT)}")

    runbook_text = RUNBOOK.read_text()
    for marker in REQUIRED_RUNBOOK_MARKERS:
        if marker not in runbook_text:
            fail(f"{RUNBOOK.relative_to(ROOT)} missing required content: {marker!r}")

    if not INCIDENT_TEMPLATE.is_file():
        fail(f"missing {INCIDENT_TEMPLATE.relative_to(ROOT)}")

    incident_text = INCIDENT_TEMPLATE.read_text()
    if "anomaly-signals.md" not in incident_text:
        fail(
            "docs/templates/incident-dex-indexer.md must link to "
            "docs/runbooks/anomaly-signals.md in Triage"
        )

    if not SECURITY_POSTURE.is_file():
        fail(f"missing {SECURITY_POSTURE.relative_to(ROOT)}")

    posture_text = SECURITY_POSTURE.read_text()
    if "anomaly-signals.md" not in posture_text:
        fail("docs/security-posture.md must link to docs/runbooks/anomaly-signals.md")

    if not BLACKLIST_RUNBOOK.is_file():
        fail(f"missing {BLACKLIST_RUNBOOK.relative_to(ROOT)}")

    blacklist_text = BLACKLIST_RUNBOOK.read_text()
    if "anomaly-signals.md" not in blacklist_text:
        fail("docs/runbooks/blacklist-decision.md must link to anomaly-signals.md")

    if not SKILL.is_file():
        fail(f"missing {SKILL.relative_to(ROOT)}")

    print(
        "OK: anomaly signals runbook covers SEC-G02 topics and is linked from "
        "incident template, security-posture, and blacklist-decision runbook"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
