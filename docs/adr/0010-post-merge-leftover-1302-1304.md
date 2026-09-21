# ADR 0010: Post-merge leftover after PRs 1302–1304

## Status

Proposed ([#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305))

Ordinary leftover ops after PRs **1302–1304** landed on `origin/main` (`729b097f`). This ADR does **not** relabel `/protocol`, change heal SQL, flip Coolify auto-deploy, store wasm, spend, expand custody/policy, or write `DESIGN: APPROVE`. Keywords on [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) (`architecture`, `deploy`, `Design decision required`) are **not** approval. Deploy/spend/custody/policy expansion stays under [agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297).

Playbook: [`skills/AGENTS_POST_MERGE_OPS_1305.md`](../../skills/AGENTS_POST_MERGE_OPS_1305.md) (**M1305-1–M1305-8**). Overview: [`architecture.md`](../architecture.md#post-merge-leftover-ops). Invariants: [`qa-invariants.md`](../qa-invariants.md) **Q23**.

Product decisions stay in child skills. Do **not** duplicate **P1240** or **R1277** here.

**Numbering (leftover 3).** ADR **0005** on `main` is protocol-fee hops ([#1269](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1269)). ADR **0008** / **Q21** `{#post-merge-ops-1300}` are reserved by sister leftover [#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300) (`docs/adr/0008-post-merge-leftover-1287-1298.md` on `origin/cac-design-issue-1300`). ADR **0009** / **Q22** `{#ops-hub-wrap-1302}` are reserved by the hub-wrap verify bundle ([#1302](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1302), `docs/adr/0009-verify-issue-1290-hub-wrap-labels.md` on `origin/cac-design-issue-1302` after that branch re-slotted off a second 0008). This leftover is **0010** / **Q23**. Keep this file as `docs/adr/0010-post-merge-leftover-1302-1304.md`. Do **not** merge `origin/cac-design-issue-1300`, `origin/cac-design-issue-1302`, or `origin/cac-design-issue-1277` as-is.

## Outcome

Close leftover **#1305** only after the leftover-complete close-comment template (Rollout / Integration) is pasted on the issue — not after green child `make verify-issue-1290` / `make verify-issue-1277` on a laptop. HTTP `[PASS]` is **not** sqlx leftover-complete. Child Playwright is **not** leftover-complete. If **#1305** is still closed on the 2026-09-21 06:46 comment (`GET /health` SHA + `top-pairs`), **reopen it as a leftover precondition**. Reopen is **not** leftover-complete. Leftover-complete still records on **#1305**.

1. **Coolify `/protocol` visual** (issue leftover 1 / related #1290 AC). After frontend rebuild from `729b097f+`, **leftover-1 proof is operator DOM glance** on live `dex.cl8y.com/protocol`: hub **`cUSTC / USD`** + **`cLUNC / USD`** vs CEX tabs **`USTC`** / **`LUNC`** / **`vFDUSD`** / click **vFDUSD** and record the selected label (**not** **`VFDUSD`**). Idle `/protocol` selects **USTC** (`parseProtocolOracleTicker` in `protocolOracleTicker.ts`). **Selected not `VFDUSD`** is the #1240 CSS guard (`ProtocolOracleCard.tsx` `textTransform: 'none'`). Idle USTC does **not** prove that guard — the operator **must click the vFDUSD tab**. Child `protocol-page.spec.ts` asserts both hub dts. The #1290 delta is hub `lunc` showing **cLUNC**, not **LUNC**. Hub `<dt>` is assembled at render (`{HUB_PRICE_TICKER_LABEL[ticker]} / USD` in `ProtocolDexHubPrices.tsx`) — production chunks keep ticker literals and `" / USD"`, **not** the concatenated display string. Concatenated hub labels stay **operator DOM glance only**. Selected-tab casing stays operator glance (CSS-in-JS is not a stable curl assert). Child `protocol-page` still has a monthly **`YY-MM`** tick flake unrelated to hub wrap (#703) — leftover-complete **must not** wait on that.

   Leftover HTTP (hashed Vite chunk grep, same class as #701 `pool-row-vol` / `volume_usd_24h`) pins **unquoted substrings** with **any quote style** (Vite minify emits double quotes). Never require source `'`. Grepping `'cLUNC'` including `'` false-FAILs a correct Coolify rebuild. **All four** HTTP pins — **`cLUNC wrap`**, **`cLUNC`**, **`cUSTC`**, **`vFDUSD`** — are **supporting rebuild-presence greps**. **None prove leftover 1.** Unquoted **`cLUNC`** is a substring of pre-1302 **`cLUNC wrap`** (`ProtocolDexHubPrices.tsx` copy aria since #570 / `a356757f`) and of registry/wrap copy (`tokenRegistry` since #507); hashed-chunk grep **PASS**es a Coolify build that never shipped the hub relabel (`lunc: 'LUNC'` → `cLUNC` in `0b4597f6` / #1290). Implement `verify-issue-1305` live HTTP **must not** treat `cLUNC` as leftover-1 complete.

   Grepping concatenated **`cLUNC / USD`** / **`cUSTC / USD`** **in hashed chunks** will FAIL a correct Coolify rebuild and look leftover-incomplete. Leftover-1 is this glance, not HTTP. Quote-agnostic HTTP is supporting rebuild presence, not leftover-1 and not leftover-complete. This is not #1300 (that leftover greps **`protocol-top-pairs` / `Top pairs (30d)`**) and not #1302 leftover-complete (unpublished ADR 0009 treats the glance as this sister). Do **not** infer leftover-complete from the 2026-09-21 06:46 issue close comment — that is **#1300** / [ADR 0006](./0006-indexer-health-git-sha.md), not this contract.

2. **Coolify indexer migrate** (issue leftover 2). Apply sqlx in filename order: #1300’s `20260921120000_traders_rolling_volume_numeric_38_0` then `20260921120001_pair_volume_30d`, then **this leftover** `20260921130000_traders_lifetime_heal_from_swaps`. **#1305 attests `…30000` only.** `…000` / `…001` stay [#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300). The [#1276](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1276) indexer auto-deploy checkbox is **still off** until that leftover / #297. **If the checkbox is off, leftover 2 is a manual Coolify indexer deploy of `729b097f+`.** Do **not** wait on flipping the checkbox. Healthy `GET /health` (`status=ok`, with or without `git_sha`) is **not** `_sqlx_migrations` evidence ([ADR 0006](./0006-indexer-health-git-sha.md)). Split attest (how-to, not bans-only): `_sqlx_migrations` `20260921130000` `success=true` via the **SELECT** in Observability / Coolify leftovers (Coolify DB / indexer `DATABASE_URL` — not `scripts/lib/postgres-psql.sh`); heal silence/one log via **indexer logs** matching the `traders.rs` needle `traders_healed lifetime totals from swap_events (GitLab #1277)`. Heal is **not** queryable from `DATABASE_URL`. `poller.rs` runs `heal_trader_lifetime_from_swaps_if_needed` **once at startup**, then D5 `refresh_all_volume_windows`. Heal is **not** in `run_volume_refresh_loop` and is **not** a D5 step. Startup heal (before D5) is a **no-op** (`Ok(false)`, no `traders_healed` info) **or** one `traders_healed` info; later process starts stay silent if the gate is false. Heal errors stay `tracing::error` and must not abort `run_indexer`.

3. **ADR collision** (issue leftover 3). This file **is** the re-slot. Implement **inserts** 0010 / Q23 / architecture `#post-merge-leftover-ops` (keep the one-line **#1300 / Q21** sister stub) — do **not** whole-file checkout overlap docs from sister design tips. Do **not** add `0008-*.md` or `0009-*.md` on this leftover MR.

4. **Optional I11** (issue leftover 4 / **M1305-4**). USD-only lifetime skew **must not** make `trader_lifetime_diverges_from_swaps` true (would re-run heal SQL on every process start). Gate `SQL_TRADER_LIFETIME_DIVERGES` stays **trades + raw `total_volume` only**. Do **not** add `total_volume_usd` to the EXISTS. Existing I10 cases in `indexer_trader_rolling_numeric.rs` are missing-trader / ghost-zero heal — do **not** call this Stay test I10. Optional implement test **I11**: matching trades+raw + wrong USD → `trader_lifetime_diverges_from_swaps == false` and heal returns `false`. Not a leftover-complete gate.

5. **Merge recipe** (issue leftover 5). Future lands follow [`docs/runbooks/forgejo-pr-merge.md`](../runbooks/forgejo-pr-merge.md): dismiss the `.* @code/maintainers` self-request, then normal `fj pr merge`. Do **not** `force_merge`. #1302 / #1303 used `force_merge` before that recipe was followed; both merge commits **are** ancestors of `origin/main` (`92c84406`, `a7919548`) — do **not** rewrite them. #1304 used dismiss + normal merge (`729b097f`). If **#1305** is still closed on the 06:46 comment, reopen it as a leftover precondition (not leftover-complete).

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
- Expanding the heal gate to USD (would re-run heal SQL on every process start whenever #553 USD is stale vs raw).

`CODEOWNERS` is `.* @code/maintainers`. Forgejo `block_on_official_review_requests` makes `fj pr merge` return HTTP **405** until the team self-request is dismissed. That is leftover 5, not a branch-protection change.

## Non-goals

- Closing or absorbing [#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300). Attesting sqlx `…000` / `…001`. Grepping Coolify **`protocol-top-pairs`**. columbus-5 pair wasm. LocalTerra four walks. Closing #1264 or #1279.
- Merging unpublished `origin/cac-design-issue-1300` / `1302` / `1277` as-is. Taking ADR **0008** / **0009** or **Q21** / **Q22**. Whole-file checkout of `architecture.md` / `qa-invariants.md` / `docs/README.md` from those tips.
- Relabeling `/protocol`, changing `HUB_PRICE_TICKER_LABEL`, adding a `clunc` hub path, or scraping production HTML as #1302 leftover-complete.
- Grepping concatenated **`cLUNC / USD`** / **`cUSTC / USD`** in hashed Vite chunks. Requiring source `'` in leftover HTTP (`'cLUNC'`). Treating any HTTP pin — **`cLUNC`**, **`cUSTC`**, **`vFDUSD`**, **`cLUNC wrap`** — as leftover-1 proof. Leftover-1 is operator DOM glance only.
- Changing heal SQL, adding `total_volume_usd` to `SQL_TRADER_LIFETIME_DIVERGES`, or rewriting #1303/#1304.
- Flipping Coolify auto-deploy, HMAC / `autonomy.rs`, founder card, wallet spend, or wasm store.
- Copying #1276 `EXPECT_SHA`. Inventing leftover `DATABASE_URL` for `make verify-issue-1305`. Inventing `VERIFY1305_LEFTOVER_COMPLETE=1`. Using `postgres-psql.sh` against prod. Treating HTTP PASS or `GET /health` as `_sqlx_migrations` leftover-complete.
- Closing leftover-complete on child Playwright `protocol-page` or #703 monthly `YY-MM` ticks.
- Inferring leftover-complete from the 06:46 issue close comment. Treating a reopen of #1305 as leftover-complete.
- Rewriting #1302 / #1303 history. Using `force_merge` on later PRs.
- Opening a design-only PR from `cac-design-issue-1305`.
- `DESIGN: APPROVE`.

## Decision

### Component / state / interface

| Layer | Change in *this* leftover | Already on `main` |
|-------|---------------------------|-------------------|
| Frontend `/protocol` | Operator Coolify rebuild + glance + quote-agnostic HTTP attest | Hub wrap labels (**P1240** / #1290); `verify-issue-1290` sketch |
| Indexer schema | None (heal is data DML) | Widen `20260921120000`; 30d `20260921120001`; heal `20260921130000` |
| Indexer runtime | Manual Coolify indexer deploy of `729b097f+` while #1276 checkbox is off; startup heal (before D5) no-op or one log | `heal_trader_lifetime_from_swaps_if_needed` once at startup, then D5; sqlx `&mut **tx` |
| HTTP APIs | No new fields | Unscoped trader JSON still `bd_plain_string`; hub ids `custc`/`lunc` |
| Verify | Implement `make verify-issue-1305` (children **1290** with `VERIFY_ISSUE_1290_SKIP_E2E=1` by default, **1277**; live HTTP optional supporting pins only) | Children exist; **no 1305 wrapper yet**. Index lines that name the target stay tagged **(implement)** until slice 1. Do **not** add the script on this design branch. |
| Merge process | Docs only (dismiss then merge) | Runbook already forbids `force_merge` |

No CosmWasm, no `VITE_*`, no CAC `COOLIFY_APP_MAP`.

### Affected invariants

| ID | Owner | This leftover |
|----|-------|---------------|
| **P1240-1–P1240-8** | PROTOCOL_STATS / PROTOCOL_HUB | Coolify visual attest only |
| **R1277-1–R1277-8** | trader rolling NUMERIC | Heal migrate + startup-before-D5 no-op; USD cap **R1277-5** unchanged |
| **D5** (#577) | volume window decay | Heal still runs **once at startup before** `refresh_all_volume_windows`. Not in `run_volume_refresh_loop`. |
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
| Grep concatenated `cLUNC / USD` like #701 greps `pool-row-vol` | Hub copy is assembled at render; minify splits ticker and `" / USD"`. False leftover-incomplete. Concatenated strings stay **DOM glance only**. |
| Grep leftover HTTP as source `'cLUNC'` | Vite minify emits `"cLUNC"`. Quote-agnostic unquoted substrings, #701-style. |
| Treat unquoted HTTP `cLUNC` as leftover-1 | Substring of pre-1302 `cLUNC wrap` / registry copy; PASSes a Coolify build that never shipped `lunc: 'cLUNC'`. Leftover-1 is operator DOM glance. |
| Pack sqlx / heal / glance into the make Check | Live make is HTTP-only (and that HTTP is not leftover-1). Leftover-complete stays operator sqlx `success=true` + heal logs + hub glance. |
| Wait leftover-complete on child `protocol-page` | Monthly `YY-MM` tick flake is #703, not hub wrap. |
| Expand heal gate to USD | Re-trips poller whenever #553 USD drifts; issue leftover 4 is explicit Stay. |
| `VERIFY1305_LEFTOVER_COMPLETE=1` | #701 does not have it; #1276 uses it for SHA tip-match this ticket forbids. HTTP PASS is not sqlx leftover-complete. |
| Treat `GET /health` as sqlx evidence | ADR 0006: liveness ≠ `_sqlx_migrations`. |
| `force_merge` because CODEOWNERS 405 | Forbidden. Dismiss self-request. |
| Agent Coolify UI / #297 checkbox | Out of authority. Ordinary leftover migrate/rebuild is operator, not policy expansion. |

### Complexity added / removed

**Added:** leftover playbook **M1305**, **Q23**, this ADR **0010**, implement wrapper `verify-issue-1305`, operator attest of `…30000` + hub wrap visual. Optional **I11** USD-gate test.

**Removed:** the gap where PRs 1302–1304 had no leftover card (heal migrate sat only as a comment on #1300; hub wrap visual had no Coolify owner; ADR 0008 was claimed twice). sqlx 0.8 compile hole closed by #1304 (not reopened here).

Net: ops registry without a second product surface and without dual ADR numbers.

## Migration

sqlx `20260921130000_traders_lifetime_heal_from_swaps` is **already on `main`**. Leftover is **apply in production**, not a new file. Order: `…000` → `…001` → `…30000` (unique prefix rule, [ADR 0005](./0005-protocol-fee-multihop-hops.md) / [ADR 0006](./0006-indexer-health-git-sha.md)). Do **not** rename after prod apply (checksum mismatch). No `down.sql`. One-shot DML is idempotent with the gated Rust heal (INSERT `ON CONFLICT DO NOTHING`, lifetime UPDATE, leftover-zero, #553 USD refresh, leftover-USD NULL).

Indexer: while the #1276 auto-deploy checkbox is **off**, leftover 2 is a **manual** Coolify indexer deploy of `729b097f+`. Frontend: Coolify auto-deploy (already on for Vite) should rebuild `729b097f+`. Dual-app skew vs indexer migrate is expected.

No wasm. No pause. No treasury rotate.

## Observability

Leftover-2 how-to (operator; Coolify DB / indexer logs — not liveness):

**sqlx** (Coolify DB / indexer `DATABASE_URL`; not `postgres-psql.sh`; not `GET /health`):

```sql
SELECT version, description, success FROM _sqlx_migrations
WHERE version = 20260921130000;
```

Expect `version=20260921130000` and `success=true`. A healthy boot does **not** imply this row.

**heal** (indexer logs; not `DATABASE_URL`): the attest needle is the `tracing::info!` string in `indexer/src/db/queries/traders.rs`:

`traders_healed lifetime totals from swap_events (GitLab #1277)`

One of that line on the first start after migrate, then silence on later process starts if the gate is false. `trader lifetime heal failed` at **error** (poller continues). Do **not** treat `GET /health` as this attest. Do **not** query heal from `DATABASE_URL`.

- No public heal metric. Do not add `/health` keys. Do not scrape Coolify `/status` JSON or paste app UUIDs/tokens on the issue.
- Frontend leftover HTTP: hashed-chunk grep unquoted **`cLUNC wrap`**, **`cLUNC`**, **`cUSTC`**, **`vFDUSD`** (any quote style; never require source `'`). **All four are supporting rebuild-presence greps. None prove leftover 1.** Implement live HTTP must not treat **`cLUNC`** as leftover-1 complete. Leftover-1 is operator glance: hub **`cUSTC / USD`** + **`cLUNC / USD`** vs CEX **`USTC`** / **`LUNC`** / **`vFDUSD`** / click **vFDUSD** selected not **`VFDUSD`** (idle `/protocol` is **USTC**). Do **not** grep concatenated **`cLUNC / USD`** in hashed chunks.
- Verify script prints `[PASS]` / `[FAIL]` / SKIP; exit `1` iff `FAIL > 0`. Sibling `VERIFY*_IID` unreachable = FAIL. Keep only `VERIFY1305_REQUIRE_LIVE` / `VERIFY1305_IID=1305` fail-closed on unreachable. Do **not** copy `EXPECT_SHA`. Do **not** invent `VERIFY1305_LEFTOVER_COMPLETE`. HTTP PASS is not sqlx leftover-complete; `success=true` stays the SELECT above.

## Failure modes

| Failure | Behavior |
|---------|----------|
| Child 1290 or 1277 FAIL | Stack FAIL. Do not skip children. Default child 1290 uses `VERIFY_ISSUE_1290_SKIP_E2E=1` so a #703 `YY-MM` flake does not fail the leftover wrapper. |
| Coolify hosts unreachable | SKIP unless `VERIFY1305_REQUIRE_LIVE=1` or `VERIFY1305_IID=1305` → **FAIL** |
| HTTP PASS while `…30000` unattested | Not leftover-complete. Sqlx `success=true` is operator Coolify DB only. |
| Treat HTTP `cLUNC` as leftover-1 complete | Forbidden. Substring of pre-1302 `cLUNC wrap` / registry copy; false leftover-1 on a Coolify build that never shipped `lunc: 'cLUNC'`. Leftover-1 is operator DOM glance. |
| Treat green `make verify-issue-1305` as leftover-complete | Forbidden. Make is children + optional HTTP. Leftover-complete is operator sqlx `success=true` + heal logs + hub glance. |
| `…30000` missing after indexer image of `729b097f+` | Leftover-complete FAIL (operator). If #1276 checkbox is off, leftover 2 was never **manually** deployed. |
| Healthy `GET /health` used as sqlx evidence | Forbidden. Liveness ≠ `_sqlx_migrations` (ADR 0006). |
| `…30000` present while #1300’s `…000` missing | sqlx cannot skip; boot would have failed migrate. Not a #1305-only state. |
| Repeated `traders_healed` on later process starts when trades+raw already match | Gate bug (likely USD folded into EXISTS). Heal is startup-before-D5, not a poller tick. Fix-forward; do not expand the gate. |
| Heal compile regression (`&mut *tx`) | `make verify-issue-1277` FAIL. Keep `&mut **tx`. |
| Grep concatenated `cLUNC / USD` as this leftover’s HTTP marker | Forbidden. False leftover-incomplete on a correct rebuild. Concatenated strings stay DOM glance only. |
| Grep leftover HTTP as source `'cLUNC'` | Forbidden. Quote-agnostic unquoted substrings. |
| Grep `protocol-top-pairs` as this leftover’s marker | Forbidden. That is #1300. |
| Infer leftover-complete from the 06:46 close comment | Forbidden. That is #1300 / 0006. If still closed on that comment, reopen as a leftover precondition (not leftover-complete). |
| Close comment uses `GET /health`, `protocol-top-pairs`, hashed-chunk `cLUNC` PASS, or green `make verify-issue-1305` | Forbidden. Those already closed this card wrongly. Paste the leftover-complete template (sqlx SELECT + heal needle + leftover-1 glance including click **vFDUSD**). |
| Glance leftover-1 from idle `/protocol` only | Forbidden. Idle selects **USTC** (`protocolOracleTicker.ts`). Selected-not-`VFDUSD` is the #1240 CSS guard after clicking the **vFDUSD** tab. |
| Merge sister design branches as-is | ADR collision. Insert 0010/Q23 only. |
| `fj pr merge` HTTP 405 | Dismiss maintainers self-request; retry normal merge. Not `force_merge`. |
| Agent flips Coolify auto-deploy | Forbidden (#297). |
| `VERIFY1305_IID=1305` without live hosts | FAIL (unreachable), not leftover-complete PASS. No `LEFTOVER_COMPLETE` flag. |

## Ordered implementation slices

| Slice | Who | Deliverable | Blocks |
|-------|-----|-------------|--------|
| **0 — this design** | design_author | ADR **0010**, architecture `#post-merge-leftover-ops` (Q23 plus one-line **#1300 / Q21** sister stub so implement insert cannot drop Q21), **Q23**, playbook **M1305**, README index tagged **(implement)**, runbook merge-recipe note, child-skill one-liners. Branch `cac-design-issue-1305` only (no design-only PR). If #1305 is still closed on the 06:46 comment, reopen as leftover precondition (not leftover-complete). | Slice 1 |
| **1 — leftover verify** | implement | `scripts/qa/verify-issue-1305.sh` + `Makefile` `verify-issue-1305` + `docs/testing.md` / `scripts/qa/README.md` / `AGENTS.md` wiring (drop **(implement)** tags once the target exists). Children **1290** (default `VERIFY_ISSUE_1290_SKIP_E2E=1` — docs + `hubPriceTicker` + `verify-issue-1240`) and **1277**. Copy `require_live()` from #701 (`REQUIRE_LIVE` / `IID=1305` only — **no** `LEFTOVER_COMPLETE`). Live frontend HTTP: unquoted **`cLUNC wrap`**, **`cLUNC`**, **`cUSTC`**, **`vFDUSD`** (any quote style; never require source `'`). **All four are supporting rebuild-presence. Live HTTP must not treat `cLUNC` as leftover-1 complete.** Leftover-1 is operator glance **`cUSTC / USD`** + **`cLUNC / USD`** vs CEX **`USTC`** / **`LUNC`** / **`vFDUSD`** / click **vFDUSD** selected not **`VFDUSD`**. Do **not** grep concatenated `cLUNC / USD` in hashed chunks. Do **not** grep `protocol-top-pairs`. Do **not** copy `EXPECT_SHA`. Do **not** attest `…000`/`…001`. Do **not** invent leftover `DATABASE_URL`. Optional leftover e2e: run child **without** `SKIP_E2E` (existing 5 workers, child’s `PLAYWRIGHT_WEB_PORT=30129`; do **not** invent a second leftover e2e flag or leak a different port). Optional leftover e2e is **not** leftover-complete. Optional **I11** USD-gate test in `indexer_trader_rolling_numeric.rs` (same slice, not close-gate; do **not** name it I10). | Slice 2 (docs greps) |
| **2 — crosslinks** | implement | Keep Q23 / ADR 0010 / playbook greppable. One-liners already in slice 0; implement must not drop them. **Insert** if #1300/#1302 later land overlap files — do not whole-file checkout those tips. | none for in-repo |
| **3 — Coolify indexer** | operator | If the #1276 checkbox is off: **manual** Coolify indexer deploy of `729b097f+`. Attest `_sqlx_migrations` with the Observability **SELECT** (`version=20260921130000` `success=true` — Coolify DB / indexer `DATABASE_URL`). Confirm startup-before-D5 heal with the `traders.rs` needle `traders_healed lifetime totals from swap_events (GitLab #1277)` **via indexer logs**. Heal is not queryable from `DATABASE_URL`. Healthy `/health` is not this attest. Ordinary leftover — not #297. | leftover-complete |
| **4 — Coolify frontend** | operator | Rebuild `729b097f+`. **Leftover-1:** glance hub **`cUSTC / USD`** + **`cLUNC / USD`** vs CEX **`USTC`** / **`LUNC`** / **`vFDUSD`**, then **click the vFDUSD tab** and record the selected label (**not** **`VFDUSD`**). Idle `/protocol` is **USTC**. Quote-agnostic HTTP pins are supporting rebuild-presence, not leftover-1. Record glance on **#1305**. | leftover-complete |
| **5 — merge recipe** | implement docs (slice 0) + future PR operators | Runbook note is the contract. Later PRs dismiss then merge. | not a Coolify gate |

**Open issue dependencies:** none that block leftover implement. Sister **#1300** does not block #1305 (`…30000` is a later prefix on the same boot). Unpublished ADR 0009 / PR **#1306** wiring is not a wait for this Coolify glance.

**Do not wait on:** #1276 checkbox flip, child Playwright / #703 ticks, `cac-design-issue-1276` / `1277` / `1300` / `1302`. Leftover 2 still **runs** a manual indexer deploy while the checkbox stays off.

## Tests

| ID | Path | Expect |
|----|------|--------|
| T1 | `make verify-issue-1305` (implement) | Children **1290** (`VERIFY_ISSUE_1290_SKIP_E2E=1`) and **1277**; docs greps for Q23 / M1305 / ADR 0010. Make is **not** leftover-complete. |
| T2 | Live off | Coolify probes SKIP |
| T3 | `VERIFY1305_REQUIRE_LIVE=1` unreachable | FAIL |
| T4 | `VERIFY1305_IID=1305` unreachable | FAIL (same as #701). No `LEFTOVER_COMPLETE` flag. |
| T5 | Child FAIL | Stack FAIL |
| T6 | `make verify-issue-1277` | Includes sqlx `&mut **tx` grep / I10 rolling+heal; 6/6 after #1304 |
| T7 | Child #1290 from leftover wrapper | Default `VERIFY_ISSUE_1290_SKIP_E2E=1`: docs + `hubPriceTicker` + `verify-issue-1240`. Optional leftover e2e: unset `SKIP_E2E`, 5 workers, no extra `PLAYWRIGHT_WEB_PORT`. `YY-MM` flake is not leftover-complete. |
| T8 | Optional I11 | USD-only skew does **not** set `trader_lifetime_diverges_from_swaps`; heal returns `false`. Do **not** name this I10 (I10 is missing-trader / ghost-zero). |
| T9 | Live Coolify after migrate | Operator leftover-complete: paste the close-comment template (Rollout / Integration). **sqlx:** Observability SELECT `version=20260921130000` `success=true` via Coolify DB / indexer `DATABASE_URL` (manual indexer deploy of `729b097f+` if #1276 checkbox still off). Healthy `/health` is not this attest. **heal:** indexer-log needle `traders_healed lifetime totals from swap_events (GitLab #1277)` once then silence (not `DATABASE_URL`). **Leftover-1:** operator glance hub **`cUSTC / USD`** + **`cLUNC / USD`** vs CEX **`USTC`** / **`LUNC`** / **`vFDUSD`** / click **vFDUSD** selected not **`VFDUSD`** (idle is **USTC**). Frontend hashed-chunk unquoted **`cLUNC wrap`** / **`cLUNC`** / **`cUSTC`** / **`vFDUSD`** (any quote style; never require source `'`) are supporting rebuild-presence — **not leftover-1**. Implement live HTTP must not treat `cLUNC` as leftover-1 complete. HTTP PASS is not leftover-1 and not sqlx leftover-complete. Forbidden in the close comment: `GET /health`, `protocol-top-pairs`, hashed-chunk `cLUNC` PASS, green make. No leftover `DATABASE_URL`. No `EXPECT_SHA`. |

Postgres-only child tests stay Postgres-only. Leftover Playwright is optional child #1290 e2e only (5 workers, existing port). Do **not** invent a second five-worker leftover spec. `e2e-tx` stays 1 worker. Do not leak `PLAYWRIGHT_WEB_PORT`.

## Rollout

1. If **#1305** is still closed on the 06:46 comment, **reopen it as a leftover precondition**. Reopen is not leftover-complete. Leftover-complete still records on **#1305**.
2. Merge leftover *implement* (script + wiring) to `main` via a **code** PR (not this design branch). Dismiss CODEOWNERS self-request; normal merge; no `force_merge`.
3. Operator: if the #1276 indexer auto-deploy checkbox is **off**, **manually** deploy the indexer app at `729b097f+`. If it is already on, a protected-main land rebuilds. Do **not** flip the checkbox from this ticket.
4. Confirm leftover-2 with the Observability **SELECT** (`version=20260921130000` `success=true`) and the `traders.rs` heal needle via **indexer logs**. Do **not** query heal from `DATABASE_URL`. Do **not** use `GET /health` as sqlx attest.
5. Operator: frontend rebuild; leftover-1 is hub **`cUSTC / USD`** + **`cLUNC / USD`** vs CEX **`USTC`** / **`LUNC`** / **`vFDUSD`**, then **click the vFDUSD tab** (idle `/protocol` is **USTC**) and record selected **not** **`VFDUSD`**. Quote-agnostic HTTP pins are supporting, not leftover-1.
6. Close **#1305** by pasting the leftover-complete comment below (same shape as [ADR 0006](./0006-indexer-health-git-sha.md) slice 3; no UUID/token/host). Leave **#1300** open unless its own leftover-complete holds. Do **not** close on the 06:46 comment.

### Leftover-complete close-comment template (required; no UUID/token/host)

Paste on [#1305](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305) when leftover-complete is claimed. The 06:46 close used `GET /health` + `protocol-top-pairs`; that is **not** this contract.

```
sqlx: SELECT version, description, success FROM _sqlx_migrations
      WHERE version = 20260921130000;
      → version=20260921130000 success=true
      (Coolify DB / indexer DATABASE_URL; not postgres-psql.sh; not GET /health)

heal: indexer logs — one
      "traders_healed lifetime totals from swap_events (GitLab #1277)"
      on first start after migrate, then silence on later process starts
      (not DATABASE_URL)

glance leftover-1: dex.cl8y.com/protocol hub <dt>
      cUSTC / USD + cLUNC / USD
      vs CEX tabs USTC / LUNC / vFDUSD
      selected vFDUSD tab label stays vFDUSD (not VFDUSD)

FORBIDDEN in this comment: GET /health, protocol-top-pairs,
hashed-chunk cLUNC PASS, green make verify-issue-1305
```

No wasm store from the leftover MR.

## Rollback

Coolify-era indexer rollback is **three-way** ([ADR 0006](./0006-indexer-health-git-sha.md) / [`rollback-decision.md`](../runbooks/rollback-decision.md) § Auto-deploy era). `20260921130000` has **no** `down.sql` → **2(c) is unavailable**. Heal is idempotent DML on lifetime columns; restoring a pre-1303 image that **does not ship** `…30000` while the ledger row exists makes production `sqlx::migrate!()` reject (no `set_ignore_missing`) → **2(b)** keep schema + hotfix that still ships N, or **2(a)** only when the baseline image already included this file.

Frontend: redeploy previous Vite app (labels revert to whatever that image had). Do not revert #1290 product to “close” leftover.

This leftover is not a chain halt.

## Integration completion criteria

- Slice-0 on `main` via implement inserts: this ADR **0010**, architecture `#post-merge-leftover-ops` (keep the **#1300 / Q21** sister stub), **Q23**, playbook **M1305**. No `0008-*.md` / `0009-*.md` from this leftover MR.
- `make verify-issue-1305` exists on `main` and is green locally (children **1290** with `VERIFY_ISSUE_1290_SKIP_E2E=1`, **1277** + docs). Optional leftover e2e may stay skipped. Green make is **not** leftover-complete.
- Coolify leftover-2: **#1305 attests `20260921130000` only** via the Observability **SELECT** (`success=true` — Coolify DB / indexer `DATABASE_URL`; **manual** indexer deploy of `729b097f+` while the #1276 checkbox is off). Healthy `GET /health` is not this attest. `…000` / `…001` are **#1300**, not a #1305 FAIL if already applied. Startup-before-D5 heal via indexer-log needle `traders_healed lifetime totals from swap_events (GitLab #1277)` (not `DATABASE_URL`).
- Coolify leftover-1 is operator glance hub **`cUSTC / USD`** + **`cLUNC / USD`** vs CEX **`USTC`** / **`LUNC`** / **`vFDUSD`** / click **vFDUSD** selected not **`VFDUSD`** (idle `/protocol` is **USTC**). Hashed-chunk unquoted **`cLUNC wrap`**, **`cLUNC`**, **`cUSTC`**, **`vFDUSD`** (any quote style; never require source `'`) are supporting rebuild-presence — **none prove leftover 1**. Implement live HTTP must not treat **`cLUNC`** as leftover-1 complete. Do **not** grep concatenated **`cLUNC / USD`** in hashed chunks. Do **not** grep **`protocol-top-pairs`** (that is **#1300 leftover-complete**).
- Child Playwright / #703 `YY-MM` ticks are **not** leftover-complete.
- Optional **I11** USD-gate test may land in slice 1; leftover may close without it. Do **not** name it I10.
- Merge recipe documented; later PRs are not `force_merge`. #1302/#1303 remain ancestors of `main`.
- `cac-design-issue-1300` / `1302` / `1277` not merged as-is.
- #1300 still the owner of `…000`/`…001` / pair wasm / LocalTerra walks.
- No founder card. No Coolify auto-deploy flip. No `VERIFY1305_LEFTOVER_COMPLETE`. No `DESIGN: APPROVE`.
- If #1305 is still closed on the 06:46 comment, reopen it as a leftover precondition. Do **not** treat that comment or the reopen as leftover-complete.
- Close **#1305** only by pasting this leftover-complete comment (same as Rollout; no UUID/token/host). Do **not** close on green make, HTTP `cLUNC` PASS, or “healthy boot implies migrate.”

```
sqlx: SELECT version, description, success FROM _sqlx_migrations
      WHERE version = 20260921130000;
      → version=20260921130000 success=true
      (Coolify DB / indexer DATABASE_URL; not postgres-psql.sh; not GET /health)

heal: indexer logs — one
      "traders_healed lifetime totals from swap_events (GitLab #1277)"
      on first start after migrate, then silence on later process starts
      (not DATABASE_URL)

glance leftover-1: dex.cl8y.com/protocol hub <dt>
      cUSTC / USD + cLUNC / USD
      vs CEX tabs USTC / LUNC / vFDUSD
      selected vFDUSD tab label stays vFDUSD (not VFDUSD)

FORBIDDEN in this comment: GET /health, protocol-top-pairs,
hashed-chunk cLUNC PASS, green make verify-issue-1305
```

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
