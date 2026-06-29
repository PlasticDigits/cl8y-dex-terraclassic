#!/usr/bin/env python3
"""GitLab #433 (SEC-F13): guard that indexer logging never emits secrets.

Scans ``indexer/src`` for logging / print macro calls
(``tracing::{info,warn,error,debug,trace}!``, ``log::{...}!``, ``println!``,
``eprintln!``, ``print!``, ``eprint!``, ``dbg!``) whose arguments reference a
secret-bearing identifier, or that log the whole ``Config`` struct via ``Debug``.

``Config`` (``indexer/src/config.rs``) derives ``Debug`` and holds ``database_url``
(a ``postgres://user:password@host/db`` string), so a ``{:?}`` on the whole struct
would leak the DB password. This check catches both the direct-field case
(e.g. ``tracing::info!("{}", config.database_url)``) and the whole-struct-Debug
case (e.g. ``tracing::debug!("{:?}", config)``).

Exit 0 = clean. Exit 1 = a potential secret-logging site found (printed file:line).

A verified-safe line may opt out with a trailing ``// log-secrets-ok: <reason>``
marker inside the macro arguments (use sparingly, with justification).

Run via: ``make lint-log-secrets``
"""
import re
import sys
from pathlib import Path

SRC = Path(__file__).resolve().parent.parent / "indexer" / "src"

LOG_MACRO = re.compile(
    r"\b(?:tracing|log)\s*::\s*(?:info|warn|error|debug|trace)\s*!"
    r"|\b(?:println|eprintln|print|eprint|dbg)\s*!"
)

# Secret-bearing identifiers. Deliberately NOT bare "token": the indexer is full of
# trading-token references (token0, ask_token, token volumes) that are not secrets.
SECRET = re.compile(
    r"\b(database_url|db_url|db_password|password|passwd|mnemonic|seed_phrase|"
    r"private_key|privkey|secret|secret_key|bearer|api_key|apikey|auth_token|"
    r"access_token|credential|connection_string|conn_str|dsn)\b",
    re.IGNORECASE,
)

# The whole `config` var (not `config.field`) — Config: Debug exposes database_url.
WHOLE_CONFIG = re.compile(r"&?\bconfig\b(?!\s*\.)")
# A Debug formatter is present (`{:?}`, `{:#?}`, or the `?config` field-capture shorthand).
DEBUG_FMT = re.compile(r"\{[^}]*\?[^}]*\}|\?\s*config\b")

OPT_OUT = re.compile(r"//\s*log-secrets-ok")


def arg_span(text, open_paren):
    """Balance parens from `open_paren`, skipping string literals.

    Returns (span_text, end_index) where end_index is just past the matching `)`.
    """
    depth, i = 0, open_paren
    in_str = esc = False
    while i < len(text):
        c = text[i]
        if in_str:
            if esc:
                esc = False
            elif c == "\\":
                esc = True
            elif c == '"':
                in_str = False
        elif c == '"':
            in_str = True
        elif c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
            if depth == 0:
                return text[open_paren : i + 1], i + 1
        i += 1
    return text[open_paren:], len(text)


def scan(path):
    text = path.read_text(encoding="utf-8", errors="replace")
    findings = []
    for m in LOG_MACRO.finditer(text):
        p = text.find("(", m.end() - 1)
        if p == -1:
            continue
        span, end = arg_span(text, p)
        # opt-out marker may sit after the closing `)` (e.g. `...); // log-secrets-ok`),
        # so scan the whole statement region: macro's first line .. end of closing line.
        line_start = text.rfind("\n", 0, m.start()) + 1
        stmt_end = text.find("\n", end)
        region = text[line_start : stmt_end if stmt_end != -1 else len(text)]
        if OPT_OUT.search(region):
            continue
        reason = None
        if SECRET.search(span):
            reason = "secret-bearing identifier in log argument"
        elif WHOLE_CONFIG.search(span) and DEBUG_FMT.search(span):
            reason = "whole Config struct logged via Debug (exposes database_url)"
        if reason:
            line_no = text.count("\n", 0, m.start()) + 1
            findings.append((line_no, reason, " ".join(span[:120].split())))
    return findings


def main():
    if not SRC.is_dir():
        print(f"ERROR: {SRC} not found", file=sys.stderr)
        return 2
    total = 0
    for path in sorted(SRC.rglob("*.rs")):
        for line_no, reason, snippet in scan(path):
            rel = path.relative_to(SRC.parent.parent)
            print(f"{rel}:{line_no}: {reason}\n    {snippet}")
            total += 1
    if total:
        print(
            f"\nFAIL: {total} potential secret-logging site(s) in indexer/src. "
            "Redact the value, or add `// log-secrets-ok: <reason>` if proven safe."
        )
        return 1
    print("OK: no secret-bearing values in indexer log/print macro arguments (SEC-F13 / #433).")
    return 0


if __name__ == "__main__":
    sys.exit(main())
