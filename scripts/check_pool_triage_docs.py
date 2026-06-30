#!/usr/bin/env python3
"""Verify pool triage runbook covers SEC-G03 (#436) and is cross-linked."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RUNBOOK = ROOT / "docs/runbooks/emergency-commands.md"
INCIDENT_TEMPLATE = ROOT / "docs/templates/incident-dex-indexer.md"
SECURITY_MODEL = ROOT / "docs/security-model.md"
SKILL = ROOT / "skills/AGENTS_POOL_TRIAGE.md"
VERIFY_SCRIPT = ROOT / "scripts/qa/verify-issue-436.sh"

REQUIRED_RUNBOOK_MARKERS: tuple[str, ...] = (
    "SEC-G03",
    "## Quick pool triage",
    "pair_reserves",
    "ORDER BY approx_liquidity_units DESC",
    "sort=volume_24h&order=desc",
    '{"pool":{}}',
    "make check-pool-triage-docs",
)

REQUIRED_INCIDENT_MARKERS: tuple[str, ...] = (
    "Quick pool triage",
    "emergency-commands.md#quick-pool-triage-sec-g03",
    "pair_reserves",
    "volume_24h",
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
    for marker in REQUIRED_INCIDENT_MARKERS:
        if marker not in incident_text:
            fail(
                f"{INCIDENT_TEMPLATE.relative_to(ROOT)} missing required content: {marker!r}"
            )

    if not SECURITY_MODEL.is_file():
        fail(f"missing {SECURITY_MODEL.relative_to(ROOT)}")

    security_text = SECURITY_MODEL.read_text()
    if "quick-pool-triage-sec-g03" not in security_text:
        fail(
            "docs/security-model.md must link to emergency-commands quick pool triage (SEC-G03)"
        )

    if not SKILL.is_file():
        fail(f"missing {SKILL.relative_to(ROOT)}")

    if not VERIFY_SCRIPT.is_file():
        fail(f"missing {VERIFY_SCRIPT.relative_to(ROOT)}")

    print(
        "OK: pool triage runbook (SEC-G03) documents reserve/volume ranking and is cross-linked"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
