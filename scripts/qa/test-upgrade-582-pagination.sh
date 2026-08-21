#!/usr/bin/env bash
# GitLab #584 — pagination mock: ≥31 pairs across full pages + a short last page.
# Does not accept "we only have 14 on mainnet today".
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
# shellcheck source=../lib/upgrade-582-code-id-pin.sh
source "$REPO_ROOT/scripts/lib/upgrade-582-code-id-pin.sh"

die() { echo "FAIL: $*" >&2; exit 1; }

make_pair() {
  local i="$1"
  jq -nc --argjson i "$i" \
    '{asset_infos:[{token:{contract_addr:("terra1a"+($i|tostring))}},{token:{contract_addr:("terra1b"+($i|tostring))}}],contract_addr:("terra1p"+($i|tostring)),liquidity_token:("terra1lp"+($i|tostring))}'
}

make_page() {
  local start="$1"
  local n="$2"
  local i json="[]"
  for ((i = start; i < start + n; i++)); do
    json="$(jq -c --argjson p "$(make_pair "$i")" '. + [$p]' <<<"$json")"
  done
  jq -nc --argjson pairs "$json" '{pairs:$pairs}'
}

echo "== 31 pairs: full page of 30 + short page of 1 =="
upgrade582_fetch_pairs_page() {
  local sa="${1:-}"
  if [[ -z "$sa" || "$sa" == "null" ]]; then
    make_page 0 30
  elif printf '%s' "$sa" | grep -q 'terra1a29'; then
    make_page 30 1
  else
    die "unexpected start_after for 31-pair mock: $sa"
  fi
}

mapfile -t ROWS < <(upgrade582_enumerate_pairs)
[[ "${#ROWS[@]}" -eq 31 ]] || die "expected 31 pairs, got ${#ROWS[@]}"
echo "  ok: 31 pairs, 2 queries, start_after on page 2 set"

echo "== 61 pairs: two full pages + short third =="
upgrade582_fetch_pairs_page() {
  local sa="${1:-}"
  if [[ -z "$sa" || "$sa" == "null" ]]; then
    make_page 0 30
  elif printf '%s' "$sa" | grep -q 'terra1a29'; then
    make_page 30 30
  elif printf '%s' "$sa" | grep -q 'terra1a59'; then
    make_page 60 1
  else
    die "unexpected start_after for 61-pair mock: $sa"
  fi
}

mapfile -t ROWS < <(upgrade582_enumerate_pairs)
[[ "${#ROWS[@]}" -eq 61 ]] || die "expected 61 pairs, got ${#ROWS[@]}"
echo "  ok: 61 pairs, 3 queries"

echo "== limit is 30 (never 60) =="
[[ "$UPGRADE582_PAIR_PAGE_LIMIT" -eq 30 ]] || die "UPGRADE582_PAIR_PAGE_LIMIT=$UPGRADE582_PAIR_PAGE_LIMIT"
echo "  ok"

echo "OK pagination"
