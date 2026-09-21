# Agent playbook: post-merge PRs 1287–1298 leftover verify (Forgejo #1300)

Audience: third-party agents verifying Coolify indexer migrate + frontend rebuild + columbus-5 pair wasm + LocalTerra/manual after [PRs 1287–1298](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls) landed on `main` (`6d34da13`, includes hotfixes #1299 / #1301). PRs **1302–1304** later landed (`729b097f` / `92c84406`); those leftovers are sister [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305). Child `make verify-issue-*` targets already existed on the merge commits.

**Issue:** [Forgejo **#1300**](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300)
**Parents (closed unless a merged invariant is wrong):** #1286/#1287, #1234, #1265, #1240, #1277, #1285, #1255, #1219, #1218, #1263, **#1264** (AC1 measured; do **not** reopen). **Stay open:** [#1279](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1279) (ops-bot 1.5). Tracker-close comments on #1300 do **not** waive the four walk notes. The tracker is already **closed**; leftover-*ops* **comments (or reopens)** it until those notes exist. Forgejo closed ≠ leftover-complete.
**Invariants:** [`docs/qa-invariants.md`](../docs/qa-invariants.md) **Q21** (**M1300-1–M1300-8**)
**Design:** [`docs/adr/0008-post-merge-leftover-1287-1298.md`](../docs/adr/0008-post-merge-leftover-1287-1298.md)
**Verify:** `make verify-issue-1300` (implement slice; this playbook is the contract)

Indexer **auto-deploy checkbox** leftover stays on [#1276](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1276) / [ADR 0006](../docs/adr/0006-indexer-health-git-sha.md) — [agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297). Do **not** treat `origin/cac-design-issue-1276` or `origin/cac-design-issue-1277` as this stack. **#1302 landed** (`92c84406`) on [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) — do **not** revert it. Hub wrap Coolify visual (`cLUNC / USD` vs CEX `LUNC`) is **#1305 leftover-complete**, not #1300. Do **not** merge `origin/cac-design-issue-1302` as-is (overlap on `architecture.md` / `qa-invariants.md` / README). That branch already publishes `docs/adr/0009-verify-issue-1290-hub-wrap-labels.md` / **Q22** and reserves 0008/Q21 for this ticket. A later **0009** follow-up MR that inserts without taking 0008/Q21 is allowed. Do **not** reopen closed parents for ops/QA. Do **not** wait on GitLab CI quota. Do **not** file a founder card. Keywords on #1300 are not architecture approval.

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
| **M1300-1** | Local regression is `make verify-issue-1300`, which runs children **1287, 1234, 1265, 1240, 1279, 1277, 1264, 1285, 1255, 1219, 1218, 1263**. A child FAIL fails the stack. Green `make verify-issue-1300` **never** means leftover-complete. Live Coolify leftover probes SKIP unless hosts answer (FAIL when `VERIFY1300_REQUIRE_LIVE=1` or `VERIFY1300_IID=1300`). Default leftover verify probes `make has-localterra` (down → SKIP; up → PASS the probe) and **does not** execute the four leftover walks even when the chain is up. **`VERIFY1300_REQUIRE_HAS_LOCALTERRA=1` is fail-closed `make has-localterra` only** — no Playwright, does **not** execute the walks. **Slice-2 FAIL:** defining `VERIFY1300_REQUIRE_CHAIN` as Playwright / `run_leftover_e2e`. **`VERIFY1300_SKIP_HAS_LOCALTERRA=1`** skips the probe (do **not** ship `SKIP_HAS_LOCALTERRA` — that name never skips). Tracker-close comments on #1300 do **not** waive the four walks. The tracker is already **closed**; leftover-*ops* **comments (or reopens)** it until those notes exist. Forgejo closed ≠ leftover-complete. Walks stay leftover-complete **leftover-*ops* evidence** (how-tos below). `has-localterra` down is **not** leftover-complete. Chain-probe SKIP/PASS is **not** item-5 evidence. Cloud Agent leftover-*ops* (not leftover implement / slice 2) may `make setup-cloud-localterra` when item 5 still lacks walk notes; that is **not** implement-slice close. Slice 2 never executes the walks. Do **not** close on probe SKIP. Child Vitest is **not** the walk. Leftover stacked Playwright is SKIP until named (**M1300-7**). Do **not** copy #1276 `EXPECT_SHA` leftover-complete onto this ticket. Do **not** invent leftover `DATABASE_URL`. Do **not** add sqlx `20260921130000` to this verify. |
| **M1300-2** | Coolify indexer migrate **`20260921120000_traders_rolling_volume_numeric_38_0` then `20260921120001_pair_volume_30d`**, then indexer redeploy. **#1300 attests those two versions only.** Same boot may also apply `20260921130000_traders_lifetime_heal_from_swaps`; that row is **[#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305)**, not a #1300 FAIL and not a reason to keep #1300 open. Do **not** add a third version to `make verify-issue-1300`. Operator attests `_sqlx_migrations` (`…000` then `…001` `success=true`, apply order) via Coolify DB / indexer `DATABASE_URL` — not `postgres-psql.sh`. After one rollup tick, live **`GET /api/v1/protocol/top-pairs`** is ≤5 factory-listed economic rows; `?limit=6` → **400** (**I1263-6**). Vite `/protocol` is frontend rebuild evidence, not sqlx/`pair_volume_30d` evidence. Trader `NUMERIC(38, 0)` leftover-complete is migrate applied + aggregator still running (no public metric). No `down.sql` for these versions. |
| **M1300-3** | Coolify **frontend rebuild** from `6d34da13+`. Production HTTP marker: **`protocol-top-pairs` / `Top pairs (30d)`** in hashed Vite **`chunk("ProtocolPage")`** (#673/#686; `App.tsx` `/protocol` is lazy `ProtocolPage`). Do **not** copy #701’s `PoolPage\|chartsOverviewStats\|pairCatalog` filter — that is a live false-FAIL. Do **not** grep hub `cLUNC / USD` (that is **#1305 leftover-complete**). LocalTerra / manual (**keep issue item 5** as leftover-complete leftover-*ops* evidence; how-tos below; tracker-close comments do **not** waive them; leftover-*ops* **comments (or reopens) the closed tracker** until the notes exist; Forgejo closed ≠ leftover-complete): **#1218** Pay LUNC / Receive USTR **or** JADE/RUBY (E7 stand-in; wrap prefix + solver hops; do **not** require Route `cUSTC` or `USTR`); **#1255 LocalTerra only** — Minting (`mint_control`) is a paid **50 UST1 SKU** (`PayWithAnyToken`); free create cannot mint; a `dex.cl8y.com` Create Token is spend + live CW20; set **Decimals** to **18**, enable **Minting** at create; mint cap empty (unlimited — **valid**) or human `≥1`; after mint **Create Pair** vs UST1/cUSTC/EMBER so the CW20 is a `getAllTokens` picker id; Swap **pick** as Pay (do **not** paste `terra1`); leftover pass is Max of minted raw **`1000000000000000000`** → human `1` (**Q1255-6**) — drop typed `1` execute/sim unless leftover-*ops* **seeds LP**; do not mix mint cap with the raw Mint field; **#1219** LocalTerra (in-tree 1.17.0) dust ladder copy / Place disabled, no wallet popup — **not columbus-5 wasm leftover** (issue leftover 3 is `#1264`); **#1263** LocalTerra `/protocol` **Top pairs (30d)** renders with indexer-from-this-tip-up, ≤5 rows (Coolify-only: `?limit=6` → 400). Child Vitest is **not** the walk. Leftover-complete still needs those four walk notes **and** operator Coolify + LCD cw2 1.17.0 probe. Green `make verify-issue-1300` never means close. `has-localterra` down is **not** leftover-complete. Cloud Agent leftover-*ops* may `make setup-cloud-localterra` when item 5 still lacks walk notes; that is **not** implement-slice close. Chain-probe SKIP/PASS is not item-5 evidence. |
| **M1300-4** | columbus-5 pair leftover-complete is LCD cw2 **1.17.0** via [`scripts/upgrade-582-code-id-pin.sh`](../scripts/upgrade-582-code-id-pin.sh) **`UPGRADE582_PROBE_ONLY=1`**. Leftover-ops recorded **1.17.0 / 11672**. A second `store` is a leftover defect. Store / 2-of-3 / `UpdateConfig { pair_code_id }` only if the probe is still **1.16.0** (`UPGRADE582_SKIP_FACTORY_MIGRATE=1`; factory already **1.10.0 / 11629**). Versions: [`AGENTS_CW20_CODE_ID_PIN.md`](./AGENTS_CW20_CODE_ID_PIN.md). Do **not** follow [`docs/runbooks/cw20-code-id-ops.md`](../docs/runbooks/cw20-code-id-ops.md) 1.15.0 **RAN 2026-08-21**. Leftover agents never `terrad tx`. `verify-issue-1234` must not claim `verify-issue-582` unless it runs it. |
| **M1300-5** | Do **not** reopen **#1264**. Envelope **2,710,000**. Do **not** raise `WRAP_ROUTER_COMBO_OVERHEAD_GAS`. Operator AC1: tx `53B06B653D78AF3683F51A065FC640074A3A2E76CAE343A97B3E0F0BD79BAC36`, `gas_wanted=2710000`, `gas_used=2630228`, class **not-A** / **not-B**, issue **closed**. Child `make verify-issue-1264` stays in the stack. Optional `VERIFY1264_COLUMBUS_TX` may pin that hash. **G1264-4** (USTC Max/gas LUNC-only) needs no code follow-up. **#1279** leftover-complete is ops-bot `QA_TEMPLATE.md` **1.5.1–1.5.11**; green make does not close it. **#1240** hub `cLUNC / USD` rewrite is already on main; Coolify hub wrap visual leftover is **#1305**, not #1300. |
| **M1300-6** | Do **not** reopen closed parents for ops/QA (including **#1264**). Do **not** wait on GitLab CI. Do **not** close #1279 from this ticket. Tracker-close comments on #1300 do **not** waive the four walks. The tracker is already **closed**; leftover-*ops* **comments (or reopens)** it until those notes exist. Forgejo closed ≠ leftover-complete. Do **not** treat `VERIFY1279_IID=1279` / `LEFTOVER_COMPLETE=1` as PASS. **#1302 landed** on **#1305** — do **not** revert it. Hub wrap Coolify visual is **#1305 leftover-complete**. Do **not** merge `origin/cac-design-issue-1302` as-is (overlap on `architecture.md` / `qa-invariants.md` / README). That branch already publishes `docs/adr/0009-verify-issue-1290-hub-wrap-labels.md` / **Q22**. A later **0009** follow-up MR that inserts without taking 0008/Q21 is allowed. Do **not** touch `cac-design-issue-1276` / `1277`. Do **not** flip Coolify auto-deploy. Do **not** file a founder card. Keywords on #1300 are not #297 authority or `DESIGN: APPROVE`. |
| **M1300-7** | Leftover stacked Playwright is **manual**; `VERIFY1300_LEFTOVER_E2E=1` **SKIP until named**. `VERIFY1300_REQUIRE_HAS_LOCALTERRA` is not that flag. Defining `VERIFY1300_REQUIRE_CHAIN` as Playwright / `run_leftover_e2e` is a leftover-implement FAIL. Child verifies already run their own specs (`wrap-swap.spec.ts` / #1218, ladder UI / #1219). `protocol-page.spec.ts` does **not** assert Top pairs. Do **not** invent five-worker leftover coverage. Do **not** leak a non-3173 `PLAYWRIGHT_WEB_PORT` into children (CORS). `e2e-tx` stays **1 worker**. |
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

   Vite `/protocol` is **frontend** rebuild evidence, not sqlx/`pair_volume_30d` evidence. Dual-app skew: indexer migrated / Vite stale vs the reverse is expected until both rebuilds land. Named frontend HTTP marker: hashed Vite **`chunk("ProtocolPage")`** (#673/#686; `App.tsx` lazy `ProtocolPage`) contains **`protocol-top-pairs`** and **`Top pairs (30d)`**. Do **not** copy #701 `PoolPage|chartsOverviewStats|pairCatalog`. Do **not** grep hub `cLUNC / USD` (that is **#1305 leftover-complete**).
2. Frontend: rebuild from current `main` (`6d34da13+`). Confirm **`protocol-top-pairs` / `Top pairs (30d)`** in `ProtocolPage-*.js`.
3. Do **not** infer the indexer auto-deploy checkbox from HTTP ([ADR 0006](../docs/adr/0006-indexer-health-git-sha.md)).

`make verify-issue-1300` live is **HTTP only**: indexer **`GET /api/v1/protocol/top-pairs`** plus frontend **`protocol-top-pairs` / `Top pairs (30d)`** in `chunk("ProtocolPage")`. Do **not** copy #701 PoolPage regex. Do **not** grep hub `cLUNC / USD`. Do **not** copy #1276 `EXPECT_SHA`. Do **not** invent leftover `DATABASE_URL`. Leftover probes SKIP unless hosts answer. Fail closed with `VERIFY1300_REQUIRE_LIVE=1`.

Trader `NUMERIC(38, 0)` has no public metric; leftover-complete for `#1277` is migrate applied + aggregator still running.

## Leftover-complete LocalTerra walks (operator evidence)

`make verify-issue-1300` does **not** execute these walks. Default leftover verify probes `make has-localterra` (down → SKIP; up → PASS the probe). `VERIFY1300_REQUIRE_HAS_LOCALTERRA=1` is fail-closed `make has-localterra` only. **`VERIFY1300_SKIP_HAS_LOCALTERRA=1`** skips the probe (do **not** ship `SKIP_HAS_LOCALTERRA`). Leftover-*ops* **comments (or reopens) the closed tracker** with the four how-tos. Tracker-close comments on #1300 do **not** waive those four notes. Forgejo closed ≠ leftover-complete. `has-localterra` down is **not** leftover-complete. Slice 2 leftover implement does **not** provision LocalTerra or record walks. Optional: Cloud Agent leftover-*ops* may `make setup-cloud-localterra` when item 5 still lacks walk notes; that is **not** implement-slice close. Do **not** close leftover-complete on probe SKIP. Chain-probe SKIP/PASS is not item-5 evidence.

| Walk | How-to | Pass |
|------|--------|------|
| **#1218** | LocalTerra + `make dev`: Swap Pay LUNC / Receive **USTR or JADE/RUBY** (same stand-in as `frontend-dapp/e2e/wrap-swap.spec.ts` **E7**). `deploy-dex-local` does **not** seed USTR — do **not** require LocalTerra USTR. Allowed substitute: manual columbus-5 Swap LUNC → USTR on `dex.cl8y.com` (live already has mapped GET). | Pass is wrap prefix + solver hops, not BFS 2-hop (**H1218-8**). Pay LUNC wraps to **cLUNC**, then solver hops (typically cLUNC → UST1 → USTR on columbus-5; LocalTerra E7 stand-in is typically cLUNC → EMBER → JADE). `wrap-then-cUSTC` is H1218-8 shorthand. Do **not** require the Route row to contain `cUSTC` or `USTR`. Child `VERIFY_ISSUE_1218_CHAIN=1` is wrap-swap E7/E8, not this leftover walk. |
| **#1255** | **LocalTerra only** (same class as #1219 not-prod). **Minting** (`mint_control`) is a paid **50 UST1 SKU** (`PayWithAnyToken`); **free create cannot mint**. A `dex.cl8y.com` Create Token is spend + a live CW20 — **not** this walk. Create Token **18-dec** factory CW20. Set **Decimals** to **18** (`create-token-decimals`; default is `'6'` in `CreateTokenPage.tsx`). Enable **Minting** at create (`mint_control` is create-only). Mint cap may be **empty (unlimited — valid)** or human **`≥1`**. `initialBalances: []`. Create Token does **not** create a pair; `mintCommunityTax` only mints. LocalTerra trading tokens are **6-dec**; the only seeded 18-dec CW20 is **TCL8Y** and is **forbidden**. After mint, **Create Pair** against a LocalTerra factory quote (**UST1 / cUSTC / EMBER**) so the CW20 is a `getAllTokens` picker id. Then Swap **pick as Pay**. Do **not** paste `terra1` (**Q1255-4** / **QS-3**: `SwapPage` `lcdEnabled` only when `fromInPicker`). For Max / **Q1255-6**: on Manage Token Mint enter raw **`1000000000000000000`** (18 zeros), not `1`. Manager Mint is raw CosmWasm `mint.amount` (`Amount (raw)` in `ManageTokenPage.tsx`; `mintCommunityTax` forwards the string with no `toRawAmount`). Create Token **mint cap** is human (`parseHumanRaw`) — do **not** mix it with the raw Mint field, and do **not** invent a human Mint amount because the cap is empty. Do **not** use registry-pinned TCL8Y / CL8Y / USTR. | Leftover-complete is Swap **pick** (not paste) + Max of minted raw **`10^18`** → human `1` (**Q1255-6**). LCD 18 vs default 6. **Drop** typed `1` execute/sim from leftover pass: an empty pair has no route. Optional leftover-*ops* that still run typed `1` execute/sim raw `10^18` must **seed LP** so the new pair quotes — that is **not** leftover-complete. Typing `1` on Manage Token Mint mints **1 raw**; Max of that is `1e-18` at 18-dec and does **not** prove Q1255-6. Max of a zero balance is `0` at any scale and does **not** prove Q1255-6. |
| **#1219** | **LocalTerra** (in-tree pair **cw2 1.17.0**) `/limits` or `/trade` Ladder with a rung below min remaining (10). A `dex.cl8y.com` UI pass is **not columbus-5 wasm leftover** (issue leftover 2 / ADR Outcome 3). Listed fleet is already **1.17.0 / 11672**. | Copy **Minimum size is 10 units**; Place disabled; no wallet popup (**S1219-8**). |
| **#1263** | LocalTerra `/protocol` with **indexer from this tip up**. | **Top pairs (30d)** section **renders** (`protocol-top-pairs`); table ≤5 rows (empty OK **only** when the section is visible). If the indexer on that tip is down, the section **hides** (`isProtocolTopPairsUnavailable` on 404/501) — a missing section is **not** a frontend miss; record indexer-up. Weaker than Coolify HTTP — does **not** prove `?limit=6` → 400. That 400 is **Coolify-only** (`GET /api/v1/protocol/top-pairs?limit=6`; **I1263-6** / `protocol_top_pairs.rs`). `protocol-page.spec.ts` does not assert Top pairs. |

## columbus-5 leftovers (operator)

Leftover-complete is **LCD cw2 1.17.0** (`UPGRADE582_PROBE_ONLY=1`). Leftover-ops recorded listed pair **1.17.0 / 11672**. A second `store` is a leftover defect. Store / 2-of-3 only if the probe is still **1.16.0**. Leftover agents never `terrad tx`.

```bash
# leftover agents / leftover-*ops* — read-only (no store / migrate / 2-of-3):
UPGRADE582_PROBE_ONLY=1 ./scripts/upgrade-582-code-id-pin.sh

# operator 2-of-3 ONLY if probe is still 1.16.0. Factory already 1.10.0 / 11629:
UPGRADE582_SKIP_FACTORY_MIGRATE=1 ./scripts/upgrade-582-code-id-pin.sh
# script still asserts factory ≥ 1.9.0, then UpdateConfig { pair_code_id }
```

Versions: [`AGENTS_CW20_CODE_ID_PIN.md`](./AGENTS_CW20_CODE_ID_PIN.md). Do **not** follow [`docs/runbooks/cw20-code-id-ops.md`](../docs/runbooks/cw20-code-id-ops.md) Launch checklist (factory 1.9.0 + pair 1.15.0 **RAN 2026-08-21**). Pair-first migrate still freezes gated writes; leftover must **not** invent a pair-only `terrad tx`. Not a founder card. Not leftover-agent `terrad tx`.

`#1264` AC1 is **measured** (tx `53B06B653D78AF3683F51A065FC640074A3A2E76CAE343A97B3E0F0BD79BAC36`, `gas_wanted=2710000`, `gas_used=2630228`, class not-A/not-B). Envelope stays **2,710,000**. Do **not** raise `WRAP_ROUTER_COMBO_OVERHEAD_GAS`. Do **not** reopen #1264. Child `make verify-issue-1264` stays in the stack. Optional `VERIFY1264_COLUMBUS_TX` may pin that hash.

## Leftover-complete

Leftover-complete is **operator + leftover-*ops***. Green `make verify-issue-1300` **never** means close. Tracker-close comments on #1300 do **not** waive the four walk notes. The tracker is already **closed**; leftover-*ops* **comments (or reopens)** it until those notes exist. Forgejo closed ≠ leftover-complete.

Leftover-complete for **#1300** only when all hold (same as [ADR 0008](../docs/adr/0008-post-merge-leftover-1287-1298.md) Integration):

- Coolify: **#1300 attests `20260921120000` then `20260921120001` only** (`success=true`, apply order) — operator Coolify DB / indexer `DATABASE_URL`. Same boot may also apply `20260921130000`; that row is **#1305**, not a #1300 FAIL and not a reason to keep #1300 open. Do **not** add a third version to `make verify-issue-1300`.
- Live `GET /api/v1/protocol/top-pairs` items length ≤5 economic; gems (`COLUMBUS5_GEM_ADDRESSES`) absent; `?limit=6` → **400**.
- Coolify frontend serves `6d34da13+` with HTTP marker **`protocol-top-pairs` / `Top pairs (30d)`** in `chunk("ProtocolPage")` (#673/#686). Do **not** copy #701 PoolPage regex. Do **not** grep hub `cLUNC / USD` (that is **#1305 leftover-complete**).
- LocalTerra / manual (issue item 5, leftover-*ops*): the four walks recorded per how-tos above. Leftover-*ops* **comments (or reopens) the closed tracker** until those notes exist. Forgejo closed ≠ leftover-complete. `#1255` is **LocalTerra-only**: after mint **Create Pair** vs UST1/cUSTC/EMBER, Swap **pick** (not paste); leftover pass is Max of minted raw `10^18` → human `1` (**Q1255-6**); drop execute/sim unless leftover-*ops* seeds LP. Minting is a paid **50 UST1 SKU** (`PayWithAnyToken`); free create cannot mint. Unlimited mint cap is valid. `has-localterra` down is **not** leftover-complete. Chain-probe SKIP/PASS is not item-5 evidence. Child Vitest is **not** the walk. Default leftover verify probes `has-localterra` (down → SKIP) and does not execute the walks. `VERIFY1300_REQUIRE_HAS_LOCALTERRA=1` is fail-closed `has-localterra` only — does not execute the walks, not invented five-worker e2e. Defining `VERIFY1300_REQUIRE_CHAIN` as Playwright / `run_leftover_e2e` is a leftover-implement FAIL. **`VERIFY1300_SKIP_HAS_LOCALTERRA` only.** Slice 2 leftover implement does **not** provision LocalTerra or record walks. Optional: Cloud Agent leftover-*ops* may `make setup-cloud-localterra` when item 5 still lacks walk notes; that is **not** implement-slice close. `VERIFY1300_LEFTOVER_E2E=1` stays SKIP until named.
- columbus-5 listed pair LCD cw2 **1.17.0** (`UPGRADE582_PROBE_ONLY=1`; code **11672** recorded) is **required to close**. Store/2-of-3 only if probe is still 1.16.0. A second store is a leftover defect. Leftover agents never `terrad tx`.
- **#1264** stays **closed**. Envelope **2,710,000**. Do **not** reopen. Child `make verify-issue-1264` stays in the stack. Optional `VERIFY1264_COLUMBUS_TX=53B06B653D78AF3683F51A065FC640074A3A2E76CAE343A97B3E0F0BD79BAC36`. **#1279** stays open; leftover-complete is ops-bot 1.5.
- **#1302 landed** on **#1305** (`92c84406`). This leftover **must not revert it**. Do **not** merge `origin/cac-design-issue-1302` as-is. A later **0009** follow-up MR that inserts without taking 0008/Q21 is allowed.

Do **not** close #1300 on green child `make verify-issue-*` or green `make verify-issue-1300` alone.

## Do / don’t

- **Do** run `make verify-issue-1300` from a git worktree after pulling `main` (once implement lands the script).
- **Do** run leftover-*ops* / Cloud Agent QA walks per how-tos when issue item 5 still lacks walk notes. Optional `make setup-cloud-localterra` if the chain is down; that is **not** leftover-implement close. Do **not** treat probe SKIP or Forgejo closed as leftover-complete. Comment (or reopen) the closed tracker until the four notes exist.
- **Do** link `frontend-dapp/node_modules` from the primary checkout in a git worktree. Do **not** `npm install` over a worktree symlink (#1299).
- **Don’t** reopen closed parents unless a merged invariant is wrong (including **#1264**).
- **Don’t** reopen #1264 or raise `WRAP_ROUTER_COMBO_OVERHEAD_GAS`. **Don’t** close #1279 from this ticket.
- **Don’t** spawn a solver (ADR 0007 Stay).
- **Don’t** treat GitLab CI quota as leftover evidence.
- **Don’t** flip Coolify auto-deploy or edit `autonomy.rs` / HMAC.
- **Don’t** treat keywords on #1300 as #297 authority or `DESIGN: APPROVE`.
- **Don’t** treat August 1.15.0 “already RAN” as leftover-complete for pair 1.17.0. **Don’t** store pair wasm again when LCD cw2 is already **1.17.0 / 11672**.
- **Don’t** invent leftover Playwright five-worker coverage (`VERIFY1300_LEFTOVER_E2E=1` SKIP until named). `VERIFY1300_REQUIRE_HAS_LOCALTERRA=1` is fail-closed `has-localterra` only — not that e2e flag and not “run the four walks”. Defining `VERIFY1300_REQUIRE_CHAIN` as Playwright / `run_leftover_e2e` is a leftover-implement FAIL. Use **`VERIFY1300_SKIP_HAS_LOCALTERRA` only** (short `SKIP_HAS_LOCALTERRA` never skips). Do **not** invent named make commands for the walks. Leftover implement (slice 2) must **not** provision LocalTerra or record walks.
- **Don’t** treat chain-probe SKIP/PASS as leftover-complete for issue item 5 (`has-localterra` down is not leftover-complete). Tracker-close comments on #1300 do **not** waive the four walks. Forgejo closed ≠ leftover-complete; leftover-*ops* comments (or reopens) the closed tracker until the notes exist.
- **Don’t** require LocalTerra USTR for #1218, Route `cUSTC`, Create Token Max of a zero balance as **Q1255-6**, Manage Token Mint `1` as 1 human (that field is raw), invent a human Mint amount because mint cap is empty (unlimited is valid), a `dex.cl8y.com` Create Token as the #1255 walk (spend + live CW20), Create Token without **Create Pair** as the #1255 walk, paste `terra1` on Swap as **Q1255-4**, typed `1` execute/sim without seed LP as leftover-complete, free create as able to mint, registry-pinned TCL8Y/CL8Y/USTR for #1255, or a `dex.cl8y.com` #1219 UI pass as columbus-5 wasm leftover.
- **Don’t** copy #701 `PoolPage|chartsOverviewStats|pairCatalog` for Coolify frontend (use #673/#686 `chunk("ProtocolPage")`).
- **Don’t** revert merged PR #1302 or grep hub `cLUNC / USD` as this leftover’s frontend marker (#1305). Do **not** merge `origin/cac-design-issue-1302` as-is. Do **not** ban a later **0009** follow-up MR that inserts without taking 0008/Q21.
- **Don’t** FAIL or keep #1300 open because `20260921130000` is present or missing (#1305).
- **Don’t** treat green `make verify-issue-1300` as leftover-complete.

## Regression

`VERIFY1300_SKIP_HAS_LOCALTERRA=1` skips the `has-localterra` probe (make never executes the walks). Do **not** ship `SKIP_HAS_LOCALTERRA`. `VERIFY1300_REQUIRE_HAS_LOCALTERRA=1` fail-closes that probe. Defining `VERIFY1300_REQUIRE_CHAIN` as Playwright / `run_leftover_e2e` is a leftover-implement FAIL. Probe SKIP/PASS is not leftover-complete for issue item 5. Tracker-close comments on #1300 do **not** waive the four walks. Forgejo closed ≠ leftover-complete. `VERIFY1300_LEFTOVER_E2E=1` stays SKIP until named. Green `make verify-issue-1300` never means close.

```bash
make verify-issue-1300
VERIFY1300_SKIP_CHILDREN=1 make verify-issue-1300
VERIFY1300_SKIP_LIVE=1 VERIFY1300_SKIP_HAS_LOCALTERRA=1 make verify-issue-1300
VERIFY1300_REQUIRE_LIVE=1 make verify-issue-1300
VERIFY1300_REQUIRE_HAS_LOCALTERRA=1 make verify-issue-1300
VERIFY1300_LEFTOVER_E2E=1 make verify-issue-1300
# leftover-*ops* / Cloud Agent QA — item 5 still lacks walk notes and chain down:
# optional provision, then comment (or reopen) closed #1300 with the four walks
# (make / leftover implement never executes the walks; probe SKIP / Forgejo closed is not leftover-complete)
make setup-cloud-localterra
```
