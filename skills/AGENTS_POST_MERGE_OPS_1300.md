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
| **M1300-1** | Local regression is `make verify-issue-1300`, which runs children **1287, 1234, 1265, 1240, 1279, 1277, 1264, 1285, 1255, 1219, 1218, 1263**. A child FAIL fails the stack. Live Coolify leftover probes SKIP unless hosts answer (FAIL when `VERIFY1300_REQUIRE_LIVE=1` or `VERIFY1300_IID=1300`). Default leftover verify probes `make has-localterra` (down → SKIP; up → PASS the probe) and **does not** execute the four leftover walks even when the chain is up. **`VERIFY1300_REQUIRE_CHAIN=1` is fail-closed `make has-localterra` only** — no Playwright, does **not** execute the walks, do **not** copy #701 Playwright FAIL. `VERIFY1300_SKIP_CHAIN=1` skips the probe. Walks stay leftover-complete **operator evidence** (how-tos below). `has-localterra` down is **not** leftover-complete — keep #1300 open, same class as wasm still 1.16.0. Chain-probe SKIP/PASS is **not** item-5 evidence. Cloud Agent leftover-*ops* (not leftover implement / slice 2) may `make setup-cloud-localterra` when item 5 is still open; that is **not** implement-slice close. Slice 2 never executes the walks. Do **not** close on probe SKIP. Child Vitest is **not** the walk. Leftover stacked Playwright is SKIP until named (**M1300-7**) — `REQUIRE_CHAIN` is not that flag. Do **not** copy #1276 `EXPECT_SHA` leftover-complete onto this ticket. Do **not** invent leftover `DATABASE_URL`. Do **not** add sqlx `20260921130000` to this verify. |
| **M1300-2** | Coolify indexer migrate **`20260921120000_traders_rolling_volume_numeric_38_0` then `20260921120001_pair_volume_30d`**, then indexer redeploy. **#1300 attests those two versions only.** Same boot may also apply `20260921130000_traders_lifetime_heal_from_swaps`; that row is **[#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305)**, not a #1300 FAIL and not a reason to keep #1300 open. Do **not** add a third version to `make verify-issue-1300`. Operator attests `_sqlx_migrations` (`…000` then `…001` `success=true`, apply order) via Coolify DB / indexer `DATABASE_URL` — not `postgres-psql.sh`. After one rollup tick, live **`GET /api/v1/protocol/top-pairs`** is ≤5 factory-listed economic rows; `?limit=6` → **400** (**I1263-6**). Vite `/protocol` is frontend rebuild evidence, not sqlx/`pair_volume_30d` evidence. Trader `NUMERIC(38, 0)` leftover-complete is migrate applied + aggregator still running (no public metric). No `down.sql` for these versions. |
| **M1300-3** | Coolify **frontend rebuild** from `6d34da13+`. Production HTTP marker: **`protocol-top-pairs` / `Top pairs (30d)`** (hashed Vite chunk grep, same class as #701). Do **not** grep hub `cLUNC / USD` (that is **#1305 leftover-complete**). LocalTerra / manual (**keep issue item 5** as leftover-complete operator evidence; how-tos below): **#1218** Pay LUNC / Receive USTR **or** JADE/RUBY (E7 stand-in; wrap prefix + solver hops; do **not** require Route `cUSTC` or `USTR`); **#1255** Create Token 18-dec unlisted, type `1` (`10^18` raw), mint 1 human (`mint_control`) before Max; **#1219** LocalTerra (in-tree 1.17.0) dust ladder copy / Place disabled, no wallet popup — not `dex.cl8y.com` while fleet is 1.16.0; **#1263** LocalTerra `/protocol` **Top pairs (30d)** renders with indexer-from-this-tip-up, ≤5 rows (Coolify-only: `?limit=6` → 400). Child Vitest is **not** the walk. Close #1300 only after those four walk notes. `has-localterra` down is **not** leftover-complete — keep #1300 open, same class as wasm still 1.16.0. Cloud Agent leftover-*ops* may `make setup-cloud-localterra` when item 5 is still open; that is **not** implement-slice close. Chain-probe SKIP/PASS is not item-5 evidence. |
| **M1300-4** | columbus-5 pair wasm via [`scripts/upgrade-582-code-id-pin.sh`](../scripts/upgrade-582-code-id-pin.sh): `UPGRADE582_PAIR_VERSION` **1.17.0** (listed fleet was **1.16.0**). Factory is already **1.10.0 / 11629** → `UPGRADE582_SKIP_FACTORY_MIGRATE=1` (script still asserts factory ≥ 1.9.0). Script **`UpdateConfig { pair_code_id }`** so new `CreatePair` instantiates 1.17.0. Versions: [`AGENTS_CW20_CODE_ID_PIN.md`](./AGENTS_CW20_CODE_ID_PIN.md). Do **not** follow [`docs/runbooks/cw20-code-id-ops.md`](../docs/runbooks/cw20-code-id-ops.md) 1.15.0 **RAN 2026-08-21**. Leftover agents: `UPGRADE582_PROBE_ONLY=1` — do **not** `store` / `migrate` / 2-of-3 / invent a pair-only `terrad tx`. `verify-issue-1234` must not claim `verify-issue-582` unless it runs it. |
| **M1300-5** | **#1264** stays open. Envelope **2,710,000**. AC1 columbus-5 USTC→USTR `gas_used` unmeasured. **G1264-4** (USTC Max/gas LUNC-only) needs no code follow-up. **#1279** leftover-complete is ops-bot `QA_TEMPLATE.md` **1.5.1–1.5.11**; green make does not close it. **#1240** hub `cLUNC / USD` rewrite is already on main; Coolify hub wrap visual leftover is **#1305**, not #1300. |
| **M1300-6** | Do **not** reopen closed parents for ops/QA. Do **not** wait on GitLab CI. Do **not** close #1264. Do **not** close #1279 from this ticket. Do **not** treat `VERIFY1279_IID=1279` / `LEFTOVER_COMPLETE=1` as PASS. **#1302 landed** on **#1305** — do **not** revert it. Hub wrap Coolify visual is **#1305 leftover-complete**. Do **not** merge `origin/cac-design-issue-1302` (unpublished second 0008). Do **not** touch `cac-design-issue-1276` / `1277`. Do **not** flip Coolify auto-deploy. Do **not** file a founder card. Keywords on #1300 are not #297 authority or `DESIGN: APPROVE`. |
| **M1300-7** | Leftover stacked Playwright is **manual**; `VERIFY1300_LEFTOVER_E2E=1` **SKIP until named**. `REQUIRE_CHAIN` is not that flag. Child verifies already run their own specs (`wrap-swap.spec.ts` / #1218, ladder UI / #1219). `protocol-page.spec.ts` does **not** assert Top pairs. Do **not** invent five-worker leftover coverage. Do **not** leak a non-3173 `PLAYWRIGHT_WEB_PORT` into children (CORS). `e2e-tx` stays **1 worker**. |
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

## Leftover-complete LocalTerra walks (operator evidence)

`make verify-issue-1300` does **not** execute these walks. Default leftover verify probes `make has-localterra` (down → SKIP; up → PASS the probe). `VERIFY1300_REQUIRE_CHAIN=1` is fail-closed `make has-localterra` only. `SKIP_CHAIN=1` skips the probe. Record the four how-tos on #1300, **or keep #1300 open**. `has-localterra` down is **not** leftover-complete (same class as wasm still 1.16.0). Slice 2 leftover implement does **not** provision LocalTerra or record walks. Optional: Cloud Agent leftover-*ops* may `make setup-cloud-localterra` when item 5 is still open; that is **not** implement-slice close. Do **not** close on probe SKIP. Chain-probe SKIP/PASS is not item-5 evidence.

| Walk | How-to | Pass |
|------|--------|------|
| **#1218** | LocalTerra + `make dev`: Swap Pay LUNC / Receive **USTR or JADE/RUBY** (same stand-in as `frontend-dapp/e2e/wrap-swap.spec.ts` **E7**). `deploy-dex-local` does **not** seed USTR — do **not** require LocalTerra USTR. Allowed substitute: manual columbus-5 Swap LUNC → USTR on `dex.cl8y.com` (live already has mapped GET). | Pass is wrap prefix + solver hops, not BFS 2-hop (**H1218-8**). Pay LUNC wraps to **cLUNC**, then solver hops (typically cLUNC → UST1 → USTR on columbus-5; LocalTerra E7 stand-in is typically cLUNC → EMBER → JADE). `wrap-then-cUSTC` is H1218-8 shorthand. Do **not** require the Route row to contain `cUSTC` or `USTR`. Child `VERIFY_ISSUE_1218_CHAIN=1` is wrap-swap E7/E8, not this leftover walk. |
| **#1255** | Create Token **18-dec unlisted** factory CW20 (`initialBalances: []` in `communityTaxCreateForm.ts`). Swap pick as Pay; type `1`. For Max / **Q1255-6**: enable **Minting** (`mint_control`) at create, then mint **1 human unit** (manager Mint; not a settings invoice) so wallet raw is `10^18`. Do **not** use registry-pinned TCL8Y / CL8Y / USTR. | Typed `1`: execute/sim raw **`10^18`** (not `10^6`). After mint 1 human: Max of raw `10^18` is human `1` (**Q1255-6**). Max of a zero balance is `0` at any scale and does **not** prove Q1255-6. |
| **#1219** | **LocalTerra** (in-tree pair **cw2 1.17.0**) `/limits` or `/trade` Ladder with a rung below min remaining (10). A `dex.cl8y.com` UI pass while listed fleet is still **1.16.0** is **not** leftover item 3 (columbus-5 wasm / L24). | Copy **Minimum size is 10 units**; Place disabled; no wallet popup (**S1219-8**). |
| **#1263** | LocalTerra `/protocol` with **indexer from this tip up**. | **Top pairs (30d)** section **renders** (`protocol-top-pairs`); table ≤5 rows (empty OK **only** when the section is visible). If the indexer on that tip is down, the section **hides** (`isProtocolTopPairsUnavailable` on 404/501) — a missing section is **not** a frontend miss; record indexer-up. Weaker than Coolify HTTP — does **not** prove `?limit=6` → 400. That 400 is **Coolify-only** (`GET /api/v1/protocol/top-pairs?limit=6`; **I1263-6** / `protocol_top_pairs.rs`). `protocol-page.spec.ts` does not assert Top pairs. |

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
- LocalTerra / manual (issue item 5, leftover-*ops*): the four walks recorded per how-tos above. `has-localterra` down is **not** leftover-complete — keep #1300 open, same class as wasm still 1.16.0. Chain-probe SKIP/PASS is not item-5 evidence. Child Vitest is **not** the walk. Default leftover verify probes `has-localterra` (down → SKIP) and does not execute the walks. `VERIFY1300_REQUIRE_CHAIN=1` is fail-closed `has-localterra` only — does not execute the walks, not invented five-worker e2e. Slice 2 leftover implement does **not** provision LocalTerra or record walks. Optional: Cloud Agent leftover-*ops* may `make setup-cloud-localterra` when item 5 is still open; that is **not** implement-slice close. `VERIFY1300_LEFTOVER_E2E=1` stays SKIP until named (`REQUIRE_CHAIN` is not that flag).
- columbus-5 listed pair cw2 **1.17.0** (including factory `UpdateConfig { pair_code_id }`), **or** leftover comment records wasm still outstanding (then **#1300 stays open**).
- **#1264** and **#1279** stay open. Envelope **2,710,000**; AC1 unmeasured unless an operator hash is attached. `#1279` leftover-complete is ops-bot 1.5.
- **#1302 landed** on **#1305** (`92c84406`). This leftover **must not revert it**. Do **not** merge `origin/cac-design-issue-1302`.

Do **not** close #1300 on green child `make verify-issue-*` alone.

## Do / don’t

- **Do** run `make verify-issue-1300` from a git worktree after pulling `main` (once implement lands the script).
- **Do** run leftover-*ops* / Cloud Agent QA walks per how-tos when issue item 5 is still open. Optional `make setup-cloud-localterra` if the chain is down; that is **not** leftover-implement close. Do **not** close on probe SKIP.
- **Do** link `frontend-dapp/node_modules` from the primary checkout in a git worktree. Do **not** `npm install` over a worktree symlink (#1299).
- **Don’t** reopen closed parents unless a merged invariant is wrong.
- **Don’t** close #1264 or #1279 from this ticket.
- **Don’t** spawn a solver (ADR 0007 Stay).
- **Don’t** treat GitLab CI quota as leftover evidence.
- **Don’t** flip Coolify auto-deploy or edit `autonomy.rs` / HMAC.
- **Don’t** treat keywords on #1300 as #297 authority or `DESIGN: APPROVE`.
- **Don’t** treat August 1.15.0 “already RAN” as leftover-complete for pair 1.17.0.
- **Don’t** invent leftover Playwright five-worker coverage (`VERIFY1300_LEFTOVER_E2E=1` SKIP until named). `VERIFY1300_REQUIRE_CHAIN=1` is fail-closed `has-localterra` only — not that e2e flag and not “run the four walks”. Do **not** invent named make commands for the walks. Leftover implement (slice 2) must **not** provision LocalTerra or record walks.
- **Don’t** treat chain-probe SKIP/PASS as leftover-complete for issue item 5 (`has-localterra` down keeps #1300 open).
- **Don’t** require LocalTerra USTR for #1218, Route `cUSTC`, Create Token Max of a zero balance as **Q1255-6**, registry-pinned TCL8Y/CL8Y/USTR for #1255, or a `dex.cl8y.com` #1219 UI pass while listed fleet is 1.16.0.
- **Don’t** revert merged PR #1302 or grep hub `cLUNC / USD` as this leftover’s frontend marker (#1305).
- **Don’t** FAIL or keep #1300 open because `20260921130000` is present or missing (#1305).

## Regression

`VERIFY1300_SKIP_CHAIN=1` skips the `has-localterra` probe (make never executes the walks). `VERIFY1300_REQUIRE_CHAIN=1` fail-closes that probe. Probe SKIP/PASS is not leftover-complete for issue item 5. `VERIFY1300_LEFTOVER_E2E=1` stays SKIP until named.

```bash
make verify-issue-1300
VERIFY1300_SKIP_CHILDREN=1 make verify-issue-1300
VERIFY1300_SKIP_LIVE=1 VERIFY1300_SKIP_CHAIN=1 make verify-issue-1300
VERIFY1300_REQUIRE_LIVE=1 make verify-issue-1300
VERIFY1300_REQUIRE_CHAIN=1 make verify-issue-1300
VERIFY1300_LEFTOVER_E2E=1 make verify-issue-1300
# leftover-*ops* / Cloud Agent QA — item 5 still open and chain down:
# optional provision, then record the four walks on #1300
# (make / leftover implement never executes the walks; probe SKIP is not leftover-complete)
make setup-cloud-localterra
```
