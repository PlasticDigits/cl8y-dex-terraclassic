#!/usr/bin/env python3
"""Verify incident communications templates cover SEC-G05 (#438) and are cross-linked."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
INCIDENT_TEMPLATE = ROOT / "docs/templates/incident-dex-indexer.md"
BLACKLIST_RUNBOOK = ROOT / "docs/runbooks/blacklist-decision.md"
EMERGENCY_RUNBOOK = ROOT / "docs/runbooks/emergency-commands.md"
USER_FAQ = ROOT / "docs/user-incident-faq.md"
SKILL = ROOT / "skills/AGENTS_INCIDENT_COMMS_TEMPLATES.md"

REQUIRED_TEMPLATE_MARKERS: tuple[str, ...] = (
    "SEC-G05",
    "## Appendix: Communications templates",
    "### 1. Pair paused",
    "### 2. Blacklist applied",
    "### 3. Exploit under investigation",
    "### 4. False alarm retraction",
    "### 5. Postmortem summary",
    "[TIMESTAMP_UTC]",
    "[PAIR_ADDRESS]",
    "[WALLET_ADDRESS]",
    "[TOKEN_ADDRESS]",
    "[BLACKLIST_TARGET]",
    "[IMPACT_DESCRIPTION]",
    "[ESTIMATED_RESOLUTION]",
    "[COMPLETED_ACTIONS]",
    "[CONTACT_CHANNEL]",
    "[REASON_IF_DISCLOSABLE]",
    "user-incident-faq.md",
)


def fail(msg: str) -> None:
    print(f"ERROR: {msg}", file=sys.stderr)
    sys.exit(1)


def main() -> int:
    if not INCIDENT_TEMPLATE.is_file():
        fail(f"missing {INCIDENT_TEMPLATE.relative_to(ROOT)}")

    template_text = INCIDENT_TEMPLATE.read_text()
    for marker in REQUIRED_TEMPLATE_MARKERS:
        if marker not in template_text:
            fail(
                f"{INCIDENT_TEMPLATE.relative_to(ROOT)} missing required content: {marker!r}"
            )

    if "Appendix: Communications templates" not in template_text:
        fail("incident template must link Communications section to appendix")

    if not BLACKLIST_RUNBOOK.is_file():
        fail(f"missing {BLACKLIST_RUNBOOK.relative_to(ROOT)}")

    blacklist_text = BLACKLIST_RUNBOOK.read_text()
    if "Communications templates" not in blacklist_text:
        fail(
            "docs/runbooks/blacklist-decision.md must link to incident communications templates"
        )

    if not EMERGENCY_RUNBOOK.is_file():
        fail(f"missing {EMERGENCY_RUNBOOK.relative_to(ROOT)}")

    emergency_text = EMERGENCY_RUNBOOK.read_text()
    if "Communications templates" not in emergency_text:
        fail(
            "docs/runbooks/emergency-commands.md must link to incident communications templates"
        )

    if not USER_FAQ.is_file():
        fail(f"missing {USER_FAQ.relative_to(ROOT)}")

    faq_text = USER_FAQ.read_text()
    if "Communications templates" not in faq_text:
        fail("docs/user-incident-faq.md must link to incident communications templates")

    if not SKILL.is_file():
        fail(f"missing {SKILL.relative_to(ROOT)}")

    print(
        "OK: incident communications templates cover SEC-G05 scenarios and are cross-linked"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
