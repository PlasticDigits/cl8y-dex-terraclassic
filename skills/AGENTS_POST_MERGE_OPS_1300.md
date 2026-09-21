# Agent playbook: post-merge PRs 1287–1298 leftover verify (Forgejo #1300)

Audience: third-party agents verifying Coolify indexer migrate + frontend rebuild + columbus-5 pair wasm + LocalTerra/manual after [PRs 1287–1298](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls) landed on `main` (`6d34da13`, includes hotfixes #1299 / #1301). Child `make verify-issue-*` targets already existed on the merge commits.

**Issue:** [Forgejo **#1300**](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300)
**Parents (closed unless a merged invariant is wrong):** #1286/#1287, #1234, #1265, #1240, #1277, #1285, #1255, #1219, #1218, #1263. **Stay open:** [#1264](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1264) (AC1 unmeasured), [#1279](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1279) (ops-bot 1.5).
**Invariants:** [`docs/qa-invariants.md`](../docs/qa-invariants.md) **Q21** (**M1300-1–M1300-8**)
**Design:** [`docs/adr/0008-post-merge-leftover-1287-1298.md`](../docs/adr/0008-post-merge-leftover-1287-1298.md)
**Verify:** `make verify-issue-1300` (implement slice; this playbook is the contract)

Indexer **auto-deploy checkbox** leftover stays on [#1276](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1276) / [ADR 0006](../docs/adr/0006-indexer-health-git-sha.md) — [agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297). Do **not** treat `origin/cac-design-issue-1276` or `origin/cac-design-issue-1277` as this stack. Do **not** merge open PR #1302. Do **not** reopen closed parents for ops/QA. Do **not** wait on GitLab CI quota. Do **not** file a founder card. Keywords on #1300 are not architecture approval.

## Merged PR(s)

| PR | Issue | Skill |
|----|-------|-------|
| 1287 | #1286 / #1287 pre-push origin/main skip + unpublished range | [`AGENTS_GIT_COMMIT_HOOKS.md`](./AGENTS_GIT_COMMIT_HOOKS.md) |
| 1288 | #1234 F6 on UpdateLimitOrderPrice + CleanLimitBook | [`AGENTS_CW20_CODE_ID_PIN.md`](./AGENTS_CW20_CODE_ID_PIN.md) |
| 1289 | #1265 ADR 0007 Stay; no solver spawn | [`AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md`](./AGENTS_INDEXER_HYBRID_BEST_EXECUTION.md) |
| 1290 | #1240 hub cLUNC vs CEX LUNC | [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) **P1240** |
| 1291 | #1279 Lunc Dash scaffolding | [`AGENTS_OPS_LUNCDASH_VERIFY.md`](./AGENTS_OPS_LUNCDASH_VERIFY.md) |
| 1292 | #1277 traders rolling NUMERIC(38, 0) | [`AGENTS_INDEXER_TRADER_ROLLING_NUMERIC.md`](./AGENTS_INDEXER_TRADER_ROLLING_NUMERIC.md) |
| 1293 | #1264 USTC wrap+2hop 2.71M | [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md) **G1264** |
| 1294 | #1285 TaxPreview send_msg | [`AGENTS_FRONTEND_EXTRA_DEBIT_SELL.md`](./AGENTS_FRONTEND_EXTRA_DEBIT_SELL.md) **S1285** |
| 1295 | #1255 unlisted CW20 LCD decimals | [`AGENTS_FRONTEND_SWAP_AMOUNT_SCALE.md`](./AGENTS_FRONTEND_SWAP_AMOUNT_SCALE.md) |
| 1296 | #1219 named min remaining L24 | [`AGENTS_LIMIT_ORDER_BATCH_LADDER.md`](./AGENTS_LIMIT_ORDER_BATCH_LADDER.md) |
| 1297 | #1218 wrap-enter GET /route/solve | [`AGENTS_FRONTEND_WRAP_ENTER_ROUTE_SOLVE.md`](./AGENTS_FRONTEND_WRAP_ENTER_ROUTE_SOLVE.md) |
| 1298 | #1263 Protocol top-5 30d | [`AGENTS_INDEXER_PROTOCOL_TOP_PAIRS.md`](./AGENTS_INDEXER_PROTOCOL_TOP_PAIRS.md) |
| 1299 | hotfix node_modules symlink | (no feature skill; link from primary checkout) |
| 1301 | hotfix commit-msg leftover FAIL | [`AGENTS_GIT_COMMIT_HOOKS.md`](./AGENTS_GIT_COMMIT_HOOKS.md) |

## Invariants (M1300-1–M1300-8)

| ID | Rule |
|----|------|
| **M1300-1** | Local regression is `make verify-issue-1300`, which runs children **1287, 1234, 1265, 1240, 1279, 1277, 1264, 1285, 1255, 1219, 1218, 1263**. A child FAIL fails the stack. Live Coolify leftover probes SKIP unless hosts answer (FAIL when `VERIFY1300_REQUIRE_LIVE=1` or `VERIFY1300_IID=1300`). Leftover Playwright SKIP unless LocalTerra is up (FAIL when `VERIFY1300_REQUIRE_CHAIN=1`). Do **not** copy #1276 `EXPECT_SHA` leftover-complete onto this ticket. |
| **M1300-2** | Coolify indexer migrate **`20260921120000_traders_rolling_volume_numeric_38_0` then `20260921120001_pair_volume_30d`**, then indexer redeploy. Confirm `_sqlx_migrations`. After one rollup tick, live `/protocol` Top pairs is ≤5 factory-listed economic rows (**P1263**). No `down.sql` for these versions. |
| **M1300-3** | Coolify **frontend rebuild** from `6d34da13+`. LocalTerra / manual: **#1218** LUNC→USTR Route wrap-then-cUSTC; **#1255** 18-dec unlisted factory CW20; **#1219** dust ladder no wallet popup; **#1263** `/protocol` ≤5 economic rows. |
| **M1300-4** | columbus-5 pair wasm store+migrate in-tree cw2 **1.17.0** (listed fleet was **1.16.0**) so **F6** #1234 and **L24** #1219 are on chain. Leftover agents do **not** `store`/`migrate`. `verify-issue-1234` must not claim `verify-issue-582` unless it runs it. |
| **M1300-5** | **#1264** stays open. Envelope **2,710,000**. AC1 columbus-5 USTC→USTR `gas_used` unmeasured. **G1264-4** (USTC Max/gas LUNC-only) needs no code follow-up. **#1279** leftover-complete is ops-bot `QA_TEMPLATE.md` **1.5.1–1.5.11**; green make does not close it. **#1240** hub `cLUNC / USD` rewrite is already on main. |
| **M1300-6** | Do **not** reopen closed parents for ops/QA. Do **not** wait on GitLab CI. Do **not** close #1264. Do **not** treat `VERIFY1279_IID=1279` / `LEFTOVER_COMPLETE=1` as PASS. Do **not** merge PR #1302. Do **not** touch `cac-design-issue-1276` / `1277`. Do **not** flip Coolify auto-deploy. Do **not** file a founder card. |
| **M1300-7** | Optional leftover Playwright at **5 workers** when `VERIFY1300_LEFTOVER_E2E=1` or chain is required. Do **not** leak a non-3173 `PLAYWRIGHT_WEB_PORT` into children (CORS). `e2e-tx` stays **1 worker**. |
| **M1300-8** | This playbook + **Q21** + [ADR 0008](../docs/adr/0008-post-merge-leftover-1287-1298.md) + child skills stay crosslinked. GitLab CI quota is not a substitute for local verify. |

## Coolify leftovers (operator)

1. Indexer: apply `20260921120000` then `20260921120001` (sqlx on boot). Confirm `_sqlx_migrations`. Wait ~5 min for `refresh_pair_volumes_30d`. Probe `/protocol` Top pairs.
2. Frontend: rebuild from current `main` (`6d34da13+`).
3. Do **not** infer the indexer auto-deploy checkbox from HTTP ([ADR 0006](../docs/adr/0006-indexer-health-git-sha.md)).

`make verify-issue-1300` records leftover probes as SKIP unless hosts answer. Fail closed with `VERIFY1300_REQUIRE_LIVE=1`.

## columbus-5 leftovers (operator)

Pair store+migrate to cw2 **1.17.0** via the existing F6 runbook ([`docs/runbooks/cw20-code-id-ops.md`](../docs/runbooks/cw20-code-id-ops.md)). Not a founder card. Not leftover-agent `terrad tx`.

`#1264` AC1: keep 2.71M until a measured `gas_used` hash. Optional `VERIFY1264_COLUMBUS_TX`.

## Do / don’t

- **Do** run `make verify-issue-1300` from a git worktree after pulling `main` (once implement lands the script).
- **Do** link `frontend-dapp/node_modules` from the primary checkout in a git worktree. Do **not** `npm install` over a worktree symlink (#1299).
- **Don’t** reopen closed parents unless a merged invariant is wrong.
- **Don’t** close #1264 or #1279 from this ticket.
- **Don’t** spawn a solver (ADR 0007 Stay).
- **Don’t** treat GitLab CI quota as leftover evidence.
- **Don’t** flip Coolify auto-deploy or edit `autonomy.rs` / HMAC.

## Regression

```bash
make verify-issue-1300
VERIFY1300_SKIP_CHILDREN=1 make verify-issue-1300
VERIFY1300_SKIP_LIVE=1 VERIFY1300_SKIP_CHAIN=1 make verify-issue-1300
VERIFY1300_REQUIRE_LIVE=1 make verify-issue-1300
VERIFY1300_LEFTOVER_E2E=1 make verify-issue-1300
```
