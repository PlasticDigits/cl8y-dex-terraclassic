#!/usr/bin/env bash
# Unit checks for GitLab #325 QA redeploy decision guide + optimizations (no Docker).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"

_fail() {
  echo "FAIL: $*" >&2
  exit 1
}

grep -q 'deploy-local-no-build' "$REPO_ROOT/Makefile" \
  || _fail 'Makefile must define deploy-local-no-build'

grep -q 'test-qa-redeploy-decision' "$REPO_ROOT/Makefile" \
  || _fail 'Makefile must define test-qa-redeploy-decision'

grep -q 'deploy_up_to_date' "$REPO_ROOT/scripts/lib/deploy-up-to-date.sh" \
  || _fail 'missing scripts/lib/deploy-up-to-date.sh'

grep -q 'deploy_up_to_date' "$REPO_ROOT/scripts/qa/start-qa.sh" \
  || _fail 'start-qa.sh must use deploy_up_to_date skip'

grep -q 'terrad_wait_tx_inclusion' "$REPO_ROOT/scripts/lib/terrad-wait-tx.sh" \
  || _fail 'missing terrad-wait-tx helper'

grep -q 'terrad-wait-tx.sh' "$REPO_ROOT/scripts/deploy-dex-local.sh" \
  || _fail 'deploy-dex-local.sh must source terrad-wait-tx.sh'

grep -q 'QA_DEPLOY_SEED' "$REPO_ROOT/scripts/deploy-dex-local.sh" \
  || _fail 'deploy-dex-local.sh must support QA_DEPLOY_SEED profiles'

grep -q 'qa-phase-timing' "$REPO_ROOT/scripts/qa/start-qa.sh" \
  || _fail 'start-qa.sh must log phase timing'

grep -q 'fetch-qa-ci-artifacts' "$REPO_ROOT/scripts/qa/fetch-qa-ci-artifacts.sh" \
  || _fail 'missing fetch-qa-ci-artifacts.sh'

grep -q 'publish-qa-ci-artifacts' "$REPO_ROOT/scripts/qa/publish-qa-ci-artifacts.sh" \
  || _fail 'missing publish-qa-ci-artifacts.sh'

grep -q 'qa-wasm-artifacts' "$REPO_ROOT/.gitlab-ci.yml" \
  || _fail '.gitlab-ci.yml must publish qa-wasm artifacts'

grep -q 'qa-indexer-binary' "$REPO_ROOT/.gitlab-ci.yml" \
  || _fail '.gitlab-ci.yml must publish qa-indexer binary'

grep -q 'make reset-qa' "$REPO_ROOT/skills/AGENTS_QA_REDEPLOY_DECISION.md" \
  || _fail 'decision guide must document reset-qa'

grep -q 'deploy-local-no-build' "$REPO_ROOT/skills/AGENTS_QA_REDEPLOY_DECISION.md" \
  || _fail 'decision guide must document deploy-local-no-build'

grep -q 'AGENTS_QA_REDEPLOY_DECISION' "$REPO_ROOT/scripts/qa/README.md" \
  || _fail 'scripts/qa/README.md must link decision guide'

echo "OK: qa redeploy decision (#325)"
