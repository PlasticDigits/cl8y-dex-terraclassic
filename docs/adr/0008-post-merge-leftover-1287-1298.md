# ADR 0008: Post-merge leftover after PRs 1287–1298

## Status

Proposed ([#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300))

Ordinary leftover ops after the 1287–1298 stack landed on `origin/main` (first draft tip: `6d34da13`, hotfixes [#1299](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1299) / [#1301](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1301)). Later PRs **1302–1304** are also on `origin/main` (`729b097f`, merge [#1302](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1302) `92c84406`); those leftovers are sister [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305), not this ticket. This ADR does **not** retune envelopes, spawn a solver, flip Coolify auto-deploy, store columbus-5 wasm, spend, or expand custody/policy. Keep this file as `docs/adr/0008-post-merge-leftover-1287-1298.md` (this branch took **0008** first).

Playbook: [`skills/AGENTS_POST_MERGE_OPS_1300.md`](../../skills/AGENTS_POST_MERGE_OPS_1300.md) (**M1300-1–M1300-8**). Overview: [`architecture.md`](../architecture.md#post-merge-leftover-ops). Invariants: [`qa-invariants.md`](../qa-invariants.md) **Q21**.

Product decisions stay in the child ADRs/skills. Do **not** duplicate them here.

Keywords on [#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300) (`architecture`, `deploy`, `Design decision required`) are **not** approval to write `DESIGN: APPROVE`, flip Coolify, or file a founder card. Deploy/spend/custody/policy expansion stays under [agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297). This leftover does **not** expand that policy.

## Outcome

Close leftover **#1300** only after **operator + leftover-*ops*** evidence for the merged stack. Green `make verify-issue-1300` **never** means close. Green child `make verify-issue-*` on a laptop is not leftover-complete.

1. **Coolify indexer.** Apply sqlx in filename order: `#1277` `20260921120000_traders_rolling_volume_numeric_38_0` then `#1263` `20260921120001_pair_volume_30d`. **#1300 attests those two versions only.** Same indexer boot may also apply `#1305` `20260921130000_traders_lifetime_heal_from_swaps`; that row is **[#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305)**, not a #1300 FAIL and not a reason to keep #1300 open. Do **not** add a third version to `make verify-issue-1300`. Operator attests `…000` then `…001` in `_sqlx_migrations` (`success=true`, apply order) via Coolify DB / indexer `DATABASE_URL` — not `scripts/lib/postgres-psql.sh`. After one aggregator tick, live **`GET /api/v1/protocol/top-pairs`** is ≤5 factory-listed economic rows (**P1263** / **I1263-6**). Trader `NUMERIC(38, 0)` has no public metric; leftover-complete for `#1277` is migrate applied + aggregator still running.
2. **Coolify frontend.** Rebuild from `6d34da13+`. Production-stable HTTP marker is **`protocol-top-pairs` / `Top pairs (30d)`** (hashed Vite chunk grep, same HTTP-grep technique as #701 leftover frontend probe). Do **not** grep hub `cLUNC / USD` (that is #1305 leftover-complete). Dual-app skew during rebuild is expected ([ADR 0006](./0006-indexer-health-git-sha.md)).
3. **columbus-5 pair wasm.** Store+migrate in-tree pair **cw2 1.17.0** so listed fleet picks up **F6** `#1234` (`UpdateLimitOrderPrice` / `CleanLimitBook` gate) and **L24** `#1219` named min remaining. Operator path is [`scripts/upgrade-582-code-id-pin.sh`](../../scripts/upgrade-582-code-id-pin.sh) (`UPGRADE582_PAIR_VERSION` **1.17.0**). Factory on columbus-5 is already **1.10.0 / 11629** → `UPGRADE582_SKIP_FACTORY_MIGRATE=1` (script still asserts factory ≥ 1.9.0). The script **`UpdateConfig { pair_code_id }`** so new `CreatePair` instantiates 1.17.0 — skipping that is a leftover defect, not optional. Versions: [`skills/AGENTS_CW20_CODE_ID_PIN.md`](../../skills/AGENTS_CW20_CODE_ID_PIN.md) (pair **1.17.0**, fleet was **1.16.0**). Do **not** follow [`docs/runbooks/cw20-code-id-ops.md`](../runbooks/cw20-code-id-ops.md) Launch checklist (still factory **1.9.0** + pair **1.15.0** **RAN 2026-08-21** / 11602 / 11601). Leftover agents use `UPGRADE582_PROBE_ONLY=1` (read-only); they do **not** `store` / `migrate` / 2-of-3. Pair-first migrate still freezes gated writes; leftover must **not** invent a pair-only `terrad tx`.
4. **Keep `#1264` open.** Envelope stays **2,710,000**. Columbus-5 wrap+2hop USTC→USTR `gas_used` is still **unmeasured** (AC1). **G1264-4** (USTC Max/gas-gate is LUNC-only) survived `#1218` — no code follow-up.
5. **`#1279` leftover-complete is ops-bot** `QA_TEMPLATE.md` **1.5.1–1.5.11**. Green `make verify-issue-1279` does **not** close `#1279`.
6. **LocalTerra / manual** (issue leftover item 5 / **M1300-3** — not a substitute for Coolify). Split two gates:

   | Gate | Behavior |
   |------|----------|
   | `make verify-issue-1300` chain probe | down → SKIP (default); `VERIFY1300_REQUIRE_HAS_LOCALTERRA=1` → FAIL; **never** run the four walks |
   | leftover-complete for issue item 5 | record the four how-tos on #1300, **or keep #1300 open** |

   Leftover-complete for item 5 is leftover-*ops* walk evidence on #1300 (how-tos below). `has-localterra` down is **not** leftover-complete. Keep #1300 open until cw2 **1.17.0** and the four walks. Coolify live HTTP SKIP is already not leftover-complete; columbus-5 missing 1.17.0 already keeps #1300 open; walks follow the same split. Slice 2 (leftover implement / `make verify-issue-1300`) does **not** provision LocalTerra and does **not** record walks. Slice 7 (leftover-*ops* / Cloud Agent QA) owns the four how-tos. Optional: Cloud Agent leftover-*ops* may `make setup-cloud-localterra` when item 5 is still open; that is **not** implement-slice close. Do **not** close on probe SKIP. Chain-probe SKIP/PASS is **not** item-5 evidence.

   **Chain probe / `VERIFY1300_REQUIRE_HAS_LOCALTERRA=1` contract:** leftover verify may call `make has-localterra` as a probe only. No Playwright. Do **not** copy #701 leftover Playwright FAIL (`VERIFY701_REQUIRE_CHAIN=1` FAILs when Playwright is missing; `run_leftover_e2e`). **Slice-2 FAIL:** defining `VERIFY1300_REQUIRE_CHAIN` as Playwright / `run_leftover_e2e` is a leftover-implement FAIL (that Q18/Q19 name fail-closes leftover Playwright; this ticket does not). Default `make verify-issue-1300` (flag unset): chain down → **SKIP** the probe; chain up → **PASS** the probe; **does not** execute the four walks even when the chain is up. `VERIFY1300_REQUIRE_HAS_LOCALTERRA=1`: chain down → **FAIL** (not SKIP); chain up → PASS the probe; still **does not** execute the walks. `VERIFY1300_SKIP_HAS_LOCALTERRA=1`: skip the probe (same class as `SKIP_LIVE`). Child Vitest / laptop-green `make verify-issue-1218` (chain opt-in `VERIFY_ISSUE_1218_CHAIN=1` is wrap-swap E7/E8, not this walk) is **not** the walk. Child skills `#1255` / `#1218` / `#1263` have no LocalTerra walk recipe — leftover-complete uses the how-tos below, not those flags. `VERIFY1300_LEFTOVER_E2E=1` stays **SKIP until named** (**M1300-7**). `VERIFY1300_REQUIRE_HAS_LOCALTERRA` is **not** that flag.

   | Walk | How-to | Pass |
   |------|--------|------|
   | **#1218** | LocalTerra + `make dev`: Swap Pay LUNC / Receive **USTR or JADE/RUBY** (same stand-in as `frontend-dapp/e2e/wrap-swap.spec.ts` **E7**). `deploy-dex-local` does **not** seed USTR — do **not** require LocalTerra USTR. Allowed substitute: manual columbus-5 Swap LUNC → USTR on `dex.cl8y.com` (live already has mapped GET). | Pass is wrap prefix + solver hops, not BFS 2-hop (**H1218-8**). Pay LUNC wraps to **cLUNC**, then solver hops (typically cLUNC → UST1 → USTR on columbus-5; LocalTerra E7 stand-in is typically cLUNC → EMBER → JADE). `wrap-then-cUSTC` is H1218-8 shorthand. Do **not** require the Route row to contain `cUSTC` or `USTR`. Child `VERIFY_ISSUE_1218_CHAIN=1` is wrap-swap E7/E8, not this leftover walk. |
   | **#1255** | Create Token **18-dec unlisted** factory CW20. Set **Decimals** to **18** (`create-token-decimals`; default is `'6'` in `CreateTokenPage.tsx`). Enable **Minting** at create (`mint_control` is create-only; 50 UST1 SKU). `initialBalances: []`. Swap pick as Pay; type `1`. For Max / **Q1255-6**: on Manage Token Mint enter raw **`1000000000000000000`** (18 zeros), not `1`. Manager Mint is raw CosmWasm `mint.amount` (`Amount (raw)` in `ManageTokenPage.tsx`; `mintCommunityTax` forwards the string with no `toRawAmount`). Create Token **mint cap** is human (`parseHumanRaw`) — do **not** mix those fields. Do **not** use registry-pinned TCL8Y / CL8Y / USTR. | Typed Swap `1`: execute/sim raw **`10^18`** (not `10^6`). Max of that minted raw → human `1` (**Q1255-6**). Typing `1` on Manage Token Mint mints **1 raw**; Max of that is `1e-18` at 18-dec and does **not** prove Q1255-6. Max of a zero balance is `0` at any scale and does **not** prove Q1255-6. |
   | **#1219** | **LocalTerra** (in-tree pair **cw2 1.17.0**) `/limits` or `/trade` Ladder with a rung below min remaining (10). A `dex.cl8y.com` UI pass while listed fleet is still **1.16.0** is **not** leftover item 3 (columbus-5 wasm / L24). | Copy **Minimum size is 10 units**; Place disabled; no wallet popup (**S1219-8**). |
   | **#1263** | LocalTerra `/protocol` with **indexer from this tip up**. | **Top pairs (30d)** section **renders** (`protocol-top-pairs`); table ≤5 rows (empty OK **only** when the section is visible). If the indexer on that tip is down, the section **hides** (`isProtocolTopPairsUnavailable` on 404/501) — a missing section is **not** a frontend miss; record indexer-up. Weaker than Coolify HTTP — does **not** prove `?limit=6` → 400. That 400 is **Coolify-only** (`GET /api/v1/protocol/top-pairs?limit=6`; **I1263-6** / `protocol_top_pairs.rs`). `protocol-page.spec.ts` does not assert Top pairs. |

## Context

Woodpecker `ci/woodpecker/pr/woodpecker` and `ci/woodpecker/push/woodpecker` succeeded on the merges. Local gitleaks was clean. Follow-ups already on `main`: PR **#1299** (dropped accidental `frontend-dapp/node_modules` symlink) and PR **#1301** (`test-commit-msg-hook.sh` leftover FAIL so `make verify-issue-1287` passes).

| PR | Issue | On `main` | Leftover class |
|----|-------|-----------|----------------|
| 1287 | #1286 / #1287 | Hooks: unpublished `--not --remotes` + skip commits already on `origin/main` | **code-done** |
| 1288 | #1234 | F6 gate on `UpdateLimitOrderPrice` + `CleanLimitBook` | **columbus-5** pair wasm; comment-only `verify-issue-1234` vs `verify-issue-582` |
| 1289 | #1265 | [ADR 0007](./0007-route-solve-remaining-failures.md) **Stay**; no solver spawn | **code-done** (docs) |
| 1290 | #1240 | Hub mixed-case wrap vs CEX native | **code-done**; hub `cLUNC / USD` rewrite already shipped |
| 1291 | #1279 | Lunc Dash leftover scaffolding | **ops-bot** device QA |
| 1292 | #1277 | `traders` rolling `NUMERIC(38, 0)` | **Coolify** migrate `20260921120000` |
| 1293 | #1264 | USTC→USTR wrap+2hop same 2.71M envelope | **do-not-close**; AC1 unmeasured |
| 1294 | #1285 | TaxPreview `send_msg` leftover of #1267 | **code-done** (Vitest) |
| 1295 | #1255 | Unlisted CW20 LCD `token_info` decimals | **LocalTerra / manual** |
| 1296 | #1219 | Named min remaining (L24) | **columbus-5** (with #1234) + **LocalTerra / manual** |
| 1297 | #1218 | Wrap-enter GET `/route/solve` | **LocalTerra / manual** |
| 1298 | #1263 | Protocol top-5 30d | **Coolify** migrate `20260921120001` + **LocalTerra / manual** |
| 1299 | hotfix | Drop `frontend-dapp/node_modules` symlink | **code-done** |
| 1301 | hotfix | Hook script leftover FAIL | **code-done** (via #1287) |

**Out of this leftover stack (keyword overlap is not enough):**

- [`origin/cac-design-issue-1276`](https://git.cl8y.com/code/cl8y-dex-terraclassic) / [`origin/cac-design-issue-1277`](https://git.cl8y.com/code/cl8y-dex-terraclassic) — other workers. Issue **#1277** code is on `main` via PR 1292; the **design branch** is not this stack. Issue **#1276** Coolify auto-deploy checkbox stays [ADR 0006](./0006-indexer-health-git-sha.md) / #297.
- PR **#1302** (`92c84406`) **landed** on the 1302–1304 stack ([#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305)). This leftover **must not revert it**. Hub wrap Coolify visual (`cLUNC / USD` vs CEX `LUNC`) is **#1305 leftover-complete**, not #1300. `#1240` already has `make verify-issue-1240`. Do **not** merge [`origin/cac-design-issue-1302`](https://git.cl8y.com/code/cl8y-dex-terraclassic) **as-is** (overlap on `architecture.md` / `qa-invariants.md` / README). That branch already publishes `docs/adr/0009-verify-issue-1290-hub-wrap-labels.md` / **Q22** and reserves 0008/Q21 for this ticket. A later follow-up MR that inserts **0009** without taking 0008/Q21 is allowed.
- Empty leftover `origin/issue/1295` was already an ancestor of `main` (deleted). Do not resurrect it.

Merge notes (historical, not leftover work): PR 1293 was empty on origin after a force-update to `main`; commit `86b4e690` was restored from a local worktree then merged. sqlx version collision: both #1277 and #1263 first used `20260921120000`; pair 30d is `20260921120001` on `main` (same prefix rule as [ADR 0005](./0005-protocol-fee-multihop-hops.md) `20260916120000` / `20260916120001`).

Stacked-merge sanity already on the issue:

- **G1264-4 survived #1297.** USTC Max/gas still LUNC-only. No code follow-up.
- **#1240 leftover rename.** Hub `cLUNC / USD` vs original **P1240-5** CEX `LUNC` wording is a product rewrite **already on main** via #1290. Do not reopen #1240 for copy.
- **`verify-issue-1234.sh`** header claims `make verify-issue-582` still green but **does not invoke it** (comment-only gap). Leftover implement either runs the child or drops the claim.
- sqlx versions: **#1300 attests `…000` then `…001` only.** The issue-body comment adding `20260921130000` to item 1 is **#1305**, not a third #1300 version. Pair-wasm / Coolify / LocalTerra leftovers otherwise unchanged.

## Non-goals

- Closing [#1264](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1264) or raising `WRAP_ROUTER_COMBO_OVERHEAD_GAS` without AC1 class A `gas_used`.
- Spawning a solver / bumping `MAX_PATH_CANDIDATES` ([ADR 0007](./0007-route-solve-remaining-failures.md) **Stay**).
- Reverting merged PR #1302. Merging `origin/cac-design-issue-1302` as-is (overlap on `architecture.md` / `qa-invariants.md` / README). Banning a later **0009** follow-up MR that inserts without taking 0008/Q21. Treating `cac-design-issue-1276` / `cac-design-issue-1277` as this stack.
- Absorbing or waiting on sqlx `20260921130000_traders_lifetime_heal_from_swaps` (**#1305**). Adding a third version to `make verify-issue-1300`.
- Grepping Coolify hub `cLUNC / USD` as this leftover’s frontend HTTP marker (**#1305**).
- Reopening closed parents (#1286/#1287, #1234, #1265, #1240, #1277, #1285, #1255, #1219, #1218, #1263) for ops/QA.
- Flipping the indexer Coolify **auto-deploy checkbox**, scraping Coolify logs, publishing app UUIDs/tokens, or expanding CAC `COOLIFY_APP_MAP` ([ADR 0006](./0006-indexer-health-git-sha.md) / #297).
- Agent-executed columbus-5 `store` / `migrate`, treasury rotate, or pause.
- A founder card for this ordinary leftover. HMAC / `autonomy.rs` self-approval. `DESIGN: APPROVE`.
- Inventing `down.sql` for `20260921120000` / `20260921120001` (none in `indexer/migrations/revert/`).
- Inventing named make commands that execute the four leftover walks, or copying #701/#702 leftover Playwright (`run_leftover_e2e` / `VERIFY701_REQUIRE_CHAIN=1` FAIL when Playwright missing). Defining `VERIFY1300_REQUIRE_CHAIN` as Playwright / `run_leftover_e2e` (leftover-implement FAIL). Treating `VERIFY_ISSUE_1218_CHAIN=1` as the #1218 leftover walk. Treating `has-localterra` down / chain-probe SKIP as leftover-complete for issue item 5. Leftover implement (slice 2) provisioning LocalTerra or recording walk notes (that is leftover-*ops* / slice 7). Treating Pay LUNC / Receive USTR as the only LocalTerra #1218 how-to (`deploy-dex-local` does not seed USTR). Treating Create Token Max of a zero balance as **Q1255-6**. Treating Manage Token Mint `1` as 1 human unit (that field is raw). Treating a `dex.cl8y.com` #1219 UI pass while listed fleet is 1.16.0 as leftover item 3. Treating a missing LocalTerra **Top pairs (30d)** section as a frontend miss when the indexer on that tip is down.
- Changing **G1264-4**, hybrid-off (**H596**), or wrap+multihop pool-only (**H596-7**).
- Waiting on GitLab CI quota as leftover evidence.

## Decision

**Stack leftover like Q18/Q19, not a product ADR.** Child features already shipped. #1300 is Coolify + columbus-5 wasm + LocalTerra/manual + ops-bot, with two explicit stay-open owners (`#1264` AC1, `#1279` device QA).

| Decision | Choice |
|----------|--------|
| Q slot | **Q21** (Q20 remains Lunc Dash **#1279**) |
| sqlx order | First-applied `#1277` `20260921120000`; `#1263` stays `20260921120001`. **#1300 attests those two only.** `20260921130000` is **#1305** (not a #1300 FAIL). Do not checksum-edit `_sqlx_migrations`. Do not add a third version to `make verify-issue-1300`. |
| Envelope | Keep **2,710,000**. Do not close #1264. |
| G1264-4 | No code follow-up after wrap-enter. |
| #1240 copy | Already on main; leftover does not retitle CEX tabs. |
| #1234 vs #582 | Implement leftover must make the verify claim true (invoke) or delete it. |
| #1279 | Pre-check `make verify-issue-1279`; leftover-complete = ops-bot 1.5. `VERIFY1279_IID=1279` / `LEFTOVER_COMPLETE=1` **must FAIL**. |
| #297 | Coolify **migrate/rebuild** is ordinary leftover. That is **not** leftover-complete. Leftover-complete is **operator + leftover-*ops*** (Coolify migrate/rebuild **and** columbus-5 2-of-3 **and** the four walks). Green `make verify-issue-1300` never means close. Coolify **auto-deploy checkbox** is still #1276 / #297 — out of this ticket. columbus-5 pair store+migrate uses [`scripts/upgrade-582-code-id-pin.sh`](../../scripts/upgrade-582-code-id-pin.sh) (not the August 1.15.0 checklist); leftover agents `UPGRADE582_PROBE_ONLY=1` and SKIP/FAIL live, they do not store. |
| PR #1302 | Landed (`92c84406`) on **#1305**. Do **not** revert. Do **not** merge `origin/cac-design-issue-1302` as-is (overlap on `architecture.md` / `qa-invariants.md` / README). That branch already publishes `docs/adr/0009-verify-issue-1290-hub-wrap-labels.md` / **Q22**. A later **0009** follow-up MR that inserts without taking 0008/Q21 is allowed. Hub wrap Coolify visual is **#1305 leftover-complete**. |
| LocalTerra walks | Keep issue item 5 as leftover-complete **leftover-*ops* evidence** (slice 7 / Cloud Agent QA). Close #1300 only after the four walks (Outcome item 6 how-tos). `has-localterra` down is **not** leftover-complete — keep #1300 open until cw2 **1.17.0** and the four walks. Slice 2 leftover implement does **not** provision LocalTerra or record walks. Optional: Cloud Agent leftover-*ops* may `make setup-cloud-localterra` when item 5 is still open; that is **not** implement-slice close. Do **not** close on probe SKIP. Chain-probe SKIP/PASS is **not** item-5 evidence. **Chain probe:** default leftover verify calls `make has-localterra` (down → SKIP; up → PASS probe). **`VERIFY1300_REQUIRE_HAS_LOCALTERRA=1` = fail-closed `make has-localterra` only** — no Playwright, does not execute the walks. **Slice-2 FAIL:** defining `VERIFY1300_REQUIRE_CHAIN` as Playwright / `run_leftover_e2e` is a leftover-implement FAIL. `SKIP_HAS_LOCALTERRA=1` skips the probe. Default leftover verify does not execute walks when the chain is up. Child Vitest is not the walk. `VERIFY1300_LEFTOVER_E2E=1` SKIP until named (`REQUIRE_HAS_LOCALTERRA` is not that flag). |

## Component / state / interface changes

| Layer | Change in *this* leftover |
|-------|---------------------------|
| Schema | **None in git for this leftover.** Production must *apply* already-merged `20260921120000` then `20260921120001`. **#1300 attests those two only.** `20260921130000` may apply on the same boot — **#1305**, not this ticket. Expand-only. No revert files. |
| Indexer API | No new routes. After migrate + `refresh_pair_volumes_30d` (~5 min), **`GET /api/v1/protocol/top-pairs`** uses `pair_volume_30d`. Vite `/protocol` is **frontend** rebuild evidence, not sqlx/`pair_volume_30d` evidence. Rolling trader raw volume columns are `NUMERIC(38, 0)` (**R1277**). |
| Pair wasm | In-tree cw2 remains **1.17.0** until columbus-5 listed fleet is migrated from **1.16.0**. No message-schema change in this leftover. |
| dApp | No required chrome change. Frontend rebuild picks up already-merged wrap-enter, decimals, TaxPreview, mixed-case hub, Protocol top-5. Leftover HTTP marker: **`protocol-top-pairs` / `Top pairs (30d)`**. Do not grep hub `cLUNC / USD`. |
| Hooks | Already on `main` (#1287 / #1301). No further hook diff. |
| GET `/route/solve` | Already wrap-enter mapped on the client (#1218). Indexer still 400s `token_in=uluna`. No solver spawn. |
| Gas | No constant change. |

## Affected invariants

| ID | Effect |
|----|--------|
| **M1300-1–M1300-8** / **Q21** | New leftover stack. |
| **R1277-1–R1277-8** | Coolify must apply `NUMERIC(38, 0)` before 18-dec `SUM` can overflow prod. |
| **P1263 / I1263** | Live `GET /api/v1/protocol/top-pairs` after `20260921120001` + rollup tick. |
| **F6 / #1234** | columbus-5 pair 1.17.0; local verify already green. |
| **L24 / #1219** | Same wasm migrate; dust ladder must not popup. |
| **G1264-1–G1264-8** | Envelope + LUNC-only gate unchanged; AC1 still unmeasured. |
| **H1218 / H596-7** | Wrap-enter GET after client map; execute stays pool-only. |
| **Q1255** | Unlisted CW20 scale from LCD `token_info`. |
| **L1279 / Q20** | Device QA still ops-bot; this stack does not override Q20. |
| **R-CENSUS / ADR 0007** | Stay. F0 leftover is LocalTerra #1218, not a new ranking ticket. |
| **H1276 / ADR 0006** | Auto-deploy checkbox **not** part of leftover-complete for #1300. |
| **G1287** | Hooks already include origin/main skip + unpublished range. |
| **P1240** | Mixed-case hub already on main. Coolify hub wrap visual leftover is **#1305**, not #1300. |

## Alternatives

| Option | Why not |
|--------|---------|
| Treat green child verify / green `make verify-issue-1300` as leftover-complete | Repeats the leftover-ops failure mode: prod schema/wasm/UI stay on the previous tip. Leftover-complete is **operator + leftover-*ops***. Green verify never means close. |
| Close #1264 because PR 1293 merged | AC1 `gas_used` is still unmeasured. Envelope must not rise on hope. |
| Raise combo overhead “just in case” | Forbidden without class A measurement (**G1264-6**). |
| Revert merged PR #1302 / treat it as a leftover defect | Already on `origin/main` (`92c84406`). Sister leftover is **#1305**. Hub wrap Coolify visual is **#1305 leftover-complete**. |
| Merge `origin/cac-design-issue-1302` as-is | Overlap on `architecture.md` / `qa-invariants.md` / README. That branch already publishes `docs/adr/0009-verify-issue-1290-hub-wrap-labels.md` / **Q22** and reserves 0008/Q21 here. A later **0009** follow-up MR that inserts without taking 0008/Q21 is allowed. |
| Wait on / FAIL #1300 because `20260921130000` is present or missing | That version is **#1305**. #1300 attests `…000` then `…001` only. Same boot may apply `…30000`; not a #1300 FAIL. |
| Drop issue item 5 (Coolify frontend + child verifies enough) | Rejected: leftover-complete keeps the four LocalTerra/manual walks as **operator evidence**. Child Vitest is not the walk. |
| Treat `has-localterra` down / chain-probe SKIP as leftover-complete for item 5 | Rejected. Coolify live HTTP SKIP is not leftover-complete. Columbus-5 missing 1.17.0 keeps #1300 open. Walks follow the same split. Slice 7 leftover-*ops* / Cloud Agent QA may `make setup-cloud-localterra` when item 5 is still open; that is **not** implement-slice close. |
| Have leftover implement (slice 2) provision LocalTerra and record the four walks | Rejected. Slice 2 is the verify script and must **not** execute walks (**M1300-7**). Leftover implement inventing Playwright against that flag fails. Walk notes are slice 7 leftover-*ops*. |
| Have `make verify-issue-1300` execute the four walks under `REQUIRE_HAS_LOCALTERRA` / a copied `REQUIRE_CHAIN` | Rejected: no named command/pass-fail per walk. `REQUIRE_HAS_LOCALTERRA` is fail-closed `has-localterra` only. Defining `VERIFY1300_REQUIRE_CHAIN` as Playwright / `run_leftover_e2e` is a leftover-implement FAIL. |
| Copy #701 `REQUIRE_CHAIN` Playwright FAIL (`run_leftover_e2e`) | Contradicts **M1300-7** (`VERIFY1300_LEFTOVER_E2E=1` SKIP until named). Slice-2 FAIL if `VERIFY1300_REQUIRE_CHAIN` means Playwright. |
| Fold #1276 auto-deploy checkbox into #1300 | Different authority (#297 deploy policy) and different leftover-complete (`EXPECT_SHA`). |
| One sqlx file combining NUMERIC + `pair_volume_30d` | Would rewrite applied checksums; collision already resolved as `…000` / `…001`. |
| Agent store+migrate pair wasm from leftover verify | Deploy/custody. Operator `upgrade-582-code-id-pin.sh` only (`UPGRADE582_PROBE_ONLY=1` for leftover agents). |
| Founder card / `DESIGN: APPROVE` | Ordinary leftover. Keywords are not approval. |
| Invent `down.sql` so 2(c) rollback applies | Partial suffix revert is still **2(b)** ([ADR 0006](./0006-indexer-health-git-sha.md)). These two versions have no downs. |

## Complexity added / removed

**Added (docs + leftover verify only):** Q21 registry, playbook, `make verify-issue-1300` (implement slice 2 — probe only; green never means close), Coolify migrate ordering, columbus-5 wasm checklist, explicit do-not-close for #1264, leftover-complete walk how-tos (slice 7 leftover-*ops*), two-gate split (chain probe vs item-5 leftover-complete), `VERIFY1300_REQUIRE_HAS_LOCALTERRA=1` as fail-closed `has-localterra` only.

**Removed:** Ambiguity that child-green or green `make verify-issue-1300` == leftover-complete; treating chain-probe SKIP as leftover-complete for issue item 5; leftover implement as walk owner; ADR 0007 leftover pass `wrap-then-cUSTC`; treating Max of a zero Create Token balance as **Q1255-6**; treating Manage Token Mint `1` as 1 human unit; citing a nonexistent second 0008; the false `verify-issue-582` claim once implement fixes it.

**No new:** HTTP routes, sqlx files, wasm messages, gas constants, solver knobs.

## Migration

Already in git:

1. [`indexer/migrations/20260921120000_traders_rolling_volume_numeric_38_0.sql`](../../indexer/migrations/20260921120000_traders_rolling_volume_numeric_38_0.sql) — expand `traders.volume_*` / `total_volume` to `NUMERIC(38, 0)`.
2. [`indexer/migrations/20260921120001_pair_volume_30d.sql`](../../indexer/migrations/20260921120001_pair_volume_30d.sql) — `pair_volume_30d` rollup table.

Production: indexer boot `sqlx::migrate!()` (no `set_ignore_missing`). Operator confirms `_sqlx_migrations` has **#1300** versions `20260921120000` and `20260921120001` `success=true` **in that order** via Coolify DB / indexer `DATABASE_URL`. A `20260921130000` row, if present, is **#1305** — not a #1300 FAIL. Then one `refresh_pair_volumes` / `refresh_pair_volumes_30d` tick. `make verify-issue-1300` live does **not** query that table and does **not** require `…30000`.

Pair wasm: columbus-5 store of optimized pair artifact + migrate listed pairs from cw2 **1.16.0** → **1.17.0** via [`scripts/upgrade-582-code-id-pin.sh`](../../scripts/upgrade-582-code-id-pin.sh). Factory is already **1.10.0 / 11629** → `UPGRADE582_SKIP_FACTORY_MIGRATE=1`. The script **does** `UpdateConfig { pair_code_id }` so new `CreatePair` instantiates 1.17.0 — skipping that is a leftover defect. Do **not** treat [`docs/runbooks/cw20-code-id-ops.md`](../runbooks/cw20-code-id-ops.md) “RAN 2026-08-21” as leftover-complete. Versions: [`skills/AGENTS_CW20_CODE_ID_PIN.md`](../../skills/AGENTS_CW20_CODE_ID_PIN.md). Leftover agents: `UPGRADE582_PROBE_ONLY=1` only.

No dApp env keys. No new revert files.

## Observability

- Indexer schema: operator attests `_sqlx_migrations` rows for **`20260921120000` then `20260921120001` only** (`success=true`, apply order) via **Coolify DB / indexer `DATABASE_URL`**. Do **not** use `scripts/lib/postgres-psql.sh` against prod (LocalTerra/dev helper). Presence of `20260921130000` is **#1305**, not a #1300 FAIL. `make verify-issue-1300` live is **HTTP only**: indexer **`GET /api/v1/protocol/top-pairs`** plus frontend **`protocol-top-pairs` / `Top pairs (30d)`** (hashed Vite chunk grep, same HTTP-grep technique as #701 leftover frontend probe). Do **not** grep hub `cLUNC / USD`. Do **not** copy #1276 `EXPECT_SHA`. Do **not** invent leftover `DATABASE_URL`.
- After ~5 min: `pair_volume_30d.updated_at` recent. Live probe:

```bash
curl -sS "https://indexer.dex.cl8y.com/api/v1/protocol/top-pairs"
# items length ≤ 5; gems (COLUMBUS5_GEM_ADDRESSES) absent
curl -sS -o /dev/null -w "%{http_code}\n" \
  "https://indexer.dex.cl8y.com/api/v1/protocol/top-pairs?limit=6"
# expect 400 (I1263-6)
```

  Vite `/protocol` is **frontend** rebuild evidence, not sqlx/`pair_volume_30d` evidence. Named marker: hashed JS contains **`protocol-top-pairs`** and **`Top pairs (30d)`**. Do **not** grep hub `cLUNC / USD`.
- Trader rolling: aggregator no longer fails on 18-dec `SUM(offer_amount)`. No public metric; leftover-complete for `#1277` is migrate applied + aggregator still running.
- Pair: columbus-5 cw2 query `1.17.0` on listed pairs after `upgrade-582-code-id-pin.sh`. Leftover agents: `UPGRADE582_PROBE_ONLY=1`.
- `#1264` AC1: optional `VERIFY1264_COLUMBUS_TX` when an operator records `gas_used`; until then leftover reports **unmeasured**, not PASS-as-closed.
- `#1279`: ops-bot records 1.5.1–1.5.11; make output is pre-check only.
- Do **not** scrape Coolify `SOURCE SHA` logs. Do **not** paste `/status` JSON or tokens on the issue.

## Failure modes

| Mode | Behavior |
|------|----------|
| Child `make verify-issue-*` FAIL | Stack FAIL. Fix the child; do not SKIP_CHILDREN to paint leftover-complete. |
| Coolify indexer unreachable | SKIP unless `VERIFY1300_REQUIRE_LIVE=1` / `VERIFY1300_IID=1300` → FAIL. |
| `20260921120001` applied before `…000` | sqlx orders by version int; both must succeed. If prod skipped `…000`, traders can still overflow. Confirm both **#1300** rows. |
| Frontend rebuilt, indexer not | Dual-app skew: `GET /api/v1/protocol/top-pairs` 404/empty while Vite shows the section. Wait indexer migrate + tick. |
| Indexer migrated, frontend stale | Top pairs API live; dApp may hide the block. Rebuild frontend. |
| Pair wasm still 1.16.0 | F6 reprice / L24 min-size not on columbus-5. Leftover stays open. Local verify stays green (in-tree 1.17.0). |
| Treat August 1.15.0 “already RAN” as leftover-complete | Operator skips **1.16.0 → 1.17.0**. Leftover stays open until listed cw2 is 1.17.0. Use `upgrade-582-code-id-pin.sh`, not `cw20-code-id-ops.md` Launch checklist. |
| Treat AC1 unmeasured as close | Forbidden. Keep 2.71M. |
| `VERIFY1279_LEFTOVER_COMPLETE=1` | Must FAIL. Do not close #1279 from this stack. |
| Treat merged PR #1302 as a leftover defect / revert it | Forbidden. `92c84406` is on `origin/main`. Hub wrap Coolify visual is **#1305 leftover-complete**. |
| FAIL #1300 because `_sqlx_migrations` also has `20260921130000` (or lacks it) | That row is **#1305**. #1300 attests `…000` then `…001` only. Do not add a third version to `make verify-issue-1300`. |
| Grep Coolify hub `cLUNC / USD` as the #1300 frontend marker | Forbidden. Named marker is **`protocol-top-pairs` / `Top pairs (30d)`**. Hub wrap visual is **#1305**. |
| Treat child Vitest as LocalTerra leftover-complete | Forbidden. Close #1300 only after the four walks (Outcome item 6 how-tos). Child Vitest is not the walk. |
| Treat `has-localterra` down / chain-probe SKIP as leftover-complete for issue item 5 | Forbidden. Keep #1300 open, same class as wasm still 1.16.0. Coolify live HTTP SKIP is already not leftover-complete. Slice 2 leftover implement does **not** provision or walk. Slice 7 leftover-*ops* / Cloud Agent QA may `make setup-cloud-localterra` when item 5 is still open; that is **not** implement-slice close. Do **not** close on probe SKIP. Chain-probe SKIP/PASS is not item-5 evidence. |
| Treat leftover implement (slice 2) as the owner of walk notes | Forbidden. Slice 2 never executes the four walks. Inventing Playwright under `VERIFY1300_LEFTOVER_E2E=1` fails **M1300-7**. |
| Treat Pay LUNC / Receive USTR as the only LocalTerra #1218 how-to | Forbidden. `deploy-dex-local` does not seed USTR. Use E7 stand-in (USTR **or** JADE/RUBY) or columbus-5 `dex.cl8y.com`. Do **not** require Route `cUSTC` or `USTR`. |
| Treat Create Token then Max of a zero balance as **Q1255-6** | Forbidden. Create Token ships `initialBalances: []`. Max of `0` does not distinguish 18-dec vs 6-dec. Set **Decimals** to **18**, enable **Minting** at create (`mint_control`, 50 UST1 SKU), then Manage Token Mint raw **`1000000000000000000`**. Typing `1` on Mint is **1 raw** (Max `1e-18` at 18-dec). Do not mix mint cap (human `parseHumanRaw`) with Mint amount (raw). Do not use registry-pinned TCL8Y / CL8Y / USTR. |
| Treat a `dex.cl8y.com` #1219 UI pass while listed fleet is **1.16.0** as leftover item 3 | Forbidden. Item 3 is columbus-5 pair wasm / L24. The copy / Place-disabled walk is LocalTerra (in-tree 1.17.0) or a pair already on 1.17.0. |
| Treat a missing LocalTerra **Top pairs (30d)** section as a frontend miss | Forbidden. Empty table is OK only when the section **renders**. Indexer-down hides it (`isProtocolTopPairsUnavailable`). Record indexer-from-this-tip-up. `?limit=6` → 400 stays Coolify-only. |
| Treat `VERIFY1300_REQUIRE_HAS_LOCALTERRA=1` as “run the four walks” / copy #701 Playwright FAIL | Forbidden. `REQUIRE_HAS_LOCALTERRA` is fail-closed `make has-localterra` only. No Playwright. Default leftover verify probes `has-localterra` (down → SKIP) and does not execute walks when the chain is up. `VERIFY1300_LEFTOVER_E2E=1` is the stacked-Playwright flag and stays SKIP until named. |
| Define `VERIFY1300_REQUIRE_CHAIN` as Playwright / `run_leftover_e2e` | Leftover-implement **FAIL**. That Q18/Q19 name fail-closes leftover Playwright. This ticket’s fail-closed probe is `VERIFY1300_REQUIRE_HAS_LOCALTERRA`. |
| Agent store wasm / flip auto-deploy | Out of authority. #297 for policy expansion; operator `upgrade-582-code-id-pin.sh` for wasm. Leftover agents `UPGRADE582_PROBE_ONLY=1` only. |
| Colliding sqlx rename after prod apply | Checksum mismatch; migrator rejects. Do not rename applied files. |
| Rollback image without schema | `NUMERIC(38, 0)` and `pair_volume_30d` stay; old binary may fail migrate-on-boot if versions missing from the binary — **2(b)** keep schema + hotfix that still ships N ([ADR 0006](./0006-indexer-health-git-sha.md)). |

## Ordered implementation slices

1. **Docs registry (this design PR).** ADR 0008 + architecture pointer + **Q21** + playbook **M1300**. Pointers on ADR 0004/0005/0006/0007. No verify script in the design PR.
2. **Leftover verify script (implement).** `scripts/qa/verify-issue-1300.sh` + `Makefile` `verify-issue-1300` + `docs/testing.md` / `scripts/qa/README.md` / `AGENTS.md` wiring + child-skill “Coolify leftover: #1300” one-liners. Children: **1287, 1234, 1265, 1240, 1279, 1277, 1264, 1285, 1255, 1219, 1218, 1263**. Copy `require_live()` from #701/#702 **for Coolify HTTP only** (`VERIFY1300_REQUIRE_LIVE` / `VERIFY1300_IID=1300`). Do **not** copy #701 leftover Playwright (`VERIFY701_REQUIRE_CHAIN=1` FAIL when Playwright missing; `run_leftover_e2e`). **Slice-2 FAIL:** defining `VERIFY1300_REQUIRE_CHAIN` as Playwright / `run_leftover_e2e` is a leftover-implement FAIL. Chain probe: `timeout 20 make -s has-localterra` — default down → SKIP, `VERIFY1300_REQUIRE_HAS_LOCALTERRA=1` down → FAIL, `VERIFY1300_SKIP_HAS_LOCALTERRA=1` skips the probe; **never** execute the four walks. Does **not** provision LocalTerra. Does **not** record walk notes. Live frontend HTTP: hashed-chunk grep for **`protocol-top-pairs` / `Top pairs (30d)`** (hashed Vite chunk grep, same technique as #701). Do **not** grep hub `cLUNC / USD`. Do **not** copy #1276 `EXPECT_SHA` leftover-complete. Do **not** attest `20260921130000`. Green `make verify-issue-1300` never means close.
3. **Comment-only gap.** `verify-issue-1234.sh`: invoke `make verify-issue-582` **or** remove the header claim. Same leftover implement slice.
4. **Coolify indexer (operator).** Migrate `…000` then `…001`; attest those two `_sqlx_migrations` rows via Coolify DB / indexer `DATABASE_URL`; wait rollup; probe `GET /api/v1/protocol/top-pairs` (≤5 economic; `?limit=6` → 400). `…30000` if applied is **#1305**. Ordinary leftover — not #297.
5. **Coolify frontend (operator).** Rebuild `6d34da13+`. Production marker: **`protocol-top-pairs` / `Top pairs (30d)`**. Vite `/protocol` is frontend evidence only. Do not grep hub `cLUNC / USD`.
6. **columbus-5 pair wasm (operator).** [`scripts/upgrade-582-code-id-pin.sh`](../../scripts/upgrade-582-code-id-pin.sh): `UPGRADE582_PAIR_VERSION` **1.17.0**, `UPGRADE582_SKIP_FACTORY_MIGRATE=1`, `UpdateConfig { pair_code_id }`. Leftover agents: `UPGRADE582_PROBE_ONLY=1` — do not store / migrate / 2-of-3 / invent a pair-only `terrad tx`.
7. **LocalTerra / manual leftover-*ops* (Cloud Agent QA).** Keep issue item 5 as leftover-complete leftover-*ops* evidence. How-tos: Outcome item 6. Slice 2 does **not** execute the four walks and does **not** provision LocalTerra (no named command; do not invent). Default leftover verify probes `make has-localterra` (down → SKIP; up → PASS probe) and does **not** execute the walks when the chain is up. `VERIFY1300_REQUIRE_HAS_LOCALTERRA=1` is fail-closed `make has-localterra` only — no Playwright. **Slice-2 FAIL:** defining `VERIFY1300_REQUIRE_CHAIN` as Playwright / `run_leftover_e2e`. `SKIP_HAS_LOCALTERRA=1` skips the probe. Optional: Cloud Agent leftover-*ops* may `make setup-cloud-localterra` when item 5 is still open; that is **not** implement-slice close. Do **not** close on probe SKIP. Chain-probe SKIP/PASS is not item-5 evidence. Leftover stacked Playwright is **manual**; `VERIFY1300_LEFTOVER_E2E=1` **SKIP until named** (`REQUIRE_HAS_LOCALTERRA` is not that flag). Child verifies already run their own specs (`wrap-swap.spec.ts` / #1218, ladder UI / #1219). `protocol-page.spec.ts` does **not** assert Top pairs. Do not invent five-worker leftover coverage. `e2e-tx` stays **1 worker**. `:3173` CORS: do not leak `PLAYWRIGHT_WEB_PORT`.
8. **Stay-open owners.** Record #1264 AC1 still unmeasured; #1279 still ops-bot. Do not close those issues from #1300.

**Open issue dependencies:** none that block starting leftover implement. `#1264` and `#1279` remain open *after* #1300 leftover-complete. Sister **#1305** does **not** block #1300.

**Do not wait on:** #1276 checkbox, #1305 leftover-complete, `cac-design-issue-1276` / `1277` / `1302`.

## Tests

| ID | Path | Expect |
|----|------|--------|
| T1 | `make verify-issue-1300` | Children listed above; docs greps for Q21 / M1300 / ADR 0008 |
| T2 | Live off | Coolify probes SKIP |
| T3 | `VERIFY1300_REQUIRE_LIVE=1` unreachable | FAIL |
| T4 | `VERIFY1300_IID=1300` unreachable | FAIL (same as #701) |
| T5 | Child FAIL | Stack FAIL |
| T6 | `make verify-issue-1264` | Envelope 2.71M; no combo raise |
| T7 | `VERIFY1279_IID=1279` / `LEFTOVER_COMPLETE=1` | FAIL |
| T8 | chain probe | Default leftover verify calls `make has-localterra` only (no Playwright). Chain down → **SKIP**. Chain up → **PASS** the probe; **does not** execute the four walks. `VERIFY1300_REQUIRE_HAS_LOCALTERRA=1`: chain down → **FAIL** (not SKIP); chain up → PASS the probe; still **does not** execute the walks. Defining `VERIFY1300_REQUIRE_CHAIN` as Playwright / `run_leftover_e2e` is a leftover-implement FAIL. `VERIFY1300_SKIP_HAS_LOCALTERRA=1`: skip the probe. Probe SKIP/PASS is **not** item-5 leftover-complete. Walks stay leftover-complete leftover-*ops* evidence (Outcome item 6 how-tos, slice 7); `has-localterra` down keeps #1300 open. `VERIFY1300_LEFTOVER_E2E=1` remains **SKIP until named**. Slice 2 leftover implement does not provision or walk. Green T1/T8 never means close. |
| T9 | `verify-issue-1234` after slice 3 | Claim matches behavior |
| T10 | Live Coolify after migrate | Operator: **#1300** sqlx versions `20260921120000` then `20260921120001` `success=true` via Coolify DB / indexer `DATABASE_URL` (`20260921130000` is **#1305**, not a #1300 FAIL). HTTP: `GET /api/v1/protocol/top-pairs` items ≤5 economic, gems absent; `?limit=6` → **400**. Frontend: hashed-chunk grep **`protocol-top-pairs` / `Top pairs (30d)`**. Do **not** grep hub `cLUNC / USD`. No leftover `DATABASE_URL`. No `EXPECT_SHA`. |

Postgres-only child tests stay Postgres-only. No CosmWasm in leftover verify except existing child pair tests. No new indexer integration suite.

## Rollout

1. Merge leftover *implement* (script + wiring) to `main`.
2. Operator: indexer migrate + restart (or auto-deploy if already on — still not this ticket’s job to flip the checkbox).
3. Confirm `_sqlx_migrations` **`…000` then `…001`** (operator Coolify DB / indexer `DATABASE_URL`; `…30000` is **#1305**) + `GET /api/v1/protocol/top-pairs`.
4. Operator: frontend rebuild; confirm **`protocol-top-pairs` / `Top pairs (30d)`**. Do not grep hub `cLUNC / USD`.
5. Operator: columbus-5 pair 1.17.0 via `upgrade-582-code-id-pin.sh` when ready (`UpdateConfig { pair_code_id }`); leftover stays open until cw2 matches.
6. Slice 7 leftover-*ops* / Cloud Agent QA: four walks per Outcome item 6 how-tos. Optional `make setup-cloud-localterra` when item 5 is still open; that is **not** implement-slice close. Do **not** close on probe SKIP. `make verify-issue-1300` (slice 2) does not execute those walks.
7. Close **#1300** only with **operator + leftover-*ops***: Coolify HTTP + sqlx attest (`…000` then `…001`) + pair cw2 **1.17.0** **and** the four walk notes. Green `make verify-issue-1300` never means close. Chain-probe SKIP/PASS is not item-5 evidence. Leave **#1264** and **#1279** open unless their own leftover-complete holds.

No wasm store from the leftover MR. No pause. No treasury rotate.

## Rollback

Coolify-era indexer rollback is **three-way** ([ADR 0006](./0006-indexer-health-git-sha.md) / [`rollback-decision.md`](../runbooks/rollback-decision.md) § Auto-deploy era). These two versions have **no** `down.sql` → **2(c) is unavailable**. If a restore of a pre-`20260921120000` image is required, that is **2(b)** (keep schema + hotfix that still ships N) or **2(a)** only when the baseline image already included both files.

Frontend: redeploy previous Vite app. Pair wasm: do not migrate backward; fix-forward on 1.17.0.

This leftover is not a chain halt.

## Integration completion criteria

Leftover-complete is **operator + leftover-*ops***. Green `make verify-issue-1300` **never** means close.

- `make verify-issue-1300` exists on `main` and is green locally (children + docs). That is the implement probe, not leftover-complete.
- Coolify: **#1300 attests `20260921120000` then `20260921120001` only** (`success=true`, apply order — operator Coolify DB / indexer `DATABASE_URL`). Same boot may also apply `20260921130000`; that row is **#1305**, not a #1300 FAIL and not a reason to keep #1300 open. Do **not** add a third version to `make verify-issue-1300`. Live **`GET /api/v1/protocol/top-pairs`** items ≤5 economic, gems absent; `?limit=6` → **400**.
- Coolify frontend serves `6d34da13+` with production HTTP marker **`protocol-top-pairs` / `Top pairs (30d)`**. Do **not** grep hub `cLUNC / USD` (that is **#1305 leftover-complete**). Vite `/protocol` is frontend evidence only.
- LocalTerra / manual (issue item 5, slice 7 leftover-*ops*): the four walks recorded per Outcome item 6 how-tos. `has-localterra` down is **not** leftover-complete — keep #1300 open. Chain-probe SKIP/PASS is not item-5 evidence. Child Vitest is not the walk. Default leftover verify probes `has-localterra` (down → SKIP) and does not execute the walks. `VERIFY1300_REQUIRE_HAS_LOCALTERRA=1` is fail-closed `has-localterra` only — does not execute the walks, not five-worker e2e. Defining `VERIFY1300_REQUIRE_CHAIN` as Playwright / `run_leftover_e2e` is a leftover-implement FAIL. Slice 2 leftover implement does **not** provision LocalTerra or record walks. Optional: Cloud Agent leftover-*ops* may `make setup-cloud-localterra` when item 5 is still open; that is **not** implement-slice close.
- columbus-5 listed pair cw2 **1.17.0** via `upgrade-582-code-id-pin.sh` (including `UpdateConfig { pair_code_id }`) is **required to close**. Otherwise **#1300 stays open**. Operator 2-of-3; leftover agents `UPGRADE582_PROBE_ONLY=1` only. A leftover comment that wasm is still outstanding is **not** a close path.
- `#1264` still open; envelope **2,710,000**; AC1 unmeasured unless an operator hash is attached.
- `#1279` still ops-bot; make did not close it.
- **#1302 landed** on the 1302–1304 stack ([#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305), `92c84406`). This leftover **must not revert it**. Hub wrap Coolify visual is **#1305 leftover-complete**, not #1300. Do **not** merge `origin/cac-design-issue-1302` as-is (overlap on `architecture.md` / `qa-invariants.md` / README). That branch already publishes `docs/adr/0009-verify-issue-1290-hub-wrap-labels.md` / **Q22**. A later **0009** follow-up MR that inserts without taking 0008/Q21 is allowed.
- `cac-design-issue-1276` / `1277` untouched.
- No founder card. No Coolify auto-deploy flip. No agent wasm store. No `DESIGN: APPROVE`.

## Links

- Sister leftover: [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) (PRs 1302–1304; hub wrap Coolify visual; sqlx `20260921130000`)
- [ADR 0004](./0004-terraclassic-retail-gas-census.md) — wrap+2hop envelope; #1264 AC1
- [ADR 0005](./0005-protocol-fee-multihop-hops.md) — sqlx unique-prefix leftover pattern
- [ADR 0006](./0006-indexer-health-git-sha.md) — Coolify attest / three-way rollback / #297 checkbox
- [ADR 0007](./0007-route-solve-remaining-failures.md) — Stay; wrap-enter leftover is #1218 LocalTerra
- Playbook: [`AGENTS_POST_MERGE_OPS_1300.md`](../../skills/AGENTS_POST_MERGE_OPS_1300.md)
- F6 operator path: [`scripts/upgrade-582-code-id-pin.sh`](../../scripts/upgrade-582-code-id-pin.sh) · Versions [`AGENTS_CW20_CODE_ID_PIN.md`](../../skills/AGENTS_CW20_CODE_ID_PIN.md)
- Verify (implement): `make verify-issue-1300`
- Authority: [agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297)
