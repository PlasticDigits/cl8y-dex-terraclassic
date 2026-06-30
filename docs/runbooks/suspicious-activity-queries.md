# Runbook: Suspicious activity discovery queries (SEC-G04)

Copy-pastable **indexer API** and **Postgres SQL** recipes for incident triage — surface wallets, pairs, or tokens with abnormal on-chain or indexed activity **before** applying pause or blacklist controls. Parent remediation: GitLab [#437](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/437) (**SEC-G04**).

**Related:** [incident template](../templates/incident-dex-indexer.md) (Triage), [blacklist decision runbook](./blacklist-decision.md) (post-discovery escalation), [emergency commands](./emergency-commands.md) (on-chain mitigation), [indexer invariants](../indexer-invariants.md), [integrators.md](../integrators.md). Agent playbook: [`skills/AGENTS_SUSPICIOUS_ACTIVITY_QUERIES.md`](../../skills/AGENTS_SUSPICIOUS_ACTIVITY_QUERIES.md).

---

## Before you query

1. **Set endpoints** (production example):

```bash
export INDEXER_URL="https://indexer.example.com"   # no trailing slash
export LCD="https://terra-classic-lcd.publicnode.com"
export DATABASE_URL="postgresql://..."             # read-only replica OK
```

**LocalTerra** (after `make setup-cloud-localterra`):

```bash
source indexer/.env
export INDEXER_URL="http://127.0.0.1:3001"
export LCD="http://127.0.0.1:1317"
```

2. **Read-only** — discovery queries do not mutate chain or DB state.
3. **Thresholds are tunable** — adjust `INTERVAL`, `LIMIT`, and `HAVING` counts for incident severity; document chosen values in the incident tracker.
4. **Escalation** — discovery alone is **not** grounds for blacklist. After confirming abnormal patterns, follow [blacklist-decision.md](./blacklist-decision.md) and [emergency-commands.md](./emergency-commands.md).

---

## 1. Top-volume traders (recent window)

Use during triage to find wallets dominating flow in the last 24 hours.

**Indexer API** (preferred — no DB access required):

```bash
curl -sG "$INDEXER_URL/api/v1/traders/leaderboard" \
  --data-urlencode "sort=volume_24h" \
  --data-urlencode "limit=25" | jq '.[:5]'
```

**Example output** (shape; values vary):

```json
[
  {
    "address": "terra1abc...",
    "total_trades": 142,
    "volume_24h": "1250000000",
    "volume_7d": "4100000000",
    "last_trade_at": "2026-06-30T12:34:56Z"
  }
]
```

**7d / 30d windows:** `sort=volume_7d` or `sort=volume_30d`. **Trade-count spike:** `sort=total_trades&limit=25`.

**Postgres** (parity / when API unavailable):

```sql
SELECT address, total_trades, volume_24h, last_trade_at
FROM traders
ORDER BY volume_24h DESC
LIMIT 25;
```

**Next step:** probe suspects with [§ 5 Blacklist compliance probes](#5-blacklist-compliance-probes) and review per-wallet trades:

```bash
export WALLET_ADDR="terra1abc..."
curl -sG "$INDEXER_URL/api/v1/traders/$WALLET_ADDR/trades" \
  --data-urlencode "limit=20" | jq '.[:3]'
```

---

## 2. Wallets with many failed transactions

The indexer indexes **successful** swaps only (`swap_events`). Failed on-chain executes appear via **LCD**, not Postgres rollups.

### 2a. Recent failed wasm executes (LCD)

Fetch recent wasm module txs and filter `code != 0` (revert, out-of-gas, hook error, etc.):

```bash
curl -sG "$LCD/cosmos/tx/v1beta1/txs" \
  --data-urlencode "events=message.module='wasm'" \
  --data-urlencode "pagination.limit=100" | \
  jq '[.tx_responses[] | select(.code != 0) | {
    hash: .txhash,
    code: .code,
    height: .height,
    sender: (.tx.body.messages[0].sender // "unknown"),
    log: (.raw_log | split("\n")[0])
  }]'
```

**Aggregate failures by sender** (rolling triage — raise `pagination.limit` or paginate for production):

```bash
curl -sG "$LCD/cosmos/tx/v1beta1/txs" \
  --data-urlencode "events=message.module='wasm'" \
  --data-urlencode "pagination.limit=200" | \
  jq '[.tx_responses[] | select(.code != 0 and (.tx.body.messages[0].sender? != null))]
      | group_by(.tx.body.messages[0].sender)
      | map({sender: .[0].tx.body.messages[0].sender, failures: length, sample_hash: .[0].txhash})
      | sort_by(.failures) | reverse | .[:15]'
```

**Suspect wallet drill-down:**

```bash
export WALLET_ADDR="terra1abc..."
curl -sG "$LCD/cosmos/tx/v1beta1/txs" \
  --data-urlencode "events=message.sender='$WALLET_ADDR'" \
  --data-urlencode "pagination.limit=50" | \
  jq '[.tx_responses[] | {hash: .txhash, code: .code, height: .height, log: (.raw_log | split("\n")[0])}]'
```

Expect `code: 0` for success; non-zero codes with `out of gas`, `max spread`, or hook errors in `log`.

### 2b. Hook warnings (indexed successful txs)

When a registered hook emits `warning` or `skipped` attrs, the indexer stores a `hook_events` row. Useful for hook-revert triage on **included** txs:

```sql
SELECT hook_address, COUNT(*) AS warn_count,
       MIN(block_time) AS first_seen, MAX(block_time) AS last_seen
FROM hook_events
WHERE block_time >= NOW() - INTERVAL '24 hours'
  AND (warning IS NOT NULL OR skipped IS NOT NULL)
GROUP BY hook_address
ORDER BY warn_count DESC
LIMIT 20;
```

**Recent hook warning samples:**

```bash
curl -sG "$INDEXER_URL/api/v1/hooks" \
  --data-urlencode "limit=20" | jq '[.[] | select(.warning != null or .skipped != null)]'
```

---

## 3. Pairs with abnormal swap count or volume spike

### 3a. Indexer API — pair list and per-pair stats

**Highest 24h volume pairs:**

```bash
curl -sG "$INDEXER_URL/api/v1/pairs" \
  --data-urlencode "sort=volume_24h" \
  --data-urlencode "order=desc" \
  --data-urlencode "limit=20" | jq '.items[:5] | .[] | {address: .pair_address, volume_24h: .volume_quote_24h, assets: [.asset_0.symbol, .asset_1.symbol]}'
```

**Per-pair 24h trade count and volume** (replace pair address):

```bash
export PAIR_ADDR="terra1pair..."
curl -s "$INDEXER_URL/api/v1/pairs/$PAIR_ADDR/stats" | jq '{trade_count, volume_quote, volume_usd, price_change_pct}'
```

**Example stats output:**

```json
{
  "trade_count": 87,
  "volume_quote": "45000000000",
  "volume_usd": "1234.56",
  "price_change_pct": 12.4
}
```

### 3b. Postgres — swap count spike (1h vs 24h baseline)

Pairs with **≥ 20 swaps in the last hour** (tune `HAVING`):

```sql
SELECT p.contract_address,
       COUNT(*) FILTER (WHERE s.block_timestamp >= NOW() - INTERVAL '1 hour') AS swaps_1h,
       COUNT(*) FILTER (WHERE s.block_timestamp >= NOW() - INTERVAL '24 hours') AS swaps_24h
FROM swap_events s
JOIN pairs p ON p.id = s.pair_id
WHERE s.block_timestamp >= NOW() - INTERVAL '24 hours'
GROUP BY p.contract_address
HAVING COUNT(*) FILTER (WHERE s.block_timestamp >= NOW() - INTERVAL '1 hour') >= 20
ORDER BY swaps_1h DESC
LIMIT 25;
```

---

## 4. Reserve / liquidity anomalies

### 4a. Large liquidity adds or removes (24h)

Sudden LP changes can indicate manipulation or drain setup:

```sql
SELECT p.contract_address, le.event_type,
       COUNT(*) AS events,
       SUM(le.asset_0_amount) AS sum_asset_0,
       SUM(le.asset_1_amount) AS sum_asset_1
FROM liquidity_events le
JOIN pairs p ON p.id = le.pair_id
WHERE le.block_timestamp >= NOW() - INTERVAL '24 hours'
GROUP BY p.contract_address, le.event_type
ORDER BY events DESC
LIMIT 30;
```

**API alternative** (per pair):

```bash
curl -sG "$INDEXER_URL/api/v1/pairs/$PAIR_ADDR/liquidity-events" \
  --data-urlencode "limit=20" | jq '.[:5]'
```

### 4b. Stale reserve mirror vs recent swap activity

`pair_reserves` is a solver mirror (not a full history). Compare snapshot age to recent swap count:

```sql
SELECT p.contract_address,
       pr.snapshot_at,
       pr.reserve_0, pr.reserve_1,
       COUNT(s.id) AS swaps_since_snapshot
FROM pair_reserves pr
JOIN pairs p ON p.id = pr.pair_id
LEFT JOIN swap_events s ON s.pair_id = pr.pair_id AND s.block_timestamp > pr.snapshot_at
GROUP BY p.contract_address, pr.snapshot_at, pr.reserve_0, pr.reserve_1
HAVING COUNT(s.id) > 50
ORDER BY swaps_since_snapshot DESC
LIMIT 20;
```

Investigate pairs with high `swaps_since_snapshot` and old `snapshot_at` alongside [indexer health](../indexer-invariants.md) and book snapshot logs.

---

## 5. Blacklist compliance probes

`GET /api/v1/compliance/blacklist-check` proxies factory `BlacklistCheck` on LCD. The indexer **does not** persist probe history in Postgres; use live probes and optional HTTP access logs (if your reverse proxy logs indexer paths).

### 5a. Single wallet / token / pair probe

```bash
export WALLET_ADDR="terra1abc..."
curl -sG "$INDEXER_URL/api/v1/compliance/blacklist-check" \
  --data-urlencode "wallet=$WALLET_ADDR" | jq .
```

```bash
export TOKEN_ADDR="terra1token..."
curl -sG "$INDEXER_URL/api/v1/compliance/blacklist-check" \
  --data-urlencode "tokens=$TOKEN_ADDR" | jq .
```

```bash
export PAIR_ADDR="terra1pair..."
curl -sG "$INDEXER_URL/api/v1/compliance/blacklist-check" \
  --data-urlencode "pair=$PAIR_ADDR" | jq .
```

**Example blocked response:**

```json
{
  "blocked": true,
  "wallet_blacklisted": true,
  "blacklisted_tokens": [],
  "pair_blacklisted": false,
  "blacklisted_pairs": []
}
```

### 5b. Batch-probe top 24h traders

After [§ 1](#1-top-volume-traders-recent-window):

```bash
curl -sG "$INDEXER_URL/api/v1/traders/leaderboard" \
  --data-urlencode "sort=volume_24h" \
  --data-urlencode "limit=10" | \
  jq -r '.[].address' | while read -r w; do
    curl -sG "$INDEXER_URL/api/v1/compliance/blacklist-check" \
      --data-urlencode "wallet=$w" | jq -c --arg w "$w" '{wallet: $w, blocked, wallet_blacklisted}'
  done
```

### 5c. Tokens active in recent swaps (discovery → probe)

Find CW20 assets with high swap volume, then batch `blacklist-check`:

```sql
SELECT a.contract_address, a.symbol,
       COUNT(*) AS swap_rows_24h,
       SUM(s.volume_usd) AS volume_usd_24h
FROM swap_events s
JOIN assets a ON a.id IN (s.offer_asset_id, s.ask_asset_id)
WHERE s.block_timestamp >= NOW() - INTERVAL '24 hours'
  AND a.contract_address IS NOT NULL
GROUP BY a.contract_address, a.symbol
ORDER BY swap_rows_24h DESC
LIMIT 15;
```

**Indexer API** (token detail with volume windows — replace address):

```bash
export TOKEN_ADDR="terra1token..."
curl -s "$INDEXER_URL/api/v1/tokens/$TOKEN_ADDR" | \
  jq '{symbol: .token.symbol, volume_stats: .volume_stats}'
```

### 5d. Compliance endpoint access logs (optional)

If production HTTP logs include indexer paths, recent proactive checks from the dApp or integrators appear as:

```bash
# Example — adjust log path / format for your deployment
grep '/api/v1/compliance/blacklist-check' /var/log/indexer/access.log | tail -50
```

There is **no** `compliance_audit` SQL table; do not expect Postgres rows for probe hits.

---

## Escalation checklist

| Finding | Next step |
|---------|-----------|
| High-volume wallet + failed tx pattern + exploit evidence | [blacklist-decision.md § Wallet](./blacklist-decision.md#wallet-blacklist-blacklistwallet) → [emergency-commands.md § 3](./emergency-commands.md) |
| Malicious CW20 / fee-on-transfer | [blacklist-decision.md § Token](./blacklist-decision.md#token-blacklist-blacklisttoken) → [cw20-whitelist-policy.md](./cw20-whitelist-policy.md) |
| Compromised or manipulated pair | Pair pause first ([emergency-commands.md § 1](./emergency-commands.md)); escalate to pair blacklist if needed |
| Indexer gaps (stale reserves, missing swaps) | [indexer-reorg-replay-dedup.md](./indexer-reorg-replay-dedup.md) — fix data before blacklist decisions |

Record tx hashes, API responses, and SQL snapshots in the incident tracker before governance broadcasts.

---

## Verification

```bash
# Doc cross-links and required sections (no chain)
make check-suspicious-activity-queries-docs

# Full #437 checklist
make verify-issue-437
```

With LocalTerra + indexer running, smoke-test API sections:

```bash
make has-localterra
curl -sf "$INDEXER_URL/api/v1/traders/leaderboard?sort=volume_24h&limit=3" | jq 'length'
curl -sf "$INDEXER_URL/api/v1/pairs?sort=volume_24h&order=desc&limit=3" | jq '.items | length'
```
