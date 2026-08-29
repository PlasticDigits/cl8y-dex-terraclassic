# Agent playbook: post-merge !477 leftover verify (GitLab #701)

Audience: third-party agents verifying Coolify migrate + live list↔stats after [!477](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/merge_requests/477) (#692 `/pool` Vol USD + pair-list `volume_usd_24h`) landed on `main` (`f4fa6788`). Child `make verify-issue-692` already existed on the merge commit.

**Issue:** [GitLab **#701**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/701)  
**Parent (closed unless a merged invariant is wrong):** [#692](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/692).  
**Invariants:** [`docs/qa-invariants.md`](../docs/qa-invariants.md) **Q18** (**M701-1–M701-8**)  
**Verify:** `make verify-issue-701`

Sibling leftovers: [#698](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/698) (API4 + FE-01 / !474–!475), [#702](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/702) (`/trade` flatten / !476). #678 acquire guidance was already on `main`; no extra leftover. Do **not** reopen #692 / #678 for ops/QA. Do **not** wait on GitLab CI quota.

## Merged MR

| MR | Issue | Skill |
|----|-------|-------|
| !477 | #692 `/pool` Vol USD + `volume_usd_24h` | [`AGENTS_INDEXER_PAIR_VOLUME_USD.md`](./AGENTS_INDEXER_PAIR_VOLUME_USD.md) |

## Invariants (M701-1–M701-8)

| ID | Rule |
|----|------|
| **M701-1** | Local regression is `make verify-issue-701`, which runs child **692**. A child FAIL fails the stack. Live Coolify leftover probes SKIP unless `indexer.dex.cl8y.com` / `dex.cl8y.com` answers (FAIL when `VERIFY701_REQUIRE_LIVE=1` or `VERIFY701_IID=701`). Leftover Playwright SKIP unless LocalTerra is up (FAIL when `VERIFY701_REQUIRE_CHAIN=1`). |
| **M701-2** | Coolify Postgres has `indexer/migrations/20260829120000_pair_volume_24h_usd.sql` (nullable `pair_volume_24h.volume_usd` + `NULLS LAST` index), then indexer redeploy. `refresh_pair_volumes` must run once so list USD is not all omitted. |
| **M701-3** | Live `GET /api/v1/pairs?sort=volume_usd_24h` includes additive `volume_usd_24h`. Invalid `sort=volume_usd` → **400**. Priced pair list USD ≈ `GET /pairs/{addr}/stats` `volume_usd` within ~5 min lag (**PVol-5**). Unpriced / idle omit/`NULL` → dApp **—**, never fake `$0`. |
| **M701-4** | Coolify **frontend rebuild** from `f4fa6788+`. `/pool` Vol cells are compact `$…` or **—** (`pool-row-vol`; never quote-token `48.2M` without `$`). Manage line + Trade/Limits/Charts pair-search badges use the same USD field. Do **not** N+1 `/stats` from `/pool`. Do **not** humanize indexer `volume_quote_24h`. |
| **M701-5** | Production gem hide unchanged (**P562**). Do **not** change `liquidity_usd` / v2 LP USD (#655 / #664). `sort=volume_24h` stays raw-quote. |
| **M701-6** | Do **not** reopen #692 / #678 for ops/QA. Do **not** wait on GitLab CI. Do **not** convert `volume_quote_24h` in the browser. |
| **M701-7** | Optional LocalTerra leftover Playwright is `e2e/pool-table-547.spec.ts` at **5 workers** when `VERIFY701_LEFTOVER_E2E=1` (child #692 already runs it). Dedicated Vite **`:3173`**. Do **not** leak a different `PLAYWRIGHT_WEB_PORT` (CORS). |
| **M701-8** | This playbook + **Q18** + child skills stay crosslinked. GitLab CI quota is not a substitute for local verify. |

## Coolify leftovers (operator)

Apply migration then redeploy indexer from current `main`:

```
indexer/migrations/20260829120000_pair_volume_24h_usd.sql
```

Then probe:

```bash
curl -sS "https://indexer.dex.cl8y.com/api/v1/pairs?limit=5&sort=volume_usd_24h&order=desc"
curl -sS "https://indexer.dex.cl8y.com/api/v1/pairs/${PAIR}/stats"
curl -sS -o /dev/null -w "%{http_code}\n" \
  "https://indexer.dex.cl8y.com/api/v1/pairs?sort=volume_usd"
# expect 400
```

`make verify-issue-701` records leftover probes as SKIP unless the hosts answer. Fail closed with `VERIFY701_REQUIRE_LIVE=1`.

## Do / don’t

- **Do** run `make verify-issue-701` from a git worktree after pulling `main`.
- **Do** `make setup-indexer-postgres` for child #692.
- **Do** restart a local indexer after `git pull` so the new migration applies.
- **Don’t** reopen #692 unless a merged invariant is wrong.
- **Don’t** N+1 `/stats` or invent USD from quote volume / hub-prices / `$1` UST1.
- **Don’t** treat GitLab CI quota as leftover evidence.

## Regression

```bash
make verify-issue-701
VERIFY701_SKIP_CHILDREN=1 make verify-issue-701
VERIFY701_SKIP_LIVE=1 VERIFY701_SKIP_CHAIN=1 make verify-issue-701
VERIFY701_REQUIRE_LIVE=1 make verify-issue-701
```
