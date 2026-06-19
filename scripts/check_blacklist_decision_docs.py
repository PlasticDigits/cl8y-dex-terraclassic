#!/usr/bin/env python3
"""Verify blacklist decision runbook covers SEC-B12 (#400) and is cross-linked."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RUNBOOK = ROOT / "docs/runbooks/blacklist-decision.md"
INCIDENT_TEMPLATE = ROOT / "docs/templates/incident-dex-indexer.md"
SECURITY_MODEL = ROOT / "docs/security-model.md"
ADR = ROOT / "docs/adr/0003-governance-trading-blacklist.md"
SKILL = ROOT / "skills/AGENTS_BLACKLIST_DECISION.md"

REQUIRED_RUNBOOK_MARKERS: tuple[str, ...] = (
    "SEC-B12",
    "## Decision tree",
    "### Wallet blacklist",
    "Confirmed exploit actor",
    "### Token blacklist",
    "Malicious CW20",
    "### Pair blacklist",
    "Compromised pool",
    "Terms-of-service",
    "## False-positive rollback",
    "Preserve original evidence",
    "UnblacklistWallet",
    "UnblacklistToken",
    "UnblacklistPair",
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
    if "blacklist-decision.md" not in incident_text:
        fail(
            "docs/templates/incident-dex-indexer.md must link to "
            "docs/runbooks/blacklist-decision.md at Mitigation"
        )

    if not SECURITY_MODEL.is_file():
        fail(f"missing {SECURITY_MODEL.relative_to(ROOT)}")

    security_text = SECURITY_MODEL.read_text()
    if "blacklist-decision.md" not in security_text:
        fail("docs/security-model.md must link to docs/runbooks/blacklist-decision.md")

    if not ADR.is_file():
        fail(f"missing {ADR.relative_to(ROOT)}")

    adr_text = ADR.read_text()
    if "blacklist-decision.md" not in adr_text:
        fail("docs/adr/0003-governance-trading-blacklist.md must link to operator runbook")

    if not SKILL.is_file():
        fail(f"missing {SKILL.relative_to(ROOT)}")

    print(
        "OK: blacklist decision runbook covers SEC-B12 topics and is linked from "
        "incident template, security-model, and ADR 0003"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
