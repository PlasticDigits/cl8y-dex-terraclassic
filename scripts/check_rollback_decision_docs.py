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
    "**every** successful `_sqlx_migrations.version` newer",
    "Partial suffix revert",
    "re-enter the three-way",
    "idx_reclass",
    "any success=false?",
    "every success=false",
    "auto-deploy off is **not** a process stop",
    "auto-deploy **off** → Coolify **Stop** / scale-to-zero → snapshot → DELETE every success=false → idx_reclass",
    "start only the hotfix image",
    "`idx_reclass` is **2(b)**",
    "schema is the bug or no hotfix that still embeds N",
    "idx_restart",
    "N-shipping image",
    "scale-to-zero",
    "## 3. Contract incident",
    "prior_code_id",
    "## 4. Chain dependency incident",
    "IBC-hooks",
    "verify-no-ibc-hooks-in-contracts",
)

ADR = ROOT / "docs/adr/0006-indexer-health-git-sha.md"
HEALTH_SKILL = ROOT / "skills/AGENTS_INDEXER_HEALTH_GIT_SHA.md"
ARCHITECTURE = ROOT / "docs/architecture.md"
INVARIANTS = ROOT / "docs/indexer-invariants.md"
TESTING = ROOT / "docs/testing.md"
EVERY_AHEAD_2C = "**every** successful `_sqlx_migrations.version` newer"
DIRTY_REENTER = "re-enter the three-way"
DIRTY_SEQUENTIAL = "any `success=false`"
DIRTY_EVERY_ROW = "**every** `success=false`"
STOP_NOT_AUTODEPLOY = "auto-deploy off is **not** a process stop"
DIRTY_ORDER = (
    "auto-deploy **off** → Coolify **Stop** / scale-to-zero → snapshot → "
    "DELETE every success=false → idx_reclass"
)
DIRTY_2B_START = "start only the hotfix image"
DIRTY_2B_GUARD = "`idx_reclass` is **2(b)**"
DIRTY_2A_START = "dirty → **2(a)**: start **only** the restored prior image"
DIRTY_2C_START = "dirty → **2(c)**: stay stopped through downs"
UNQUALIFIED_DIRTY_2B = "dirty `idx_reclass` → **2(b)**"
MERMAID_FWD_START = "idx_fwd_start[2(b) start only the hotfix image]"
MERMAID_FWD_CLEAN = "idx_fwd[2(b) keep schema; hotfix; no Stop]"
MERMAID_2C_YES = "schema is the bug or no hotfix that still embeds N"
ROLLBACK_BINARY_ROW = "| **Rollback binary** |"
N_SHIPPING = "N-shipping image"
SCALE_TO_ZERO = "scale-to-zero"

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
    skill_text = SKILL.read_text()

    if not HEALTH_SKILL.is_file():
        fail(f"missing {HEALTH_SKILL.relative_to(ROOT)}")
    health_skill_text = HEALTH_SKILL.read_text()

    if not ADR.is_file():
        fail(f"missing {ADR.relative_to(ROOT)}")
    adr_text = ADR.read_text()

    if not ARCHITECTURE.is_file():
        fail(f"missing {ARCHITECTURE.relative_to(ROOT)}")
    architecture_text = ARCHITECTURE.read_text()

    if not INVARIANTS.is_file():
        fail(f"missing {INVARIANTS.relative_to(ROOT)}")
    invariants_text = INVARIANTS.read_text()

    if not TESTING.is_file():
        fail(f"missing {TESTING.relative_to(ROOT)}")
    testing_text = TESTING.read_text()

    if ROLLBACK_BINARY_ROW in runbook_text:
        fail(
            f"{RUNBOOK.relative_to(ROOT)} must not offer a pre-Coolify "
            f"{ROLLBACK_BINARY_ROW!r} row; production uses only the Auto-deploy era table"
        )
    if MERMAID_2C_YES not in runbook_text:
        fail(f"{RUNBOOK.relative_to(ROOT)} mermaid 2(c) Yes must be {MERMAID_2C_YES!r}")
    if "Yes — schema is the bug|" in runbook_text:
        fail(
            f"{RUNBOOK.relative_to(ROOT)} mermaid must not use "
            "'Yes — schema is the bug' without the no-hotfix clause"
        )
    if MERMAID_FWD_START not in runbook_text:
        fail(
            f"{RUNBOOK.relative_to(ROOT)} mermaid dirty 2(b) must be "
            f"{MERMAID_FWD_START!r}"
        )
    if MERMAID_FWD_CLEAN not in runbook_text:
        fail(
            f"{RUNBOOK.relative_to(ROOT)} mermaid clean 2(b) must be "
            f"{MERMAID_FWD_CLEAN!r}"
        )
    for line in runbook_text.splitlines():
        s = line.strip()
        if not any(
            s.startswith(n)
            for n in (
                "idx_ahead_dirty",
                "idx_pair_dirty",
                "idx_dirty",
                "idx_stop_dirty",
                "idx_reclass",
            )
        ):
            continue
        scrubbed = s.replace("idx_fwd_start", "")
        if "idx_fwd" in scrubbed:
            fail(
                f"{RUNBOOK.relative_to(ROOT)} dirty mermaid must not route "
                f"to idx_fwd: {s}"
            )

    for path, text in (
        (SKILL, skill_text),
        (HEALTH_SKILL, health_skill_text),
        (ADR, adr_text),
        (RUNBOOK, runbook_text),
        (ARCHITECTURE, architecture_text),
        (INVARIANTS, invariants_text),
        (TESTING, testing_text),
    ):
        rel = path.relative_to(ROOT)
        if DIRTY_ORDER not in text:
            fail(f"{rel} must pin dirty action list {DIRTY_ORDER!r}")
        if DIRTY_2B_GUARD not in text:
            fail(f"{rel} must pin dirty→2(b) guard {DIRTY_2B_GUARD!r}")
        if DIRTY_2B_START not in text:
            fail(f"{rel} must pin dirty→2(b) {DIRTY_2B_START!r}")
        if DIRTY_2A_START not in text:
            fail(f"{rel} must pin dirty→2(a) {DIRTY_2A_START!r}")
        if DIRTY_2C_START not in text:
            fail(f"{rel} must pin dirty→2(c) {DIRTY_2C_START!r}")
        if UNQUALIFIED_DIRTY_2B in text:
            fail(
                f"{rel} must not pin unqualified {UNQUALIFIED_DIRTY_2B!r}; "
                f"use {DIRTY_2B_GUARD!r}"
            )
        if STOP_NOT_AUTODEPLOY not in text:
            fail(f"{rel} must pin warning {STOP_NOT_AUTODEPLOY!r}")

    for path, text in (
        (SKILL, skill_text),
        (HEALTH_SKILL, health_skill_text),
        (ADR, adr_text),
        (RUNBOOK, runbook_text),
    ):
        rel = path.relative_to(ROOT)
        if EVERY_AHEAD_2C not in text:
            fail(f"{rel} must pin 2(c) every-ahead-version gate: {EVERY_AHEAD_2C!r}")
        if DIRTY_REENTER not in text:
            fail(f"{rel} must pin dirty DELETE then {DIRTY_REENTER!r}")
        if DIRTY_SEQUENTIAL not in text:
            fail(f"{rel} must pin sequential dirty gate {DIRTY_SEQUENTIAL!r}")
        if DIRTY_EVERY_ROW not in text:
            fail(f"{rel} must pin {DIRTY_EVERY_ROW!r} row delete")
        if N_SHIPPING not in text:
            fail(f"{rel} must pin {N_SHIPPING!r} still-boot failure")
        if SCALE_TO_ZERO not in text:
            fail(f"{rel} must pin Coolify Stop / {SCALE_TO_ZERO!r}")

    print(
        "OK: rollback decision runbook covers SEC-H09 (four incident types) and is "
        "linked from launch-checklist, wasm-admin-migration, emergency-commands, "
        "incident template, and security-model; 2(c) every-ahead-version + sequential "
        "dirty gate (auto-deploy off → Stop → snapshot → DELETE → idx_reclass) and "
        "qualified dirty idx_reclass start-only (when 2(b)/2(a)/2(c)) pinned "
        "in ADR 0006, architecture, invariants, testing, runbook, and both skills"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
