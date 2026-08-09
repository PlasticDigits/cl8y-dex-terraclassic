# Launch monitoring runbook (SEC-G01 / GitLab #434)

Minimum **operator eyes-on-system** for launch. The stack ships **no Prometheus `/metrics`**
endpoint ([#200](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/200)) — monitoring
is `tracing` logs + indexer Postgres queries + on-chain LCD queries. This runbook lists, for each
launch signal, a **tested command**, its **healthy baseline**, and the **escalation trigger**.

Proactive **reorg** alerting (`INDEXER_REORG_HALT` stderr prefix + `REORG_ALERT_WEBHOOK_URL`) is
already documented in [`operator-secrets.md`](../operator-secrets.md) and
[`indexer-reorg-replay-dedup.md`](indexer-reorg-replay-dedup.md); it is **signal 8** below for
completeness. Incident *response* (pause/blacklist) lives in
[`emergency-commands.md`](emergency-commands.md); this runbook is the *detection* layer that
precedes it.

## Conventions

Set these for your environment (values shown are the LocalTerra / QA stack used to validate every
command below):

```sh
PGC=cl8y-dex-terraclassic-postgres-1     # indexer Postgres container (prod: your DB host)
DB="psql -U cl8y_legal -d dex_indexer"   # indexer DB connection
LCD=http://127.0.0.1:1317                # chain LCD
RPC=http://127.0.0.1:26657               # chain RPC
INDEXER=http://127.0.0.1:3001            # indexer API
LOG=.indexer-qa.log                      # indexer stdout/stderr log (prod: journald/file sink)
PAIR=terra16jd56...                      # a pair address to spot-check (prod: each live pair)
```

> **Log-grep caveat (applies to signals 1 and 3).** The indexer emits ANSI-colored `tracing`
> output, and many lines contain the *substrings* `error` (e.g. "Parse error" inside a benign oracle
> WARN) or `429`. A naive `grep -ci error $LOG` returns thousands of false positives. Always match
> the **level token** after stripping color, e.g. `sed -E 's/\x1b\[[0-9;]*m//g' $LOG | grep -E ' ERROR '`.
> On the validation run, naive `grep error` = 7147, but the real `ERROR`-level count = 4 (all four
> were genuine reorg-halt events) — so grep the level, not the word.

---

## 1. Contract / indexer error spikes

Real `ERROR`-level events (color-stripped) plus the indexer's own failed-block ledger.

```sh
sed -E 's/\x1b\[[0-9;]*m//g' "$LOG" | grep -E ' ERROR ' | tail -20
docker exec "$PGC" $DB -tAc "SELECT count(*) FROM indexer_failed_blocks;"
docker exec "$PGC" $DB -tAc \
  "SELECT height, left(error_message,80), retry_count, last_failed_at \
   FROM indexer_failed_blocks ORDER BY last_failed_at DESC LIMIT 10;"
```

- **Baseline:** `0` `indexer_failed_blocks` rows; `ERROR`-level lines only from a known reorg-halt
  (signal 8). A clean steady state has no new `ERROR` lines.
- **Escalate if:** `indexer_failed_blocks` is non-zero and `retry_count` keeps climbing (the indexer
  cannot process a block), or a burst of new `ERROR`-level lines that is **not** a single reorg halt.
  For on-chain contract reverts, cross-check failed txs against the pair with the RPC tx search.

## 2. Indexer lag behind chain tip

The single most important liveness signal: is the indexer keeping up with the chain?

```sh
TIP=$(docker exec "$PGC" true; curl -s "$RPC/status" | jq -r '.result.sync_info.latest_block_height')
# or, when terrad is in-container: docker exec <localterra> terrad status --node "$RPC" | jq -r .sync_info.latest_block_height
CP=$(docker exec "$PGC" $DB -tAc "SELECT value FROM indexer_state WHERE key='last_indexed_height';")
echo "chain_tip=$TIP last_indexed=$CP lag=$((TIP-CP)) blocks"
```

- **Baseline:** single-digit lag (validated: `lag=6`). The poller tracks head within a few blocks.
- **Escalate if:** lag grows monotonically over successive checks (indexer stalled or DB-bound), or
  `last_indexed_height` stops advancing while the chain tip rises. A frozen `last_indexed_height`
  alongside an `ERROR`-level `indexer_reorg_halt` means signal 8 (operator recovery required).

## 3. API rate-limit (429) / 5xx rate

`tower_governor` returns HTTP 429 when over budget. Confirm limits are **enabled** in prod (they are
disabled on the QA stack), then watch for sustained 429s.

```sh
# (a) Are the limits actually on? Prod must be non-zero (RunMode::Prod clamps 0 -> default).
sed -E 's/\x1b\[[0-9;]*m//g' "$LOG" | grep -E 'RATE_LIMIT|Rate limits' | tail -5
# (b) Probe the rate (counts non-200 over a short burst, single IP):
for i in $(seq 1 60); do curl -s -o /dev/null -w '%{http_code}\n' \
  "$INDEXER/api/v1/pairs/$PAIR/limit-book?side=ask&limit=5"; done | sort | uniq -c
```

- **Baseline:** all `200` on a normal burst; in prod a brief `429` tail under genuine load is the
  limiter working as designed ([#355](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/355),
  [#426](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/426)).
- **Escalate if:** `RATE_LIMIT_RPS=0` / `RATE_LIMIT_LCD_HEAVY_RPS=0` warnings appear in a **prod**
  log (limiter off — DoS exposure), or 429s are sustained for legitimate single-client traffic
  (limit set too low; the lcd-heavy bucket is per-IP and shared across book/route-solve routes), or
  any `5xx` from the API (route/solve must degrade to LCD, never 500 — [#239](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/239)/[#369](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/369)).

## 4. Large swaps

Outsized fills are worth eyeballing for sandwich / mis-priced-route activity.

```sh
# Set THRESH to your token's micro-unit alert level (validated p99 return = 442,197,580).
THRESH=2000000000
docker exec "$PGC" $DB -tAF'|' -c \
  "SELECT block_height, tx_hash, sender, offer_amount, return_amount \
   FROM swap_events WHERE return_amount::numeric >= $THRESH \
   ORDER BY block_height DESC LIMIT 20;"
```

- **Baseline:** validated `count=9984`, `max_return=2,963,621,042`, `p99=442,197,580`. Set `THRESH`
  near the p99 so routine swarm/retail volume is quiet and only genuine outliers surface.
- **Escalate if:** a swap's `return_amount` is far above p99 from an unfamiliar `sender`, or a run of
  large swaps on one pair drains a reserve (cross-check `pair_reserves`).

## 5. Large LP withdrawals

A sudden large `remove` is the classic rug / panic-exit signal.

```sh
THRESH_LP=1000000000
docker exec "$PGC" $DB -tAF'|' -c \
  "SELECT block_height, tx_hash, provider, asset_0_amount, asset_1_amount, lp_amount \
   FROM liquidity_events WHERE event_type='remove' AND lp_amount::numeric >= $THRESH_LP \
   ORDER BY block_height DESC LIMIT 20;"
```

- **Baseline:** validated `add=421`, `remove=1` (bootstrap). Withdrawals are rare in normal flow.
- **Escalate if:** a `remove` removes a large fraction of a pair's LP supply, or several removes
  cluster on one pair in a short window (liquidity flight) — pair the alert with a `pair_reserves`
  before/after delta.

## 6. Blacklist hits

Confirm the compliance gate is live and watch for flagged addresses interacting.

```sh
# Endpoint shape (live): blocked / wallet_blacklisted / blacklisted_tokens / pair_blacklisted.
curl -s "$INDEXER/api/v1/compliance/blacklist-check?address=<addr>" | jq .
# Did a flagged sender still land a swap? (factory rejects on-chain, but flag the attempt.)
docker exec "$PGC" $DB -tAF'|' -c \
  "SELECT block_height, tx_hash, sender FROM swap_events ORDER BY block_height DESC LIMIT 200;" \
  | while IFS='|' read -r h t s; do \
      curl -s "$INDEXER/api/v1/compliance/blacklist-check?address=$s" \
        | jq -e '.blocked' >/dev/null && echo "BLACKLIST HIT: $s tx=$t height=$h"; done
```

- **Baseline:** `{"blocked":false,...}` for clean addresses; no `BLACKLIST HIT` lines.
- **Escalate if:** any `BLACKLIST HIT` (a flagged address transacting — the on-chain guard should
  have rejected it; a hit on a *landed* swap is a gate failure), or the endpoint errors/times out
  (compliance feed down — the UI gate goes blind).

## 7. Pause state changes

Track whether any pair is paused (planned maintenance vs. an unexpected governance action).

```sh
# Per pair (in-container terrad; LCD smart query also works):
docker exec <localterra> terrad query wasm contract-state smart "$PAIR" '{"is_paused":{}}' \
  --node "$RPC" -o json | jq -c '.data'
# All factory pairs at once: loop the factory's {"pairs":{}} list through is_paused.
```

- **Baseline:** `{"paused":false}` on every live pair.
- **Escalate if:** a pair reports `{"paused":true}` outside a planned window — pause is
  **governance-only** (the 2-of-3 multisig, [#397](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/397)/SEC-B09),
  so an unexpected pause means a governance action fired; confirm against the multisig and
  [`emergency-commands.md`](emergency-commands.md).

## 8. Reorg halt (existing alert)

Already wired: the indexer halts and emits `INDEXER_REORG_HALT` (stderr) +
`REORG_ALERT_WEBHOOK_URL` on a checkpoint-hash mismatch.

```sh
sed -E 's/\x1b\[[0-9;]*m//g' "$LOG" | grep -E 'indexer_reorg_halt|INDEXER_REORG_HALT' | tail -5
```

- **Baseline:** no `INDEXER_REORG_HALT` lines.
- **Escalate if:** present — the indexer has stopped and needs operator recovery; follow
  [`indexer-reorg-replay-dedup.md`](indexer-reorg-replay-dedup.md).

---

## Quick sweep (one pass)

```sh
echo "lag:"; TIP=$(curl -s "$RPC/status"|jq -r .result.sync_info.latest_block_height); \
CP=$(docker exec "$PGC" $DB -tAc "SELECT value FROM indexer_state WHERE key='last_indexed_height'"); \
echo "  $((TIP-CP)) blocks"
echo "failed_blocks:"; docker exec "$PGC" $DB -tAc "SELECT count(*) FROM indexer_failed_blocks"
echo "ERROR-level:";  sed -E 's/\x1b\[[0-9;]*m//g' "$LOG" | grep -cE ' ERROR '
echo "reorg-halt:";   sed -E 's/\x1b\[[0-9;]*m//g' "$LOG" | grep -cE 'indexer_reorg_halt'
```

Run this sweep on a schedule (cron / loop) during launch; any non-baseline line is the cue to drop
into the relevant signal section above, then [`emergency-commands.md`](emergency-commands.md) if a
response is warranted.

---

## UST1 / wrap economic probes (Phase 5 / #503)

Indexer signals above do **not** cover oracle age, window pause, wrap-mapper pause, treasury vFDUSD
capacity, or wrap solvency. For those, use:

```sh
./scripts/check-ust1-wrap-ops-health.sh
```

Full playbooks: [`ust1-wrap-production-ops.md`](ust1-wrap-production-ops.md),
[`wrap-mapper-pause.md`](wrap-mapper-pause.md), agent
[`skills/AGENTS_UST1_WRAP_PRODUCTION_OPS.md`](../../skills/AGENTS_UST1_WRAP_PRODUCTION_OPS.md)
([GitLab **#503**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/503)).
