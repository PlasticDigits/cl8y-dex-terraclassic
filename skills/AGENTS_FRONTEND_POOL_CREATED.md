# Agent playbook: `/pool` Created column (GitLab #662)

Audience: third-party agents changing pair list JSON, `/pool` table cells, `sort=created`, or relative-time formatting.

**Issue:** [GitLab **#662**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/662)  
**Invariants:** [`docs/frontend.md` § Liquidity pools list](../docs/frontend.md#liquidity-pools-list-indexer-vs-factory) (**P662-1–P662-8**); [`docs/indexer-invariants.md`](../docs/indexer-invariants.md) pair list `created_at`  
**Related:** [#547](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/547) table + deferred Created cell, [#655](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/655) v2 LP USD (does **not** own Created), [#534](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/534) catalog rank, [#562](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/562) hide gems, [#489](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/489) no lectures, [#653](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/653) no nested chrome

## Problem class

`/pool` shipped a sortable **Created** header whose cell was a hardcoded em-dash. `GET /api/v1/pairs` dropped `pairs.created_at` even though list SQL already selected it, and `sort=created` ordered by unused `created_at_block`.

## Do / don’t

- **Do** emit additive RFC3339 `created_at` on every `PairResponse` (list, detail, token-pairs) from `pairs.created_at` (indexer **first-seen**).
- **Do** `ORDER BY p.created_at {ASC|DESC}, p.id ASC` for `sort=created`. Default order **desc**.
- **Do** render `/pool` cells with `formatRelativeAge` (`N minutes/hours/days/years ago` or `just now`). Missing/invalid → `—`.
- **Do** keep age on the existing `getPairs` payload. No per-row LCD / `getPair` / interval timer (**P547-9**).
- **Don’t** claim on-chain genesis in UI copy. After `--fresh` / DB rebuild every pair looks new — operator docs only (**#489**).
- **Don’t** print raw ISO in the cell. Hover `title` is locale absolute time from a **parsed** instant, never the raw string.
- **Don’t** scrape `/gt/` `createdAtBlockTimestamp` from the dApp. Official list JSON has its own field.
- **Don’t** raise `PAIR_LIST_LIMIT_MAX`, rename `volume_quote_24h`, or add #655 `liquidity_usd` here.
- **Don’t** reconstruct age from first swap, LCD `ContractInfo`, or factory event scans in this issue.

## Invariants (P662-1–P662-8)

1. **P662-1** — List, `GET /api/v1/pairs/{addr}`, and `GET /api/v1/tokens/{addr}/pairs` share one `PairResponse` with `created_at` (RFC3339 UTC).
2. **P662-2** — `sort=created` orders by `p.created_at` (always NOT NULL) then `p.id`. Default **desc**. Invalid `sort` still **400**.
3. **P662-3** — Visible Created cell is the relative phrase (or `—`). `data-testid="pool-row-created"` stays.
4. **P662-4** — No extra HTTP or per-row timers for age. Format at render from `Date.now()`; React Query 30s stale is enough.
5. **P662-5** — Catalog default has **no** Created caret. Clicking Created uses indexer sort (no catalog overlay), default desc. Search stays `relevance`. Production still omits gems (**P562-3**).
6. **P662-6** — Parse ISO only. HTML, `javascript:`, unix-ms/s, year 0, far-future, non-finite → `—`. Text node / React children only.
7. **P662-7** — Clock is indexer first-seen, not factory `CreatePair`. Document in this playbook / `docs/frontend.md` table, not a table lecture.
8. **P662-8** — `#655` can still add a column without colliding on `created_at`. Manage `colSpan` still matches column count.

## Canonical code

| File | Role |
|------|------|
| `indexer/src/api/pairs.rs` | `PairResponse.created_at` + `pair_to_response` |
| `indexer/src/db/queries/pairs.rs` | `sort=created` `ORDER BY p.created_at` |
| `frontend-dapp/src/utils/formatDate.ts` | `formatRelativeAge` / `parseCreatedAtMs` |
| `frontend-dapp/src/components/pool/PoolPairsTable.tsx` | Cell + optional `title` |
| `frontend-dapp/src/types/index.ts` | `IndexerPair.created_at?` |

## Verify

```bash
make verify-issue-662
make verify-issue-547
```

Indexer (Postgres): `cd indexer && cargo test --test api_pairs -- --test-threads=1`

## Related

- [`AGENTS_FRONTEND_POOL_TABLE.md`](./AGENTS_FRONTEND_POOL_TABLE.md) — table chrome; **P547-4** now points here
- [`AGENTS_FRONTEND_PAIR_CATALOG_RANK.md`](./AGENTS_FRONTEND_PAIR_CATALOG_RANK.md) — no catalog overlay on Created
- [`AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md`](./AGENTS_FRONTEND_RETAIL_TEST_TOKENS.md) — gems still hidden in production
- [`AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md`](./AGENTS_FRONTEND_COPY_COGNITIVE_LOAD.md) — no first-seen essay in the header
- [`AGENTS_FRONTEND_CHROME_NESTING.md`](./AGENTS_FRONTEND_CHROME_NESTING.md) — no `card-glass` per row
