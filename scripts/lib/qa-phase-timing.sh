#!/usr/bin/env bash
# Phase timing helpers for QA bring-up scripts (GitLab #325).

_QA_TIMING_PHASE=''
_QA_TIMING_START=0
_QA_TIMING_OVERALL_START=0

qa_timing_begin_session() {
  _QA_TIMING_OVERALL_START=$(date +%s)
  echo "[timing] session started at $(date -u +%Y-%m-%dT%H:%M:%SZ)"
}

qa_timing_phase_start() {
  _QA_TIMING_PHASE="$1"
  _QA_TIMING_START=$(date +%s)
  echo "[timing] phase '${_QA_TIMING_PHASE}' started"
}

qa_timing_phase_end() {
  local end now phase duration
  end=$(date +%s)
  phase="${_QA_TIMING_PHASE:-unknown}"
  duration=$((end - _QA_TIMING_START))
  echo "[timing] phase '${phase}' finished in ${duration}s"
  _QA_TIMING_PHASE=''
}

qa_timing_session_end() {
  local end duration
  end=$(date +%s)
  duration=$((end - _QA_TIMING_OVERALL_START))
  echo "[timing] session finished in ${duration}s"
}
