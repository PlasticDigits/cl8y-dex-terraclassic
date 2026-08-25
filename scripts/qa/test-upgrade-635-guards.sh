#!/usr/bin/env bash
# Guards for scripts/upgrade-635-autoregister.sh — no chain.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
# shellcheck source=../lib/upgrade-635-autoregister.sh
source "$REPO_ROOT/scripts/lib/upgrade-635-autoregister.sh"

die() { echo "FAIL: $*" >&2; exit 1; }

for id in 8654 11612 11613 11614 11620 11621 11622; do
  upgrade635_is_forbidden_whitelist "$id" || die "expected $id forbidden"
done
upgrade635_is_forbidden_whitelist 11700 && die "11700 should be allowed"

if (upgrade635_assert_whitelist_ok 8654) 2>/dev/null; then
  die "8654 must be rejected"
fi
if (upgrade635_assert_whitelist_ok 11611) 2>/dev/null; then
  die "11611 must be rejected (already stored)"
fi
if (upgrade635_assert_whitelist_ok 11619) 2>/dev/null; then
  die "11619 must be rejected (already stored)"
fi
if (upgrade635_assert_whitelist_ok 11626) 2>/dev/null; then
  die "11626 must be rejected (already stored)"
fi
upgrade635_assert_whitelist_ok 11750

echo "OK upgrade-635 guards"
