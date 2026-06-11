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

  # Agent shell tools must inherit GitLab credentials (job.env is not automatic).
  if [[ -f /etc/gch/job.env ]]; then
    set -a
    # shellcheck source=/dev/null
    source /etc/gch/job.env
    set +a
  fi
  export GLAB_TOKEN="${GITLAB_TOKEN:-}"

  export CURSOR_API_KEY
  export DISPLAY="${DISPLAY:-:99}"

  if ! pgrep -x Xvfb >/dev/null 2>&1; then
    Xvfb :99 -screen 0 1920x1080x24 &
    sleep 1
  fi

  cd "${workspace}"

  # Long idle: no stream-json while a shell command runs (playwright, npm, etc.).
  local idle_secs="${GCH_AGENT_IDLE_TIMEOUT_SECS:-1200}"
  # Short idle: after a thinking/completed line, CLI hang is likely if still silent.
  local idle_after_thinking_secs="${GCH_AGENT_IDLE_AFTER_THINKING_SECS:-60}"
  local max_secs="${GCH_AGENT_MAX_TIMEOUT_SECS:-10800}"
  local agent_cmd=(
    agent -p "${prompt}"
    --model "${model}"
    --force
    --trust
    --workspace "${workspace}"
    --output-format stream-json
  )

  # Cursor CLI `agent -p` is supposed to exit when done but often hangs (worker /
  # background shell tasks). Stop after idle_secs with no stdout so the runner
  # can POST /complete and the controller can destroy the VM.
  local runner_dir wrap_py
  runner_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
  wrap_py="${runner_dir}/gch-agent-idle-wrap.py"
  if [[ -x "${wrap_py}" ]] || [[ -f "${wrap_py}" ]]; then
    python3 "${wrap_py}" "${idle_secs}" "${idle_after_thinking_secs}" "${max_secs}" -- "${agent_cmd[@]}"
  else
    agent -p "${prompt}" \
      --model "${model}" \
      --force \
      --trust \
      --workspace "${workspace}" \
      --output-format stream-json
  fi
}

gch_run_job() {
  local job_json
  job_json="$(fetch_job)"

  local workspace prompt model git_ref
  workspace="$(echo "${job_json}" | jq -r .workspace)"
  prompt="$(echo "${job_json}" | jq -r .prompt)"
  model="$(echo "${job_json}" | jq -r .model)"
  git_ref="$(echo "${job_json}" | jq -r '.git_ref // empty')"

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
