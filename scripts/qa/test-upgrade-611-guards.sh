#!/usr/bin/env bash
# Guards for scripts/upgrade-611-community-tax.sh — no chain.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=../lib/upgrade-611-community-tax.sh
source "$REPO_ROOT/scripts/lib/upgrade-611-community-tax.sh"

die() { echo "FAIL: $*" >&2; exit 1; }

for id in 8654 11612 11613 11614 11620 11621; do
  upgrade611_is_forbidden_whitelist "$id" || die "expected $id forbidden"
done
upgrade611_is_forbidden_whitelist 11700 && die "11700 should be allowed"
upgrade611_is_forbidden_whitelist 11700 "11701 11702" && die "11700 not in extra"
upgrade611_is_forbidden_whitelist 11701 "11701 11702" || die "extra sister must be forbidden"

if (upgrade611_assert_whitelist_ok 11611) 2>/dev/null; then
  die "11611 must be rejected (already listed)"
fi
if (upgrade611_assert_whitelist_ok 11619) 2>/dev/null; then
  die "11619 must be rejected (already listed)"
fi
if (upgrade611_assert_whitelist_ok 11613) 2>/dev/null; then
  die "11613 must be rejected"
fi
if (upgrade611_assert_whitelist_ok 11750 11750 11751) 2>/dev/null; then
  die "token id must not equal launcher/autolp extra"
fi
upgrade611_assert_whitelist_ok 11750 11751 11752

echo "OK upgrade-611 guards"
