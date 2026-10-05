#!/usr/bin/env bash
# publish-qa-ci-artifacts — publish wasm tarball and indexer binary to GitLab generic packages (CI job helper).
# GitLab #325
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

GIT_SHA="${CI_COMMIT_SHORT_SHA:-$(git rev-parse --short HEAD)}"
PROJECT_ID="${CI_PROJECT_ID:?CI_PROJECT_ID required}"
API="${CI_API_V4_URL:-https://gitlab.com/api/v4}"
TOKEN="${CI_JOB_TOKEN:?CI_JOB_TOKEN required}"

_expected_image() {
  python3 - "$REPO_ROOT/scripts/ci/image-pins.json" "$1" <<'PY'
import json
import sys
with open(sys.argv[1]) as stream:
    pins = json.load(stream)
print(sys.argv[2] + "@" + pins[sys.argv[2]])
PY
}

_publish() {
  local pkg="$1" file="$2" path="$3"
  local url="${API}/projects/${PROJECT_ID}/packages/generic/${pkg}/${GIT_SHA}/${file}"
  curl -fsSL --header "JOB-TOKEN: ${TOKEN}" --upload-file "$path" "$url"
  echo "[publish-qa-artifacts] uploaded ${pkg}/${GIT_SHA}/${file}"
}

if [ "${PUBLISH_QA_WASM:-1}" = "1" ] && compgen -G "${REPO_ROOT}/smartcontracts/artifacts/cl8y_dex_*.wasm" >/dev/null; then
  WASM_REQUIRED=()
  for file in smartcontracts/artifacts/cl8y_dex_*.wasm; do
    WASM_REQUIRED+=(--require-artifact "$file")
  done
  python3 scripts/ci/build-provenance.py --root "$REPO_ROOT" \
    --verify smartcontracts/artifacts/build-provenance.json \
    --expected-image "$(_expected_image cosmwasm/workspace-optimizer:0.16.1)" \
    "${WASM_REQUIRED[@]}"
  WASM_TAR="${REPO_ROOT}/qa-wasm-artifacts.tar.gz"
  tar czf "$WASM_TAR" -C "${REPO_ROOT}/smartcontracts" artifacts
  _publish "qa-wasm" "artifacts.tar.gz" "$WASM_TAR"
fi

INDEXER_BIN="${REPO_ROOT}/indexer/target/release/cl8y-dex-indexer"
if [ "${PUBLISH_QA_INDEXER:-1}" = "1" ] && [ -x "$INDEXER_BIN" ]; then
  INDEXER_RECEIPT="${REPO_ROOT}/indexer/target/release/build-provenance.json"
  EXPECTED_INDEXER_IMAGE="$(_expected_image rust:1.96-bookworm)"
  test "${CI_JOB_IMAGE:?CI_JOB_IMAGE required for hosted publication}" = "$EXPECTED_INDEXER_IMAGE"
  python3 scripts/ci/build-provenance.py --root "$REPO_ROOT" --verify "$INDEXER_RECEIPT" \
    --expected-image "$EXPECTED_INDEXER_IMAGE" --require-artifact "$INDEXER_BIN"
  _publish "qa-indexer" "cl8y-dex-indexer" "$INDEXER_BIN"
  _publish "qa-indexer" "build-provenance.json" "$INDEXER_RECEIPT"
fi
