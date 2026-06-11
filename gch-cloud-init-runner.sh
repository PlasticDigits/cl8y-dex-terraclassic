#!/usr/bin/env bash
# SPDX-License-Identifier: AGPL-3.0-or-later
# Shared agent runner — sourced or called from project gch-cloud-init.sh
set -euo pipefail

: "${GCH_JOB_ID:?}"
: "${GCH_CONTROLLER_URL:?}"
: "${JOB_RUNTIME_TOKEN:?}"
: "${CURSOR_API_KEY:?}"

HEARTBEAT_PID=""

cleanup() {
  if [[ -n "${HEARTBEAT_PID}" ]]; then
    kill "${HEARTBEAT_PID}" 2>/dev/null || true
  fi
}
trap cleanup EXIT

start_heartbeat() {
  (
    while true; do
      sleep 60
      curl -sf -X POST \
        -H "Authorization: Bearer ${JOB_RUNTIME_TOKEN}" \
        "${GCH_CONTROLLER_URL}/api/jobs/${GCH_JOB_ID}/heartbeat" \
        >/dev/null || true
    done
  ) &
  HEARTBEAT_PID=$!
}

fetch_job() {
  curl -sf \
    -H "Authorization: Bearer ${JOB_RUNTIME_TOKEN}" \
    "${GCH_CONTROLLER_URL}/api/jobs/${GCH_JOB_ID}"
}

post_status() {
  local phase="$1"
  local message="${2:-}"
  curl -sf -X POST \
    -H "Authorization: Bearer ${JOB_RUNTIME_TOKEN}" \
    -H "Content-Type: application/json" \
    -d "{\"phase\":\"${phase}\",\"message\":\"${message}\"}" \
    "${GCH_CONTROLLER_URL}/api/jobs/${GCH_JOB_ID}/status" \
    >/dev/null || true
}

post_complete() {
  local status="$1"
  local exit_code="${2:-0}"
  curl -sf -X POST \
    -H "Authorization: Bearer ${JOB_RUNTIME_TOKEN}" \
    -H "Content-Type: application/json" \
    -d "{\"status\":\"${status}\",\"exit_code\":${exit_code}}" \
    "${GCH_CONTROLLER_URL}/api/jobs/${GCH_JOB_ID}/complete" \
    >/dev/null || true
}

run_cursor_agent() {
  local workspace="$1"
  local prompt="$2"
  local model="$3"

  export CURSOR_API_KEY
  export DISPLAY="${DISPLAY:-:99}"

  if ! pgrep -x Xvfb >/dev/null 2>&1; then
    Xvfb :99 -screen 0 1920x1080x24 &
    sleep 1
  fi

  cd "${workspace}"
  agent -p "${prompt}" \
    --model "${model}" \
    --force \
    --trust \
    --workspace "${workspace}" \
    --output-format stream-json
}

gch_run_job() {
  local job_json
  job_json="$(fetch_job)"

  local workspace prompt model git_ref
  workspace="$(echo "${job_json}" | jq -r .workspace)"
  prompt="$(echo "${job_json}" | jq -r .prompt)"
  model="$(echo "${job_json}" | jq -r .model)"
  git_ref="$(echo "${job_json}" | jq -r .git_ref // empty)"

  post_status "boot" "cloud-init runner started"
  start_heartbeat

  if [[ -n "${git_ref}" && "${git_ref}" != "null" ]]; then
    post_status "git" "checking out ${git_ref}"
    cd "${workspace}"
    git fetch --all || true
    git checkout "${git_ref}" || true
  fi

  post_status "agent" "starting cursor agent"
  set +e
  run_cursor_agent "${workspace}" "${prompt}" "${model}"
  local exit_code=$?
  set -e

  if [[ ${exit_code} -eq 0 ]]; then
    post_complete "success" "${exit_code}"
  else
    post_complete "failed" "${exit_code}"
  fi

  return "${exit_code}"
}
