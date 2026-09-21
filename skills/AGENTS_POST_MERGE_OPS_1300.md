# Agent playbook: post-merge PRs 1287–1298 leftover verify (Forgejo #1300)

Audience: third-party agents verifying Coolify indexer migrate + frontend rebuild + columbus-5 pair wasm + LocalTerra/manual after [PRs 1287–1298](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls) landed on `main` (`6d34da13`, includes hotfixes #1299 / #1301). PRs **1302–1304** later landed (`729b097f` / `92c84406`); those leftovers are sister [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305). Child `make verify-issue-*` targets already existed on the merge commits.

**Issue:** [Forgejo **#1300**](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300)
**Parents (closed unless a merged invariant is wrong):** #1286/#1287, #1234, #1265, #1240, #1277, #1285, #1255, #1219, #1218, #1263. **Stay open:** [#1264](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1264) (AC1 unmeasured), [#1279](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1279) (ops-bot 1.5).
**Invariants:** [`docs/qa-invariants.md`](../docs/qa-invariants.md) **Q21** (**M1300-1–M1300-8**)
**Design:** [`docs/adr/0008-post-merge-leftover-1287-1298.md`](../docs/adr/0008-post-merge-leftover-1287-1298.md)
**Verify:** `make verify-issue-1300` (implement slice; this playbook is the contract)

Indexer **auto-deploy checkbox** leftover stays on [#1276](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1276) / [ADR 0006](../docs/adr/0006-indexer-health-git-sha.md) — [agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297). Do **not** treat `origin/cac-design-issue-1276` or `origin/cac-design-issue-1277` as this stack. **#1302 landed** (`92c84406`) on [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) — do **not** revert it. Hub wrap Coolify visual (`cLUNC / USD` vs CEX `LUNC`) is **#1305 leftover-complete**, not #1300. Do **not** merge unpublished `docs/adr/0008-verify-issue-1290-hub-wrap-labels.md` from `origin/cac-design-issue-1302` (second 0008). Do **not** reopen closed parents for ops/QA. Do **not** wait on GitLab CI quota. Do **not** file a founder card. Keywords on #1300 are not architecture approval.

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
| **M1300-1** | Local regression is `make verify-issue-1300`, which runs children **1287, 1234, 1265, 1240, 1279, 1277, 1264, 1285, 1255, 1219, 1218, 1263**. A child FAIL fails the stack. Live Coolify leftover probes SKIP unless hosts answer (FAIL when `VERIFY1300_REQUIRE_LIVE=1` or `VERIFY1300_IID=1300`). LocalTerra leftover walks SKIP unless the chain is up (FAIL when `VERIFY1300_REQUIRE_CHAIN=1`). **`VERIFY1300_REQUIRE_CHAIN=1` runs `make has-localterra` plus the four named walks** (#1218 LUNC→USTR Route wrap-then-cUSTC; #1255 18-dec unlisted factory CW20; #1219 dust ladder no wallet popup; #1263 `/protocol` ≤5 economic) — child Vitest is **not** the walk. Leftover stacked Playwright is SKIP until named (**M1300-7**) — `REQUIRE_CHAIN` is not an invitation to invent five-worker e2e. Do **not** copy #1276 `EXPECT_SHA` leftover-complete onto this ticket. Do **not** invent leftover `DATABASE_URL`. Do **not** add sqlx `20260921130000` to this verify. |
| **M1300-2** | Coolify indexer migrate **`20260921120000_traders_rolling_volume_numeric_38_0` then `20260921120001_pair_volume_30d`**, then indexer redeploy. **#1300 attests those two versions only.** Same boot may also apply `20260921130000_traders_lifetime_heal_from_swaps`; that row is **[#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305)**, not a #1300 FAIL and not a reason to keep #1300 open. Do **not** add a third version to `make verify-issue-1300`. Operator attests `_sqlx_migrations` (`…000` then `…001` `success=true`, apply order) via Coolify DB / indexer `DATABASE_URL` — not `postgres-psql.sh`. After one rollup tick, live **`GET /api/v1/protocol/top-pairs`** is ≤5 factory-listed economic rows; `?limit=6` → **400** (**I1263-6**). Vite `/protocol` is frontend rebuild evidence, not sqlx/`pair_volume_30d` evidence. Trader `NUMERIC(38, 0)` leftover-complete is migrate applied + aggregator still running (no public metric). No `down.sql` for these versions. |
| **M1300-3** | Coolify **frontend rebuild** from `6d34da13+`. Production HTTP marker: **`protocol-top-pairs` / `Top pairs (30d)`** (hashed Vite chunk grep, same class as #701). Do **not** grep hub `cLUNC / USD` (that is **#1305 leftover-complete**). LocalTerra / manual (**keep issue item 5**): **#1218** LUNC→USTR Route wrap-then-cUSTC; **#1255** 18-dec unlisted factory CW20; **#1219** dust ladder no wallet popup; **#1263** `/protocol` ≤5 economic rows. Child Vitest is **not** the walk. Close #1300 only after those four **or** explicit SKIP (`make has-localterra` down). |
| **M1300-4** | columbus-5 pair wasm via [`scripts/upgrade-582-code-id-pin.sh`](../scripts/upgrade-582-code-id-pin.sh): `UPGRADE582_PAIR_VERSION` **1.17.0** (listed fleet was **1.16.0**). Factory is already **1.10.0 / 11629** → `UPGRADE582_SKIP_FACTORY_MIGRATE=1` (script still asserts factory ≥ 1.9.0). Script **`UpdateConfig { pair_code_id }`** so new `CreatePair` instantiates 1.17.0. Versions: [`AGENTS_CW20_CODE_ID_PIN.md`](./AGENTS_CW20_CODE_ID_PIN.md). Do **not** follow [`docs/runbooks/cw20-code-id-ops.md`](../docs/runbooks/cw20-code-id-ops.md) 1.15.0 **RAN 2026-08-21**. Leftover agents: `UPGRADE582_PROBE_ONLY=1` — do **not** `store` / `migrate` / 2-of-3 / invent a pair-only `terrad tx`. `verify-issue-1234` must not claim `verify-issue-582` unless it runs it. |
| **M1300-5** | **#1264** stays open. Envelope **2,710,000**. AC1 columbus-5 USTC→USTR `gas_used` unmeasured. **G1264-4** (USTC Max/gas LUNC-only) needs no code follow-up. **#1279** leftover-complete is ops-bot `QA_TEMPLATE.md` **1.5.1–1.5.11**; green make does not close it. **#1240** hub `cLUNC / USD` rewrite is already on main; Coolify hub wrap visual leftover is **#1305**, not #1300. |
| **M1300-6** | Do **not** reopen closed parents for ops/QA. Do **not** wait on GitLab CI. Do **not** close #1264. Do **not** close #1279 from this ticket. Do **not** treat `VERIFY1279_IID=1279` / `LEFTOVER_COMPLETE=1` as PASS. **#1302 landed** on **#1305** — do **not** revert it. Hub wrap Coolify visual is **#1305 leftover-complete**. Do **not** merge `origin/cac-design-issue-1302` (unpublished second 0008). Do **not** touch `cac-design-issue-1276` / `1277`. Do **not** flip Coolify auto-deploy. Do **not** file a founder card. Keywords on #1300 are not #297 authority or `DESIGN: APPROVE`. |
| **M1300-7** | Leftover stacked Playwright is **manual**; `VERIFY1300_LEFTOVER_E2E=1` **SKIP until named**. Child verifies already run their own specs (`wrap-swap.spec.ts` / #1218, ladder UI / #1219). `protocol-page.spec.ts` does **not** assert Top pairs. Do **not** invent five-worker leftover coverage. Do **not** leak a non-3173 `PLAYWRIGHT_WEB_PORT` into children (CORS). `e2e-tx` stays **1 worker**. |
| **M1300-8** | This playbook + **Q21** + [ADR 0008](../docs/adr/0008-post-merge-leftover-1287-1298.md) + child skills stay crosslinked. GitLab CI quota is not a substitute for local verify. |

## Coolify leftovers (operator)

1. Indexer: apply `20260921120000` then `20260921120001` (sqlx on boot). **#1300 attests those two only.** Operator attests `_sqlx_migrations` via Coolify DB / indexer `DATABASE_URL` (`…000` then `…001` `success=true`, apply order). A `20260921130000` row is **#1305**, not a #1300 FAIL. Do **not** use `scripts/lib/postgres-psql.sh` against prod. Wait ~5 min for `refresh_pair_volumes_30d`. Then:

```bash
curl -sS "https://indexer.dex.cl8y.com/api/v1/protocol/top-pairs"
# items length ≤ 5; gems (COLUMBUS5_GEM_ADDRESSES) absent
curl -sS -o /dev/null -w "%{http_code}\n" \
  "https://indexer.dex.cl8y.com/api/v1/protocol/top-pairs?limit=6"
# expect 400 (I1263-6)
```

   Vite `/protocol` is **frontend** rebuild evidence, not sqlx/`pair_volume_30d` evidence. Dual-app skew: indexer migrated / Vite stale vs the reverse is expected until both rebuilds land. Named frontend HTTP marker: hashed Vite chunk contains **`protocol-top-pairs`** and **`Top pairs (30d)`** (same class as #701). Do **not** grep hub `cLUNC / USD` (that is **#1305 leftover-complete**).
2. Frontend: rebuild from current `main` (`6d34da13+`). Confirm **`protocol-top-pairs` / `Top pairs (30d)`**.
3. Do **not** infer the indexer auto-deploy checkbox from HTTP ([ADR 0006](../docs/adr/0006-indexer-health-git-sha.md)).

`make verify-issue-1300` live is **HTTP only**: indexer **`GET /api/v1/protocol/top-pairs`** plus frontend **`protocol-top-pairs` / `Top pairs (30d)`**. Do **not** grep hub `cLUNC / USD`. Do **not** copy #1276 `EXPECT_SHA`. Do **not** invent leftover `DATABASE_URL`. Leftover probes SKIP unless hosts answer. Fail closed with `VERIFY1300_REQUIRE_LIVE=1`.

Trader `NUMERIC(38, 0)` has no public metric; leftover-complete for `#1277` is migrate applied + aggregator still running.

## columbus-5 leftovers (operator)

Pair store+migrate to cw2 **1.17.0** via [`scripts/upgrade-582-code-id-pin.sh`](../scripts/upgrade-582-code-id-pin.sh):

```bash
# leftover agents — read-only (no store / migrate / 2-of-3):
UPGRADE582_PROBE_ONLY=1 ./scripts/upgrade-582-code-id-pin.sh

# operator (DEX 2-of-3). Factory already 1.10.0 / 11629:
UPGRADE582_SKIP_FACTORY_MIGRATE=1 ./scripts/upgrade-582-code-id-pin.sh
# script still asserts factory ≥ 1.9.0, then UpdateConfig { pair_code_id }
# so new CreatePair instantiates UPGRADE582_PAIR_VERSION 1.17.0
```

Versions: [`AGENTS_CW20_CODE_ID_PIN.md`](./AGENTS_CW20_CODE_ID_PIN.md) (pair **1.17.0**, fleet was **1.16.0**). Do **not** follow [`docs/runbooks/cw20-code-id-ops.md`](../docs/runbooks/cw20-code-id-ops.md) Launch checklist (factory 1.9.0 + pair 1.15.0 **RAN 2026-08-21**). Pair-first migrate still freezes gated writes; leftover must **not** invent a pair-only `terrad tx`. Not a founder card. Not leftover-agent `terrad tx`.

`#1264` AC1: keep 2.71M until a measured `gas_used` hash. Optional `VERIFY1264_COLUMBUS_TX`.

## Leftover-complete

Close **#1300** only when all hold (same as [ADR 0008](../docs/adr/0008-post-merge-leftover-1287-1298.md) Integration):

- Coolify: **#1300 attests `20260921120000` then `20260921120001` only** (`success=true`, apply order) — operator Coolify DB / indexer `DATABASE_URL`. Same boot may also apply `20260921130000`; that row is **#1305**, not a #1300 FAIL and not a reason to keep #1300 open. Do **not** add a third version to `make verify-issue-1300`.
- Live `GET /api/v1/protocol/top-pairs` items length ≤5 economic; gems (`COLUMBUS5_GEM_ADDRESSES`) absent; `?limit=6` → **400**.
- Coolify frontend serves `6d34da13+` with HTTP marker **`protocol-top-pairs` / `Top pairs (30d)`**. Do **not** grep hub `cLUNC / USD` (that is **#1305 leftover-complete**).
- LocalTerra / manual (issue item 5): the four walks (#1218 LUNC→USTR Route wrap-then-cUSTC; #1255 18-dec unlisted factory CW20; #1219 dust ladder no wallet popup; #1263 `/protocol` ≤5 economic) recorded, **or** explicit SKIP because `make has-localterra` is down. Child Vitest is **not** the walk. `VERIFY1300_REQUIRE_CHAIN=1` runs `has-localterra` + those four (FAIL if chain down) — not invented five-worker e2e. `VERIFY1300_LEFTOVER_E2E=1` stays SKIP until named.
- columbus-5 listed pair cw2 **1.17.0** (including factory `UpdateConfig { pair_code_id }`), **or** leftover comment records wasm still outstanding (then **#1300 stays open**).
- **#1264** and **#1279** stay open. Envelope **2,710,000**; AC1 unmeasured unless an operator hash is attached. `#1279` leftover-complete is ops-bot 1.5.
- **#1302 landed** on **#1305** (`92c84406`). This leftover **must not revert it**. Do **not** merge `origin/cac-design-issue-1302`.

Do **not** close #1300 on green child `make verify-issue-*` alone.

## Do / don’t

- **Do** run `make verify-issue-1300` from a git worktree after pulling `main` (once implement lands the script).
- **Do** link `frontend-dapp/node_modules` from the primary checkout in a git worktree. Do **not** `npm install` over a worktree symlink (#1299).
- **Don’t** reopen closed parents unless a merged invariant is wrong.
- **Don’t** close #1264 or #1279 from this ticket.
- **Don’t** spawn a solver (ADR 0007 Stay).
- **Don’t** treat GitLab CI quota as leftover evidence.
- **Don’t** flip Coolify auto-deploy or edit `autonomy.rs` / HMAC.
- **Don’t** treat keywords on #1300 as #297 authority or `DESIGN: APPROVE`.
- **Don’t** treat August 1.15.0 “already RAN” as leftover-complete for pair 1.17.0.
- **Don’t** invent leftover Playwright five-worker coverage (`VERIFY1300_LEFTOVER_E2E=1` SKIP until named). `VERIFY1300_REQUIRE_CHAIN=1` is `has-localterra` + the four named walks, not that e2e flag.
- **Don’t** revert merged PR #1302 or grep hub `cLUNC / USD` as this leftover’s frontend marker (#1305).
- **Don’t** FAIL or keep #1300 open because `20260921130000` is present or missing (#1305).

## Regression

```bash
make verify-issue-1300
VERIFY1300_SKIP_CHILDREN=1 make verify-issue-1300
VERIFY1300_SKIP_LIVE=1 VERIFY1300_SKIP_CHAIN=1 make verify-issue-1300
VERIFY1300_REQUIRE_LIVE=1 make verify-issue-1300
VERIFY1300_REQUIRE_CHAIN=1 make verify-issue-1300
VERIFY1300_LEFTOVER_E2E=1 make verify-issue-1300
```
