#!/usr/bin/env python3
"""Verify incident template timeline table and cross-links (GitLab #439, SEC-G06)."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INCIDENT_TEMPLATE = ROOT / "docs/templates/incident-dex-indexer.md"
BLACKLIST_RUNBOOK = ROOT / "docs/runbooks/blacklist-decision.md"
EMERGENCY_RUNBOOK = ROOT / "docs/runbooks/emergency-commands.md"
SKILL = ROOT / "skills/AGENTS_INCIDENT_TEMPLATE.md"

REQUIRED_TIMELINE_HEADERS: tuple[str, ...] = (
    "UTC Time",
    "Tx Hash",
    "Wallet",
    "Token",
    "Pair",
    "Admin Action",
    "User Impact",
)

TIMELINE_ANCHOR = "incident-dex-indexer.md#incident-timeline"


def fail(msg: str) -> None:
    print(f"ERROR: {msg}", file=sys.stderr)
    sys.exit(1)


def main() -> int:
    if not INCIDENT_TEMPLATE.is_file():
        fail(f"missing {INCIDENT_TEMPLATE.relative_to(ROOT)}")

    incident = INCIDENT_TEMPLATE.read_text()
    if "## Incident timeline" not in incident:
        fail("incident template must include ## Incident timeline section")

    for header in REQUIRED_TIMELINE_HEADERS:
        if header not in incident:
            fail(
                f"incident template timeline table missing column header: {header!r}"
            )

    if "|          |         |        |       |      |              |             |" not in incident:
        fail("incident template timeline table must include a blank starter row")

    if not BLACKLIST_RUNBOOK.is_file():
        fail(f"missing {BLACKLIST_RUNBOOK.relative_to(ROOT)}")

    blacklist = BLACKLIST_RUNBOOK.read_text()
    if TIMELINE_ANCHOR not in blacklist:
        fail(
            "blacklist-decision rollback checklist must link to "
            "incident template #incident-timeline"
        )

    if not EMERGENCY_RUNBOOK.is_file():
        fail(f"missing {EMERGENCY_RUNBOOK.relative_to(ROOT)}")

    emergency = EMERGENCY_RUNBOOK.read_text()
    if TIMELINE_ANCHOR not in emergency:
        fail(
            "emergency-commands must link incident tx recording to "
            "incident template #incident-timeline"
        )

    if not SKILL.is_file():
        fail(f"missing {SKILL.relative_to(ROOT)}")

    print(
        "OK: incident template timeline (SEC-G06) present and linked from "
        "blacklist-decision and emergency-commands runbooks"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
