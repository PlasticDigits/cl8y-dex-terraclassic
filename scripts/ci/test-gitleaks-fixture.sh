#!/usr/bin/env bash
# Attack/abuse check: gitleaks must fail on a dummy secret (GitLab #380 test plan).
# Does not commit or leave artifacts in the repo.
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$ROOT"

if ! command -v gitleaks >/dev/null 2>&1; then
  echo "SKIP: gitleaks not installed"
  exit 0
fi

TMP="$(mktemp -d)"
trap 'rm -rf "$TMP"' EXIT

# Build a GitHub-PAT-shaped test vector at runtime (no literal secret in git).
printf '%s\n' "ghp_$(openssl rand -hex 20)" >"${TMP}/leak-fixture.txt"

set +e
gitleaks detect --source "$TMP" --no-git -c .gitleaks.toml --verbose
RC=$?
set -e

if [[ "$RC" -eq 0 ]]; then
  echo "FAIL: gitleaks did not flag dummy GitHub PAT fixture"
  exit 1
fi

echo "PASS: gitleaks flagged dummy secret fixture (exit ${RC})"
