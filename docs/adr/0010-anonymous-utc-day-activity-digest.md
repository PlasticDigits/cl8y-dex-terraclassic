# ADR 0010: Anonymous UTC-day activity digest

## Status

Proposed — [#1206](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1206). Revision **2**. This design slice does not accept the ADR. Implement keeps **Proposed** until a reviewer accepts it.

Revision 2 keeps the same route and document. It closes three traps on current `main`: axum query structs drop unknown keys unless denied; the trailing fee rollup omits unconfigured wrap/UST1 families and stores a partial `SUM` instead of per-source fail-closed; a 60s cache must not freeze `complete` across UTC midnight.

ADR **0008** stays reserved for leftover-ops [#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300). This ticket is **0010**.

Overview: [`architecture.md`](../architecture.md#indexer-evidence-digest). Invariants: [`indexer-invariants.md`](../indexer-invariants.md) **Anonymous UTC-day digest (#1206)**. Incident SQL stays unredacted: [`suspicious-activity-queries.md`](../runbooks/suspicious-activity-queries.md).

No design-only pull request. This branch (`cac-design-issue-1206`) only transports the design.

## Outcome

`GET /api/v1/evidence/digest?day=YYYY-MM-DD` returns one JSON document for the UTC window `[day 00:00:00Z, next 00:00:00Z)`:

- Protocol activity **counts** for swap, LP add/remove, limit place/cancel/fill, wrap/unwrap fee events, and UST1 mint/redeem fee events.
- Treasury **fee-path mix** for every `FeeSource::ALL` value: `event_count`, priced `amount_usd`, `share_pct`, plus a capped `by_token` mix.

The body has no wallet, no transaction hash, no actor hash, and no unique-trader census. Empty indexed days are **200** with zeros. Today’s UTC date is **200** with `"complete": false`. A strictly future day is **400**.

Sibling contracts stay as they are: trailing `GET /api/v1/protocol/fees`, grain series `GET /api/v1/protocol/fees/daily`, and `GET /api/v1/defillama/daily`.

## Context

Research readers currently stitch four different clocks and products:

| Surface | Clock | Why it is not this digest |
|---------|--------|---------------------------|
| `GET /api/v1/protocol/fees?window=` | Trailing `24h` / `7d` / `30d` rollup | Not a calendar day ([#576](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/576)). |
| `GET /api/v1/protocol/fees/daily` | UTC grain series of totals | No `by_source`. No LP or limit activity ([#689](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/689)). |
| `GET /api/v1/defillama/daily?timestamp=` | UTC day, unix midnight | Gem-excluded, **404** until rolled, listing fees only ([#631](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/631) / [#687](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/687)). |
| Pair and trader tapes, `/gt/events`, `/overview` | Various | Emit `sender` / `maker` / `owner` / `provider` or `unique_traders_24h`. |

`FeeSource::ALL` is already `swap_amm`, `book_take`, `limit_place`, `wrap`, `unwrap`, `ust1_mint`, `ust1_redeem` ([#586](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/586) / [#613](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/613) / [#614](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/614)). Swap parent rows are `swap_events` (**L10** / [#216](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/216)). LP kinds are `liquidity_events.event_type` `add` | `remove`. Limit kinds are `limit_order_placements`, `limit_order_cancellations`, `limit_order_fills`.

[#1205](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1205) is a paginated event tape that keeps `tx_hash` and `actor_hash`. That tape can be joined to LCD. This digest is the cover sheet that cannot.

The issue text both says to reject `day=today` and, in **AC10**, to return today with `"complete": false`. **AC10 wins.** Today is a legal day. Only `day > Utc::now().date_naive()` is future.

## Non-goals

- Frontend, Protocol charts, CSV, NDJSON, object storage, scheduled dumps, API keys, HMAC, or an unredacted twin.
- New ingest, `wrap_events`, or wrap **principal** volume.
- New fee sources ([#1209](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1209) / [#1210](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1210) / [#1211](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1211)).
- [#1205](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1205) event pages, [#1204](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1204) OpenAPI pack, [#1202](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1202) product analytics.
- `COUNT(DISTINCT sender|maker|owner|provider)`, per-pair boards, per-wallet splits, factory address pin.
- Changing `/protocol/fees`, `/protocol/fees/daily`, `/defillama/daily`, `/overview`, `/gt/events`, or trader routes.
- Rewriting suspicious-activity SQL. One pointer is enough.
- Deploy, spend, custody, or policy expansion ([agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297)).

## Decision

**One handler, bound aggregate SQL, assemble in Rust.** Public `GET`. Anonymity is the access control.

### Clock

Parse `day` with `NaiveDate::parse_from_str(day, "%Y-%m-%d")` only, then `and_hms_opt(0, 0, 0)` and `Utc` — the same UTC midnight as `utc_day_start` in [`indexer/src/indexer/defillama.rs`](../../indexer/src/indexer/defillama.rs). Reject empty, whitespace, `T`, `Z`, offsets, and unix timestamps before any query. Bind `$1` / `$2` as `timestamptz`. Never concatenate the query string into SQL.

The window is half-open. `$1` is that midnight. `$2` is `$1 + 1 day`. Every count and fee statement uses `block_timestamp >= $1 AND block_timestamp < $2`. Do not call `date_trunc` in SQL. Do not use `<= 23:59:59`.

| Input | Result |
|-------|--------|
| Missing `day`, `2026-13-40`, `2026-09-03T00:00:00Z`, unix `timestamp=` | **400** |
| `day` equal to today’s UTC date | **200**, `"complete": false` |
| `day` strictly before today | **200**, `"complete": true` |
| `day` strictly after today | **400** |

`complete` is `day < Utc::now().date_naive()`, evaluated when the response is built, including on a cache hit. It means the calendar day has ended. It does not mean the indexer has caught the chain tip. A lagging poller can still add rows to a past day; the next cache expiry shows the new counts. Do not copy Llama’s **404** when a rollup row is missing.

### Query allowlist

The only accepted query key is `day`. Any other key — `window`, `timestamp`, `from`, `to`, `format`, `sender`, `trader`, `maker`, `address`, `tx`, `tx_hash`, `join`, `redact`, `events`, `wallet`, `addr` — is **400**. A denylist would miss synonyms and could let a client believe it filtered by wallet.

`ProtocolFeesQuery` has no `deny_unknown_fields`, so axum drops unknown keys on `/protocol/fees`. Do not copy that struct. This handler deserializes a struct whose only field is `day: String` with `#[serde(deny_unknown_fields)]`, or it rejects the raw query when any key other than `day` is present. Duplicate `day` keys are **400**. Missing or empty `day` is **400**.

**400** bodies are these static sentences. Do not interpolate the parameter value (a bech32 in the error string would fail the anonymity check):

| Case | Body |
|------|------|
| Missing, empty, or malformed `day` | `Invalid day, expected YYYY-MM-DD` |
| Future `day` | `day is in the future` |
| Any other key, or a duplicate `day` | `Unexpected query parameter` |

`POST` / `PUT` / `DELETE` → **405**. `format=csv` is an extra key → **400** with `Unexpected query parameter`.

### Counts

Independent `COUNT(*)` queries. No join from fees to swaps or to `traders`. Counts are gross rows in the window. Do not subtract cancels from places. Do not count the resting book. `liquidity_events.event_type` is already checked to `add` | `remove`; still count only those two strings.

| JSON | SQL |
|------|-----|
| `counts.swap.swap` | `COUNT(*)` from `swap_events` in the window. Limit fills do not add to this (**L10**). One hybrid pool+book parent row is one swap. Gem pairs count. |
| `counts.lp.add` / `remove` | `liquidity_events.event_type`. Unknown types are ignored. Never emit GT `join` / `exit`. |
| `counts.limit.place` / `cancel` / `fill` | `limit_order_placements`, `limit_order_cancellations`, `limit_order_fills`. Fill count may differ from swap count. |
| `counts.wrap.wrap` / `unwrap` | `protocol_fee_events` rows whose `source` is `wrap` or `unwrap`. These are treasury-fee events, not wrap principal. |
| `counts.window.ust1_mint` / `ust1_redeem` | Same table, sources `ust1_mint` and `ust1_redeem`. Always present (zeros allowed). |

`counts.wrap.*` and `counts.window.*` equal the matching `fees.by_source[].event_count`. They are the same rows shown as activity and as a fee path. Do not add them into swap count or into a second volume figure.

### Fee-path mix

`GROUP BY source` on `protocol_fee_events` for the same window. Emit `FeeSource::ALL` in enum order (seven rows on current `main`). Unknown DB source strings are omitted. Do not keep a second hardcoded list.

Do not copy `refresh_source_breakdown` in [`indexer/src/db/queries/protocol_fees.rs`](../../indexer/src/db/queries/protocol_fees.rs). That job skips wrap/unwrap when `wrap_mapper_configured` is false and skips mint/redeem when `ust1_window_configured` is false, and it stores `SUM(fee_usd) FILTER (WHERE fee_usd IS NOT NULL)` with no unpriced count. This digest always emits seven rows. An unpinned mapper or window is idle zeros, not a missing key. Do not add `wrap_mapper_configured` or `ust1_window_configured` to the JSON.

Each source aggregate returns `event_count = COUNT(*)`, `unpriced_count = COUNT(*) FILTER (WHERE fee_usd IS NULL)`, and `priced_usd = SUM(fee_usd) FILTER (WHERE fee_usd IS NOT NULL)`. Pass those three into `daily_usd_field_fail_closed`. Serialize priced amounts and shares with `bd_plain_string` (`normalized().to_plain_string()` in [`indexer/src/api/pairs.rs`](../../indexer/src/api/pairs.rs)). Do not use `BigDecimal::to_string()` — the trailing fees handler does, and it can emit scientific notation.

Per source, fail closed ([#586](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/586) **C3** / `daily_usd_field_fail_closed`):

| Case | `event_count` | `amount_usd` |
|------|---------------|--------------|
| No rows | `0` | `"0"` |
| Rows and any `fee_usd` IS NULL | row count | JSON `null` |
| Rows, all priced, sum > 0 | row count | plain digit string |
| Rows, all priced, sum ≤ 0 | row count | JSON `null` |

Headline `fees.total_usd` is `daily_headline_usd(total_event_count, priced_sum)` ([#687](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/687) **EFee-6**): priced `SUM(fee_usd)` of non-null stamps only. Zero fee rows → `"0"`. Any fee rows and priced sum ≤ 0 → JSON `null`. One unpriced source must not wipe a priced wrap. Clients must not expect `total_usd` to equal the sum of displayed per-source `amount_usd` when a source is fail-closed `null`.

`share_pct` uses that headline priced sum:

- Priced sum ≤ 0 → every `share_pct` is JSON `null` (including idle `"0"` amounts).
- Idle source and priced sum > 0 → `"0"`.
- Fail-closed source → `null`.
- Priced source → `source_usd / priced_sum * 100` via `bd_plain_string` ([#557](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/557)).

Not counted: `spread_amount`, burn tax, gas, hook fees, community-tax extra-debit, LP principal, book escrow (**PFee** / **L7**).

### Token mix

`GROUP BY asset_id` joined only to `assets` for `symbol` and contract or native denom. Cap **8** named rows (`TOKEN_CAP` in [`indexer/src/db/queries/protocol_fees.rs`](../../indexer/src/db/queries/protocol_fees.rs)), then one `is_other: true` row with `symbol` `"other"` and `contract_or_denom` omitted. Order by priced USD descending, nulls last, `asset_id` ascending. Nine assets → eight named plus `other`. Eight or fewer → no `other` row. A day with no fee rows → `"by_token": []` (present, not omitted).

Each asset uses the same fail-closed helper (`unpriced_count` for that `asset_id`). The `other` row uses the tail’s combined unpriced count and priced sum, so one unpriced dust token makes `other.amount_usd` null instead of a partial sum.

`contract_or_denom` is `assets.contract_address` when `is_cw20`, otherwise `assets.denom`. Do not select `pairs.contract_address`. `by_token` may contain CW20 **token** contracts and denoms `uusd` / `uluna`. It must not contain trader bech32, the wrap-mapper address, or the factory address. Omit `asset_id`, `amount_human`, and `rank`.

Gems stay in **counts**. Gem fee stamps stay unpriced (`null` per source). This route is not L639-safe and is not a Llama clone.

### Router, cache, governors

Register on the global `api_router` in [`indexer/src/api/mod.rs`](../../indexer/src/api/mod.rs) next to `/api/v1/protocol/fees`. Do not add the path to `lcd_heavy_router`. No LCD on the request path.

In-process cache **60s**, key = canonical `day` only, value = counts and fees. Not the peer IP. On a hit, still set `complete` from the clock so a fill at 23:59Z cannot keep `complete: false` after 00:00Z for that same day key. Existing global `tower_governor` (peer IP, no `X-Forwarded-For`) still applies. The route is registered with `get` only. Global CORS allows POST, but this path does not add `.post`, so POST is **405**. DB errors go through `internal_err()` → `"Internal server error"` with no sqlx text. Stay under the existing 30s `TimeoutLayer`.

Handler module: `indexer/src/api/evidence_digest.rs`. If [#1205](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1205) has already added `evidence.rs` when this is implemented, add this path as a **second route** with its own response types. Do not put digest fields on paginated events. Do not wait for #1205.

`utoipa` path, schemas, and tag **Evidence** on `ApiDoc`, so `GET /api-docs/openapi.json` lists `/api/v1/evidence/digest`.

### Response shape

```json
{
  "day": "2026-09-03",
  "timezone": "UTC",
  "complete": true,
  "counts": {
    "swap": { "swap": 10 },
    "lp": { "add": 2, "remove": 1 },
    "limit": { "place": 4, "cancel": 1, "fill": 3 },
    "wrap": { "wrap": 1, "unwrap": 2 },
    "window": { "ust1_mint": 0, "ust1_redeem": 0 }
  },
  "fees": {
    "total_usd": "12.34",
    "by_source": [],
    "by_token": []
  }
}
```

`by_source` length is `FeeSource::ALL.len()`. Counts are JSON numbers. USD and shares are plain digit strings or `null`. No `events` array. No `skip_serializing` of zero counts.

## Component / state / interface changes

| Layer | Change |
|-------|--------|
| HTTP | New `GET /api/v1/evidence/digest`. No change to existing route JSON. |
| SQL | Read-only `COUNT` / `GROUP BY` with bound timestamps. Tables: `swap_events`, `liquidity_events`, three limit tables, `protocol_fee_events`, `assets`. |
| Schema | Four new BRIN indexes (below). No new table. No ingest change. |
| Cache | 60s map of counts and fees, keyed by `day`. `complete` is not cached. |
| OpenAPI | Path + schemas + tag **Evidence**. |
| dApp | None. |

SQL text in the handler must not contain `traders`, `sender`, `maker`, `owner`, `provider`, `COUNT(DISTINCT`, or `pair_reserves`, and must not select `tx_hash`.

## Affected invariants

| ID | Effect |
|----|--------|
| **E1206-1** | One UTC day; `complete` is calendar-ended at response time, not chain-tip and not a cached flag. |
| **E1206-2** | Counts as in the table above. **L10** unchanged: fills are not swap volume or swap count. |
| **E1206-3** | `counts.wrap` is fee events. `counts.window` keeps mint/redeem off wrap. |
| **E1206-4** | Seven `FeeSource::ALL` rows even when wrap/UST1 pins are unset. Idle `"0"`. Per-source `unpriced_count` fail-closed. Headline `daily_headline_usd`. |
| **E1206-5** | No people columns, no tx ids, no `COUNT(DISTINCT` people, no join to `traders`. Token contracts only on `by_token`. |
| **E1206-6** | Query allowlist is `{day}` via `deny_unknown_fields` or a raw-key check. Duplicate `day` is **400**. Static **400** text. **405** for other methods. |
| **E1206-7** | Global governor, not LCD-heavy. 60s cache. `internal_err` on DB failure. |
| **E1206-8** | Gems count. Llama gem-exclude and **404** stay on `/defillama/daily` only. |
| **L10 / PFee / L7 / #1269** | Unchanged. Digest reads rows those invariants already store. |
| **DoS path 4** | Named exception for this one GET. Overview, protocol fees, volume/liquidity series, and Llama stay rollup-only. |

## Alternatives

| Option | Why not |
|--------|---------|
| Add `by_source` onto `/protocol/fees/daily` | Chart series and rollup tables. Wrong clock grain API. Still no LP/limit counts. |
| Alias `/defillama/daily` | Gem filter, **404**, unix timestamp, no place/cancel/LP counts, embeds contract pins the digest omits. |
| Strip `tx_hash` off #1205 pages | The tape is designed to stay chain-linkable. A cover sheet is a different product. |
| New UTC rollup table | Issue forbids new tables. A second rollup can drift from the event tables this route is supposed to count. |
| `COUNT(DISTINCT sender)` hashed | Still a person census. Forbidden. |
| Join `protocol_fee_events` to `swap_events` on `tx_hash` | Re-identifies the day. Forbidden. |
| Reject `day=today` | Contradicts **AC10**. Today is incomplete, not invalid. |
| Denylist of bad query keys | Misses `wallet=` / `addr=`. Allowlist `{day}` is closed. |
| LCD-heavy governor | This handler does not call LCD. |
| Forever-cache past days | Indexer catch-up would stay invisible. 60s matches other GETs. `complete` stays outside that cache. |
| Copy the trailing fee rollup (skip unconfigured families, partial `SUM`) | Drops idle wrap/UST1 rows and is not fail-closed inside a source. |

## Complexity added / removed

**Added:** one GET, one handler module, four BRIN indexes, a 60-second day cache, an OpenAPI tag, one invariant row.

**Removed:** nothing. Trailing fees, fee series, and Llama stay.

Net: one bounded read path so evidence readers stop paging unredacted tapes for a daily cover sheet.

## Migration

No new table. Do not edit `20260821120000_protocol_fees.sql` or `20260604120100_swap_events_block_timestamp_brin.sql`.

`swap_events` already has `idx_swaps_block_timestamp_brin`. `protocol_fee_events` already has `protocol_fee_events_ts_idx` and `protocol_fee_events_source_ts_idx`.

New file `indexer/migrations/20260922120000_evidence_digest_day_brin.sql` (prefix free on current `main`; do not reuse `20260921120000` / `20260921130000`):

- `idx_liq_block_timestamp_brin` on `liquidity_events (block_timestamp)`
- `idx_lo_placements_block_timestamp_brin` on `limit_order_placements (block_timestamp)`
- `idx_lo_cancellations_block_timestamp_brin` on `limit_order_cancellations (block_timestamp)`
- `idx_lo_fills_block_timestamp_brin` on `limit_order_fills (block_timestamp)`

`CREATE INDEX IF NOT EXISTS` (not `CONCURRENTLY`). `sqlx::migrate!` runs inside a transaction, and `CONCURRENTLY` cannot. Match [`20260604120100_swap_events_block_timestamp_brin.sql`](../../indexer/migrations/20260604120100_swap_events_block_timestamp_brin.sql). Pair-leading btree indexes (`idx_liq_pair_time`, `idx_lo_*_pair_time`) stay for pair tapes; they do not serve a protocol-wide day predicate.

If `20260922120000` is already taken when implement starts, bump the numeric suffix and the paired `down.sql` name together. Do not edit a migration that has already shipped.

Paired `indexer/migrations/revert/20260922120000_evidence_digest_day_brin.down.sql` drops **only** those four indexes. Do not drop `idx_swaps_block_timestamp_brin` ([#281](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/281)).

No wasm. No factory `UpdateConfig`. No dApp env keys.

## Observability

Indexer tracing stays log-only ([#200](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/200)). No `/metrics` scrape. No per-request success log.

- DB failure: existing `internal_err` path. Client sees `"Internal server error"`.
- Handler elapsed above 5s: `tracing::warn` with the canonical `day` and elapsed milliseconds. No SQL text and no row payload.
- Cache hit: no extra log.

Operators compare a seeded day with SQL `COUNT(*)` off the request path. They do not use this route to list wallets.

## Failure modes

| Mode | Behavior |
|------|----------|
| SQL metacharacters in `day` | `NaiveDate` parse fails → **400**. Tables unchanged. |
| Extra or identity query keys | **400** `Unexpected query parameter`. Counts stay protocol-wide. Body does not echo the value. A struct that ignores unknown fields is a bug. |
| Duplicate `day` | **400** `Unexpected query parameter`. |
| Future day | **400** `day is in the future`. |
| Empty day | **200** zeros, `by_token: []`, and seven idle fee rows. Not **404**. |
| Today | **200**, `complete: false`. |
| UTC midnight during the 60s TTL | Cached counts may be up to 60s old. `complete` follows the clock. |
| Indexer behind chain tip | Past `complete: true` can still grow until catch-up. 60s cache then shows new counts. |
| Mixed null and priced `fee_usd` in one source | That source `amount_usd` is `null`. Headline keeps priced stamps from other sources. |
| Unpriced fee rows | That source `amount_usd` is `null`. Headline keeps other priced sources. |
| Wrap or UST1 pin unset | Those sources are still present with `event_count=0` and `amount_usd="0"`. |
| Unknown `source` or LP `event_type` | Omitted. No new JSON key. |
| Gem swap | Increments `counts.swap`. Llama daily still omits it. |
| 10k events in one day | One aggregate object. No `events` array. |
| Closed DB pool | **500** static text. |
| Burst over global RPS | **429** + `Retry-After`. Not the LCD-heavy limiter. |
| Reorg replay | Unique fee keys and swap uniqueness already dedupe. Digest counts whatever is stored. |

## Ordered implementation slices

| Slice | Owner | Work | Depends on |
|-------|--------|------|------------|
| **0 — this design** | design slice | ADR **0010**, architecture `#indexer-evidence-digest`, invariant row, README index, runbook pointer. Transport on `cac-design-issue-1206` only. | — |
| **1 — indexes** | implement | Migration + paired `down.sql` above. | Slice 0 accepted |
| **2 — handler** | implement | `evidence_digest.rs`, router, OpenAPI, 60s counts cache, `deny_unknown_fields`, fail-closed USD. | Slice 1 |
| **3 — tests** | implement | `indexer/tests/api_evidence_digest.rs` and `make verify-issue-1206` (cargo test + docs grep for this ADR and the architecture anchor). | Slice 2 |
| **4 — indexer boot** | leftover ops | Coolify indexer image runs `sqlx::migrate!()` and serves the route. Ordinary indexer deploy. Not a founder card. Not #297. | Slice 3 merged |

**Slice-0 apply:** insert these sections. Do not `git checkout <design-sha> --` `docs/architecture.md`, `docs/indexer-invariants.md`, `docs/README.md`, or `docs/runbooks/suspicious-activity-queries.md`. Other design branches touch the same files.

**Open issue dependencies:** none. Shipped ingest ([#586](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/586) / [#613](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/613) / [#614](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/614) / [#1269](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1269) / [#631](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/631)) is already on `main`.

**Do not block on:** #1205, #1204, #1202, #1209, #1210, #1211.

## Tests

Postgres harness (`make setup-indexer-postgres`). New `indexer/tests/api_evidence_digest.rs`. No LocalTerra. No Vitest.

| # | Expect |
|---|--------|
| T1 | Seed all surfaces and seven fee sources on day D, plus previous and next UTC day. `day=D` counts only D. |
| T2 | Hybrid parent + one fill: `counts.swap.swap=1`, `counts.limit.fill=1`. |
| T3 | LP add and remove. Body has no `join` or `exit`. |
| T4 | Wrap, unwrap, mint, redeem stay four fee rows. Wrap counts exclude mint/redeem. `counts.window` matches mint/redeem `event_count`. |
| T5 | Missing `book_take` still returns a zero row. |
| T6 | One source with both null and priced `fee_usd` → that source `amount_usd` null. Idle sibling `"0"`. Headline keeps a priced wrap via `daily_headline_usd` (**EFee-6**). |
| T6b | Assemble from an empty source map (mapper pin unset) still returns seven `by_source` rows. No `wrap_mapper_configured` key. |
| T7 | Empty day **200**, not **404**. |
| T8 | Today `complete=false`. Past day `complete=true`. |
| T9 | Events at D−1 23:30Z and D 00:30Z split across calendar days. Digest need not equal `/protocol/fees?window=24h`. |
| T10 | Token cap: nine assets → eight named + `other`. No trader bech32 in `by_token`. |
| T11 | Gem swap increments `counts.swap`. |
| T12 | OpenAPI path present. |
| T13 | Existing protocol-fee, fee-series, and defillama tests unchanged. |
| T14 | `serde_json::to_string` lacks seeded actor bech32, fixture tx hex, and keys `sender`, `maker`, `owner`, `provider`, `trader`, `actor_hash`, `tx_hash`, `unique_traders`. |
| A1 | `day` injection strings → **400**, tables intact. |
| A2 | Identity and join keys, including `sender=`, → **400** `Unexpected query parameter` (not a 200 that ignores the key). Static body. Protocol-wide counts. |
| A3 | `unique_traders` absent even when overview would show traders. |
| A4 | `day=2099-01-01`, `timestamp=`, `window=24h` → **400**. |
| A5 | Non-GET → **405**. `format=csv` → **400**. |
| A6 | Closed pool → `"Internal server error"`. |
| A7 | Governor burst → **429** + `Retry-After`. Route absent from the LCD-heavy list. |
| A8 | No `events` array. SQL string assert: no `traders`, `sender`, `maker`, `owner`, `provider`, `COUNT(DISTINCT`, `pair_reserves`. |
| A9 | Two senders: one protocol total, no per-sender field. |
| A10 | `complete` is a pure function of `(day, now)`. The cache value has no `complete` field. |

`make verify-issue-1206` runs that test and greps this ADR plus the architecture anchor. Keep `verify-issue-586`, `631`, `689`, and `1269` green.

## Rollout

1. Reviewer accepts this ADR (not this design slice).
2. Implement slices 1–3 on a normal merge request.
3. Indexer process boot applies the migration and serves the route. No frontend deploy. No wasm store.
4. Smoke: `curl` a seeded `day` and confirm seven `by_source` rows. `rg` of `terra1` on the body matches token contracts only, or nothing when the mix is native denoms.

Coolify indexer restart after merge is ordinary leftover. It does not expand deploy policy, spend, or custody.

## Rollback

- **Route:** ship an indexer binary without the handler. In-process cache disappears with the process.
- **Indexes:** optional `down.sql` drops the four new BRINs. Leaving them in place is safe.
- Do not drop `idx_swaps_block_timestamp_brin`.
- Preferred path is fix-forward.

This is not a chain halt, pause, or treasury rotate. Coolify three-way rollback ([ADR 0006](./0006-indexer-health-git-sha.md)) applies if this migration is the only schema ahead of the restored image: the paired `down.sql` exists, so a full restore may take path **2(c)** for this version. Partial restores that keep the new binary stay on **2(b)** and keep the indexes.

## Integration completion criteria

- **AC1–AC15** from [#1206](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1206) hold, with today allowed (`complete: false`), `counts.window` always present, and seven `by_source` rows even when wrap/UST1 pins are unset.
- `cargo test --manifest-path indexer/Cargo.toml --test api_evidence_digest -- --nocapture` green, including T6/T6b (fail-closed and idle families) and A2/A10 (unknown query keys and `complete` outside the cache).
- `GET /api-docs/openapi.json` contains `/api/v1/evidence/digest`.
- `/protocol/fees`, `/protocol/fees/daily`, `/defillama/daily`, `/gt/events`, and trader routes keep their response shapes.
- Invariant row **Anonymous UTC-day digest (#1206)** is on `main` via insert, not only on `cac-design-issue-1206`.
- Handler SQL matches **E1206-5**. Amounts use `bd_plain_string`. Extra query keys are **400**, not ignored.

## Links

- Issue: [#1206](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1206)
- Not this ticket: [#1205](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1205), [#1204](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1204), [#1202](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1202)
- Clock and fees: [#576](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/576), [#586](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/586), [#631](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/631), [#687](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/687), [#689](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/689)
