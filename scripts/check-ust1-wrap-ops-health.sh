#!/usr/bin/env bash
# Read-only columbus-5 health probe for UST1 window + wrap stack (GitLab #503).
#
# Checks (no keys / no mutating txs):
#   - window effective_swap: paused, oracle paused/age vs max_oracle_age_sec
#   - wrap-mapper config.paused + fee_wrap_bps/fee_unwrap_bps (or pre-migrate fee_bps) + treasury match
#   - treasury vFDUSD balance + allowance to ust1-window
#   - wrap solvency: bank uluna/uusd on treasury ≥ cLUNC/cUSTC total_supply
#
# Exit codes:
#   0 — all hard checks passed (warnings allowed)
#   1 — hard failure (LCD error, unexpected pause when STRICT=1, insolvency)
#   2 — usage / missing deps
#
# Env:
#   UST1_OPS_LCD_URL          LCD base (default publicnode)
#   UST1_OPS_STRICT_PAUSE=1      Fail if window/oracle/wrap-mapper/treasury wrapping paused
#   UST1_OPS_STRICT_STALE=1      Fail if oracle stale (default: report only)
#   UST1_OPS_STRICT_INVENTORY=1  Fail if vFDUSD balance/allowance below warn thresholds
#   See scripts/lib/ust1-wrap-ops-defaults.sh for address overrides.
#
# Example:
#   ./scripts/check-ust1-wrap-ops-health.sh
#   UST1_OPS_STRICT_PAUSE=1 UST1_OPS_STRICT_STALE=1 ./scripts/check-ust1-wrap-ops-health.sh
#   UST1_OPS_STRICT_PAUSE=1 UST1_OPS_STRICT_STALE=1 UST1_OPS_STRICT_INVENTORY=1 ./scripts/check-ust1-wrap-ops-health.sh
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$REPO_ROOT"

# shellcheck source=scripts/lib/ust1-wrap-ops-defaults.sh
source "$REPO_ROOT/scripts/lib/ust1-wrap-ops-defaults.sh"
# shellcheck source=scripts/lib/lcd-smart-query.sh
source "$REPO_ROOT/scripts/lib/lcd-smart-query.sh"

for cmd in curl jq; do
  command -v "$cmd" >/dev/null 2>&1 || {
    echo "ERROR: missing dependency: $cmd" >&2
    exit 2
  }
done

LCD="${UST1_OPS_LCD_URL%/}"
STRICT_PAUSE="${UST1_OPS_STRICT_PAUSE:-0}"
STRICT_STALE="${UST1_OPS_STRICT_STALE:-0}"
STRICT_INVENTORY="${UST1_OPS_STRICT_INVENTORY:-0}"

PASS=0
WARN=0
FAIL=0
declare -a RESULTS=()

ok()   { RESULTS+=("PASS  $1"); PASS=$((PASS + 1)); echo "  [PASS] $1"; }
warn() { RESULTS+=("WARN  $1"); WARN=$((WARN + 1)); echo "  [WARN] $1" >&2; }
bad()  { RESULTS+=("FAIL  $1"); FAIL=$((FAIL + 1)); echo "  [FAIL] $1" >&2; }

smart() {
  local contract="$1" msg="$2"
  lcd_decode_smart_data "$(lcd_smart_query_raw "$LCD" "$contract" "$msg")"
}

bank_amount() {
  local addr="$1" denom="$2"
  local url="${LCD}/cosmos/bank/v1beta1/balances/${addr}/by_denom?denom=${denom}"
  local body
  body="$(curl -sS --connect-timeout 5 --max-time 20 "$url" 2>/dev/null || true)"
  if [[ -z "$body" ]]; then
    echo ""
    return 1
  fi
  echo "$body" | jq -r '.balance.amount // "0"'
}

echo "════════════════════════════════════════════════════════════════"
echo "  UST1 / wrap ops health (read-only) — GitLab #503"
echo "  LCD=$LCD"
echo "════════════════════════════════════════════════════════════════"

echo ""
echo "[1] ust1-window effective_swap"
EFF="$(smart "$UST1_OPS_WINDOW" '{"effective_swap":{}}')" || {
  bad "effective_swap LCD query"
  EFF=""
}
if [[ -n "$EFF" && "$EFF" != "null" ]]; then
  WIN_PAUSED="$(echo "$EFF" | jq -r '.paused // false')"
  ORA_PAUSED="$(echo "$EFF" | jq -r '.oracle.paused // false')"
  LAST="$(echo "$EFF" | jq -r '.oracle.last_update_sec // 0')"
  MAX_AGE="$(echo "$EFF" | jq -r '.max_oracle_age_sec // 0')"
  RATE="$(echo "$EFF" | jq -r '.oracle.rate // "0"')"
  FEE="$(echo "$EFF" | jq -r '.fee_bps // 0')"
  NOW="$(date +%s)"
  AGE=$((NOW - LAST))
  echo "    window.paused=$WIN_PAUSED oracle.paused=$ORA_PAUSED fee_bps=$FEE rate=$RATE"
  echo "    oracle.last_update_sec=$LAST age_sec=$AGE max_oracle_age_sec=$MAX_AGE"

  if [[ "$WIN_PAUSED" == "true" ]]; then
    if [[ "$STRICT_PAUSE" == "1" ]]; then bad "window paused (STRICT)"; else warn "window paused"; fi
  else
    ok "window not paused"
  fi
  if [[ "$ORA_PAUSED" == "true" ]]; then
    if [[ "$STRICT_PAUSE" == "1" ]]; then bad "oracle paused (STRICT)"; else warn "oracle paused"; fi
  else
    ok "oracle not paused"
  fi
  if [[ "$LAST" == "0" ]] || [[ "$AGE" -gt "$MAX_AGE" ]]; then
    if [[ "$STRICT_STALE" == "1" ]]; then bad "oracle stale age=${AGE}s max=${MAX_AGE}s (STRICT)"; else warn "oracle stale age=${AGE}s max=${MAX_AGE}s"; fi
  else
    ok "oracle age within max_oracle_age_sec"
  fi
  if [[ "$AGE" -gt "$UST1_OPS_ORACLE_SILENCE_SECS" ]]; then
    warn "oracle silence threshold exceeded (age=${AGE}s > ${UST1_OPS_ORACLE_SILENCE_SECS}s) — pager should have fired"
  else
    ok "oracle age under silence alert threshold (${UST1_OPS_ORACLE_SILENCE_SECS}s)"
  fi
  # fee_bps is on-chain authoritative (approved target was 100; live may differ after governance)
  ok "window fee_bps=$FEE (on-chain)"
fi

echo ""
echo "[2] wrap-mapper config"
WCFG="$(smart "$UST1_OPS_WRAP_MAPPER" '{"config":{}}')" || {
  bad "wrap-mapper config LCD query"
  WCFG=""
}
if [[ -n "$WCFG" && "$WCFG" != "null" ]]; then
  W_PAUSED="$(echo "$WCFG" | jq -r '.paused // false')"
  W_WRAP_FEE="$(echo "$WCFG" | jq -r '.fee_wrap_bps // empty')"
  W_UNWRAP_FEE="$(echo "$WCFG" | jq -r '.fee_unwrap_bps // empty')"
  W_LEGACY_FEE="$(echo "$WCFG" | jq -r '.fee_bps // empty')"
  W_TREAS="$(echo "$WCFG" | jq -r '.treasury // ""')"
  W_GOV="$(echo "$WCFG" | jq -r '.governance // ""')"
  echo "    paused=$W_PAUSED fee_wrap_bps=${W_WRAP_FEE:-<empty>} fee_unwrap_bps=${W_UNWRAP_FEE:-<empty>} fee_bps=${W_LEGACY_FEE:-<empty>}"
  echo "    treasury=$W_TREAS"
  echo "    governance=$W_GOV"
  if [[ "$W_PAUSED" == "true" ]]; then
    if [[ "$STRICT_PAUSE" == "1" ]]; then bad "wrap-mapper paused (STRICT)"; else warn "wrap-mapper paused"; fi
  else
    ok "wrap-mapper not paused"
  fi
  if [[ -n "$W_WRAP_FEE" || -n "$W_UNWRAP_FEE" ]]; then
    if [[ -z "$W_WRAP_FEE" || -z "$W_UNWRAP_FEE" ]]; then
      bad "wrap-mapper split fees partial (fee_wrap_bps=${W_WRAP_FEE:-<empty>} fee_unwrap_bps=${W_UNWRAP_FEE:-<empty>}) — fail closed"
    else
      ok "wrap-mapper fee_wrap_bps=$W_WRAP_FEE fee_unwrap_bps=$W_UNWRAP_FEE (on-chain; UI must query; #516)"
    fi
  elif [[ -n "$W_LEGACY_FEE" ]]; then
    warn "wrap-mapper still single fee_bps=$W_LEGACY_FEE (ustr-cmm#9 migrate pending; UI maps both sides)"
  else
    bad "wrap-mapper config missing fee_wrap_bps/fee_unwrap_bps and fee_bps"
  fi
  if [[ "$W_TREAS" == "$UST1_OPS_TREASURY" ]]; then
    ok "wrap-mapper treasury matches registry CMM treasury"
  else
    bad "wrap-mapper treasury mismatch (on-chain=$W_TREAS expected=$UST1_OPS_TREASURY)"
  fi
  if [[ "$W_GOV" == "$UST1_OPS_WRAP_GOVERNANCE" ]]; then
    ok "wrap-mapper governance matches registry"
  else
    warn "wrap-mapper governance=$W_GOV (registry default $UST1_OPS_WRAP_GOVERNANCE)"
  fi
fi

echo ""
echo "[2b] treasury wrapping_paused"
TCFG="$(smart "$UST1_OPS_TREASURY" '{"config":{}}')" || {
  bad "treasury config LCD query"
  TCFG=""
}
if [[ -n "$TCFG" && "$TCFG" != "null" ]]; then
  T_WRAP_PAUSED="$(echo "$TCFG" | jq -r '.wrapping_paused // false')"
  echo "    wrapping_paused=$T_WRAP_PAUSED"
  if [[ "$T_WRAP_PAUSED" == "true" ]]; then
    if [[ "$STRICT_PAUSE" == "1" ]]; then bad "treasury wrapping_paused (STRICT)"; else warn "treasury wrapping_paused"; fi
  else
    ok "treasury wrapping not paused"
  fi
fi

echo ""
echo "[3] treasury vFDUSD balance + allowance → window"
VFD_BAL="$(smart "$UST1_OPS_VFDUSD" "$(jq -nc --arg a "$UST1_OPS_TREASURY" '{balance:{address:$a}}')")" || VFD_BAL=""
VFD_AMT="0"
if [[ -n "$VFD_BAL" && "$VFD_BAL" != "null" ]]; then
  VFD_AMT="$(echo "$VFD_BAL" | jq -r '.balance // "0"')"
fi
echo "    treasury vFDUSD balance=$VFD_AMT"
if [[ "$VFD_AMT" =~ ^[0-9]+$ ]] && [[ "$VFD_AMT" -ge "$UST1_OPS_VFDUSD_BALANCE_WARN" ]]; then
  ok "treasury vFDUSD balance ≥ warn threshold ($UST1_OPS_VFDUSD_BALANCE_WARN)"
elif [[ "$STRICT_INVENTORY" == "1" ]]; then
  bad "treasury vFDUSD balance=$VFD_AMT below warn threshold $UST1_OPS_VFDUSD_BALANCE_WARN (STRICT_INVENTORY)"
else
  warn "treasury vFDUSD balance=$VFD_AMT below warn threshold $UST1_OPS_VFDUSD_BALANCE_WARN"
fi

ALLOW="$(smart "$UST1_OPS_VFDUSD" "$(jq -nc --arg o "$UST1_OPS_TREASURY" --arg s "$UST1_OPS_WINDOW" '{allowance:{owner:$o,spender:$s}}')")" || ALLOW=""
ALLOW_AMT="0"
if [[ -n "$ALLOW" && "$ALLOW" != "null" ]]; then
  ALLOW_AMT="$(echo "$ALLOW" | jq -r '.allowance // "0"')"
fi
echo "    allowance treasury→window=$ALLOW_AMT"
if [[ "$ALLOW_AMT" =~ ^[0-9]+$ ]] && [[ "$ALLOW_AMT" -ge "$UST1_OPS_VFDUSD_ALLOWANCE_WARN" ]]; then
  ok "vFDUSD allowance ≥ warn threshold ($UST1_OPS_VFDUSD_ALLOWANCE_WARN)"
elif [[ "$STRICT_INVENTORY" == "1" ]]; then
  bad "vFDUSD allowance=$ALLOW_AMT below warn threshold $UST1_OPS_VFDUSD_ALLOWANCE_WARN (STRICT_INVENTORY)"
else
  warn "vFDUSD allowance=$ALLOW_AMT below warn threshold $UST1_OPS_VFDUSD_ALLOWANCE_WARN (withdraw capacity)"
fi

echo ""
echo "[4] wrap solvency (treasury native ≥ CW20 supply)"
CLUNC_INFO="$(smart "$UST1_OPS_CLUNC" '{"token_info":{}}')" || CLUNC_INFO=""
CUSTC_INFO="$(smart "$UST1_OPS_CUSTC" '{"token_info":{}}')" || CUSTC_INFO=""
CLUNC_SUPPLY="$(echo "${CLUNC_INFO:-null}" | jq -r '.total_supply // empty')"
CUSTC_SUPPLY="$(echo "${CUSTC_INFO:-null}" | jq -r '.total_supply // empty')"
ULUNA_BAL="$(bank_amount "$UST1_OPS_TREASURY" "uluna" || true)"
UUSD_BAL="$(bank_amount "$UST1_OPS_TREASURY" "uusd" || true)"
echo "    treasury uluna=${ULUNA_BAL:-<empty>}  cLUNC supply=${CLUNC_SUPPLY:-<empty>}"
echo "    treasury uusd=${UUSD_BAL:-<empty>}   cUSTC supply=${CUSTC_SUPPLY:-<empty>}"

solvency_ok() {
  local native="$1" supply="$2" label="$3"
  if [[ -z "$native" || -z "$supply" ]]; then
    bad "$label: LCD bank/supply query empty (refusing to treat as 0)"
    return
  fi
  if ! [[ "$native" =~ ^[0-9]+$ && "$supply" =~ ^[0-9]+$ ]]; then
    bad "$label: non-numeric native=$native supply=$supply"
    return
  fi
  if [[ "$native" -ge "$supply" ]]; then
    ok "$label: treasury native ≥ CW20 supply"
  else
    bad "$label: INSOLVENT treasury=$native < supply=$supply — pause unwrap"
  fi
}
solvency_ok "$ULUNA_BAL" "$CLUNC_SUPPLY" "cLUNC/uluna"
solvency_ok "$UUSD_BAL" "$CUSTC_SUPPLY" "cUSTC/uusd"

echo ""
echo "[5] wrap-mapper rate_limit (read-only)"
for denom in uluna uusd; do
  RL="$(smart "$UST1_OPS_WRAP_MAPPER" "$(jq -nc --arg d "$denom" '{rate_limit:{denom:$d}}')")" || RL=""
  if [[ -z "$RL" || "$RL" == "null" ]]; then
    warn "rate_limit denom=$denom: LCD query failed"
    continue
  fi
  HAS_CFG="$(echo "$RL" | jq -r 'if .config == null then "none" else "set" end')"
  USED="$(echo "$RL" | jq -r '.amount_used // "0"')"
  if [[ "$HAS_CFG" == "none" ]]; then
    ok "rate_limit denom=$denom: no cap (unlimited)"
  else
    MAX="$(echo "$RL" | jq -r '.config.max_amount_per_window // "?"')"
    WIN="$(echo "$RL" | jq -r '.config.window_seconds // "?"')"
    ok "rate_limit denom=$denom: max=$MAX / ${WIN}s used=$USED"
  fi
done

echo ""
echo "════════════════════════════════════════════════════════════════"
printf '%s\n' "${RESULTS[@]}"
echo "────────────────────────────────────────────────────────────────"
echo "  PASS: $PASS   WARN: $WARN   FAIL: $FAIL"
echo "════════════════════════════════════════════════════════════════"

if [[ "$FAIL" -gt 0 ]]; then
  exit 1
fi
exit 0
