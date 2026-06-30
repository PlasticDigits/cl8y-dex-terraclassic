#!/usr/bin/env python3
"""Verify rollback decision runbook covers SEC-H09 (#445) and is cross-linked."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RUNBOOK = ROOT / "docs/runbooks/rollback-decision.md"
LAUNCH = ROOT / "docs/runbooks/launch-checklist.md"
WASM = ROOT / "docs/runbooks/wasm-admin-migration.md"
EMERGENCY = ROOT / "docs/runbooks/emergency-commands.md"
INCIDENT = ROOT / "docs/templates/incident-dex-indexer.md"
SECURITY = ROOT / "docs/security-model.md"
SKILL = ROOT / "skills/AGENTS_ROLLBACK_DECISION.md"

REQUIRED_RUNBOOK_MARKERS: tuple[str, ...] = (
    "SEC-H09",
    "## Top-level decision tree",
    "## 1. Frontend-only incident",
    "### Decision criteria",
    "### Rollback path (commands)",
    "### Limitations",
    "### Recovery verification",
    "## 2. Indexer incident",
    "down.sql",
    "## 3. Contract incident",
    "prior_code_id",
    "## 4. Chain dependency incident",
    "IBC-hooks",
    "verify-no-ibc-hooks-in-contracts",
)

FOUR_TYPES: tuple[str, ...] = (
    "Frontend-only",
    "Indexer incident",
    "Contract incident",
    "Chain dependency",
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

    for incident_type in FOUR_TYPES:
        if incident_type not in runbook_text:
            fail(
                f"{RUNBOOK.relative_to(ROOT)} must document incident type: {incident_type!r}"
            )

    if not LAUNCH.is_file():
        fail(f"missing {LAUNCH.relative_to(ROOT)}")

    launch_text = LAUNCH.read_text()
    if "rollback-decision.md" not in launch_text:
        fail(
            "docs/runbooks/launch-checklist.md rollback section must link to "
            "docs/runbooks/rollback-decision.md"
        )

    if not WASM.is_file():
        fail(f"missing {WASM.relative_to(ROOT)}")

    wasm_text = WASM.read_text()
    if "rollback-decision.md" not in wasm_text:
        fail("docs/runbooks/wasm-admin-migration.md must link to rollback-decision.md")

    if not EMERGENCY.is_file():
        fail(f"missing {EMERGENCY.relative_to(ROOT)}")

    emergency_text = EMERGENCY.read_text()
    if "rollback-decision.md" not in emergency_text:
        fail("docs/runbooks/emergency-commands.md must link to rollback-decision.md")

    if not INCIDENT.is_file():
        fail(f"missing {INCIDENT.relative_to(ROOT)}")

    incident_text = INCIDENT.read_text()
    if "rollback-decision.md" not in incident_text:
        fail(
            "docs/templates/incident-dex-indexer.md must link to "
            "docs/runbooks/rollback-decision.md at Mitigation"
        )

    if not SECURITY.is_file():
        fail(f"missing {SECURITY.relative_to(ROOT)}")

    security_text = SECURITY.read_text()
    if "rollback-decision.md" not in security_text:
        fail("docs/security-model.md must link to docs/runbooks/rollback-decision.md")

    if not SKILL.is_file():
        fail(f"missing {SKILL.relative_to(ROOT)}")

    print(
        "OK: rollback decision runbook covers SEC-H09 (four incident types) and is "
        "linked from launch-checklist, wasm-admin-migration, emergency-commands, "
        "incident template, and security-model"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
