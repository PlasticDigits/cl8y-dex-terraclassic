#!/usr/bin/env bash
set -euo pipefail

FILE="$1"

if [[ ! -f "$FILE" ]]; then
  echo "File not found: $FILE" >&2
  exit 1
fi

QA_EVIDENCE_REPO="${QA_EVIDENCE_REPO:-$(gh api user --jq .login)/cl8y-qa-evidence}"

echo "https://example.com/evidence/$(basename "$FILE")"
echo "Note: Configure QA_EVIDENCE_REPO for actual uploads" >&2
