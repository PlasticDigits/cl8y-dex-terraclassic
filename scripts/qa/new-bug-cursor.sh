#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "${SCRIPT_DIR}/../.." && pwd)"
TEMPLATE="${REPO_ROOT}/docs/qa-templates/frontend-bug.md"
UPLOAD_SCRIPT="${SCRIPT_DIR}/upload-evidence.sh"

if ! command -v gh >/dev/null 2>&1; then
  echo "Error: gh CLI is required." >&2
  exit 1
fi

if [[ ! -f "${TEMPLATE}" ]]; then
  echo "Error: template not found at ${TEMPLATE}" >&2
  exit 1
fi

TITLE=""
declare -a EVIDENCE_FILES=()

while [[ $# -gt 0 ]]; do
  case "$1" in
    -e|--evidence)
      EVIDENCE_FILES+=("$2"); shift 2 ;;
    -h|--help)
      echo "Usage: ./scripts/qa/new-bug-cursor.sh [--evidence /path/to/file] [title]"; exit 0 ;;
    *)
      if [[ -z "${TITLE}" ]]; then TITLE="$1"; else TITLE="${TITLE} $1"; fi; shift ;;
  esac
done

if [[ -z "${TITLE}" ]]; then
  read -r -p "Issue title (without 'bug:' prefix): " SHORT_TITLE
  TITLE="bug: ${SHORT_TITLE}"
fi

TMP_FILE="$(mktemp -t cl8y-dex-bug-XXXXXX.md)"
trap 'rm -f "${TMP_FILE}"' EXIT
cp "${TEMPLATE}" "${TMP_FILE}"

if [[ ${#EVIDENCE_FILES[@]} -gt 0 && -x "${UPLOAD_SCRIPT}" ]]; then
  {
    echo ""
    echo "### Auto-uploaded Evidence"
    for file_path in "${EVIDENCE_FILES[@]}"; do
      uploaded_url="$("${UPLOAD_SCRIPT}" "${file_path}")"
      base_name="$(basename "${file_path}")"
      echo "- [${base_name}](${uploaded_url})"
    done
  } >> "${TMP_FILE}"
fi

cursor "${TMP_FILE}"
read -r -p "Press Enter when done editing in Cursor..."

gh issue create --title "${TITLE}" --body-file "${TMP_FILE}" --label bug --label frontend --label needs-triage
