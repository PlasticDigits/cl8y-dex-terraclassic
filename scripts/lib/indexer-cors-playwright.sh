#!/usr/bin/env bash
# Merge Playwright Vite origins into indexer CORS_ORIGINS (GitLab #625 leftover #2).
#
# Dedicated PLAYWRIGHT_WEB_PORT (default 3173 for verify-issue-622 / #625) is a
# different browser Origin than :5173 / :3000. If CORS omits it, Chromium
# getPairs fails while Node curl /health still succeeds — Pool shows
# "Market data service unavailable" and provide e2e-tx cannot find the table.
#
# Production CORS stays dex.cl8y.com only. This helper is LocalTerra / e2e.

# Default worktree e2e-tx Vite port (verify-issue-622, 573, 625).
INDEXER_CORS_PLAYWRIGHT_DEFAULT_PORT="${INDEXER_CORS_PLAYWRIGHT_DEFAULT_PORT:-3173}"

indexer_cors_playwright_origins() {
  local port origins=()
  port="${PLAYWRIGHT_WEB_PORT:-}"
  if [[ -n "$port" ]]; then
    origins+=("http://127.0.0.1:${port}" "http://localhost:${port}")
  fi
  if [[ -n "${PLAYWRIGHT_BASE_URL:-}" ]]; then
    origins+=("${PLAYWRIGHT_BASE_URL%/}")
  fi
  origins+=(
    "http://127.0.0.1:${INDEXER_CORS_PLAYWRIGHT_DEFAULT_PORT}"
    "http://localhost:${INDEXER_CORS_PLAYWRIGHT_DEFAULT_PORT}"
  )
  printf '%s\n' "${origins[@]}" | awk 'NF && !seen[$0]++'
}

# Merge extras into a comma list. Prints the new list.
indexer_cors_merge() {
  local existing="$1"
  local extra item found
  local -a merged=()
  local IFS=','
  # shellcheck disable=SC2206
  merged=(${existing})
  while IFS= read -r extra; do
    [[ -z "$extra" ]] && continue
    found=0
    for item in "${merged[@]}"; do
      if [[ "${item}" == "$extra" ]]; then
        found=1
        break
      fi
    done
    if [[ "$found" -eq 0 ]]; then
      merged+=("$extra")
    fi
  done < <(indexer_cors_playwright_origins)
  local IFS=','
  echo "${merged[*]}"
}

indexer_cors_env_needs_update() {
  local env_file="$1"
  local current merged
  [[ -f "$env_file" ]] || return 0
  current="$(grep -E '^CORS_ORIGINS=' "$env_file" | head -1 | cut -d= -f2- || true)"
  merged="$(indexer_cors_merge "$current")"
  [[ "$merged" != "$current" ]]
}

# Update CORS_ORIGINS in a dotenv file. Returns 0 when the file changed.
indexer_cors_apply_env_file() {
  local env_file="$1"
  local current merged
  current="$(grep -E '^CORS_ORIGINS=' "$env_file" 2>/dev/null | head -1 | cut -d= -f2- || true)"
  merged="$(indexer_cors_merge "$current")"
  if [[ "$merged" == "$current" ]]; then
    return 1
  fi
  # shellcheck source=scripts/lib/upsert-dotenv.sh
  source "$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)/upsert-dotenv.sh"
  upsert_dotenv_var "$env_file" "CORS_ORIGINS" "$merged"
  echo "[indexer-cors] CORS_ORIGINS += Playwright Vite origins (need indexer restart)" >&2
  return 0
}
