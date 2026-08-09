#!/usr/bin/env python3
"""Verify user-incident FAQ exists, covers SEC-A03 topics, and is linked (GitLab #390)."""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
FAQ = ROOT / "docs/user-incident-faq.md"
SECURITY_MODEL = ROOT / "docs/security-model.md"
LEGAL_COPY = ROOT / "frontend-dapp/src/components/legal/legalCopy.ts"
LEGAL_FOOTER = ROOT / "frontend-dapp/src/components/legal/LegalFooterNotice.tsx"

# Section headings or distinctive phrases that must appear in the FAQ.
REQUIRED_FAQ_MARKERS: tuple[str, ...] = (
    "## Pair pause",
    "## Wallet blacklist",
    "## Token blacklist",
    "## Pair blacklist",
    "## Rate limits",
    "## UST1 oracle window",
    "## Wrap pause",
    "oracle stale",
    "Withdraw liquidity",
    "Limit order escrow",
    "UnblacklistWallet",
    "UnblacklistToken",
    "UnblacklistPair",
    "HTTP 429",
    "Retry-After",
    "wrap-mapper",
)


def fail(msg: str) -> None:
    print(f"ERROR: {msg}", file=sys.stderr)
    sys.exit(1)


def main() -> int:
    if not FAQ.is_file():
        fail(f"missing {FAQ.relative_to(ROOT)}")

    faq_text = FAQ.read_text()
    for marker in REQUIRED_FAQ_MARKERS:
        if marker not in faq_text:
            fail(f"{FAQ.relative_to(ROOT)} missing required content: {marker!r}")

    if not SECURITY_MODEL.is_file():
        fail(f"missing {SECURITY_MODEL.relative_to(ROOT)}")

    security_text = SECURITY_MODEL.read_text()
    if "user-incident-faq.md" not in security_text:
        fail("docs/security-model.md must link to docs/user-incident-faq.md")

    if not LEGAL_COPY.is_file():
        fail(f"missing {LEGAL_COPY.relative_to(ROOT)}")

    legal_copy_text = LEGAL_COPY.read_text()
    if "user-incident-faq.md" not in legal_copy_text:
        fail("legalCopy.ts must reference user-incident-faq.md")

    if not LEGAL_FOOTER.is_file():
        fail(f"missing {LEGAL_FOOTER.relative_to(ROOT)}")

    footer_text = LEGAL_FOOTER.read_text()
    if "USER_INCIDENT_FAQ" not in footer_text:
        fail("LegalFooterNotice.tsx must use USER_INCIDENT_FAQ_* from legalCopy.ts")

    # Avoid re-adding duplicate full FAQ bodies elsewhere.
    incident_heading_re = re.compile(r"^## What happens during an incident\?", re.MULTILINE)
    for md in (ROOT / "docs").rglob("*.md"):
        if md == FAQ:
            continue
        text = md.read_text()
        if incident_heading_re.search(text):
            fail(
                f"{md.relative_to(ROOT)} duplicates the incident FAQ heading; "
                "link to docs/user-incident-faq.md instead"
            )

    print(f"OK: {FAQ.relative_to(ROOT)} covers SEC-A03 topics and is linked from security docs + footer")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
