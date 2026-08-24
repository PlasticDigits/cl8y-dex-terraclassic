#!/usr/bin/env bash
# Unit tests for scripts/lib/cw20-funding-kind.sh (GitLab #620).
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
# shellcheck source=scripts/lib/cw20-funding-kind.sh
source "$REPO_ROOT/scripts/lib/cw20-funding-kind.sh"

fail() { echo "FAIL: $*" >&2; exit 1; }

VITE_LUNC_C_TOKEN_ADDRESS="terra1wraplunc"
VITE_USTC_C_TOKEN_ADDRESS="terra1wrapustc"
VITE_TOKEN_COMMUNITY_TAX_ADDRESS="terra1qatax"

[[ "$(classify_cw20_funding_kind terra1wraplunc)" == skip ]] || fail "wrap LUNC must skip"
[[ "$(classify_cw20_funding_kind terra1wrapustc)" == skip ]] || fail "wrap USTC must skip"
[[ "$(classify_cw20_funding_kind terra1qatax)" == transfer ]] || fail "pinned tax must Transfer"
[[ "$(classify_cw20_funding_kind terra1gem)" == mint ]] || fail "gem must Mint"
[[ "$(classify_cw20_funding_kind terra1gem terra1launcher)" == transfer ]] \
  || fail "GetLauncherOrigin.launcher must Transfer"
[[ "$(classify_cw20_funding_kind terra1gem "" 1)" == transfer ]] \
  || fail "GetLauncherOrigin decoded with null launcher must Transfer (#623)"
[[ "$(classify_cw20_funding_kind terra1gem null)" == mint ]] || fail "null origin stays Mint"
[[ "$(classify_cw20_funding_kind terra1gem "")" == mint ]] || fail "empty origin stays Mint"

unset VITE_TOKEN_COMMUNITY_TAX_ADDRESS
[[ "$(classify_cw20_funding_kind terra1qatax)" == mint ]] || fail "unpinned tax address is Mint without origin"
[[ "$(classify_cw20_funding_kind terra1qatax terra1launcher)" == transfer ]] \
  || fail "unpinned + origin still Transfer"

[[ "$(classify_tax_provision_action terra1qatax terra1qatax terra1x46 terra1x46)" == fail_seed ]] \
  || fail "pinned tax + test1→test1 must fail_seed"
[[ "$(classify_tax_provision_action terra1old terra1qatax terra1x46 terra1x46)" == skip ]] \
  || fail "leftover tax + test1→test1 must skip"
[[ "$(classify_tax_provision_action terra1qatax terra1qatax terra1fund terra1x46)" == transfer ]] \
  || fail "distinct funder may Transfer"

echo "PASS: cw20-funding-kind classify (skip/transfer/mint + tax provision action)"
