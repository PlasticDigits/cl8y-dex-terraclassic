#!/usr/bin/env python3
"""Verify emergency command cookbook exists and is cross-linked (GitLab #399, SEC-B11)."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
COOKBOOK = ROOT / "docs/runbooks/emergency-commands.md"
INCIDENT_TEMPLATE = ROOT / "docs/templates/incident-dex-indexer.md"
SECURITY_MODEL = ROOT / "docs/security-model.md"
ADR = ROOT / "docs/adr/0003-governance-trading-blacklist.md"
SKILL = ROOT / "skills/AGENTS_EMERGENCY_COMMANDS.md"
VERIFY_SCRIPT = ROOT / "scripts/qa/verify-issue-399.sh"


def fail(msg: str) -> None:
    print(f"ERROR: {msg}", file=sys.stderr)
    sys.exit(1)


REQUIRED_COOKBOOK_MARKERS: tuple[str, ...] = (
    "set_pair_paused",
    "blacklist_wallet",
    "unblacklist_wallet",
    "blacklist_token",
    "unblacklist_token",
    "blacklist_pair",
    "unblacklist_pair",
    "$FACTORY_ADDR",
    "$GOVERNANCE_KEY",
    "$CHAIN_ID",
    "$PAIR_ADDR",
    "$WALLET_ADDR",
    "$TOKEN_ADDR",
    "--gas auto",
    "--gas-adjustment 1.4",
    '{"is_paused":{}}',
    "blacklist_check",
    "make verify-issue-399",
)


def main() -> int:
    if not COOKBOOK.is_file():
        fail(f"missing {COOKBOOK.relative_to(ROOT)}")

    cookbook = COOKBOOK.read_text()
    for marker in REQUIRED_COOKBOOK_MARKERS:
        if marker not in cookbook:
            fail(f"{COOKBOOK.relative_to(ROOT)} missing required content: {marker!r}")

    if not INCIDENT_TEMPLATE.is_file():
        fail(f"missing {INCIDENT_TEMPLATE.relative_to(ROOT)}")

    incident = INCIDENT_TEMPLATE.read_text()
    if "emergency-commands.md" not in incident:
        fail("incident template must link to docs/runbooks/emergency-commands.md in Mitigation")

    if not SECURITY_MODEL.is_file():
        fail(f"missing {SECURITY_MODEL.relative_to(ROOT)}")

    security = SECURITY_MODEL.read_text()
    if "emergency-commands.md" not in security:
        fail("docs/security-model.md must link to docs/runbooks/emergency-commands.md")

    if not ADR.is_file():
        fail(f"missing {ADR.relative_to(ROOT)}")

    adr = ADR.read_text()
    if "emergency-commands.md" not in adr:
        fail("docs/adr/0003-governance-trading-blacklist.md must link to emergency-commands.md")

    if not SKILL.is_file():
        fail(f"missing {SKILL.relative_to(ROOT)}")

    if not VERIFY_SCRIPT.is_file():
        fail(f"missing {VERIFY_SCRIPT.relative_to(ROOT)}")

    print(
        "OK: emergency-commands cookbook covers SEC-B11 operations and is cross-linked"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
