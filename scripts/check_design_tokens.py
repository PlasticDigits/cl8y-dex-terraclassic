#!/usr/bin/env python3
"""Verify Tailwind + trade-bootstrap token alignment (GitLab #416)."""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TAILWIND = ROOT / "frontend-dapp/tailwind.config.js"
BOOTSTRAP = ROOT / "frontend-dapp/public/bootstrap/trade-bootstrap.css"
THEME_DARK = ROOT / "frontend-dapp/src/theme-dark.css"
THEME_LIGHT = ROOT / "frontend-dapp/src/theme-light.css"
DESIGN_DOC = ROOT / "docs/design-system.md"
QA_TEMPLATE = ROOT / "QA_TEMPLATE.md"
QA_PASS = ROOT / "QA_PASS_2026-03-13.md"

BLUE_HEX = re.compile(
    r"#(?:3b82f6|2563eb|60a5fa|38bdf8|0f172a|1e293b|334155)",
    re.IGNORECASE,
)


def token_value(css: str, name: str) -> str | None:
    match = re.search(rf"--{re.escape(name)}:\s*([^;]+);", css)
    return match.group(1).strip() if match else None


def main() -> int:
    errors: list[str] = []

    if not TAILWIND.is_file():
        errors.append(f"missing {TAILWIND}")
    if not BOOTSTRAP.is_file():
        errors.append(f"missing {BOOTSTRAP}")

    if errors:
        for err in errors:
            print(f"ERROR: {err}", file=sys.stderr)
        return 1

    tailwind = TAILWIND.read_text()
    bootstrap = BOOTSTRAP.read_text()
    theme_dark = THEME_DARK.read_text()
    theme_light = THEME_LIGHT.read_text()

    if BLUE_HEX.search(tailwind):
        errors.append("tailwind.config.js still contains legacy blue hex palette")
    if "primary:" in tailwind or "dex:" in tailwind:
        errors.append("tailwind.config.js still defines deprecated primary/dex palettes")
    if "var(--bg-0)" not in tailwind:
        errors.append("tailwind.config.js must alias bg.0 to var(--bg-0)")

    if BLUE_HEX.search(bootstrap):
        errors.append("trade-bootstrap.css contains legacy blue hex values")
    if "var(--bg-0)" not in bootstrap:
        errors.append("trade-bootstrap.css must use var(--bg-0) for page background")

    for theme_name, theme_css in (("dark", theme_dark), ("light", theme_light)):
        bg0 = token_value(theme_css, "bg-0")
        if not bg0:
            errors.append(f"theme-{theme_name}.css missing --bg-0")
            continue
        if f"--bg-0: {bg0}" not in bootstrap:
            errors.append(
                f"trade-bootstrap.css --bg-0 drift from theme-{theme_name}.css ({bg0})"
            )

    if DESIGN_DOC.is_file():
        doc = DESIGN_DOC.read_text()
        if "## Tailwind" not in doc and "§ Tailwind" not in doc and "Tailwind color" not in doc:
            errors.append("docs/design-system.md missing Tailwind token section")
    else:
        errors.append(f"missing {DESIGN_DOC}")

    if QA_TEMPLATE.is_file():
        qa = QA_TEMPLATE.read_text()
        if "header" not in qa.lower() or "footer theme toggle" in qa.lower():
            errors.append("QA_TEMPLATE.md must reference header theme toggle, not footer")
    else:
        errors.append(f"missing {QA_TEMPLATE}")

    if QA_PASS.is_file():
        qa_pass = QA_PASS.read_text()
        if "Footer theme toggle" in qa_pass and "historical" not in qa_pass.lower():
            errors.append(
                "QA_PASS_2026-03-13.md footer theme toggle rows need historical superseded note"
            )

    if errors:
        for err in errors:
            print(f"ERROR: {err}", file=sys.stderr)
        return 1

    print("OK: design token alignment checks passed")
    return 0


if __name__ == "__main__":
    sys.exit(main())
