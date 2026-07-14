#!/usr/bin/env python3
"""Verify Tailwind + trade-bootstrap token alignment (GitLab #488 blue+gold)."""

from __future__ import annotations

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
TAILWIND = ROOT / "frontend-dapp/tailwind.config.js"
BOOTSTRAP = ROOT / "frontend-dapp/public/bootstrap/trade-bootstrap.css"
THEME_DARK = ROOT / "frontend-dapp/src/theme-dark.css"
THEME_LIGHT = ROOT / "frontend-dapp/src/theme-light.css"
INDEX_CSS = ROOT / "frontend-dapp/src/index.css"
DESIGN_DOC = ROOT / "docs/design-system.md"
QA_TEMPLATE = ROOT / "QA_TEMPLATE.md"
QA_PASS = ROOT / "QA_PASS_2026-03-13.md"

# Legacy amber page identity — must not remain dominant after #488
WARM_BROWN_BG = re.compile(r"#(?:0e0908|f4e0cb|1d110f)\b", re.IGNORECASE)
# Deprecated parallel Tailwind hex palettes from #416 (not the #488 system blue)
LEGACY_TAILWIND_BLUE = re.compile(
    r"#(?:3b82f6|2563eb|60a5fa|38bdf8|0f172a|1e293b|334155)",
    re.IGNORECASE,
)
BROWN_WARNING_FILL = re.compile(r"rgba\(\s*74,\s*(?:40|55),\s*12\b")
GOLD_PAGE_WASH = re.compile(r"rgba\(\s*232,\s*184,\s*74\b")


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
    index_css = INDEX_CSS.read_text() if INDEX_CSS.is_file() else ""

    if LEGACY_TAILWIND_BLUE.search(tailwind):
        errors.append("tailwind.config.js still contains legacy hard-coded blue hex palette")
    if "primary:" in tailwind or "dex:" in tailwind:
        errors.append("tailwind.config.js still defines deprecated primary/dex palettes")
    if "var(--bg-0)" not in tailwind:
        errors.append("tailwind.config.js must alias bg.0 to var(--bg-0)")
    if "var(--blue)" not in tailwind or "var(--gold)" not in tailwind:
        errors.append("tailwind.config.js must alias blue/gold to CSS variables (#488)")

    if WARM_BROWN_BG.search(bootstrap):
        errors.append("trade-bootstrap.css still uses warm brown page backgrounds")
    if "var(--bg-0)" not in bootstrap:
        errors.append("trade-bootstrap.css must use var(--bg-0) for page background")

    for theme_name, theme_css in (("dark", theme_dark), ("light", theme_light)):
        bg0 = token_value(theme_css, "bg-0")
        blue = token_value(theme_css, "blue")
        gold = token_value(theme_css, "gold")
        if not bg0:
            errors.append(f"theme-{theme_name}.css missing --bg-0")
            continue
        if f"--bg-0: {bg0}" not in bootstrap:
            errors.append(
                f"trade-bootstrap.css --bg-0 drift from theme-{theme_name}.css ({bg0})"
            )
        if WARM_BROWN_BG.search(bg0):
            errors.append(f"theme-{theme_name}.css --bg-0 is still warm brown ({bg0})")
        if blue != "#448aff":
            errors.append(f"theme-{theme_name}.css --blue must be #448aff (got {blue})")
        if gold != "#e8b84a":
            errors.append(f"theme-{theme_name}.css --gold must be #e8b84a (got {gold})")
        if "--mint: var(--blue)" not in theme_css and token_value(theme_css, "mint") != "var(--blue)":
            errors.append(f"theme-{theme_name}.css --mint must alias --blue for CTA compat")
        if BROWN_WARNING_FILL.search(theme_css):
            errors.append(f"theme-{theme_name}.css still uses brown warning fill rgba(74, …, 12)")

    if index_css:
        if BROWN_WARNING_FILL.search(index_css):
            errors.append("index.css still uses brown warning fill rgba(74, …, 12)")
        body_match = re.search(r"body\s*\{[^}]*background:[^}]*\}", index_css, re.DOTALL)
        if body_match and GOLD_PAGE_WASH.search(body_match.group(0)):
            errors.append("index.css body background still contains gold radial wash")
        if re.search(
            r"\.app-nav-link-active[\s\S]*background:\s*var\(--gold-surface\)",
            index_css,
        ):
            errors.append("index.css nav active state must not use --gold-surface fill")
        if re.search(r"rgba\(\s*249,\s*115,\s*22\b", index_css):
            errors.append("index.css still uses orange brand glow rgba(249, 115, 22) — use cool chrome-glow")
        if re.search(r"\.app-modal-backdrop[\s\S]{0,200}rgba\(\s*7,\s*4,\s*3\b", index_css):
            errors.append("index.css .app-modal-backdrop still uses warm brown overlay")
    elif not INDEX_CSS.is_file():
        errors.append(f"missing {INDEX_CSS}")

    dark_bg0 = token_value(theme_dark, "bg-0")
    light_bg0 = token_value(theme_light, "bg-0")
    if dark_bg0 != "#0d111c":
        errors.append(f"theme-dark.css --bg-0 must be #0d111c (got {dark_bg0})")
    if light_bg0 != "#f4f6fb":
        errors.append(f"theme-light.css --bg-0 must be #f4f6fb (got {light_bg0})")

    if DESIGN_DOC.is_file():
        doc = DESIGN_DOC.read_text()
        if "blue" not in doc.lower() or "gold" not in doc.lower():
            errors.append("docs/design-system.md must document blue+gold system (#488)")
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

    print("OK: design token alignment checks passed (blue+gold #488)")
    return 0


if __name__ == "__main__":
    sys.exit(main())
