#!/usr/bin/env python3
"""Static chrome-nesting guard (GitLab #653).

Fails when production TSX:
  1. Uses a `shell-panel*` class and a default (card) `<StatBox` in the same file.
  2. Uses both `shell-panel*` and `card-glass` class strings outside the allowlist.
  3. Puts `card-glass` back on a must-flat metric-grid file.

Does not eval source. Parses with regex only.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
SRC = ROOT / "frontend-dapp/src"
ALLOWLIST_PATH = ROOT / "scripts/chrome_nesting_allowlist.txt"
DESIGN_DOC = ROOT / "docs/design-system.md"
SKILL = ROOT / "skills/AGENTS_FRONTEND_DESIGN_SYSTEM.md"

STATBOX_OPEN = re.compile(r"<StatBox\b([^>]*?)(/?)>", re.DOTALL)
VARIANT_FLAT = re.compile(r"""variant\s*=\s*(?:"flat"|'flat'|\{["']flat["']\})""")
CLASS_HAS_SHELL = re.compile(
    r"""(?:className|class)\s*=\s*(?:\{)?["'`][^"'`]*\bshell-panel""",
)
CLASS_HAS_CARD = re.compile(
    r"""(?:className|class)\s*=\s*(?:\{)?["'`][^"'`]*\bcard-glass""",
)
BLANKET_OK = re.compile(
    r"Nested `card-glass` inside a page `shell-panel` is OK for distinct inner blocks",
    re.IGNORECASE,
)

# Metric grids this ticket flattened — must not grow card-glass chips again.
MUST_NOT_HAVE_CARD_GLASS = (
    "frontend-dapp/src/pages/ChartsPage.tsx",
    "frontend-dapp/src/components/trader/TraderSummaryStats.tsx",
    "frontend-dapp/src/components/protocol/ProtocolGlobalStats.tsx",
    "frontend-dapp/src/components/protocol/ProtocolFeeStats.tsx",
)

SKIP_STATBOX_CHECK = frozenset(
    {
        "frontend-dapp/src/components/ui/StatBox.tsx",
    }
)


def is_prod_tsx(path: Path) -> bool:
    if path.suffix != ".tsx":
        return False
    if path.name.endswith(".test.tsx") or path.name.endswith(".spec.tsx"):
        return False
    if "__tests__" in path.parts:
        return False
    return True


def rel(path: Path) -> str:
    return str(path.relative_to(ROOT)).replace("\\", "/")


def load_allowlist() -> set[str]:
    if not ALLOWLIST_PATH.is_file():
        return set()
    out: set[str] = set()
    for raw in ALLOWLIST_PATH.read_text().splitlines():
        line = raw.strip()
        if not line or line.startswith("#"):
            continue
        out.add(line)
    return out


def main() -> int:
    errors: list[str] = []
    allowlist = load_allowlist()
    used_allow: set[str] = set()
    inventory: list[str] = []

    for path in sorted(SRC.rglob("*.tsx")):
        if not is_prod_tsx(path):
            continue
        text = path.read_text()
        key = rel(path)
        has_shell = bool(CLASS_HAS_SHELL.search(text))
        has_card = bool(CLASS_HAS_CARD.search(text))
        boxes = list(STATBOX_OPEN.finditer(text)) if key not in SKIP_STATBOX_CHECK else []

        if has_shell and boxes:
            for match in boxes:
                attrs = match.group(1)
                if not VARIANT_FLAT.search(attrs):
                    errors.append(f"{key}: <StatBox> inside a shell-panel file must set variant=\"flat\"")

        if has_shell and has_card:
            inventory.append(key)
            if key in allowlist:
                used_allow.add(key)
            else:
                errors.append(
                    f"{key}: shell-panel + card-glass class nest is not on {ALLOWLIST_PATH.name}"
                )

        if key in MUST_NOT_HAVE_CARD_GLASS and has_card:
            errors.append(f"{key}: metric-grid file must not use card-glass (#653)")

    unused = sorted(allowlist - used_allow)
    for key in unused:
        target = ROOT / key
        if not target.is_file():
            errors.append(f"{ALLOWLIST_PATH.name}: stale path {key}")
        else:
            errors.append(f"{ALLOWLIST_PATH.name}: unused allowlist entry {key}")

    for doc in (DESIGN_DOC, SKILL):
        if not doc.is_file():
            errors.append(f"missing {rel(doc)}")
            continue
        body = doc.read_text()
        if BLANKET_OK.search(body):
            errors.append(
                f"{rel(doc)}: blanket 'nested card-glass is OK for distinct inner blocks' must be an allowlist, not a yes"
            )

    if errors:
        for err in errors:
            print(f"ERROR: {err}", file=sys.stderr)
        return 1

    print("OK: chrome nesting checks passed (GitLab #653)")
    if inventory:
        print("Allowlisted shell-panel + card-glass files:")
        for key in inventory:
            print(f"  {key}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
