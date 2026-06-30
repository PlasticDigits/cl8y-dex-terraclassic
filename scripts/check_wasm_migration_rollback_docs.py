#!/usr/bin/env python3
"""Verify wasm migration rollback limitations are documented (SEC-H05, GitLab #443)."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
MIGRATION_RUNBOOK = ROOT / "docs/runbooks/wasm-admin-migration.md"
LAUNCH_RUNBOOK = ROOT / "docs/runbooks/launch-checklist.md"
TESTING_DOC = ROOT / "docs/testing.md"
SKILL = ROOT / "skills/AGENTS_WASM_MIGRATION_ROLLBACK.md"

REQUIRED_MARKERS: tuple[tuple[Path, str], ...] = (
    (MIGRATION_RUNBOOK, "## Rollback and limitations (SEC-H05)"),
    (MIGRATION_RUNBOOK, "terrad query wasm code"),
    (MIGRATION_RUNBOOK, "prior `code_id`"),
    (MIGRATION_RUNBOOK, "Admin cleared"),
    (MIGRATION_RUNBOOK, "indexer/migrations/revert/"),
    (MIGRATION_RUNBOOK, "Partial migration recovery"),
    (MIGRATION_RUNBOOK, "verify-issue-443"),
    (LAUNCH_RUNBOOK, "wasm-admin-migration.md#rollback-and-limitations-sec-h05"),
    (LAUNCH_RUNBOOK, "SEC-H05"),
    (TESTING_DOC, "indexer/migrations/revert/"),
)


def main() -> int:
    errors: list[str] = []

    for path, marker in REQUIRED_MARKERS:
        if not path.is_file():
            errors.append(f"missing {path.relative_to(ROOT)}")
            continue
        if marker not in path.read_text():
            errors.append(f"{path.relative_to(ROOT)} missing required content: {marker!r}")

    if not SKILL.is_file():
        errors.append(f"missing {SKILL.relative_to(ROOT)}")

    if errors:
        for err in errors:
            print(f"ERROR: {err}", file=sys.stderr)
        return 1

    print(
        "OK: wasm migration rollback limitations (SEC-H05) documented in "
        "migration runbook with launch-checklist cross-link"
    )
    return 0


if __name__ == "__main__":
    sys.exit(main())
