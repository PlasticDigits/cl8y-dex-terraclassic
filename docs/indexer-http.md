# Indexer HTTP pack

Copy-paste read API for the five evidence surfaces: **swaps** (tape), **pools**, **fees**, **burns** (hooks), and **volume windows**. Semantics live in [`indexer-invariants.md`](./indexer-invariants.md). This page does not add routes.

Quotes and history HTTP are not on-chain execute. Pair `swap` vs solver/hybrid execute stays [#707](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/707).

## Base URL

| Name | Value |
|------|--------|
| Public read API | `https://indexer.dex.cl8y.com` |
| Local indexer | `http://127.0.0.1:3001` |

Send `Accept: application/json`. Almost every route is **GET**. `POST` on a GET-only path returns **405**. The only write-nothing POST is `POST /api/v1/route/solve` (quote merge; body cap 128 KiB → **413**). It is not one of the five surfaces.

```bash
INDEXER="${INDEXER:-https://indexer.dex.cl8y.com}"
```

`$PAIR`, `$TOKEN`, `$TRADER`, and `$HOOK` are **one** address taken from a prior JSON field (`contract_address`, `hook_address`). Pass that whole path segment. Do not join untrusted fragments into the path. Query values use `-G --data-urlencode`.

Global governor default **60 RPS**. LCD-heavy routes default **10 RPS**. Abuse returns **429** and `Retry-After`. Do not loop LCD-heavy paths.

## OpenAPI

The live spec is generated from `ApiDoc` in `indexer/src/api/mod.rs`. There is no checked-in response dump.

```bash
curl -sS "$INDEXER/api-docs/openapi.json" | jq '.paths | keys'
curl -sS "$INDEXER/swagger-ui/"
```

| Path | Role |
|------|------|
| `GET /api-docs/openapi.json` | OpenAPI 3 JSON |
| `GET /swagger-ui/` | Swagger UI |

## Three clocks

| Clock | What it is | Example |
|-------|------------|---------|
| **Trailing** `24h` \| `7d` \| `30d` | `Utc::now() − N`. Not a midnight UTC reset ([#576](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/576)). | `GET /api/v1/overview`, `GET /api/v1/protocol/fees?window=` |
| **UTC series** | Calendar buckets on `protocol/volume/daily` and `protocol/fees/daily` ([#652](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/652) / [#668](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/668)). | `GET /api/v1/protocol/volume/daily?days=7` |
| **DeFiLlama UTC day** | One `timestamp` with `unix % 86400 == 0`. Gem-excluded listing day. Not overview and not trailing 24h ([#631](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/631)). | `GET /api/v1/defillama/daily?timestamp=` |

## Anti-mixups

- **L10.** Headline volume is one parent `swap_events` row per taker tx. Do not SUM `limit_order_fills` into pair volume.
- **Fees ≠ burns ≠ burn tax.** Treasury fees are `GET /api/v1/protocol/fees`. Burns are `GET /api/v1/hooks` with `action` values that start with `after_swap_burn`. Terra Classic burn tax (`tax_amount` as chain tax) is neither.
- **Raw ≠ USD.** `volume_quote_24h`, pair stats `volume_base` / `volume_quote`, and trader rolling `volume_24h` / `7d` / `30d` are raw integer strings. USD fields are a decimal string, JSON `null` (unpriced), or `"0"` (idle). Trader retail USD is `total_volume_usd`, not the raw rolling windows.
- **Pools ≠ `/gt`.** Official pool list is `GET /api/v1/pairs`. Do not scrape `/gt` for the pool table. Gem pairs are omitted from `/cg`, `/cmc`, and `/gt`.
- There is **no** `/burns`, `/swaps`, or `/api/v1/listings` route.

## Path table

| Family | Method | Path | Clock | LCD-heavy | limit | Unhappy |
|--------|--------|------|-------|-----------|-------|---------|
| Pools | GET | `/api/v1/pairs` | trailing 24h stamp on volume fields | no | default 50, max 100; `offset` > 10000 → **400** | bad `sort` → **400** |
| Pools | GET | `/api/v1/pairs/{addr}` | n/a | no | n/a | unknown pair → **404**; `code_id_frozen` may still be listed |
| Pools | GET | `/api/v1/pairs/{addr}/liquidity-events` | n/a | no | default 50, max 200 (clamp ≥ 1) | unknown pair → **404** |
| Pools | GET | `/api/v1/pairs/{addr}/stats` | trailing 24h | no | n/a | `volume_base` / `volume_quote` raw digits; `volume_usd` string or null |
| Pools | GET | `/api/v1/tokens/{addr}/pairs` | n/a | no | n/a | pairs that include the asset |
| Swaps | GET | `/api/v1/pairs/{addr}/trades` | n/a | no | default 50, max 200 | tape of `swap_events`; optional `pool_leg_volume` / `book_leg_volume` |
| Swaps | GET | `/api/v1/traders/{addr}/trades` | n/a | no | default 50, max 200 | `format=json` or `format=csv` only; other format → **400** |
| Fees | GET | `/api/v1/protocol/fees` | trailing `24h` \| `7d` \| `30d` | no | n/a | any other `window` → **400**; `by_source` / `by_token` |
| Fees | GET | `/api/v1/protocol/fees/daily` | UTC series | no | grain cap | `grain=week`, extra `from` / `to` / `window` / `days` → **400** |
| Fees | GET | `/api/v1/overview` | trailing 24h/7d/30d | no | n/a | `total_fees_{24h,7d,30d}_usd`: idle `"0"`, activity+unpriced `null` |
| Fees | GET | `/api/v1/defillama/daily` | DeFiLlama UTC day | no | one day | unaligned `timestamp` → **400** |
| Burns | GET | `/api/v1/hooks` | n/a | no | default 50, max 200 (clamp ≥ 1) | optional `hook_address`; LP mint/burn stays on `liquidity-events` |
| Volume | GET | `/api/v1/overview` | trailing | no | n/a | volume **and** fee windows |
| Volume | GET | `/api/v1/tokens/{addr}` | trailing `volume_stats[].window` | no | n/a | windows `24h` / `7d` / `30d`; idle zeros; lifetime is not zeroed |
| Volume | GET | `/api/v1/protocol/volume/daily` | UTC series | no | `days=7` or `30`, or `grain` + `limit` | `from` / `to` → **400**; `grain=week` → **400** |

`sort=volume_24h` on `/api/v1/pairs` compares raw quote. `sort=volume_usd_24h` and `sort=liquidity_usd` are NULLS LAST. `created_at` is indexer first-seen.

## Example curls

```bash
INDEXER="${INDEXER:-https://indexer.dex.cl8y.com}"

# Pools — trailing 24h USD stamp on the list (not a live SUM).
curl -sS -G "$INDEXER/api/v1/pairs" \
  --data-urlencode "sort=volume_usd_24h" \
  --data-urlencode "order=desc" \
  --data-urlencode "limit=5"
# Then set PAIR to one items[].contract_address (whole segment).
curl -sS "$INDEXER/api/v1/pairs/$PAIR"
curl -sS -G "$INDEXER/api/v1/pairs/$PAIR/liquidity-events" --data-urlencode "limit=20"
curl -sS "$INDEXER/api/v1/pairs/$PAIR/stats"

# Swaps (tape). Do not SUM limit_order_fills into this volume (L10).
curl -sS -G "$INDEXER/api/v1/pairs/$PAIR/trades" --data-urlencode "limit=20"
curl -sS -G "$INDEXER/api/v1/traders/$TRADER/trades" --data-urlencode "limit=20"

# Volume windows — trailing census.
curl -sS "$INDEXER/api/v1/overview"
curl -sS "$INDEXER/api/v1/tokens/$TOKEN"

# Volume windows — UTC series (not trailing 24h).
curl -sS -G "$INDEXER/api/v1/protocol/volume/daily" --data-urlencode "days=7"
curl -sS -G "$INDEXER/api/v1/protocol/volume/daily" \
  --data-urlencode "grain=hourly" \
  --data-urlencode "limit=24"

# Volume windows — DeFiLlama UTC day (2024-07-01 00:00:00 UTC; unix % 86400 == 0).
curl -sS -G "$INDEXER/api/v1/defillama/daily" --data-urlencode "timestamp=1719792000"

# Fees — trailing allowlist. Not hook burns and not Terra Classic burn tax.
curl -sS -G "$INDEXER/api/v1/protocol/fees" --data-urlencode "window=24h"
curl -sS -G "$INDEXER/api/v1/protocol/fees" --data-urlencode "window=7d"
curl -sS -G "$INDEXER/api/v1/protocol/fees" --data-urlencode "window=30d"
curl -sS -G "$INDEXER/api/v1/protocol/fees/daily" \
  --data-urlencode "grain=daily" \
  --data-urlencode "limit=14"

# Burns — hooks. action may be after_swap_burn. Amount from burn_amount / burn_token.
curl -sS -G "$INDEXER/api/v1/hooks" --data-urlencode "limit=50"
curl -sS -G "$INDEXER/api/v1/hooks" \
  --data-urlencode "hook_address=$HOOK" \
  --data-urlencode "limit=50"
```

Trader history CSV is `format=csv` on `/api/v1/traders/{addr}/trades` only. Cells that start with `=`, `+`, `-`, or `@` are prefixed so a spreadsheet does not treat them as formulas. `/protocol/fees` has no CSV.

`jq` filters that match documented fields: `.items[0].contract_address`, `.window`, `.[0].action`, `.volume_usd`.

## Unhappy curls

```bash
# window allowlist — expect 400
curl -sS -G "$INDEXER/api/v1/protocol/fees" --data-urlencode "window=1h"

# unknown pair — expect 404
curl -sS "$INDEXER/api/v1/pairs/not-a-pair/trades"

# GET-only route — expect 405
curl -sS -X POST "$INDEXER/api/v1/pairs"
```

`window=24h;DROP TABLE` and `grain=week` are **400**. Extra `from` / `to` / `window` on `/api/v1/protocol/fees/daily` is **400**. `limit=-1` and `limit=0` clamp to **1**. `limit=999999` clamps to the route max (hooks and trades 200, pair list 100).

## LCD-heavy appendix

One shot only. Default **10 RPS**. **429** means back off. Do not put these in a research loop.

`GET /api/v1/route/solve` is a **quote**, not swap history. `pool_only=true` is pool-only. Execute-path wording stays on [#707](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/707).

```bash
curl -sS -G "$INDEXER/api/v1/route/solve" \
  --data-urlencode "token_in=$TOKEN_IN" \
  --data-urlencode "token_out=$TOKEN_OUT" \
  --data-urlencode "amount_in=1000000"
```

`GET /api/v1/compliance/blacklist-check` proxies factory LCD. More than **16** `tokens` or **8** `pairs` is **400** before LCD. `GET /health` is liveness (`{"status":"ok"}`, optional `git_sha`). `GET /api/v1/health/fee-discount` is the registry probe (no per-trader data). Both are on the router and in OpenAPI; they are not evidence surfaces.

`/gt/events` is combined swap+liquidity for a block range (max 2000 blocks / 5000 rows). It is not the pool census.

## Drift check

`make verify-issue-1204` greps this pack and runs the `ApiDoc` unit test. Served OpenAPI is also asserted in `indexer/tests/security.rs` (`openapi_spec_available`).
