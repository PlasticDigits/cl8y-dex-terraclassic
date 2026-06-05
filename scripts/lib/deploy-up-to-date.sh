#!/usr/bin/env bash
# Shared deploy freshness probe (stamp == HEAD, env aligned, factory LCD probe).
# Used by scripts/qa/start-qa.sh and scripts/setup-cloud-agent-localterra.sh (GitLab #325).
#
# Source after REPO_ROOT is set. Requires scripts/lib/lcd-smart-query.sh on PATH via source.

deploy_up_to_date() {
  local repo_root="${1:-${REPO_ROOT:-}}"
  local env_local="${repo_root}/frontend-dapp/.env.local"
  local stamp="${repo_root}/.qa-deploy-stamp"

  [[ -f "$stamp" && -f "$env_local" ]] || return 1

  local head stamp_sha factory_stamp factory_env
  head="$(git -C "$repo_root" rev-parse --short HEAD 2>/dev/null || true)"
  stamp_sha="$(grep -E '^git_sha=' "$stamp" | tail -n1 | sed 's/^git_sha=//')"
  [[ -n "$head" && "$head" = "$stamp_sha" ]] || return 1

  factory_stamp="$(grep -E '^factory_address=' "$stamp" | tail -n1 | sed 's/^factory_address=//')"
  factory_env="$(grep -E '^VITE_FACTORY_ADDRESS=' "$env_local" | tail -n1 | sed 's/^VITE_FACTORY_ADDRESS=//')"
  [[ -n "$factory_stamp" && "$factory_stamp" = "$factory_env" ]] || return 1

  compgen -G "${repo_root}/smartcontracts/artifacts/cl8y_dex_*.wasm" >/dev/null || return 1

  # shellcheck source=scripts/lib/wasm-artifacts-stale.sh
  source "${repo_root}/scripts/lib/wasm-artifacts-stale.sh"
  dex_wasm_stale_vs_sources "$repo_root" || return 1
  dex_wasm_newer_than_stamp "$repo_root" "$stamp" && return 1

  # shellcheck source=scripts/lib/lcd-smart-query.sh
  source "${repo_root}/scripts/lib/lcd-smart-query.sh"
  local lcd="http://127.0.0.1:${DEX_TERRA_LCD_PORT:-1317}"
  lcd_smart_query_ok "$lcd" "$factory_stamp" '{"pairs":{"start_after":null,"limit":1}}'
}
