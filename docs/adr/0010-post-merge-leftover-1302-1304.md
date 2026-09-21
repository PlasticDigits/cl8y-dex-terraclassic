# ADR 0010: Post-merge leftover after PRs 1302–1304

## Status

Proposed ([#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305))

Ordinary leftover ops after PRs **1302–1304** landed on `origin/main` (`729b097f`). This ADR does **not** relabel `/protocol`, change heal SQL, flip Coolify auto-deploy, store wasm, spend, expand custody/policy, or write `DESIGN: APPROVE`. Keywords on [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) (`architecture`, `deploy`, `Design decision required`) are **not** approval. Deploy/spend/custody/policy expansion stays under [agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297).

Playbook: [`skills/AGENTS_POST_MERGE_OPS_1305.md`](../../skills/AGENTS_POST_MERGE_OPS_1305.md) (**M1305-1–M1305-8**). Overview: [`architecture.md`](../architecture.md#post-merge-leftover-ops). Invariants: [`qa-invariants.md`](../qa-invariants.md) **Q23**.

Product decisions stay in child skills. Do **not** duplicate **P1240** or **R1277** here.

**Numbering (leftover 3).** ADR **0005** on `main` is protocol-fee hops ([#1269](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1269)). ADR **0008** / **Q21** `{#post-merge-ops-1300}` are reserved by sister leftover [#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300) (`docs/adr/0008-post-merge-leftover-1287-1298.md` on `origin/cac-design-issue-1300`). ADR **0009** / **Q22** `{#ops-hub-wrap-1302}` are reserved by the hub-wrap verify bundle ([#1302](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1302), `docs/adr/0009-verify-issue-1290-hub-wrap-labels.md` on `origin/cac-design-issue-1302` after that branch re-slotted off a second 0008). This leftover is **0010** / **Q23**. Keep this file as `docs/adr/0010-post-merge-leftover-1302-1304.md`. Do **not** merge `origin/cac-design-issue-1300`, `origin/cac-design-issue-1302`, or `origin/cac-design-issue-1277` as-is.

## Outcome

Close leftover **#1305** only after operator evidence for this stack — not after green child `make verify-issue-1290` / `make verify-issue-1277` on a laptop. HTTP `[PASS]` is **not** sqlx leftover-complete. Child Playwright is **not** leftover-complete.

1. **Coolify `/protocol` visual** (issue leftover 1 / related #1290 AC). After frontend rebuild from `729b097f+`, **operator glance** on live `dex.cl8y.com/protocol`: hub **`cUSTC / USD`** vs CEX tabs **`USTC`** / **`LUNC`** / **`vFDUSD`**. Selected vFDUSD tab is **not** flattened to **`VFDUSD`**. Hub `<dt>` is assembled at render (`{HUB_PRICE_TICKER_LABEL[ticker]} / USD` in `ProtocolDexHubPrices.tsx`) — production chunks keep `"cLUNC"` / `"cUSTC"` and `" / USD"`, **not** the concatenated display string. Leftover HTTP (hashed Vite chunk grep, same class as #701 `pool-row-vol` / `volume_usd_24h`) pins **literals that survive minify**:
   - `cLUNC wrap` (`wrapAriaName`)
   - `HUB_PRICE_TICKER_LABEL` values `'cLUNC'` and `'cUSTC'` (`hubPriceTicker.ts`)
   - oracle map `'vFDUSD'` (`protocolOracleTicker.ts`)

   Grepping concatenated **`cLUNC / USD`** / **`cUSTC / USD`** will FAIL a correct Coolify rebuild and look leftover-incomplete. Selected-tab casing stays operator glance (CSS-in-JS is not a stable curl assert). Child `protocol-page` still has a monthly **`YY-MM`** tick flake unrelated to hub wrap (#703) — leftover-complete **must not** wait on that. This glance + minify-safe HTTP is **#1305 leftover-complete**, not #1300 (that leftover greps **`protocol-top-pairs` / `Top pairs (30d)`**) and not #1302 leftover-complete (unpublished ADR 0009 treats the glance as this sister). Do **not** infer leftover-complete from the 2026-09-21 06:46 issue close comment (`GET /health` SHA + `top-pairs`) — that is **#1300** / [ADR 0006](./0006-indexer-health-git-sha.md), not this contract.

2. **Coolify indexer migrate** (issue leftover 2). Apply sqlx in filename order: #1300’s `20260921120000_traders_rolling_volume_numeric_38_0` then `20260921120001_pair_volume_30d`, then **this leftover** `20260921130000_traders_lifetime_heal_from_swaps`. **#1305 attests `…30000` only.** `…000` / `…001` stay [#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300). The [#1276](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1276) indexer auto-deploy checkbox is **still off** until that leftover / #297. **If the checkbox is off, leftover 2 is a manual Coolify indexer deploy of `729b097f+`.** Do **not** wait on flipping the checkbox. Healthy `GET /health` (`status=ok`, with or without `git_sha`) is **not** `_sqlx_migrations` evidence ([ADR 0006](./0006-indexer-health-git-sha.md)). Operator attests `_sqlx_migrations` `20260921130000` `success=true` via Coolify DB / indexer `DATABASE_URL` — not `scripts/lib/postgres-psql.sh`. After boot, D5 `heal_trader_lifetime_from_swaps_if_needed` is a **no-op** (`Ok(false)`, no `traders_healed` info) **or** logs that line **once** then subsequent polls no-op. Heal errors stay `tracing::error` and must not abort `run_indexer`.

3. **ADR collision** (issue leftover 3). This file **is** the re-slot. Implement **inserts** 0010 / Q23 / architecture `#post-merge-leftover-ops` — do **not** whole-file checkout overlap docs from sister design tips. Do **not** add `0008-*.md` or `0009-*.md` on this leftover MR.

4. **Optional I10** (issue leftover 4). USD-only lifetime skew **must not** re-trip the poller gate. Gate `SQL_TRADER_LIFETIME_DIVERGES` stays **trades + raw `total_volume` only**. Do **not** add `total_volume_usd` to the EXISTS. Optional implement test: matching trades+raw + wrong USD → `trader_lifetime_diverges_from_swaps == false` and heal returns `false`. Not a leftover-complete gate.

5. **Merge recipe** (issue leftover 5). Future lands follow [`docs/runbooks/forgejo-pr-merge.md`](../runbooks/forgejo-pr-merge.md): dismiss the `.* @code/maintainers` self-request, then normal `fj pr merge`. Do **not** `force_merge`. #1302 / #1303 used `force_merge` before that recipe was followed; both merge commits **are** ancestors of `origin/main` (`92c84406`, `a7919548`) — do **not** rewrite them. #1304 used dismiss + normal merge (`729b097f`).

Sister leftover for PRs 1287–1298 remains **#1300**. Do **not** close #1300 from this ticket. Do **not** revert #1302.

## Context

Woodpecker `ci/woodpecker/pr/woodpecker` and `ci/woodpecker/push/woodpecker` succeeded on these merges. Local gitleaks was clean. Workstation verify already recorded on the issue: `make verify-issue-1290` 4/4 (Playwright `protocol-page`, 5 workers); `make verify-issue-1277` 6/6 after #1304. That workstation Playwright is **not** leftover-complete (monthly `YY-MM` tick flake is #703, not hub wrap).

| PR | Linked | On `main` | Leftover class |
|----|--------|-----------|----------------|
| 1302 | #1290 / #1240 | `make verify-issue-1290` bundle (P1240 + `hubPriceTicker` + Playwright) | **Coolify frontend visual**; unpublished ADR 0009 wiring gaps stay on `cac-design-issue-1302` / optional follow-up PR **#1306**, not this Coolify glance |
| 1303 | #1277 / #1292 | Trader lifetime heal from `swap_events` (migrate + gated poller before D5) | **Coolify** migrate `20260921130000`; did **not** compile (`Executor` not implemented for `&mut Transaction`) |
| 1304 | #1303 / #1277 | sqlx 0.8 `&mut **tx` in `apply_trader_lifetime_heal_tx` (match `hub_prices`) | **code-done**; child `make verify-issue-1277` stays the compile/docs gate |

**Out of this leftover stack (keyword overlap is not enough):**

- Sister [#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300) — Coolify `…000`/`…001`, columbus-5 pair 1.17.0, LocalTerra four walks, #1264 stay-open, #1279 ops-bot. Do **not** attest `…000`/`…001` here. Do **not** grep `protocol-top-pairs` as this leftover’s frontend marker.
- [#1276](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1276) / [ADR 0006](./0006-indexer-health-git-sha.md) — indexer Coolify auto-deploy checkbox + `VERIFY1276_EXPECT_SHA`. Do **not** copy `EXPECT_SHA` leftover-complete onto #1305. Do **not** treat healthy `/health` as sqlx evidence. Do **not** invent `VERIFY1305_LEFTOVER_COMPLETE=1` (#1276 uses that flag for SHA tip-match; #701 does not have it).
- The 2026-09-21 06:46 issue close comment (`top-pairs` + `/health` SHA) — **#1300** / 0006, not this leftover-complete contract.
- `origin/cac-design-issue-1277` — unpublished `docs/adr/0005-trader-rolling-volume-numeric.md` collides with protocol-fee **0005** already on `main`. Issue **#1277** code is on `main` via PRs 1292 / 1303 / 1304; the **design branch** is not this stack.
- Relabeling hub/oracle copy, ticker ids, or Venus headings (**P1240-8**). Product already shipped in #1290.
- Expanding the heal gate to USD (would re-trip every poller loop whenever #553 USD is stale vs raw).

`CODEOWNERS` is `.* @code/maintainers`. Forgejo `block_on_official_review_requests` makes `fj pr merge` return HTTP **405** until the team self-request is dismissed. That is leftover 5, not a branch-protection change.

## Non-goals

- Closing or absorbing [#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300). Attesting sqlx `…000` / `…001`. Grepping Coolify **`protocol-top-pairs`**. columbus-5 pair wasm. LocalTerra four walks. Closing #1264 or #1279.
- Merging unpublished `origin/cac-design-issue-1300` / `1302` / `1277` as-is. Taking ADR **0008** / **0009** or **Q21** / **Q22**. Whole-file checkout of `architecture.md` / `qa-invariants.md` / `docs/README.md` from those tips.
- Relabeling `/protocol`, changing `HUB_PRICE_TICKER_LABEL`, adding a `clunc` hub path, or scraping production HTML as #1302 leftover-complete.
- Grepping concatenated **`cLUNC / USD`** / **`cUSTC / USD`** in hashed Vite chunks.
- Changing heal SQL, adding `total_volume_usd` to `SQL_TRADER_LIFETIME_DIVERGES`, or rewriting #1303/#1304.
- Flipping Coolify auto-deploy, HMAC / `autonomy.rs`, founder card, wallet spend, or wasm store.
- Copying #1276 `EXPECT_SHA`. Inventing leftover `DATABASE_URL` for `make verify-issue-1305`. Inventing `VERIFY1305_LEFTOVER_COMPLETE=1`. Using `postgres-psql.sh` against prod. Treating HTTP PASS or `GET /health` as `_sqlx_migrations` leftover-complete.
- Closing leftover-complete on child Playwright `protocol-page` or #703 monthly `YY-MM` ticks.
- Inferring leftover-complete from the 06:46 issue close comment.
- Rewriting #1302 / #1303 history. Using `force_merge` on later PRs.
- Opening a design-only PR from `cac-design-issue-1305`.
- `DESIGN: APPROVE`.

## Decision

### Component / state / interface

| Layer | Change in *this* leftover | Already on `main` |
|-------|---------------------------|-------------------|
| Frontend `/protocol` | Operator Coolify rebuild + glance + minify-safe HTTP attest | Hub wrap labels (**P1240** / #1290); `verify-issue-1290` sketch |
| Indexer schema | None (heal is data DML) | Widen `20260921120000`; 30d `20260921120001`; heal `20260921130000` |
| Indexer runtime | Manual Coolify indexer deploy of `729b097f+` while #1276 checkbox is off; poller heal no-op or one log | `heal_trader_lifetime_from_swaps_if_needed` before D5; sqlx `&mut **tx` |
| HTTP APIs | No new fields | Unscoped trader JSON still `bd_plain_string`; hub ids `custc`/`lunc` |
| Verify | Implement `make verify-issue-1305` (children **1290** with `VERIFY_ISSUE_1290_SKIP_E2E=1` by default, **1277**) | Children exist; no 1305 wrapper yet |
| Merge process | Docs only (dismiss then merge) | Runbook already forbids `force_merge` |

No CosmWasm, no `VITE_*`, no CAC `COOLIFY_APP_MAP`.

### Affected invariants

| ID | Owner | This leftover |
|----|-------|---------------|
| **P1240-1–P1240-8** | PROTOCOL_STATS / PROTOCOL_HUB | Coolify visual attest only |
| **R1277-1–R1277-8** | trader rolling NUMERIC | Heal migrate + D5 no-op; USD cap **R1277-5** unchanged |
| **D5** (#577) | volume window decay | Heal still runs **before** `refresh_all_volume_windows` |
| **H1276-*** | ADR 0006 | Not this ticket. Heal version has **no** `down.sql` → Coolify rollback **2(c) unavailable** for `…30000` (same three-way as 0006). Checkbox-off means leftover 2 is **manual** indexer deploy. |
| **Q21** / **M1300** | unpublished #1300 ADR 0008 | Sister; do not steal |
| **Q22** / **B1290** | unpublished #1302 ADR 0009 | Sister wiring; glance is here |
| **Q23** / **M1305** | this ADR | New |

### Alternatives

| Alternative | Why not |
|-------------|---------|
| Merge `cac-design-issue-1300` / `1302` / `1277` as-is | Dual `0008-*.md`; 1277’s `0005-trader-*.md` vs protocol-fee 0005; overlap-file clobber. Issue leftover 3 forbids it. |
| Take 0008 or 0009 on this ticket | Collides with unpublished sister designs. Re-slot to **0010**. |
| Fold Coolify hub glance into #1300 | #1300 already named `protocol-top-pairs` as its HTTP marker and assigned hub wrap to #1305. |
| Fold Coolify hub glance into #1302 leftover-complete | Unpublished ADR 0009 leftover-complete is the wiring MR, not production HTML. Splitting close-out twice. |
| Grep concatenated `cLUNC / USD` like #701 greps `pool-row-vol` | Hub copy is assembled at render; minify splits ticker and `" / USD"`. False leftover-incomplete. |
| Wait leftover-complete on child `protocol-page` | Monthly `YY-MM` tick flake is #703, not hub wrap. |
| Expand heal gate to USD | Re-trips poller whenever #553 USD drifts; issue leftover 4 is explicit Stay. |
| `VERIFY1305_LEFTOVER_COMPLETE=1` | #701 does not have it; #1276 uses it for SHA tip-match this ticket forbids. HTTP PASS is not sqlx leftover-complete. |
| Treat `GET /health` as sqlx evidence | ADR 0006: liveness ≠ `_sqlx_migrations`. |
| `force_merge` because CODEOWNERS 405 | Forbidden. Dismiss self-request. |
| Agent Coolify UI / #297 checkbox | Out of authority. Ordinary leftover migrate/rebuild is operator, not policy expansion. |

### Complexity added / removed

**Added:** leftover playbook **M1305**, **Q23**, this ADR **0010**, implement wrapper `verify-issue-1305`, operator attest of `…30000` + hub wrap visual. Optional USD-gate test.

**Removed:** the gap where PRs 1302–1304 had no leftover card (heal migrate sat only as a comment on #1300; hub wrap visual had no Coolify owner; ADR 0008 was claimed twice). sqlx 0.8 compile hole closed by #1304 (not reopened here).

Net: ops registry without a second product surface and without dual ADR numbers.

## Migration

sqlx `20260921130000_traders_lifetime_heal_from_swaps` is **already on `main`**. Leftover is **apply in production**, not a new file. Order: `…000` → `…001` → `…30000` (unique prefix rule, [ADR 0005](./0005-protocol-fee-multihop-hops.md) / [ADR 0006](./0006-indexer-health-git-sha.md)). Do **not** rename after prod apply (checksum mismatch). No `down.sql`. One-shot DML is idempotent with the gated Rust heal (INSERT `ON CONFLICT DO NOTHING`, lifetime UPDATE, leftover-zero, #553 USD refresh, leftover-USD NULL).

Indexer: while the #1276 auto-deploy checkbox is **off**, leftover 2 is a **manual** Coolify indexer deploy of `729b097f+`. Frontend: Coolify auto-deploy (already on for Vite) should rebuild `729b097f+`. Dual-app skew vs indexer migrate is expected.

No wasm. No pause. No treasury rotate.

## Observability

- Indexer: `traders_healed lifetime totals from swap_events (GitLab #1277)` at **info** when heal SQL ran; `trader lifetime heal failed` at **error** (poller continues). Leftover-complete after migrate: absence of repeated info on later polls, or one info then silence. Do **not** treat `GET /health` as this attest.
- No public heal metric. Do not add `/health` keys. Do not scrape Coolify `/status` JSON or paste app UUIDs/tokens on the issue.
- Frontend leftover HTTP: hashed-chunk grep **`cLUNC wrap`**, `'cLUNC'`, `'cUSTC'`, `'vFDUSD'`. Operator glance: hub **`cUSTC / USD`** vs CEX **`LUNC`** / selected not **`VFDUSD`**. Do **not** grep concatenated **`cLUNC / USD`**.
- Verify script prints `[PASS]` / `[FAIL]` / SKIP; exit `1` iff `FAIL > 0`. Sibling `VERIFY*_IID` unreachable = FAIL. Keep only `VERIFY1305_REQUIRE_LIVE` / `VERIFY1305_IID=1305` fail-closed on unreachable. Do **not** copy `EXPECT_SHA`. Do **not** invent `VERIFY1305_LEFTOVER_COMPLETE`. HTTP PASS is not sqlx leftover-complete; `success=true` stays operator Coolify DB.

## Failure modes

| Failure | Behavior |
|---------|----------|
| Child 1290 or 1277 FAIL | Stack FAIL. Do not skip children. Default child 1290 uses `VERIFY_ISSUE_1290_SKIP_E2E=1` so a #703 `YY-MM` flake does not fail the leftover wrapper. |
| Coolify hosts unreachable | SKIP unless `VERIFY1305_REQUIRE_LIVE=1` or `VERIFY1305_IID=1305` → **FAIL** |
| HTTP PASS while `…30000` unattested | Not leftover-complete. Sqlx `success=true` is operator Coolify DB only. |
| `…30000` missing after indexer image of `729b097f+` | Leftover-complete FAIL (operator). If #1276 checkbox is off, leftover 2 was never **manually** deployed. |
| Healthy `GET /health` used as sqlx evidence | Forbidden. Liveness ≠ `_sqlx_migrations` (ADR 0006). |
| `…30000` present while #1300’s `…000` missing | sqlx cannot skip; boot would have failed migrate. Not a #1305-only state. |
| Repeated `traders_healed` every poller tick | Gate bug (likely USD folded into EXISTS). Fix-forward; do not expand the gate. |
| Heal compile regression (`&mut *tx`) | `make verify-issue-1277` FAIL. Keep `&mut **tx`. |
| Grep concatenated `cLUNC / USD` as this leftover’s HTTP marker | Forbidden. False leftover-incomplete on a correct rebuild. |
| Grep `protocol-top-pairs` as this leftover’s marker | Forbidden. That is #1300. |
| Infer leftover-complete from the 06:46 close comment | Forbidden. That is #1300 / 0006. |
| Merge sister design branches as-is | ADR collision. Insert 0010/Q23 only. |
| `fj pr merge` HTTP 405 | Dismiss maintainers self-request; retry normal merge. Not `force_merge`. |
| Agent flips Coolify auto-deploy | Forbidden (#297). |
| `VERIFY1305_IID=1305` without live hosts | FAIL (unreachable), not leftover-complete PASS. No `LEFTOVER_COMPLETE` flag. |

## Ordered implementation slices

| Slice | Who | Deliverable | Blocks |
|-------|-----|-------------|--------|
| **0 — this design** | design_author | ADR **0010**, architecture `#post-merge-leftover-ops`, **Q23**, playbook **M1305**, README index, runbook merge-recipe note, child-skill one-liners. Branch `cac-design-issue-1305` only (no design-only PR). | Slice 1 |
| **1 — leftover verify** | implement | `scripts/qa/verify-issue-1305.sh` + `Makefile` `verify-issue-1305` + `docs/testing.md` / `scripts/qa/README.md` / `AGENTS.md` wiring. Children **1290** (default `VERIFY_ISSUE_1290_SKIP_E2E=1` — docs + `hubPriceTicker` + `verify-issue-1240`) and **1277**. Copy `require_live()` from #701 (`REQUIRE_LIVE` / `IID=1305` only — **no** `LEFTOVER_COMPLETE`). Live frontend HTTP: minify-safe **`cLUNC wrap`**, `'cLUNC'`, `'cUSTC'`, `'vFDUSD'`. Do **not** grep concatenated `cLUNC / USD`. Do **not** grep `protocol-top-pairs`. Do **not** copy `EXPECT_SHA`. Do **not** attest `…000`/`…001`. Do **not** invent leftover `DATABASE_URL`. Optional leftover e2e: run child **without** `SKIP_E2E` (existing 5 workers, child’s `PLAYWRIGHT_WEB_PORT=30129`; do **not** invent a second leftover e2e flag or leak a different port). Optional leftover e2e is **not** leftover-complete. Optional I10 USD-gate test in `indexer_trader_rolling_numeric.rs` (same slice, not close-gate). | Slice 2 (docs greps) |
| **2 — crosslinks** | implement | Keep Q23 / ADR 0010 / playbook greppable. One-liners already in slice 0; implement must not drop them. **Insert** if #1300/#1302 later land overlap files — do not whole-file checkout those tips. | none for in-repo |
| **3 — Coolify indexer** | operator | If the #1276 checkbox is off: **manual** Coolify indexer deploy of `729b097f+`. Attest `_sqlx_migrations` **`20260921130000`** `success=true` (Coolify DB / indexer `DATABASE_URL`). Confirm D5 heal no-op or one log. Healthy `/health` is not this attest. Ordinary leftover — not #297. | leftover-complete |
| **4 — Coolify frontend** | operator | Rebuild `729b097f+`. Hub wrap vs CEX glance + minify-safe HTTP literals. Record on **#1305**. | leftover-complete |
| **5 — merge recipe** | implement docs (slice 0) + future PR operators | Runbook note is the contract. Later PRs dismiss then merge. | not a Coolify gate |

**Open issue dependencies:** none that block leftover implement. Sister **#1300** does not block #1305 (`…30000` is a later prefix on the same boot). Unpublished ADR 0009 / PR **#1306** wiring is not a wait for this Coolify glance.

**Do not wait on:** #1276 checkbox flip, child Playwright / #703 ticks, `cac-design-issue-1276` / `1277` / `1300` / `1302`. Leftover 2 still **runs** a manual indexer deploy while the checkbox stays off.

## Tests

| ID | Path | Expect |
|----|------|--------|
| T1 | `make verify-issue-1305` | Children **1290** (`VERIFY_ISSUE_1290_SKIP_E2E=1`) and **1277**; docs greps for Q23 / M1305 / ADR 0010 |
| T2 | Live off | Coolify probes SKIP |
| T3 | `VERIFY1305_REQUIRE_LIVE=1` unreachable | FAIL |
| T4 | `VERIFY1305_IID=1305` unreachable | FAIL (same as #701). No `LEFTOVER_COMPLETE` flag. |
| T5 | Child FAIL | Stack FAIL |
| T6 | `make verify-issue-1277` | Includes sqlx `&mut **tx` grep / I10 rolling+heal; 6/6 after #1304 |
| T7 | Child #1290 from leftover wrapper | Default `VERIFY_ISSUE_1290_SKIP_E2E=1`: docs + `hubPriceTicker` + `verify-issue-1240`. Optional leftover e2e: unset `SKIP_E2E`, 5 workers, no extra `PLAYWRIGHT_WEB_PORT`. `YY-MM` flake is not leftover-complete. |
| T8 | Optional I10 | USD-only skew does **not** set `trader_lifetime_diverges_from_swaps`; heal returns `false` |
| T9 | Live Coolify after migrate | Operator: **`20260921130000`** `success=true` via Coolify DB / indexer `DATABASE_URL` (manual indexer deploy of `729b097f+` if #1276 checkbox still off). Healthy `/health` is not this attest. Frontend hashed-chunk **`cLUNC wrap`** / `'cLUNC'` / `'cUSTC'` / `'vFDUSD'`. Operator glance: hub **`cUSTC / USD`** vs CEX **`LUNC`** / selected not **`VFDUSD`**. HTTP PASS is not sqlx leftover-complete. No leftover `DATABASE_URL`. No `EXPECT_SHA`. |

Postgres-only child tests stay Postgres-only. Leftover Playwright is optional child #1290 e2e only (5 workers, existing port). Do **not** invent a second five-worker leftover spec. `e2e-tx` stays 1 worker. Do not leak `PLAYWRIGHT_WEB_PORT`.

## Rollout

1. Merge leftover *implement* (script + wiring) to `main` via a **code** PR (not this design branch). Dismiss CODEOWNERS self-request; normal merge; no `force_merge`.
2. Operator: if the #1276 indexer auto-deploy checkbox is **off**, **manually** deploy the indexer app at `729b097f+`. If it is already on, a protected-main land rebuilds. Do **not** flip the checkbox from this ticket.
3. Confirm `_sqlx_migrations` **`20260921130000`** + D5 heal no-op/log via Coolify DB / indexer `DATABASE_URL`. Do **not** use `GET /health` as this attest.
4. Operator: frontend rebuild; confirm hub wrap vs CEX glance + minify-safe HTTP literals.
5. Close **#1305** with probe evidence for **this** contract. Leave **#1300** open unless its own leftover-complete holds. Do **not** close on the 06:46 comment.

No wasm store from the leftover MR.

## Rollback

Coolify-era indexer rollback is **three-way** ([ADR 0006](./0006-indexer-health-git-sha.md) / [`rollback-decision.md`](../runbooks/rollback-decision.md) § Auto-deploy era). `20260921130000` has **no** `down.sql` → **2(c) is unavailable**. Heal is idempotent DML on lifetime columns; restoring a pre-1303 image that **does not ship** `…30000` while the ledger row exists makes production `sqlx::migrate!()` reject (no `set_ignore_missing`) → **2(b)** keep schema + hotfix that still ships N, or **2(a)** only when the baseline image already included this file.

Frontend: redeploy previous Vite app (labels revert to whatever that image had). Do not revert #1290 product to “close” leftover.

This leftover is not a chain halt.

## Integration completion criteria

- Slice-0 on `main` via implement inserts: this ADR **0010**, architecture `#post-merge-leftover-ops`, **Q23**, playbook **M1305**. No `0008-*.md` / `0009-*.md` from this leftover MR.
- `make verify-issue-1305` exists on `main` and is green locally (children **1290** with `VERIFY_ISSUE_1290_SKIP_E2E=1`, **1277** + docs). Optional leftover e2e may stay skipped.
- Coolify: **#1305 attests `20260921130000` only** (`success=true` — operator Coolify DB / indexer `DATABASE_URL`; **manual** indexer deploy of `729b097f+` while the #1276 checkbox is off). Healthy `GET /health` is not this attest. `…000` / `…001` are **#1300**, not a #1305 FAIL if already applied. D5 heal no-op or one `traders_healed` log.
- Coolify frontend serves `729b097f+` with hashed-chunk literals **`cLUNC wrap`**, `'cLUNC'`, `'cUSTC'`, `'vFDUSD'` and operator glance hub **`cUSTC / USD`** vs CEX **`LUNC`** / selected **vFDUSD** not **VFDUSD**. Do **not** grep concatenated **`cLUNC / USD`**. Do **not** grep **`protocol-top-pairs`** (that is **#1300 leftover-complete**).
- Child Playwright / #703 `YY-MM` ticks are **not** leftover-complete.
- Optional I10 USD-gate test may land in slice 1; leftover may close without it.
- Merge recipe documented; later PRs are not `force_merge`. #1302/#1303 remain ancestors of `main`.
- `cac-design-issue-1300` / `1302` / `1277` not merged as-is.
- #1300 still the owner of `…000`/`…001` / pair wasm / LocalTerra walks.
- No founder card. No Coolify auto-deploy flip. No `VERIFY1305_LEFTOVER_COMPLETE`. No `DESIGN: APPROVE`.
- Do **not** treat the 06:46 issue close comment as leftover-complete.

## Links

- Sister leftover: [#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300) (PRs 1287–1298; sqlx `…000`/`…001`; ADR **0008** / **Q21** reserved)
- Hub-wrap verify bundle: unpublished [ADR 0009](https://git.cl8y.com/code/cl8y-dex-terraclassic/src/branch/cac-design-issue-1302/docs/adr/0009-verify-issue-1290-hub-wrap-labels.md) / **Q22** on `origin/cac-design-issue-1302` (do not merge as-is); related [#1290](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1290) / [#1240](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1240)
- [ADR 0005](./0005-protocol-fee-multihop-hops.md) — sqlx unique-prefix leftover pattern (not the unpublished 1277 `0005-trader-*`)
- [ADR 0006](./0006-indexer-health-git-sha.md) — Coolify attest / three-way rollback / #297 checkbox; `/health` is not sqlx evidence
- Playbook: [`AGENTS_POST_MERGE_OPS_1305.md`](../../skills/AGENTS_POST_MERGE_OPS_1305.md)
- Children: [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](../../skills/AGENTS_FRONTEND_PROTOCOL_STATS.md) **P1240**; [`AGENTS_INDEXER_TRADER_ROLLING_NUMERIC.md`](../../skills/AGENTS_INDEXER_TRADER_ROLLING_NUMERIC.md) **R1277**
- Merge: [`runbooks/forgejo-pr-merge.md`](../runbooks/forgejo-pr-merge.md)
- Verify (implement): `make verify-issue-1305`
- Authority: [agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297)
