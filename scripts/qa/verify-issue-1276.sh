#!/usr/bin/env bash
# Automated verification for Forgejo #1276 — indexer /health git SHA + auto-deploy leftover.
#
# Local: parser unit tests, serial API health tests, Dockerfile bake pin, leftover-complete
# FAIL-before-curl gate. Live Coolify is SKIP unless VERIFY1276_REQUIRE_LIVE=1 /
# VERIFY1276_IID=1276 / VERIFY1276_LEFTOVER_COMPLETE=1.
#
# Leftover-complete (operator, not implement): under IID=1276 or LEFTOVER_COMPLETE=1,
# VERIFY1276_EXPECT_SHA must be non-empty hex after trim or this script FAIL before curl.
# Bare IID without EXPECT_SHA is an intentional FAIL, not a #701 bug.
# Bake presence: REQUIRE_LIVE=1 without IID/EXPECT_SHA may PASS on hex git_sha.
#
# Do not scrape Coolify. Do not infer the auto-deploy checkbox from HTTP.
#
# Refs: skills/AGENTS_INDEXER_HEALTH_GIT_SHA.md, docs/adr/0006-indexer-health-git-sha.md
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }
skip() { RESULTS+=("SKIP  $1"); echo "  [SKIP] $1"; }

run_step() {
  local label="$1"
  shift
  echo ""
  echo "[$label]"
  set +e
  "$@"
  local rc=$?
  set -e
  if [[ $rc -eq 0 ]]; then
    ok "$label"
  else
    bad "$label"
  fi
}

trim_ascii() {
  local s="${1-}"
  s="${s#"${s%%[![:space:]]*}"}"
  s="${s%"${s##*[![:space:]]}"}"
  printf '%s' "$s"
}

leftover_complete_requested() {
  [[ "${VERIFY1276_IID:-}" == "1276" || "${VERIFY1276_LEFTOVER_COMPLETE:-}" == "1" ]]
}

live_probe_requested() {
  leftover_complete_requested || [[ "${VERIFY1276_REQUIRE_LIVE:-}" == "1" ]]
}

expect_sha_trimmed() {
  trim_ascii "${VERIFY1276_EXPECT_SHA-}"
}

is_hex_sha() {
  local s="$1"
  [[ "$s" =~ ^[0-9a-fA-F]{7,40}$ ]]
}

sha_prefix_match() {
  local a b
  a="$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')"
  b="$(printf '%s' "$2" | tr '[:upper:]' '[:lower:]')"
  [[ "$a" == "$b"* || "$b" == "$a"* ]]
}

echo "════════════════════════════════════════════════════════════════"
echo "  Forgejo #1276 — indexer /health git SHA + auto-deploy leftover"
echo "════════════════════════════════════════════════════════════════"

export PATH="${HOME}/.cargo/bin:/usr/local/cargo/bin:${HOME}/.cargo/bin:${PATH}"
export RUST_TEST_THREADS="${RUST_TEST_THREADS:-1}"

INDEXER_URL="${VERIFY1276_INDEXER_URL:-https://indexer.dex.cl8y.com}"
INDEXER_URL="${INDEXER_URL%/}"

run_step "docs: ADR 0006 + skill H1276 + invariants + Makefile" \
  bash -c '
    set -euo pipefail
    test -f docs/adr/0006-indexer-health-git-sha.md
    test -f skills/AGENTS_INDEXER_HEALTH_GIT_SHA.md
    grep -q "H1276-1" skills/AGENTS_INDEXER_HEALTH_GIT_SHA.md
    grep -q "H1276-8" skills/AGENTS_INDEXER_HEALTH_GIT_SHA.md
    grep -q "select_commit_env" docs/adr/0006-indexer-health-git-sha.md
    grep -q "VERIFY1276_EXPECT_SHA" docs/adr/0006-indexer-health-git-sha.md
    grep -q "Health git SHA (#1276)" docs/indexer-invariants.md
    grep -q "verify-issue-1276" Makefile
    grep -q "make verify-issue-1276" docs/testing.md
    grep -q "indexer-production-attest" docs/architecture.md
    python3 scripts/check_rollback_decision_docs.py
  '

run_step "dockerfile: runtime ARG/ENV after COPY --from=builder, before HEALTHCHECK" \
  python3 - <<'PY'
from pathlib import Path
text = Path("docker/indexer/Dockerfile").read_text()
if "nixpacks" in text.lower():
    raise SystemExit("Nixpacks is not the prod bake path")
# Image must not run git in either stage.
for line in text.splitlines():
    s = line.strip()
    if s.startswith("#"):
        continue
    if " git " in f" {s} " or s.endswith(" git") or "git-core" in s:
        raise SystemExit(f"Dockerfile must not run git: {s}")
runtime = text.split("AS runtime", 1)
if len(runtime) != 2:
    raise SystemExit("missing runtime stage")
rt = runtime[1]
# ARG must not sit immediately after FROM runtime (apt layer bust).
# Ignore comments so "before HEALTHCHECK" prose cannot invert instruction order.
instr = []
for ln in rt.splitlines():
    s = ln.strip()
    if not s or s.startswith("#"):
        continue
    instr.append(s)
if not instr or "apt-get" not in instr[0]:
    raise SystemExit("runtime stage must start with apt-get, not commit ARG")
joined = "\n".join(instr)
copy_i = joined.find("COPY --from=builder")
arg_i = joined.find("ARG GIT_SHA")
src_i = joined.find("ARG SOURCE_COMMIT")
env_i = joined.find("ENV GIT_SHA")
hc_i = joined.find("HEALTHCHECK")
if min(copy_i, arg_i, src_i, env_i, hc_i) < 0:
    raise SystemExit("missing COPY/ARG/ENV/HEALTHCHECK in runtime stage")
if not (copy_i < arg_i < src_i < env_i < hc_i):
    raise SystemExit("ARG/ENV must be after COPY --from=builder and before HEALTHCHECK")
print("runtime ARG/ENV pin ok")
PY

run_step "source: per-request health helpers; no dotenvy in handler" \
  bash -c '
    set -euo pipefail
    grep -q "mod health_git_sha" indexer/src/api/mod.rs
    grep -q "select_commit_env" indexer/src/api/health_git_sha.rs
    grep -q "parse_git_sha" indexer/src/api/health_git_sha.rs
    grep -q "std::env::var(\"GIT_SHA\")" indexer/src/api/mod.rs
    grep -q "std::env::var(\"SOURCE_COMMIT\")" indexer/src/api/mod.rs
    if grep -nE "dotenvy::|use dotenvy" indexer/src/api/mod.rs indexer/src/api/health_git_sha.rs; then
      echo "health path must not call dotenvy" >&2
      exit 1
    fi
    grep -q "remove_var(\"GIT_SHA\")" indexer/tests/api_fee_discount_health.rs
    grep -q "remove_var(\"SOURCE_COMMIT\")" indexer/tests/api_fee_discount_health.rs
    grep -q "generic_health_unchanged" indexer/tests/api_fee_discount_health.rs
    grep -q "health_empty_git_sha_falls_through_to_source_commit" indexer/tests/api_health.rs
    grep -q "GIT_SHA" indexer/.env.example
    grep -q "SOURCE_COMMIT" indexer/.env.example
    if grep -nE "GIT_SHA=HEAD|GIT_SHA=\"HEAD\"|GIT_SHA='HEAD'" indexer/.env.example; then
      echo "must not document copying HEAD into GIT_SHA" >&2
      exit 1
    fi
  '

if [ ! -f "$REPO_ROOT/indexer/.env" ]; then
  echo ""
  echo "[bootstrap] indexer/.env missing — running make setup-indexer-postgres…"
  make setup-indexer-postgres
fi

run_step "indexer lib: select_commit_env + parse_git_sha" \
  bash -c 'cd indexer && cargo test --lib health_git_sha -- --quiet'

run_step "indexer api: /health git_sha + fee-discount unchanged" \
  bash -c 'cd indexer && cargo test --jobs 1 --test api_health -- --test-threads=1 --quiet && cargo test --jobs 1 --test api_fee_discount_health -- --test-threads=1 --quiet'

echo ""
echo "[leftover-complete FAIL-before-curl gate]"
if leftover_complete_requested && [[ -z "$(expect_sha_trimmed)" ]]; then
  bad "VERIFY1276_IID=1276 / LEFTOVER_COMPLETE=1 requires VERIFY1276_EXPECT_SHA (FAIL before curl)"
else
  ok "leftover-complete EXPECT_SHA gate (not requested, or EXPECT_SHA set)"
fi

# Always prove the intentional FAIL path locally (does not GET /health).
echo ""
echo "[selftest: bare IID without EXPECT_SHA fails before curl]"
if (
  unset VERIFY1276_EXPECT_SHA VERIFY1276_REQUIRE_LIVE VERIFY1276_LEFTOVER_COMPLETE
  VERIFY1276_IID=1276
  leftover_complete_requested && [[ -z "$(expect_sha_trimmed)" ]]
); then
  ok "bare VERIFY1276_IID=1276 without EXPECT_SHA is FAIL-before-curl"
else
  bad "bare IID leftover-complete gate did not fail closed"
fi

echo ""
echo "[live leftover probe ${INDEXER_URL}/health]"
if leftover_complete_requested && [[ -z "$(expect_sha_trimmed)" ]]; then
  skip "live GET skipped (leftover-complete FAIL already recorded before curl)"
elif ! live_probe_requested; then
  skip "live Coolify (set VERIFY1276_REQUIRE_LIVE=1 or VERIFY1276_IID=1276 + EXPECT_SHA)"
else
  if ! command -v jq >/dev/null 2>&1; then
    bad "jq required to parse live /health JSON"
  else
    expect="$(expect_sha_trimmed)"
    if [[ -n "$expect" ]] && ! is_hex_sha "$expect"; then
      bad "VERIFY1276_EXPECT_SHA must be hex 7–40 after trim"
    else
      set +e
      body="$(curl -fsS --max-time 15 "${INDEXER_URL}/health" 2>/tmp/verify1276-curl.err)"
      rc=$?
      set -e
      if [[ $rc -ne 0 ]]; then
        bad "live GET ${INDEXER_URL}/health unreachable (FAIL under live flag)"
      else
        status="$(printf '%s' "$body" | jq -r '.status // empty')"
        git_sha="$(printf '%s' "$body" | jq -r '.git_sha // empty')"
        git_sha_lc="$(printf '%s' "$git_sha" | tr '[:upper:]' '[:lower:]')"
        if [[ "$status" != "ok" ]]; then
          bad "live /health status is not ok"
        elif [[ -z "$git_sha" ]]; then
          bad "live /health omitted git_sha (FAIL, not SKIP)"
        elif ! [[ "$git_sha_lc" =~ ^[0-9a-f]{7,40}$ ]]; then
          bad "live git_sha is not lowercase hex 7–40"
        else
          if [[ -n "$expect" ]]; then
            if sha_prefix_match "$git_sha_lc" "$expect"; then
              ok "live /health git_sha prefix-matches EXPECT_SHA"
            else
              bad "live git_sha does not prefix-match VERIFY1276_EXPECT_SHA"
            fi
          else
            ok "live /health bake presence (hex git_sha; not leftover-complete)"
          fi
        fi
      fi
    fi
  fi
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
echo "  RESULTS: $PASS passed, $FAIL failed"
for r in "${RESULTS[@]}"; do echo "    $r"; done
echo "════════════════════════════════════════════════════════════════"

if [ "$FAIL" -gt 0 ]; then
  exit 1
fi
exit 0
