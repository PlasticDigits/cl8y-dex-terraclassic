# ADR 0008: Post-merge leftover after PRs 1287–1298

## Status

Proposed ([#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300))

Ordinary leftover ops after the 1287–1298 stack landed on `origin/main` (first draft tip: `6d34da13`, hotfixes [#1299](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1299) / [#1301](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1301)). Later PRs **1302–1304** are also on `origin/main` (`729b097f`, merge [#1302](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls/1302) `92c84406`); those leftovers are sister [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305), not this ticket. This ADR does **not** retune envelopes, spawn a solver, flip Coolify auto-deploy, store columbus-5 wasm, spend, or expand custody/policy. Keep this file as `docs/adr/0008-post-merge-leftover-1287-1298.md` (this branch took **0008** first).

Playbook: [`skills/AGENTS_POST_MERGE_OPS_1300.md`](../../skills/AGENTS_POST_MERGE_OPS_1300.md) (**M1300-1–M1300-8**). Overview: [`architecture.md`](../architecture.md#post-merge-leftover-ops). Invariants: [`qa-invariants.md`](../qa-invariants.md) **Q21**.

Product decisions stay in the child ADRs/skills. Do **not** duplicate them here.

Keywords on [#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300) (`architecture`, `deploy`, `Design decision required`) are **not** approval to write `DESIGN: APPROVE`, flip Coolify, or file a founder card. Deploy/spend/custody/policy expansion stays under [agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297). This leftover does **not** expand that policy.

## Outcome

Close leftover **#1300** only after operator evidence for the merged stack — not after green child `make verify-issue-*` on a laptop.

1. **Coolify indexer.** Apply sqlx in filename order: `#1277` `20260921120000_traders_rolling_volume_numeric_38_0` then `#1263` `20260921120001_pair_volume_30d`. **#1300 attests those two versions only.** Same indexer boot may also apply `#1305` `20260921130000_traders_lifetime_heal_from_swaps`; that row is **[#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305)**, not a #1300 FAIL and not a reason to keep #1300 open. Do **not** add a third version to `make verify-issue-1300`. Operator attests `…000` then `…001` in `_sqlx_migrations` (`success=true`, apply order) via Coolify DB / indexer `DATABASE_URL` — not `scripts/lib/postgres-psql.sh`. After one aggregator tick, live **`GET /api/v1/protocol/top-pairs`** is ≤5 factory-listed economic rows (**P1263** / **I1263-6**). Trader `NUMERIC(38, 0)` has no public metric; leftover-complete for `#1277` is migrate applied + aggregator still running.
2. **Coolify frontend.** Rebuild from `6d34da13+`. Production-stable HTTP marker is **`protocol-top-pairs` / `Top pairs (30d)`** (hashed Vite chunk grep, same class as #701). Do **not** grep hub `cLUNC / USD` (that is #1305 leftover-complete). Dual-app skew during rebuild is expected ([ADR 0006](./0006-indexer-health-git-sha.md)).
3. **columbus-5 pair wasm.** Store+migrate in-tree pair **cw2 1.17.0** so listed fleet picks up **F6** `#1234` (`UpdateLimitOrderPrice` / `CleanLimitBook` gate) and **L24** `#1219` named min remaining. Operator path is [`scripts/upgrade-582-code-id-pin.sh`](../../scripts/upgrade-582-code-id-pin.sh) (`UPGRADE582_PAIR_VERSION` **1.17.0**). Factory on columbus-5 is already **1.10.0 / 11629** → `UPGRADE582_SKIP_FACTORY_MIGRATE=1` (script still asserts factory ≥ 1.9.0). The script **`UpdateConfig { pair_code_id }`** so new `CreatePair` instantiates 1.17.0 — skipping that is a leftover defect, not optional. Versions: [`skills/AGENTS_CW20_CODE_ID_PIN.md`](../../skills/AGENTS_CW20_CODE_ID_PIN.md) (pair **1.17.0**, fleet was **1.16.0**). Do **not** follow [`docs/runbooks/cw20-code-id-ops.md`](../runbooks/cw20-code-id-ops.md) Launch checklist (still factory **1.9.0** + pair **1.15.0** **RAN 2026-08-21** / 11602 / 11601). Leftover agents use `UPGRADE582_PROBE_ONLY=1` (read-only); they do **not** `store` / `migrate` / 2-of-3. Pair-first migrate still freezes gated writes; leftover must **not** invent a pair-only `terrad tx`.
4. **Keep `#1264` open.** Envelope stays **2,710,000**. Columbus-5 wrap+2hop USTC→USTR `gas_used` is still **unmeasured** (AC1). **G1264-4** (USTC Max/gas-gate is LUNC-only) survived `#1218` — no code follow-up.
5. **`#1279` leftover-complete is ops-bot** `QA_TEMPLATE.md` **1.5.1–1.5.11**. Green `make verify-issue-1279` does **not** close `#1279`.
6. **LocalTerra / manual** (issue leftover item 5 / **M1300-3** — not a substitute for Coolify; keep these walks). Close #1300 only after the four walks, **or** an explicit SKIP because `make has-localterra` is down. Child Vitest / laptop-green `make verify-issue-1218` (chain opt-in `VERIFY_ISSUE_1218_CHAIN=1`) is **not** the walk. `VERIFY1300_REQUIRE_CHAIN=1` runs `make has-localterra` plus those four walks (FAIL if chain down) — not invented five-worker e2e. `VERIFY1300_LEFTOVER_E2E=1` stays **SKIP until named** (**M1300-7**). Walks: `#1218` LUNC→USTR Route wrap-then-cUSTC; `#1255` 18-dec unlisted factory CW20; `#1219` dust ladder no wallet popup; `#1263` `/protocol` ≤5 economic rows.

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
- PR **#1302** (`92c84406`) **landed** on the 1302–1304 stack ([#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305)). This leftover **must not revert it**. Hub wrap Coolify visual (`cLUNC / USD` vs CEX `LUNC`) is **#1305 leftover-complete**, not #1300. `#1240` already has `make verify-issue-1240`. Do **not** merge unpublished `docs/adr/0008-verify-issue-1290-hub-wrap-labels.md` from [`origin/cac-design-issue-1302`](https://git.cl8y.com/code/cl8y-dex-terraclassic) (second 0008; #1305 already flags the collision).
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
- Reverting merged PR #1302. Merging unpublished `docs/adr/0008-verify-issue-1290-hub-wrap-labels.md` from `origin/cac-design-issue-1302` (second 0008). Treating `cac-design-issue-1276` / `cac-design-issue-1277` as this stack.
- Absorbing or waiting on sqlx `20260921130000_traders_lifetime_heal_from_swaps` (**#1305**). Adding a third version to `make verify-issue-1300`.
- Grepping Coolify hub `cLUNC / USD` as this leftover’s frontend HTTP marker (**#1305**).
- Reopening closed parents (#1286/#1287, #1234, #1265, #1240, #1277, #1285, #1255, #1219, #1218, #1263) for ops/QA.
- Flipping the indexer Coolify **auto-deploy checkbox**, scraping Coolify logs, publishing app UUIDs/tokens, or expanding CAC `COOLIFY_APP_MAP` ([ADR 0006](./0006-indexer-health-git-sha.md) / #297).
- Agent-executed columbus-5 `store` / `migrate`, treasury rotate, or pause.
- A founder card for this ordinary leftover. HMAC / `autonomy.rs` self-approval. `DESIGN: APPROVE`.
- Inventing `down.sql` for `20260921120000` / `20260921120001` (none in `indexer/migrations/revert/`).
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
| #297 | Coolify **migrate/rebuild** is ordinary leftover (same class as #701). Coolify **auto-deploy checkbox** is still #1276 / #297 — out of this ticket. columbus-5 pair store+migrate uses [`scripts/upgrade-582-code-id-pin.sh`](../../scripts/upgrade-582-code-id-pin.sh) (not the August 1.15.0 checklist); leftover agents `UPGRADE582_PROBE_ONLY=1` and SKIP/FAIL live, they do not store. |
| PR #1302 | Landed (`92c84406`) on **#1305**. Do **not** revert. Do **not** merge `origin/cac-design-issue-1302` (unpublished second 0008). Hub wrap Coolify visual is **#1305 leftover-complete**. |
| LocalTerra walks | Keep issue item 5. Close #1300 only after the four walks **or** explicit SKIP (`make has-localterra` down). `VERIFY1300_REQUIRE_CHAIN=1` = `has-localterra` + those four. Child Vitest is not the walk. |

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
| Treat green child verify as leftover-complete | Repeats the #701/#702 failure mode: prod schema/wasm/UI stay on the previous tip. |
| Close #1264 because PR 1293 merged | AC1 `gas_used` is still unmeasured. Envelope must not rise on hope. |
| Raise combo overhead “just in case” | Forbidden without class A measurement (**G1264-6**). |
| Revert merged PR #1302 / treat it as a leftover defect | Already on `origin/main` (`92c84406`). Sister leftover is **#1305**. Hub wrap Coolify visual is **#1305 leftover-complete**. |
| Merge `origin/cac-design-issue-1302` ADR 0008 | Unpublished second `0008` (`0008-verify-issue-1290-hub-wrap-labels.md`). This branch keeps `0008-post-merge-leftover-1287-1298.md`. |
| Wait on / FAIL #1300 because `20260921130000` is present or missing | That version is **#1305**. #1300 attests `…000` then `…001` only. Same boot may apply `…30000`; not a #1300 FAIL. |
| Drop issue item 5 (Coolify frontend + child verifies enough) | Rejected: leftover-complete keeps the four LocalTerra/manual walks (or explicit `has-localterra` down SKIP). Child Vitest is not the walk. |
| Fold #1276 auto-deploy checkbox into #1300 | Different authority (#297 deploy policy) and different leftover-complete (`EXPECT_SHA`). |
| One sqlx file combining NUMERIC + `pair_volume_30d` | Would rewrite applied checksums; collision already resolved as `…000` / `…001`. |
| Agent store+migrate pair wasm from leftover verify | Deploy/custody. Operator `upgrade-582-code-id-pin.sh` only (`UPGRADE582_PROBE_ONLY=1` for leftover agents). |
| Founder card / `DESIGN: APPROVE` | Ordinary leftover. Keywords are not approval. |
| Invent `down.sql` so 2(c) rollback applies | Partial suffix revert is still **2(b)** ([ADR 0006](./0006-indexer-health-git-sha.md)). These two versions have no downs. |

## Complexity added / removed

**Added (docs + leftover verify only):** Q21 registry, playbook, `make verify-issue-1300` (implement slice), Coolify migrate ordering, columbus-5 wasm checklist, explicit do-not-close for #1264.

**Removed:** Ambiguity that child-green == leftover-complete for this stack; the false `verify-issue-582` claim once implement fixes it.

**No new:** HTTP routes, sqlx files, wasm messages, gas constants, solver knobs.

## Migration

Already in git:

1. [`indexer/migrations/20260921120000_traders_rolling_volume_numeric_38_0.sql`](../../indexer/migrations/20260921120000_traders_rolling_volume_numeric_38_0.sql) — expand `traders.volume_*` / `total_volume` to `NUMERIC(38, 0)`.
2. [`indexer/migrations/20260921120001_pair_volume_30d.sql`](../../indexer/migrations/20260921120001_pair_volume_30d.sql) — `pair_volume_30d` rollup table.

Production: indexer boot `sqlx::migrate!()` (no `set_ignore_missing`). Operator confirms `_sqlx_migrations` has **#1300** versions `20260921120000` and `20260921120001` `success=true` **in that order** via Coolify DB / indexer `DATABASE_URL`. A `20260921130000` row, if present, is **#1305** — not a #1300 FAIL. Then one `refresh_pair_volumes` / `refresh_pair_volumes_30d` tick. `make verify-issue-1300` live does **not** query that table and does **not** require `…30000`.

Pair wasm: columbus-5 store of optimized pair artifact + migrate listed pairs from cw2 **1.16.0** → **1.17.0** via [`scripts/upgrade-582-code-id-pin.sh`](../../scripts/upgrade-582-code-id-pin.sh). Factory is already **1.10.0 / 11629** → `UPGRADE582_SKIP_FACTORY_MIGRATE=1`. The script **does** `UpdateConfig { pair_code_id }` so new `CreatePair` instantiates 1.17.0 — skipping that is a leftover defect. Do **not** treat [`docs/runbooks/cw20-code-id-ops.md`](../runbooks/cw20-code-id-ops.md) “RAN 2026-08-21” as leftover-complete. Versions: [`skills/AGENTS_CW20_CODE_ID_PIN.md`](../../skills/AGENTS_CW20_CODE_ID_PIN.md). Leftover agents: `UPGRADE582_PROBE_ONLY=1` only.

No dApp env keys. No new revert files.

## Observability

- Indexer schema: operator attests `_sqlx_migrations` rows for **`20260921120000` then `20260921120001` only** (`success=true`, apply order) via **Coolify DB / indexer `DATABASE_URL`**. Do **not** use `scripts/lib/postgres-psql.sh` against prod (LocalTerra/dev helper). Presence of `20260921130000` is **#1305**, not a #1300 FAIL. `make verify-issue-1300` live is **HTTP only**: indexer **`GET /api/v1/protocol/top-pairs`** plus frontend **`protocol-top-pairs` / `Top pairs (30d)`** (hashed Vite chunk grep, same class as #701). Do **not** grep hub `cLUNC / USD`. Do **not** copy #1276 `EXPECT_SHA`. Do **not** invent leftover `DATABASE_URL`.
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
| Treat child Vitest as LocalTerra leftover-complete | Forbidden. Close #1300 only after the four walks **or** explicit SKIP (`make has-localterra` down). `VERIFY1300_REQUIRE_CHAIN=1` is `has-localterra` + those four, not five-worker e2e. |
| Agent store wasm / flip auto-deploy | Out of authority. #297 for policy expansion; operator `upgrade-582-code-id-pin.sh` for wasm. Leftover agents `UPGRADE582_PROBE_ONLY=1` only. |
| Colliding sqlx rename after prod apply | Checksum mismatch; migrator rejects. Do not rename applied files. |
| Rollback image without schema | `NUMERIC(38, 0)` and `pair_volume_30d` stay; old binary may fail migrate-on-boot if versions missing from the binary — **2(b)** keep schema + hotfix that still ships N ([ADR 0006](./0006-indexer-health-git-sha.md)). |

## Ordered implementation slices

1. **Docs registry (this design PR).** ADR 0008 + architecture pointer + **Q21** + playbook **M1300**. Pointers on ADR 0004/0005/0006/0007. No verify script in the design PR.
2. **Leftover verify script (implement).** `scripts/qa/verify-issue-1300.sh` + `Makefile` `verify-issue-1300` + `docs/testing.md` / `scripts/qa/README.md` / `AGENTS.md` wiring + child-skill “Coolify leftover: #1300” one-liners. Children: **1287, 1234, 1265, 1240, 1279, 1277, 1264, 1285, 1255, 1219, 1218, 1263**. Copy `require_live()` from #701/#702. Live frontend HTTP: hashed-chunk grep for **`protocol-top-pairs` / `Top pairs (30d)`** (same class as #701). Do **not** grep hub `cLUNC / USD`. Do **not** copy #1276 `EXPECT_SHA` leftover-complete. Do **not** attest `20260921130000`.
3. **Comment-only gap.** `verify-issue-1234.sh`: invoke `make verify-issue-582` **or** remove the header claim. Same leftover implement slice.
4. **Coolify indexer (operator).** Migrate `…000` then `…001`; attest those two `_sqlx_migrations` rows via Coolify DB / indexer `DATABASE_URL`; wait rollup; probe `GET /api/v1/protocol/top-pairs` (≤5 economic; `?limit=6` → 400). `…30000` if applied is **#1305**. Ordinary leftover — not #297.
5. **Coolify frontend (operator).** Rebuild `6d34da13+`. Production marker: **`protocol-top-pairs` / `Top pairs (30d)`**. Vite `/protocol` is frontend evidence only. Do not grep hub `cLUNC / USD`.
6. **columbus-5 pair wasm (operator).** [`scripts/upgrade-582-code-id-pin.sh`](../../scripts/upgrade-582-code-id-pin.sh): `UPGRADE582_PAIR_VERSION` **1.17.0**, `UPGRADE582_SKIP_FACTORY_MIGRATE=1`, `UpdateConfig { pair_code_id }`. Leftover agents: `UPGRADE582_PROBE_ONLY=1` — do not store / migrate / 2-of-3 / invent a pair-only `terrad tx`.
7. **LocalTerra / manual (keep issue item 5).** Four walks: #1218 LUNC→USTR Route wrap-then-cUSTC; #1255 18-dec unlisted factory CW20; #1219 dust ladder no wallet popup; #1263 `/protocol` ≤5 economic rows. Child Vitest is **not** the walk. `VERIFY1300_REQUIRE_CHAIN=1` runs `make has-localterra` plus those four (FAIL if chain down). Chain down without REQUIRE_CHAIN → leftover-complete may record explicit SKIP (`make has-localterra` down). Leftover stacked Playwright is **manual**; `VERIFY1300_LEFTOVER_E2E=1` **SKIP until named**. Child verifies already run their own specs (`wrap-swap.spec.ts` / #1218, ladder UI / #1219). `protocol-page.spec.ts` does **not** assert Top pairs. Do not invent five-worker leftover coverage. `e2e-tx` stays **1 worker**. `:3173` CORS: do not leak `PLAYWRIGHT_WEB_PORT`.
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
| T8 | `VERIFY1300_REQUIRE_CHAIN=1` | Runs `make has-localterra` plus the four leftover walks (#1218 LUNC→USTR Route wrap-then-cUSTC; #1255 18-dec unlisted factory CW20; #1219 dust ladder no wallet popup; #1263 `/protocol` ≤5 economic). Child Vitest is **not** the walk. Chain down → **FAIL** (not SKIP). Without REQUIRE_CHAIN, chain down → leftover-complete may record explicit SKIP (`make has-localterra` down). `VERIFY1300_LEFTOVER_E2E=1` remains **SKIP until named** (stacked Playwright; do not invent five-worker coverage). |
| T9 | `verify-issue-1234` after slice 3 | Claim matches behavior |
| T10 | Live Coolify after migrate | Operator: **#1300** sqlx versions `20260921120000` then `20260921120001` `success=true` via Coolify DB / indexer `DATABASE_URL` (`20260921130000` is **#1305**, not a #1300 FAIL). HTTP: `GET /api/v1/protocol/top-pairs` items ≤5 economic, gems absent; `?limit=6` → **400**. Frontend: hashed-chunk grep **`protocol-top-pairs` / `Top pairs (30d)`**. Do **not** grep hub `cLUNC / USD`. No leftover `DATABASE_URL`. No `EXPECT_SHA`. |

Postgres-only child tests stay Postgres-only. No CosmWasm in leftover verify except existing child pair tests. No new indexer integration suite.

## Rollout

1. Merge leftover *implement* (script + wiring) to `main`.
2. Operator: indexer migrate + restart (or auto-deploy if already on — still not this ticket’s job to flip the checkbox).
3. Confirm `_sqlx_migrations` **`…000` then `…001`** (operator Coolify DB / indexer `DATABASE_URL`; `…30000` is **#1305**) + `GET /api/v1/protocol/top-pairs`.
4. Operator: frontend rebuild; confirm **`protocol-top-pairs` / `Top pairs (30d)`**. Do not grep hub `cLUNC / USD`.
5. Operator: columbus-5 pair 1.17.0 via `upgrade-582-code-id-pin.sh` when ready (`UpdateConfig { pair_code_id }`); leftover stays open until cw2 matches.
6. Manual/LocalTerra four walks, **or** explicit SKIP (`make has-localterra` down).
7. Close **#1300** with probe evidence. Leave **#1264** and **#1279** open unless their own leftover-complete holds.

No wasm store from the leftover MR. No pause. No treasury rotate.

## Rollback

Coolify-era indexer rollback is **three-way** ([ADR 0006](./0006-indexer-health-git-sha.md) / [`rollback-decision.md`](../runbooks/rollback-decision.md) § Auto-deploy era). These two versions have **no** `down.sql` → **2(c) is unavailable**. If a restore of a pre-`20260921120000` image is required, that is **2(b)** (keep schema + hotfix that still ships N) or **2(a)** only when the baseline image already included both files.

Frontend: redeploy previous Vite app. Pair wasm: do not migrate backward; fix-forward on 1.17.0.

This leftover is not a chain halt.

## Integration completion criteria

- `make verify-issue-1300` exists on `main` and is green locally (children + docs).
- Coolify: **#1300 attests `20260921120000` then `20260921120001` only** (`success=true`, apply order — operator Coolify DB / indexer `DATABASE_URL`). Same boot may also apply `20260921130000`; that row is **#1305**, not a #1300 FAIL and not a reason to keep #1300 open. Do **not** add a third version to `make verify-issue-1300`. Live **`GET /api/v1/protocol/top-pairs`** items ≤5 economic, gems absent; `?limit=6` → **400**.
- Coolify frontend serves `6d34da13+` with production HTTP marker **`protocol-top-pairs` / `Top pairs (30d)`**. Do **not** grep hub `cLUNC / USD` (that is **#1305 leftover-complete**). Vite `/protocol` is frontend evidence only.
- LocalTerra / manual (issue item 5): the four walks (#1218 / #1255 / #1219 / #1263) recorded, **or** explicit SKIP because `make has-localterra` is down. Child Vitest is not the walk. `VERIFY1300_REQUIRE_CHAIN=1` is `has-localterra` + those four, not five-worker e2e.
- columbus-5 listed pair cw2 **1.17.0** via `upgrade-582-code-id-pin.sh` (including `UpdateConfig { pair_code_id }`), **or** leftover comment records that wasm is still outstanding (then **#1300 stays open**).
- `#1264` still open; envelope **2,710,000**; AC1 unmeasured unless an operator hash is attached.
- `#1279` still ops-bot; make did not close it.
- **#1302 landed** on the 1302–1304 stack ([#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305), `92c84406`). This leftover **must not revert it**. Hub wrap Coolify visual is **#1305 leftover-complete**, not #1300. Do **not** merge unpublished `docs/adr/0008-verify-issue-1290-hub-wrap-labels.md` from `origin/cac-design-issue-1302`.
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
