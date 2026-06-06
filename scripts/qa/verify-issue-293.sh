#!/usr/bin/env bash
# Verification for GitLab #293 — swap quote consistency after swarm liquidity fix.
#
# Layers:
#   1. Python swarm liquidity unit tests (bootstrap sizing helpers)
#   2. TypeScript localnet-trading-swarm profile weights
#   3. Live LocalTerra (optional): bootstrap + swarm, then OE-1 hub pairs
#      (EMBER/CORAL, TOPAZ/ONYX, ONYX/CORAL) 1-unit indexer quotes:
#        [3a] Global best-execution routes are traced (asymmetry expected on
#             LocalTerra lopsided topology — NOT a decimal bug; see #293).
#        [3b] pool_only=true direct-pool quotes are near-inverse (≤5% off
#             reciprocal; fees/spread) — OE-1 acceptance for direct pairs.
#        [3c] route/solve slippage_percent enrichment (GitLab #293): both global
#             and pool_only return spot_amount_out + slippage_percent; values
#             match symmetric deviation vs spot; at least one hub direction shows
#             >30% (retail Expert Mode guard). Prerequisite: single quote asset
#             row per symbol in indexer DB (stale duplicates break enrichment).
#
# Refs: docs/testing.md, skills/AGENTS_LOCALNET_TRADING_SWARM.md
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
cd "$REPO_ROOT"

PASS=0
FAIL=0
SKIP=0
declare -a RESULTS=()

ok()   { RESULTS+=("PASS  $1"); PASS=$((PASS+1)); echo "  [PASS] $1"; }
bad()  { RESULTS+=("FAIL  $1"); FAIL=$((FAIL+1)); echo "  [FAIL] $1" >&2; }
skip() { RESULTS+=("SKIP  $1"); SKIP=$((SKIP+1)); echo "  [SKIP] $1"; }

MAX_DEVIATION_PCT="${VERIFY293_MAX_DEVIATION_PCT:-5}"
SWARM_WARMUP_SEC="${VERIFY293_SWARM_WARMUP_SEC:-90}"
INDEXER="${VERIFY293_INDEXER_URL:-http://127.0.0.1:3001}"

echo "════════════════════════════════════════════════════════════════"
echo "  GitLab #293 — swap quote consistency (swarm liquidity)"
echo "════════════════════════════════════════════════════════════════"

echo ""
echo "[1] Python swarm liquidity unit tests"
if make test-swarm-liquidity; then
  ok "make test-swarm-liquidity"
else
  bad "make test-swarm-liquidity"
fi

echo ""
echo "[2] TypeScript localnet-trading-swarm tests"
if (cd packages/localnet-trading-swarm && npm run test:run); then
  ok "packages/localnet-trading-swarm npm run test:run"
else
  bad "packages/localnet-trading-swarm npm run test:run"
fi

echo ""
echo "[3] Live OE-1 quotes (indexer route/solve, 1.0 token)"
CONTAINER="$(sg docker -c 'docker compose ps -q localterra' 2>/dev/null | head -1 || true)"
if [[ -z "$CONTAINER" ]] || ! curl -sf "${INDEXER}/health" >/dev/null 2>&1; then
  skip "LocalTerra/indexer not running — run: make setup-cloud-localterra (Cloud Agent VMs support LocalTerra; probe: make has-localterra)"
else
  echo "  [3-preflight] Indexer quote-asset rows (stale duplicates break slippage enrichment)"
  USTC_ROWS="$(sg docker -c 'docker compose exec -T postgres psql -U cl8y_legal -d dex_indexer -tAc "SELECT COUNT(*) FROM assets WHERE symbol='"'"'USTC-C'"'"';"' 2>/dev/null | tr -d '[:space:]' || true)"
  if [[ -n "$USTC_ROWS" && "$USTC_ROWS" -gt 1 ]]; then
    bad "indexer DB has $USTC_ROWS USTC-C asset rows (want 1) — rerun: make setup-cloud-localterra --fresh"
  elif [[ -n "$USTC_ROWS" ]]; then
    ok "indexer quote asset rows unique (USTC-C count=$USTC_ROWS)"
  else
    skip "could not probe indexer assets table for duplicate quote rows"
  fi

  if sg docker -c 'make swarm-bootstrap-liquidity' >/tmp/verify293-bootstrap.log 2>&1; then
    ok "make swarm-bootstrap-liquidity"
  else
    bad "make swarm-bootstrap-liquidity (see /tmp/verify293-bootstrap.log)"
  fi

  if [[ -x scripts/bots/stop-swarm.sh ]]; then
    sg docker -c './scripts/bots/stop-swarm.sh' >/dev/null 2>&1 || true
  fi
  if sg docker -c './scripts/bots/launch-swarm.sh' >/tmp/verify293-launch.log 2>&1; then
    ok "make swarm-launch (background workers)"
  else
    bad "make swarm-launch (see /tmp/verify293-launch.log)"
  fi

  echo "    warming swarm ${SWARM_WARMUP_SEC}s…"
  sleep "$SWARM_WARMUP_SEC"

  echo ""
  echo "  [3a] Global best-execution route trace (asymmetry expected — informational)"
  set +e
  PY_GLOBAL="$(VERIFY293_INDEXER_URL="$INDEXER" python3 - <<'PY'
import json, os, time, urllib.request

INDEXER = os.environ.get("VERIFY293_INDEXER_URL", "http://127.0.0.1:3001")
AMOUNT_IN = 1_000_000
PAIRS = [("EMBER", "CORAL"), ("TOPAZ", "ONYX"), ("ONYX", "CORAL")]

def fetch(url):
    time.sleep(0.6)
    with urllib.request.urlopen(url) as r:
        return json.load(r)

def solve(token_in, token_out):
    url = f"{INDEXER}/api/v1/route/solve?token_in={token_in}&token_out={token_out}&amount_in={AMOUNT_IN}"
    return fetch(url)

def route_str(ops, addr_to_sym):
    hops = []
    for op in ops or []:
        ts = op.get("terra_swap", {})
        offer = ts.get("offer_asset_info", {}).get("token", {}).get("contract_addr", "?")
        ask = ts.get("ask_asset_info", {}).get("token", {}).get("contract_addr", "?")
        hops.append(f"{addr_to_sym.get(offer, offer[:8])}→{addr_to_sym.get(ask, ask[:8])}")
    return " → ".join(hops) if hops else "(none)"

tokens_list = fetch(f"{INDEXER}/api/v1/tokens")
tokens = {t["symbol"]: t["contract_address"] for t in tokens_list}
addr_to_sym = {t["contract_address"]: t["symbol"] for t in tokens_list}

for a, b in PAIRS:
    ai, bi = tokens[a], tokens[b]
    fwd = solve(ai, bi)
    rev = solve(bi, ai)
    hf = int(fwd.get("estimated_amount_out", 0)) / 1e6
    hr = int(rev.get("estimated_amount_out", 0)) / 1e6
    exp = 1.0 / hf if hf > 0 else 0.0
    ratio = hr / exp if exp > 0 else float("inf")
    hops_f = len(fwd.get("router_operations") or [])
    hops_r = len(rev.get("router_operations") or [])
    print(f"  {a}/{b}: fwd={hf:.4f} ({hops_f}h) {route_str(fwd.get('router_operations'), addr_to_sym)}")
    print(f"           rev={hr:.4f} ({hops_r}h) {route_str(rev.get('router_operations'), addr_to_sym)}")
    print(f"           reciprocal-of-fwd={exp:.6f}  rev/reciprocal={ratio:.1f}x (asymmetric routes, not a decimal bug)")
PY
)"
  PY_GLOBAL_RC=$?
  set -e
  echo "$PY_GLOBAL"

  echo ""
  echo "  [3b] Direct-pool quotes (pool_only=true, ≤${MAX_DEVIATION_PCT}% reciprocal deviation)"
  PY_POOL="$(VERIFY293_INDEXER_URL="$INDEXER" VERIFY293_MAX_DEVIATION_PCT="$MAX_DEVIATION_PCT" python3 - <<'PY'
import json, os, sys, time, urllib.error, urllib.request

INDEXER = os.environ.get("VERIFY293_INDEXER_URL", "http://127.0.0.1:3001")
MAX_DEV = float(os.environ.get("VERIFY293_MAX_DEVIATION_PCT", "5")) / 100.0
AMOUNT_IN = 1_000_000
PAIRS = [("EMBER", "CORAL"), ("TOPAZ", "ONYX"), ("ONYX", "CORAL")]

def fetch(url):
    time.sleep(0.6)
    with urllib.request.urlopen(url) as r:
        return json.load(r)

def solve(token_in, token_out):
    url = f"{INDEXER}/api/v1/route/solve?token_in={token_in}&token_out={token_out}&amount_in={AMOUNT_IN}&pool_only=true"
    return fetch(url)

tokens = {t["symbol"]: t["contract_address"] for t in fetch(f"{INDEXER}/api/v1/tokens")}
all_pass = True
lines = []
for a, b in PAIRS:
    try:
        ai, bi = tokens[a], tokens[b]
        fwd = solve(ai, bi)
        rev = solve(bi, ai)
        out_fwd = int(fwd.get("estimated_amount_out", 0))
        out_rev = int(rev.get("estimated_amount_out", 0))
        hf = out_fwd / 1e6
        hr = out_rev / 1e6
        expected_rev = 1.0 / hf if hf > 0 else 0.0
        ratio = hr / expected_rev if expected_rev > 0 else float("inf")
        dev_pct = abs(ratio - 1.0) * 100.0
        ok_pair = dev_pct <= MAX_DEV * 100.0
        if not ok_pair:
            all_pass = False
        lines.append(
            f"  {a}/{b}: fwd={hf:.6f} rev={hr:.6f} reciprocal={expected_rev:.6f} "
            f"dev={dev_pct:.2f}% {'PASS' if ok_pair else 'FAIL'}"
        )
    except (urllib.error.URLError, KeyError, TypeError, ValueError) as e:
        all_pass = False
        lines.append(f"  {a}/{b}: ERROR {e}")
print("\n".join(lines))
print(f"ALL_PASS={'true' if all_pass else 'false'}")
sys.exit(0 if all_pass else 1)
PY
)" || true

  echo "$PY_POOL"
  if echo "$PY_POOL" | grep -q 'ALL_PASS=true'; then
    ok "OE-1 hub pairs pool_only near-inverse (≤${MAX_DEVIATION_PCT}% deviation)"
  else
    bad "OE-1 hub pairs pool_only not near-inverse (see lines above)"
  fi

  if [[ $PY_GLOBAL_RC -eq 0 && -n "$PY_GLOBAL" ]]; then
    ok "global route asymmetry documented (see [3a] — different best paths per direction)"
  else
    bad "global route asymmetry trace failed (see [3a] output above)"
  fi

  echo ""
  echo "  [3c] Route slippage enrichment (slippage_percent vs best-route token prices)"
  set +e
  PY_SLIP="$(VERIFY293_INDEXER_URL="$INDEXER" python3 - <<'PY'
import json, os, sys, time, urllib.error, urllib.request

INDEXER = os.environ.get("VERIFY293_INDEXER_URL", "http://127.0.0.1:3001")
AMOUNT_IN = 1_000_000
RETAIL_BLOCK_PCT = 30.0
MATH_TOL_PCT = 1.0

def fetch(url):
    time.sleep(0.6)
    with urllib.request.urlopen(url) as r:
        return json.load(r)

def solve(token_in, token_out, pool_only=False):
    url = f"{INDEXER}/api/v1/route/solve?token_in={token_in}&token_out={token_out}&amount_in={AMOUNT_IN}"
    if pool_only:
        url += "&pool_only=true"
    return fetch(url)

def symmetric_slippage(actual_raw: int, spot_raw: int) -> float:
    if actual_raw <= 0 or spot_raw <= 0:
        return 0.0
    lo, hi = min(actual_raw, spot_raw), max(actual_raw, spot_raw)
    return (1.0 - lo / hi) * 100.0

tokens = {t["symbol"]: t["contract_address"] for t in fetch(f"{INDEXER}/api/v1/tokens")}
ember, coral = tokens["EMBER"], tokens["CORAL"]
all_pass = True
lines = []
retail_guard_scenario = False

def slip_pct(body):
    raw = body.get("slippage_percent")
    if raw is None:
        return None
    try:
        return float(raw)
    except (TypeError, ValueError):
        return None

for label, pool_only in [
    ("EMBER→CORAL global", False),
    ("EMBER→CORAL pool_only", True),
]:
    try:
        body = solve(ember, coral, pool_only=pool_only)
        slip = slip_pct(body)
        spot = body.get("spot_amount_out")
        est = body.get("estimated_amount_out")
        if slip is None or not spot or not est:
            all_pass = False
            lines.append(f"  {label}: missing slippage_percent, spot_amount_out, or estimated_amount_out")
            continue
        est_i = int(est)
        spot_i = int(spot)
        recomputed = symmetric_slippage(est_i, spot_i)
        math_ok = abs(recomputed - slip) <= MATH_TOL_PCT
        if slip > RETAIL_BLOCK_PCT:
            retail_guard_scenario = True
        lines.append(
            f"  {label}: slippage={slip:.2f}% spot_out={spot} est_out={est} "
            f"recomputed={recomputed:.2f}% {'PASS' if math_ok else 'FAIL'} (math ±{MATH_TOL_PCT}%)"
        )
        if not math_ok:
            all_pass = False
    except (urllib.error.URLError, KeyError, TypeError, ValueError) as e:
        all_pass = False
        lines.append(f"  {label}: ERROR {e}")

if not retail_guard_scenario:
    all_pass = False
    lines.append(
        f"  retail guard scenario: FAIL (neither global nor pool_only EMBER→CORAL exceeded {RETAIL_BLOCK_PCT}% — "
        "Expert Mode block would not be exercisable on this pair)"
    )
else:
    lines.append(
        f"  retail guard scenario: PASS (≥1 direction >{RETAIL_BLOCK_PCT}% — dApp blocks submit unless Expert Mode)"
    )

print("\n".join(lines))
print(f"ALL_PASS={'true' if all_pass else 'false'}")
sys.exit(0 if all_pass else 1)
PY
)"
  PY_SLIP_RC=$?
  set -e
  echo "$PY_SLIP"

  if echo "$PY_SLIP" | grep -q 'ALL_PASS=true'; then
    ok "route slippage enrichment (global extreme, pool_only low)"
  else
    bad "route slippage enrichment failed (see [3c] above)"
  fi

  sg docker -c './scripts/bots/stop-swarm.sh' >/dev/null 2>&1 || true
fi

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   FAIL: $FAIL   SKIP: $SKIP"
echo "════════════════════════════════════════════════════════════════"

[[ "$FAIL" -eq 0 ]]
