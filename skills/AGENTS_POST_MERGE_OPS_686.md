# Agent playbook: post-merge !459–!468 leftover verify (GitLab #686)

Audience: third-party agents verifying the integrated tip after [!459](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/merge_requests/459)–[!468](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/merge_requests/468) landed on `main` (no CI wait, no automerge). Child `make verify-issue-{674,675,676,677,678,679,680,683}` already existed on the merge commits. Tip also includes the Coolify `tsc` invert-legs fix (`f67f2198+`).

**Issue:** [GitLab **#686**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/686)  
**Parents (closed unless a merged invariant is wrong):** [#674](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/674)–[#680](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/680), [#683](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/683).  
**Invariants:** [`docs/qa-invariants.md`](../docs/qa-invariants.md) **Q16** (**M686-1–M686-8**)  
**Verify:** `make verify-issue-686`

Do **not** reopen the child issues unless a merged invariant is wrong. Do **not** wait on GitLab CI quota. Do **not** turn hybrid off (#596). Do **not** rewrite non-null `fee_usd` stamps or expand GET `/hub-prices` beyond the four hub cells. Do **not** touch [#684](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/684) (GeckoTerminal `/gt/events`) in this leftover. #681 is a duplicate of #679; #682 is a duplicate of #683.

## Merged MRs (tip `36d64528+` / current `main`)

| MR | Issue | Skill |
|----|-------|-------|
| !459 | #674 Portfolio hide test gems | [`AGENTS_FRONTEND_PORTFOLIO_TEST_PAIRS.md`](./AGENTS_FRONTEND_PORTFOLIO_TEST_PAIRS.md) |
| !461 | #675 Unrealized P&L / hub mark | [`AGENTS_FRONTEND_PORTFOLIO_UNREALIZED.md`](./AGENTS_FRONTEND_PORTFOLIO_UNREALIZED.md) |
| !462 | #676 18-dec trader positions | [`AGENTS_INDEXER_TRADER_POSITIONS_DECIMALS.md`](./AGENTS_INDEXER_TRADER_POSITIONS_DECIMALS.md) |
| !463 | #677 Protocol 24h liquidity + dense x-axis | [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) |
| !464 | #679 Mixed hybrid router gas ~192 LUNC | [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md) |
| !465 | #680 Charts UST1/USD hero + `?price=` | [`AGENTS_FRONTEND_CHARTS_UST1_HERO.md`](./AGENTS_FRONTEND_CHARTS_UST1_HERO.md) |
| !467 | #678 Swap/Trade unfunded-pay acquire guidance | [`AGENTS_FRONTEND_SWAP_ACQUIRE_GUIDANCE.md`](./AGENTS_FRONTEND_SWAP_ACQUIRE_GUIDANCE.md) |
| !468 | #683 Economic fee USD (CL8Y + listed) | [`AGENTS_INDEXER_ECONOMIC_FEE_USD.md`](./AGENTS_INDEXER_ECONOMIC_FEE_USD.md) |

Conflict notes already on `main`: #674 test-gem hide + #675 mark/unrealized on the same positions table; #676 `NUMERIC(78,18)` notes with #675 dApp MTM; Makefile verify-target unions.

## Invariants (M686-1–M686-8)

| ID | Rule |
|----|------|
| **M686-1** | Local regression is `make verify-issue-686`, which runs children **674, 675, 676, 677, 678, 679, 680, 683**. A child FAIL fails the stack. Live Coolify leftover probes SKIP unless `indexer.dex.cl8y.com` / `dex.cl8y.com` answers (FAIL when `VERIFY686_REQUIRE_LIVE=1` or `VERIFY686_IID=686`). Leftover Playwright SKIP unless LocalTerra is up (FAIL when `VERIFY686_REQUIRE_CHAIN=1`). |
| **M686-2** | Coolify Postgres has indexer migrations `20260827120000_trader_positions_numeric_78.sql` (#676) and `20260827140000_economic_token_marks.sql` (#683), then the indexer is redeployed. After migrate, a wallet that traded UST1/USTR and CL8Y-cb/cUSTC has per-pair `trade_count` on `GET /api/v1/traders/{addr}/positions` matching that pair’s `/trades`. If still diverged, `cl8y-dex-indexer rebuild-positions`. |
| **M686-3** | Coolify indexer env sets `HUB_CL8Y_ADDRESS` (columbus-5 official CL8Y CW20; LocalTerra uses deployed TCL8Y). After restart, prior NULL CL8Y `fee_usd` rows fill once; non-null stamps stay put (**EFee-5** / **C568-1**). GET `/hub-prices` stays four cells (`custc`\|`lunc`\|`ust1`\|`ustr`); extra ticker **400**. |
| **M686-4** | Live `/protocol` token mix shows **CL8Y-cb** human + `$` when a qualifying CL8Y↔hub pair exists. Liquidity tile is **24h-only** Δ% (no `protocol-stat-liquidity-30d`). UTC volume x-axis stays dense (**P668-9**). |
| **M686-5** | Coolify **frontend rebuild** from current `main` (`36d64528+`, including `f67f2198` tsc invert-legs). `/portfolio`: gems hidden by default; **Show test pairs** divider; Mark + Unrealized. `/charts` first load → UST1/cUSTC with USD of 1 UST1 (`?price=UST1`); `/trade` UST1/cUSTC first visit still other-side. Swap oversized/unfunded UST1 → **Quote only** / **Get UST1** → `/ust1`. Mixed hybrid 4-hop **Network fee (est.)** ~192 LUNC is the dApp source of truth (Station mobile/WC auto-gas residual stays documented). |
| **M686-6** | Do **not** reopen #674–#680 / #683 for ops/QA. File a new ticket if a merged invariant is wrong. Do **not** wait on GitLab CI. Do **not** turn hybrid off (#596) to “fix” Station auto-gas. Do **not** rewrite non-null `fee_usd` or expand GET `/hub-prices`. Do **not** touch #684 in this leftover. |
| **M686-7** | Optional LocalTerra leftover Playwright is `e2e-smoke` at **5 workers**: portfolio (#674), protocol-page (#677), Charts smoke (#680). `e2e-tx` stays **1 worker**. Child verifies already include 674/677 smoke when chain/Playwright is present — leftover e2e is the stacked re-run when `VERIFY686_LEFTOVER_E2E=1` or children are skipped. Do **not** export a dedicated `PLAYWRIGHT_WEB_PORT` (other than **3173**) into child verifies — indexer CORS includes `:3173`; a leaked port hides `/pool` (`getPairs` CORS). |
| **M686-8** | This playbook + **Q16** + child skills stay crosslinked. GitLab CI quota is not a substitute for local verify. |

## Coolify leftovers (operator)

Apply migrations on Coolify Postgres, redeploy indexer with `HUB_CL8Y_ADDRESS`, rebuild frontend from `36d64528+`:

```
# indexer
20260827120000_trader_positions_numeric_78.sql
20260827140000_economic_token_marks.sql
```

Then probe:

```bash
curl -sS "https://indexer.dex.cl8y.com/api/v1/hub-prices"
# four tickers only
curl -sS -o /dev/null -w "%{http_code}\n" \
  "https://indexer.dex.cl8y.com/api/v1/hub-prices/cl8y"
# expect 400
curl -sS "https://indexer.dex.cl8y.com/api/v1/protocol/fees?window=24h"
# by_token may include CL8Y-cb with amount_usd when a CL8Y↔hub pair exists
curl -sS "https://indexer.dex.cl8y.com/api/v1/traders/${VERIFY686_TRADER}/positions"
# after #676 migrate: no 1e+ scientific strings; trade_count matches /trades per pair
```

`make verify-issue-686` records leftover probes as SKIP unless the hosts answer. Fail closed with `VERIFY686_REQUIRE_LIVE=1`. Optional `VERIFY686_TRADER` overrides the default columbus-5 wallet used for the positions `trade_count` leftover.

## Do / don’t

- **Do** run `make verify-issue-686` from a git worktree after pulling `main`.
- **Do** `make setup-indexer-postgres` for indexer children (#676 / #683).
- **Do** link `packages/localnet-trading-swarm/node_modules` from the primary checkout in a git worktree (`make verify-issue-679` / leftover bootstrap). Do **not** `npm install` over a worktree `frontend-dapp/node_modules` symlink — npm replaces the link and drops swarm Vitest.
- **Do** `make setup-cloud-localterra` when leftover Playwright SKIP and **M686-7** / #674 / #679 chain is still open. Child #677 uses dedicated Vite `:30677` and frees a stale occupant before start (`reuseExistingServer` is false when `PLAYWRIGHT_WEB_PORT` is set).
- **Don’t** reopen #674–#680 / #683 unless a merged invariant is wrong.
- **Don’t** turn hybrid off or LCD-sim as the sole envelope (#679 residual).
- **Don’t** add a fifth hub-prices ticker or rewrite historical `fee_usd`.
- **Don’t** treat GitLab CI quota as leftover evidence.
- **Don’t** work #684 GeckoTerminal `/gt/events` on this ticket.

## Regression

```bash
make verify-issue-686
# docs + source + live probes (no 8 children):
VERIFY686_SKIP_CHILDREN=1 make verify-issue-686
# docs + children only:
VERIFY686_SKIP_LIVE=1 VERIFY686_SKIP_CHAIN=1 make verify-issue-686
# fail if Coolify leftovers cannot run:
VERIFY686_REQUIRE_LIVE=1 make verify-issue-686
# leftover Playwright (5 workers) even when children already ran smoke:
VERIFY686_LEFTOVER_E2E=1 make verify-issue-686
```
