#!/usr/bin/env bash
# Automated verification for #1240 — /protocol mixed-case product tickers
# (cUSTC / vFDUSD) must not be flattened by Tailwind `uppercase`.
#
# Proves (unit + docs; no chain, no indexer API change):
#   1. Hub <dt> / oracle H2 / tabs / Venus heading omit `uppercase`.
#   2. Ticker maps still print cUSTC / vFDUSD (ids unchanged).
#   3. ProtocolPage + StatBox exact-case RTL.
#   4. Skills / frontend.md P1240-1–P1240-8 + AGENTS / testing crosslinks.
#
# Refs: skills/AGENTS_FRONTEND_PROTOCOL_STATS.md,
#       frontend-dapp/src/components/protocol/ProtocolOracleCard.tsx,
#       frontend-dapp/src/components/protocol/ProtocolDexHubPrices.tsx
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
declare -a RESULTS=()

ok()  { RESULTS+=("PASS  $1"); PASS=$((PASS + 1)); echo "  [PASS] $1"; }
bad() { RESULTS+=("FAIL  $1"); FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }

run_step() {
  local label="$1"
  shift
  echo ""
  echo "[$label]"
  if "$@"; then
    ok "$label"
  else
    bad "$label"
  fi
}

echo "════════════════════════════════════════════════════════════════"
echo "  #1240 — /protocol mixed-case tickers (cUSTC / vFDUSD)"
echo "════════════════════════════════════════════════════════════════"

run_step "frontend: Protocol page exact-case tickers" \
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/pages/ProtocolPage.test.tsx

run_step "frontend: StatBox preserveLabelCase" \
  bash scripts/with-node.sh --cwd frontend-dapp -- npm test -- --run \
    src/components/ui/__tests__/StatBox.test.tsx

run_step "code: ticker-bearing nodes omit uppercase; chrome H2 keeps it" \
  python3 - <<'PY'
from pathlib import Path
import re
import sys

hub = Path("frontend-dapp/src/components/protocol/ProtocolDexHubPrices.tsx").read_text()
oracle = Path("frontend-dapp/src/components/protocol/ProtocolOracleCard.tsx").read_text()

dt = re.search(r"<dt className=\"([^\"]+)\"", hub)
if not dt:
    print("missing hub <dt>", file=sys.stderr)
    sys.exit(1)
if "uppercase" in dt.group(1).split():
    print(f"hub dt still uppercase: {dt.group(1)}", file=sys.stderr)
    sys.exit(1)

h2_chrome = re.search(r"<h2 className=\"([^\"]+)\"[^>]*>\s*\n\s*DEX hub prices", hub)
if not h2_chrome:
    # className is on the same line as DEX hub prices in this file
    m = re.search(r'<h2 className="([^"]+)"[^>]*>\s*DEX hub prices', hub)
    if not m:
        print("missing DEX hub prices h2", file=sys.stderr)
        sys.exit(1)
    if "uppercase" not in m.group(1).split():
        print("DEX hub prices h2 lost chrome uppercase", file=sys.stderr)
        sys.exit(1)
else:
    if "uppercase" not in h2_chrome.group(1).split():
        print("DEX hub prices h2 lost chrome uppercase", file=sys.stderr)
        sys.exit(1)

oracle_h2 = re.search(r'<h2\s+className="([^"]+)"', oracle)
if not oracle_h2:
    print("missing oracle h2", file=sys.stderr)
    sys.exit(1)
if "uppercase" in oracle_h2.group(1).split():
    print(f"oracle h2 still uppercase: {oracle_h2.group(1)}", file=sys.stderr)
    sys.exit(1)

tab = re.search(r"className=\{`([^`]+)`\}", oracle)
if not tab:
    print("missing oracle tab className", file=sys.stderr)
    sys.exit(1)
if "uppercase" in tab.group(1).split():
    print(f"oracle tab still uppercase: {tab.group(1)}", file=sys.stderr)
    sys.exit(1)

venus_h3 = re.search(r"<h3\s+className=\"([^\"]+)\"[\s\S]{0,120}1 vFDUSD Price", oracle)
if not venus_h3:
    print("missing Venus h3", file=sys.stderr)
    sys.exit(1)
if "uppercase" in venus_h3.group(1).split():
    print(f"Venus h3 still uppercase: {venus_h3.group(1)}", file=sys.stderr)
    sys.exit(1)

if "preserveLabelCase" not in oracle:
    print("Venus StatBox must pass preserveLabelCase", file=sys.stderr)
    sys.exit(1)
print("ticker nodes omit uppercase; hub chrome H2 keeps it")
PY

run_step "code: ticker maps unchanged (cUSTC / vFDUSD; no id remap)" \
  bash -c '
    set -euo pipefail
    grep -q "custc: '\''cUSTC'\''" frontend-dapp/src/utils/hubPriceTicker.ts
    grep -q "lunc: '\''LUNC'\''" frontend-dapp/src/utils/hubPriceTicker.ts
    grep -q "ust1: '\''UST1'\''" frontend-dapp/src/utils/hubPriceTicker.ts
    grep -q "ustr: '\''USTR'\''" frontend-dapp/src/utils/hubPriceTicker.ts
    grep -q "ustc: '\''USTC'\''" frontend-dapp/src/utils/protocolOracleTicker.ts
    grep -q "vfdusd: '\''vFDUSD'\''" frontend-dapp/src/utils/protocolOracleTicker.ts
    grep -q "PROTOCOL_ORACLE_TICKERS = \['\''ustc'\'', '\''lunc'\'', '\''vfdusd'\''\]" frontend-dapp/src/utils/protocolOracleTicker.ts
    grep -q "HUB_PRICE_TICKERS = \['\''custc'\'', '\''lunc'\'', '\''ust1'\'', '\''ustr'\''\]" frontend-dapp/src/utils/hubPriceTicker.ts
    ! grep -q "custc: '\''CUSTC'\''" frontend-dapp/src/utils/hubPriceTicker.ts
    ! grep -q "vfdusd: '\''VFDUSD'\''" frontend-dapp/src/utils/protocolOracleTicker.ts
  '

run_step "docs: frontend.md P1240-1–P1240-8" \
  bash -c 'grep -qE "\*\*P1240-1" docs/frontend.md && \
  grep -qE "\*\*P1240-8" docs/frontend.md && \
  grep -qE "#1240" docs/frontend.md && \
  grep -qE "cUSTC / USD" docs/frontend.md && \
  grep -qE "verify-issue-1240" docs/frontend.md'

run_step "skill: AGENTS_FRONTEND_PROTOCOL_STATS P1240 + verify" \
  bash -c 'grep -qE "\*\*P1240-1" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md && \
  grep -qE "\*\*P1240-8" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md && \
  grep -qE "make verify-issue-1240" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md && \
  grep -qE "uppercase" skills/AGENTS_FRONTEND_PROTOCOL_STATS.md'

run_step "skill: hub + Venus playbooks crosslink #1240" \
  bash -c 'grep -qE "#1240" skills/AGENTS_FRONTEND_PROTOCOL_HUB.md && \
  grep -qE "P1240" skills/AGENTS_FRONTEND_PROTOCOL_HUB.md && \
  grep -qE "#1240" skills/AGENTS_INDEXER_VENUS_VFDUSD.md && \
  grep -qE "mixed-case" skills/AGENTS_INDEXER_VENUS_VFDUSD.md'

run_step "AGENTS.md + testing.md playbook" \
  bash -c 'grep -qE "verify-issue-1240" AGENTS.md && \
  grep -qE "P1240-1" AGENTS.md && \
  grep -qE "verify-issue-1240" docs/testing.md && \
  grep -qE "P1240-1" docs/testing.md'

echo ""
echo "────────────────────────────────────────────────────────────────"
echo "  $PASS passed, $FAIL failed"
echo "────────────────────────────────────────────────────────────────"
for r in "${RESULTS[@]}"; do
  echo "  $r"
done

if [ "$FAIL" -ne 0 ]; then
  exit 1
fi
echo ""
echo "  #1240 mixed-case ticker chrome verified."
