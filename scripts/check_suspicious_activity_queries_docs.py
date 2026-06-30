#!/usr/bin/env python3
"""Verify suspicious activity discovery runbook covers SEC-G04 (#437) and is cross-linked."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RUNBOOK = ROOT / "docs/runbooks/suspicious-activity-queries.md"
INCIDENT_TEMPLATE = ROOT / "docs/templates/incident-dex-indexer.md"
BLACKLIST_RUNBOOK = ROOT / "docs/runbooks/blacklist-decision.md"
SKILL = ROOT / "skills/AGENTS_SUSPICIOUS_ACTIVITY_QUERIES.md"
VERIFY_SCRIPT = ROOT / "scripts/qa/verify-issue-437.sh"

REQUIRED_RUNBOOK_MARKERS: tuple[str, ...] = (
    "SEC-G04",
    "## 1. Top-volume traders",
    "/api/v1/traders/leaderboard",
    "## 2. Wallets with many failed transactions",
    "code != 0",
    "hook_events",
    "## 3. Pairs with abnormal swap count",
    "/api/v1/pairs",
    "/stats",
    "swap_events",
    "## 4. Reserve / liquidity anomalies",
    "liquidity_events",
    "pair_reserves",
    "## 5. Blacklist compliance probes",
    "/api/v1/compliance/blacklist-check",
    "## Escalation checklist",
    "blacklist-decision.md",
    "make check-suspicious-activity-queries-docs",
    "make verify-issue-437",
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
    if "suspicious-activity-queries.md" not in incident_text:
        fail(
            "docs/templates/incident-dex-indexer.md must link to "
            "docs/runbooks/suspicious-activity-queries.md in Triage"
        )

    if not BLACKLIST_RUNBOOK.is_file():
        fail(f"missing {BLACKLIST_RUNBOOK.relative_to(ROOT)}")

    blacklist_text = BLACKLIST_RUNBOOK.read_text()
    if "suspicious-activity-queries.md" not in blacklist_text:
        fail(
            "docs/runbooks/blacklist-decision.md must link to suspicious-activity-queries.md "
            "for discovery queries"
        )

    if not SKILL.is_file():
        fail(f"missing {SKILL.relative_to(ROOT)}")

    if not VERIFY_SCRIPT.is_file():
        fail(f"missing {VERIFY_SCRIPT.relative_to(ROOT)}")

    print(
        "OK: suspicious activity queries runbook covers SEC-G04 topics and is linked from "
        "incident template and blacklist-decision runbook"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
