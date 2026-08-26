# Agent playbook: post-merge !437–!458 leftover verify (GitLab #673)

Audience: third-party agents verifying the integrated tip after [!437](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/merge_requests/437)–[!458](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/merge_requests/458) landed on `main` (no CI wait, no automerge). Draft !456 was skipped (same #666 as !458). Child `make verify-issue-{655–672}` already existed on the merge commits.

**Issue:** [GitLab **#673**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/673)  
**Parents (closed unless a merged invariant is wrong):** [#655](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/655)–[#672](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/672) (see MR table below).  
**Invariants:** [`docs/qa-invariants.md`](../docs/qa-invariants.md) **Q15** (**M673-1–M673-8**)  
**Verify:** `make verify-issue-673`

Do **not** reopen the child issues unless a merged invariant is wrong. Do **not** wait on GitLab CI quota. Do **not** restore Charts DEX-census tiles or a second `pair_liquidity.rs`. Do **not** commit LocalTerra #558 tmp seed scripts (`scripts/tmp-558-*`).

## Merged MRs (tip `8af5563c+`)

| MR | Issue | Skill |
|----|-------|-------|
| !437 | #659 Swap seam | [`AGENTS_FRONTEND_SWAP_DIRECTION_SEAM.md`](./AGENTS_FRONTEND_SWAP_DIRECTION_SEAM.md) |
| !438 | #663 Footer Homepage/Bridge | [`AGENTS_FRONTEND_PRODUCT_LINKS.md`](./AGENTS_FRONTEND_PRODUCT_LINKS.md) |
| !439 | #658 Legal wallet hint | [`AGENTS_FRONTEND_CLICKWRAP.md`](./AGENTS_FRONTEND_CLICKWRAP.md) |
| !440 | #667 Protocol Δ% grouping | [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) |
| !441 | #669 Create Token desktop width | [`AGENTS_FRONTEND_CREATE_TOKEN_LAYOUT.md`](./AGENTS_FRONTEND_CREATE_TOKEN_LAYOUT.md) |
| !442 | #672 Connect Wallet Close | [`AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md`](./AGENTS_FRONTEND_WALLET_CONNECT_MODAL.md) |
| !443 | #661 Provide labels + wrap default | [`AGENTS_FRONTEND_POOL_PROVIDE_LABELS.md`](./AGENTS_FRONTEND_POOL_PROVIDE_LABELS.md) |
| !444 | #664 Trade/Charts v2 LP USD | [`AGENTS_FRONTEND_TRADE_IDENTITY_LP.md`](./AGENTS_FRONTEND_TRADE_IDENTITY_LP.md) |
| !446 | #668 Protocol Hourly/Daily/Monthly | [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) |
| !447 | #662 Pool Created | [`AGENTS_FRONTEND_POOL_CREATED.md`](./AGENTS_FRONTEND_POOL_CREATED.md) |
| !449 | #657 Trader global leaderboard | [`AGENTS_FRONTEND_TRADER_LEADERBOARD.md`](./AGENTS_FRONTEND_TRADER_LEADERBOARD.md) |
| !450 | #670 Migrate unlock copy | [`AGENTS_FRONTEND_TOKEN_MIGRATE.md`](./AGENTS_FRONTEND_TOKEN_MIGRATE.md) |
| !451 | #671 Wallet dropdown align | [`AGENTS_FRONTEND_WALLET_CHIP.md`](./AGENTS_FRONTEND_WALLET_CHIP.md) |
| !452 | #656 Trader 4/6 + blockie | [`AGENTS_FRONTEND_TRADER_IDENTITY.md`](./AGENTS_FRONTEND_TRADER_IDENTITY.md) |
| !454 | #665 Trader Share | [`AGENTS_FRONTEND_SHARE_LINK.md`](./AGENTS_FRONTEND_SHARE_LINK.md) |
| !455 | #655 Pool LP USD column | [`AGENTS_INDEXER_PAIR_LIQUIDITY_USD.md`](./AGENTS_INDEXER_PAIR_LIQUIDITY_USD.md) |
| !457 | #660 Pool Manage four tabs | [`AGENTS_FRONTEND_POOL_MANAGE_IA.md`](./AGENTS_FRONTEND_POOL_MANAGE_IA.md) |
| !458 | #666 Charts pair-scoped stats + board | [`AGENTS_FRONTEND_CHARTS_PAIR_SCOPED.md`](./AGENTS_FRONTEND_CHARTS_PAIR_SCOPED.md) |

Conflict notes already on `main`: catalog/docs unions; #660 four-tab IA + #661 labels; Charts uses shared `TraderLeaderboard` with `pairAddress` (no census strip). `/protocol` keeps the global census.

## Invariants (M673-1–M673-8)

| ID | Rule |
|----|------|
| **M673-1** | Local regression is `make verify-issue-673`, which runs children **655–672**. A child FAIL fails the stack. Live Coolify leftover probes SKIP unless `indexer.dex.cl8y.com` / `dex.cl8y.com` answers (FAIL when `VERIFY673_REQUIRE_LIVE=1` or `VERIFY673_IID=673`). Leftover Playwright SKIP unless LocalTerra is up (FAIL when `VERIFY673_REQUIRE_CHAIN=1`). |
| **M673-2** | Coolify Postgres has indexer migrations `20260826150000_pair_liquidity_usd.sql` (#655) and `20260826180000_protocol_volume_hourly_monthly.sql` (#668), then the indexer is redeployed. Live `GET /api/v1/pairs` always includes `created_at` (#662). `liquidity_usd` is present **when stamped** (unpriced gems omit it). `sort=liquidity_usd` is a valid allowlist key (unknown sort **400** still names it). |
| **M673-3** | Live `GET /api/v1/traders/leaderboard?pair={addr}`: unknown pair **404**; `sort=best_trade_pnl` with `pair` **400**. Unscoped leaderboard is unchanged (Best Trade stays on `/trader`). Charts pair board hides Best Trade (**CS-9**). |
| **M673-4** | Live volume grains are `GET /api/v1/protocol/volume/daily?grain=hourly\|daily\|monthly` + capped `limit` (**P668-5**). There is **no** `/protocol/volume/hourly` path. `from`/`to` → **400**. |
| **M673-5** | Coolify **frontend rebuild** from current `main` (`8af5563c+`). `/charts` has no `charts-overview-*`; 24h Stats below Find pair; pair leaderboard. `/protocol` still has global census + Hourly/Daily/Monthly. `/pool` has four Manage tabs, Provide name/symbol + wrap default on, Created age, v2 LP USD. Swap seam, footer Homepage/Bridge, Connect Wallet Close, Create Token desktop width, migrate Unlock copy stay on the bundle. |
| **M673-6** | Do **not** restore Charts DEX-census tiles or a second `pair_liquidity.rs` (the stamp lives in `pair_liquidity_usd.rs`). Do **not** commit `scripts/tmp-558-*`. Do **not** wait on GitLab CI. Do **not** reopen #655–#672 for ops/QA. File a new ticket if a merged invariant is wrong. Do not turn hybrid off (#596). |
| **M673-7** | Optional LocalTerra leftover Playwright is `e2e-smoke` at **5 workers**: pool Manage (#660), footer (#663), Charts smoke, trader-page. `e2e-tx` stays **1 worker**. Child verifies already include 660/663/657/659/665 smoke — leftover e2e is the stacked Charts smoke plus a re-run when `VERIFY673_LEFTOVER_E2E=1` or children are skipped. Do **not** export a dedicated `PLAYWRIGHT_WEB_PORT` (other than **3173**) into child verifies — indexer CORS includes `:3173`; a leaked port hides `/pool` (`getPairs` CORS → no `pool-pairs-table`). |
| **M673-8** | This playbook + **Q15** + child skills stay crosslinked. GitLab CI quota is not a substitute for local verify. |

## Coolify leftovers (operator)

Apply migrations on Coolify Postgres, redeploy indexer, rebuild frontend from `8af5563c+`:

```
# indexer
20260826150000_pair_liquidity_usd.sql
20260826180000_protocol_volume_hourly_monthly.sql
```

Then probe:

```bash
curl -sS "https://indexer.dex.cl8y.com/api/v1/pairs?sort=liquidity_usd&limit=1"
curl -sS -o /dev/null -w "%{http_code}\n" \
  "https://indexer.dex.cl8y.com/api/v1/traders/leaderboard?pair=terra1notapair"
# expect 404
curl -sS -o /dev/null -w "%{http_code}\n" \
  "https://indexer.dex.cl8y.com/api/v1/traders/leaderboard?pair=terra1notapair&sort=best_trade_pnl"
# expect 400
curl -sS "https://indexer.dex.cl8y.com/api/v1/protocol/volume/daily?grain=hourly&limit=24"
curl -sS "https://indexer.dex.cl8y.com/api/v1/protocol/volume/daily?grain=daily&limit=14"
curl -sS "https://indexer.dex.cl8y.com/api/v1/protocol/volume/daily?grain=monthly&limit=6"
```

Default `GET /pairs?limit=1` may omit `liquidity_usd` when the first row is unpriced — that is **P655** (unpriced = no field, not `$0`). Use `sort=liquidity_usd` to see stamped rows.

`make verify-issue-673` records leftover probes as SKIP unless the hosts answer. Fail closed with `VERIFY673_REQUIRE_LIVE=1`.

## Do / don’t

- **Do** run `make verify-issue-673` from a git worktree after pulling `main`.
- **Do** `make setup-indexer-postgres` for indexer children (#655 / #666 / #668).
- **Do** `make setup-cloud-localterra` when leftover Playwright SKIP and **M673-7** is still open.
- **Do** reinstall `@plasticdigits/cl8y-clickwrap` from GitLab npm if a worktree `node_modules` symlink inherited a pass-through stub (TermsGate always children). `make verify-issue-673` bootstrap does this; `make verify-issue-658` fails closed on a stub.
- **Don’t** reopen #655–#672 unless a merged invariant is wrong.
- **Don’t** restore `charts-overview-*` or a second liquidity stamp module.
- **Don’t** commit `scripts/tmp-558-*`.
- **Don’t** treat GitLab CI quota as leftover evidence.

## Regression

```bash
make verify-issue-673
# docs + source + live probes (no 18 children):
VERIFY673_SKIP_CHILDREN=1 make verify-issue-673
# docs + children only:
VERIFY673_SKIP_LIVE=1 VERIFY673_SKIP_CHAIN=1 make verify-issue-673
# fail if Coolify leftovers cannot run:
VERIFY673_REQUIRE_LIVE=1 make verify-issue-673
# leftover Playwright (5 workers) even when children already ran smoke:
VERIFY673_LEFTOVER_E2E=1 make verify-issue-673
```
