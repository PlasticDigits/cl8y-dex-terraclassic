#!/usr/bin/env bash
# GitLab #433 (SEC-F13): fail if indexer tracing macros log secret field names.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
SRC="${REPO_ROOT}/indexer/src"

# Field / env names that must not appear in tracing macro arguments.
SECRET_PATTERN='database_url|DATABASE_URL|password|mnemonic|bearer|private_key|reorg_alert_webhook_url|\bsecret\b'

matches="$(rg -n --pcre2 \
  "tracing::(info|debug|warn|error|trace)!.*(${SECRET_PATTERN})" \
  "$SRC" 2>/dev/null || true)"

if [[ -n "$matches" ]]; then
  echo "lint-indexer-log-secrets: secret-pattern field(s) found in tracing log arguments:" >&2
  echo "$matches" >&2
  echo >&2
  echo "Indexer logs must not emit DATABASE_URL, webhook URLs, mnemonics, or other secrets." >&2
  echo "See docs/operator-secrets.md and GitLab #433 (SEC-F13)." >&2
  exit 1
fi

echo "OK: no secret-pattern fields in indexer tracing log arguments (SEC-F13)"
