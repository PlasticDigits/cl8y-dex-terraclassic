#!/usr/bin/env python3
"""Verify extension fee guard scope docs (SEC-E08, GitLab #429)."""

from __future__ import annotations

import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
RUNBOOK = ROOT / "docs/runbooks/launch-checklist.md"
WALLET_QA = ROOT / "docs/runbooks/extension-fee-guard-wallet-qa.md"
SECURITY_MODEL = ROOT / "docs/security-model.md"
SKILL = ROOT / "skills/AGENTS_EXTENSION_FEE_GUARD.md"
GUARD_TS = ROOT / "frontend-dapp/src/utils/extensionSignedFeeGuard.ts"


def fail(msg: str) -> None:
    print(f"ERROR: {msg}", file=sys.stderr)
    sys.exit(1)


def main() -> int:
    for path in (RUNBOOK, WALLET_QA, SECURITY_MODEL, SKILL, GUARD_TS):
        if not path.is_file():
            fail(f"missing {path.relative_to(ROOT)}")

    runbook = RUNBOOK.read_text()
    required_runbook_markers: tuple[str, ...] = (
        "SEC-E08",
        "extension-fee-guard-wallet-qa.md",
        "verify-issue-429",
        "LocalTerra-only",
    )
    for marker in required_runbook_markers:
        if marker not in runbook:
            fail(f"{RUNBOOK.relative_to(ROOT)} missing required content: {marker!r}")

    if "Phase 4" not in runbook or runbook.find("SEC-E08") < runbook.find("## Phase 4"):
        fail("SEC-E08 gate must appear in Phase 4 or later")

    wallet_qa = WALLET_QA.read_text()
    required_wallet_markers: tuple[str, ...] = (
        "SEC-E08",
        "columbus-5",
        "localterra",
        "Keplr on mainnet",
        "make verify-issue-429",
        "Sign-off text",
    )
    for marker in required_wallet_markers:
        if marker not in wallet_qa:
            fail(f"{WALLET_QA.relative_to(ROOT)} missing required content: {marker!r}")

    security_text = SECURITY_MODEL.read_text()
    if "### Extension wallet fee guard (SEC-E08)" not in security_text:
        fail("security-model must define Extension fee guard SEC-E08 section")
    if "columbus-5" not in security_text or "localterra" not in security_text:
        fail("security-model SEC-E08 must document both mainnet and LocalTerra scope")

    skill_text = SKILL.read_text()
    if "SEC-E08" not in skill_text or "LocalTerra-only" not in skill_text:
        fail("agent skill must document LocalTerra-only scope")

    print(
        "OK: extension fee guard scope (SEC-E08) documented in launch runbook, "
        "wallet QA runbook, security-model, and agent skill"
    )
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
