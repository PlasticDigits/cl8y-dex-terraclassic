# shellcheck shell=bash
# Shared QA compose/env helpers. Source after REPO_ROOT is set.
#
# Invariants (see docs/qa-invariants.md, GitLab #202):
#   - Default `make start-qa` keeps Docker volumes (fast restarts).
#   - `QA_FRESH_VOLUMES=1` or `make reset-qa` runs `docker compose down -v` before bring-up.

qa_load_env() {
  set -a
  if [ -f "${REPO_ROOT}/.env" ]; then
    # shellcheck source=/dev/null
    source "${REPO_ROOT}/.env"
  fi
  # shellcheck source=/dev/null
  source "${REPO_ROOT}/scripts/qa/qa-host.env"
  set +a

  if [ "${QA_SHARED_HOST:-}" = "1" ]; then
    export COMPOSE_FILE="${COMPOSE_FILE:-docker-compose.yml:docker-compose.qa-shared-host.yml}"
  fi
}

# Truthy: 1, true, yes (case-insensitive).
qa_is_fresh_volumes() {
  case "${QA_FRESH_VOLUMES:-}" in
    1 | true | TRUE | yes | YES) return 0 ;;
    *) return 1 ;;
  esac
}

# Args passed to `docker compose` for teardown (empty or "-v").
qa_compose_down_volume_args() {
  if qa_is_fresh_volumes; then
    printf '%s' '-v'
  fi
}
