# Agent playbook: post-merge PRs 1302–1304 leftover verify (Forgejo #1305)

Audience: third-party agents verifying Coolify indexer migrate `20260921130000` + `/protocol` hub wrap visual after [PRs 1302–1304](https://git.cl8y.com/code/cl8y-dex-terraclassic/pulls) landed on `main` (`729b097f`). Child `make verify-issue-1290` / `make verify-issue-1277` already existed on the merge commits.

**Issue:** [Forgejo **#1305**](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1305)
**Parents (closed unless a merged invariant is wrong):** #1290 / #1240, #1277 / #1292, #1303 / #1304.
**Sister leftover (stays open):** [#1300](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1300) (PRs 1287–1298; sqlx `…000`/`…001`).
**Invariants:** [`docs/qa-invariants.md`](../docs/qa-invariants.md) **Q23** (**M1305-1–M1305-8**)
**Design:** [`docs/adr/0010-post-merge-leftover-1302-1304.md`](../docs/adr/0010-post-merge-leftover-1302-1304.md)
**Verify:** `make verify-issue-1305` **(implement)** — this playbook is the contract. Make is children + optional HTTP, **not** leftover-complete. Do not add the script on this design branch.

Indexer **auto-deploy checkbox** leftover stays on [#1276](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1276) / [ADR 0006](../docs/adr/0006-indexer-health-git-sha.md) — [agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297). If that checkbox is **off**, leftover 2 is a **manual** Coolify indexer deploy of `729b097f+`. Healthy `GET /health` is **not** `_sqlx_migrations` evidence. Do **not** merge `origin/cac-design-issue-1300` / `1302` / `1277` as-is (ADR 0008 collision; this leftover is **0010** / **Q23**). Do **not** reopen closed parents for ops/QA. If **#1305** is still closed on the 06:46 comment, **reopen it as a leftover precondition** (not leftover-complete). Do **not** wait on GitLab CI quota. Do **not** file a founder card. Keywords on #1305 are not architecture approval.

## Merged PR(s)

| PR | Issue | Skill |
|----|-------|-------|
| 1302 | #1290 / #1240 hub wrap **cLUNC** vs CEX **LUNC** verify bundle | [`AGENTS_FRONTEND_PROTOCOL_STATS.md`](./AGENTS_FRONTEND_PROTOCOL_STATS.md) **P1240** |
| 1303 | #1277 / #1292 trader lifetime heal from `swap_events` | [`AGENTS_INDEXER_TRADER_ROLLING_NUMERIC.md`](./AGENTS_INDEXER_TRADER_ROLLING_NUMERIC.md) |
| 1304 | sqlx 0.8 `&mut **tx` so heal compiles | same (compile gate in `make verify-issue-1277`) |

## Invariants (M1305-1–M1305-8)

| ID | Rule |
|----|------|
| **M1305-1** | Local regression is `make verify-issue-1305` **(implement)**, which runs children **1290** (`VERIFY_ISSUE_1290_SKIP_E2E=1` by default: docs + `hubPriceTicker` + `verify-issue-1240`) and **1277**. A child FAIL fails the stack. Live Coolify leftover probes SKIP unless hosts answer (FAIL when `VERIFY1305_REQUIRE_LIVE=1` or `VERIFY1305_IID=1305`). Live make is **HTTP only** (optional supporting pins). Make does **not** cover leftover-complete (sqlx / heal logs / leftover-1 glance stay operator). Do **not** invent `VERIFY1305_LEFTOVER_COMPLETE`. HTTP PASS is not leftover-1 and not sqlx leftover-complete. Do **not** copy #1276 `EXPECT_SHA`. Do **not** invent leftover `DATABASE_URL`. Do **not** attest sqlx `20260921120000` / `20260921120001` (those are **#1300**). |
| **M1305-2** | Coolify indexer migrate **`20260921130000_traders_lifetime_heal_from_swaps`** after #1300’s `…000` then `…001`. **#1305 attests `…30000` only** via Coolify leftovers **SELECT** (`version=20260921130000` `success=true` — Coolify DB / indexer `DATABASE_URL`; not `postgres-psql.sh`). If the #1276 checkbox is off, leftover 2 is a **manual** Coolify indexer deploy of `729b097f+`. Healthy `GET /health` is not `_sqlx_migrations` evidence. Heal is **startup-before-D5** (`poller.rs` once, then D5 `refresh_all_volume_windows`). Heal is **not** in `run_volume_refresh_loop`. Heal attest needle (indexer logs, not `DATABASE_URL`): `traders_healed lifetime totals from swap_events (GitLab #1277)` — one on first start after migrate, then silence. Heal errors must not abort `run_indexer`. No `down.sql`. |
| **M1305-3** | Coolify **frontend rebuild** from `729b097f+`. **Leftover-1 proof is operator DOM glance** on `dex.cl8y.com/protocol`: hub **`cUSTC / USD`** + **`cLUNC / USD`** vs CEX **`USTC`** / **`LUNC`** / **`vFDUSD`** / click **vFDUSD** selected not **`VFDUSD`**. Idle `/protocol` selects **USTC** (`protocolOracleTicker.ts`). **Selected not `VFDUSD`** is the #1240 CSS guard (`ProtocolOracleCard.tsx` `textTransform: 'none'`). Operator **must click the vFDUSD tab** and record the selected label. HTTP pins unquoted **`cLUNC wrap`**, **`cLUNC`**, **`cUSTC`**, **`vFDUSD`** (any quote style; never require source `'`) are **supporting rebuild-presence**. **None prove leftover 1.** Unquoted **`cLUNC`** is a substring of pre-1302 **`cLUNC wrap`** and of registry/wrap copy; hashed-chunk grep PASSes a Coolify build that never shipped `lunc: 'cLUNC'`. Implement live HTTP **must not** treat `cLUNC` as leftover-1 complete. Do **not** grep concatenated **`cLUNC / USD`** / **`cUSTC / USD`** in hashed chunks (assembled at render). Do **not** grep **`protocol-top-pairs`** (that is **#1300 leftover-complete**). Dual-app skew during rebuild is expected. |
| **M1305-4** | Optional **I11**: USD-only lifetime skew does **not** make `trader_lifetime_diverges_from_swaps` true (gate is trades + raw `total_volume` only). Do **not** add `total_volume_usd` to the EXISTS. Do **not** name this test I10 (I10 is missing-trader / ghost-zero). Optional test is not leftover-complete. |
| **M1305-5** | Merge recipe: dismiss `.* @code/maintainers` self-request, then normal `fj pr merge`. Do **not** `force_merge` ([`forgejo-pr-merge.md`](../docs/runbooks/forgejo-pr-merge.md)). #1302/#1303 `force_merge` commits **are** ancestors of `origin/main`; do not rewrite. #1304 used dismiss + normal merge. |
| **M1305-6** | Do **not** reopen #1290 / #1240 / #1277 for ops/QA. Do **not** wait on GitLab CI. Do **not** close #1300 from this ticket. Do **not** revert #1302. Do **not** merge unpublished sister design branches. Do **not** take ADR **0008** / **0009** or **Q21** / **Q22**. Do **not** flip Coolify auto-deploy. Do **not** file a founder card. Do **not** infer leftover-complete from the 06:46 issue close comment (`top-pairs` + `/health` SHA). If **#1305** is still closed on that comment, **reopen it as a leftover precondition**. Reopen is not leftover-complete. Keywords on #1305 are not #297 authority or `DESIGN: APPROVE`. |
| **M1305-7** | Child #1290 Playwright is **not** leftover-complete. Slice 1 runs #1290 with `VERIFY_ISSUE_1290_SKIP_E2E=1` by default. Optional leftover e2e stays explicit (unset `SKIP_E2E`, existing 5 workers, child’s `PLAYWRIGHT_WEB_PORT=30129`). Do **not** invent a second leftover e2e flag. Do **not** leak a different `PLAYWRIGHT_WEB_PORT`. Do **not** wait leftover-complete on #703 `YY-MM` ticks. `e2e-tx` stays **1 worker**. |
| **M1305-8** | This playbook + **Q23** + [ADR 0010](../docs/adr/0010-post-merge-leftover-1302-1304.md) + child skills stay crosslinked. GitLab CI quota is not a substitute for local verify. |

## Coolify leftovers (operator)

1. Indexer: if the #1276 checkbox is off, **manually** deploy `729b097f+`. Apply `…000` then `…001` then **`20260921130000`** (sqlx on boot). **#1305 attests `…30000` only.** Do **not** use `scripts/lib/postgres-psql.sh` against prod. Do **not** treat healthy `GET /health` as this attest. Heal is not queryable from `DATABASE_URL`.

   **sqlx how-to** (Coolify DB / indexer `DATABASE_URL`):

   ```sql
   SELECT version, description, success FROM _sqlx_migrations
   WHERE version = 20260921130000;
   ```

   Expect `version=20260921130000` `success=true`. A healthy boot does **not** imply this row.

   **heal how-to** (indexer logs; not `DATABASE_URL`): needle is the `tracing::info!` string in `indexer/src/db/queries/traders.rs`:

   `traders_healed lifetime totals from swap_events (GitLab #1277)`

   One of that line on first start after migrate, then silence on later process starts.

2. Frontend: rebuild from current `main` (`729b097f+`). **Leftover-1:** operator glance hub **`cUSTC / USD`** + **`cLUNC / USD`** vs CEX **`USTC`** / **`LUNC`** / **`vFDUSD`**, then **click the vFDUSD tab** and record the selected label (**not** **`VFDUSD`**). Idle `/protocol` selects **USTC**. HTTP pins: hashed-chunk unquoted **`cLUNC wrap`**, **`cLUNC`**, **`cUSTC`**, **`vFDUSD`** (any quote style; never require source `'`) are supporting rebuild-presence — **none prove leftover 1**. Do **not** grep concatenated **`cLUNC / USD`** in hashed chunks.

3. Do **not** infer the indexer auto-deploy checkbox from HTTP ([ADR 0006](../docs/adr/0006-indexer-health-git-sha.md)).

`make verify-issue-1305` live is **HTTP only** (supporting rebuild-presence: unquoted **`cLUNC wrap`** / **`cLUNC`** / **`cUSTC`** / **`vFDUSD`**, any quote style; never require source `'`). Live HTTP **must not** treat **`cLUNC`** as leftover-1 complete. Leftover-1 is operator DOM glance. Do **not** grep concatenated `cLUNC / USD` in hashed chunks. Do **not** grep `protocol-top-pairs`. Do **not** copy #1276 `EXPECT_SHA`. Do **not** invent leftover `DATABASE_URL` or `VERIFY1305_LEFTOVER_COMPLETE`. Leftover probes SKIP unless hosts answer. Fail closed with `VERIFY1305_REQUIRE_LIVE=1` or `VERIFY1305_IID=1305`. HTTP PASS is not leftover-1 and not sqlx leftover-complete. Make does **not** cover leftover-complete.

## Leftover-complete

Close **#1305** only when all hold (same as [ADR 0010](../docs/adr/0010-post-merge-leftover-1302-1304.md) Rollout / Integration). Paste the leftover-complete comment below (same shape as [ADR 0006](../docs/adr/0006-indexer-health-git-sha.md) slice 3; no UUID/token/host).

- If **#1305** is still closed on the 06:46 comment, reopen it first as a leftover precondition. Reopen is **not** leftover-complete.
- Coolify leftover-2: **#1305 attests `20260921130000` only** via the Coolify leftovers **SELECT** (`success=true` — operator Coolify DB / indexer `DATABASE_URL`; **manual** indexer deploy of `729b097f+` while the #1276 checkbox is off). Same boot may also apply `…000` / `…001`; those rows are **#1300**, not a #1305 FAIL.
- Healthy `GET /health` is **not** this attest.
- Startup-before-D5 heal via indexer-log needle `traders_healed lifetime totals from swap_events (GitLab #1277)` (not `DATABASE_URL`).
- Coolify leftover-1 is operator glance hub **`cUSTC / USD`** + **`cLUNC / USD`** vs CEX **`USTC`** / **`LUNC`** / **`vFDUSD`** / click **vFDUSD** selected not **`VFDUSD`** (idle `/protocol` is **USTC**). HTTP unquoted **`cLUNC wrap`**, **`cLUNC`**, **`cUSTC`**, **`vFDUSD`** (any quote style; never require source `'`) are supporting rebuild-presence — **none prove leftover 1**.
- `make verify-issue-1305` **(implement)** green locally (children with #1290 E2E skipped + docs). Make does **not** cover leftover-complete. Child Playwright / #703 ticks are not leftover-complete.
- **#1300** still open unless its own leftover-complete holds.

Do **not** close #1305 on green child `make verify-issue-1290` / `1277` alone. Do **not** close on the 06:46 issue comment (`top-pairs` + `/health` SHA). Do **not** treat reopen as leftover-complete. Do **not** infer migrate from liveness.

### Leftover-complete close-comment template (required; no UUID/token/host)

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

## Do / don’t

- **Do** run `make verify-issue-1305` **(implement)** from a git worktree after pulling `main` (once implement lands the script). Default child 1290 skips E2E. Green make is not leftover-complete.
- **Do** link `frontend-dapp/node_modules` from the primary checkout in a git worktree. Do **not** `npm install` over a worktree symlink (#1299).
- **Do** reopen **#1305** if it is still closed on the 06:46 comment (leftover precondition). Reopen is **not** leftover-complete.
- **Do** paste the leftover-complete close-comment template (sqlx SELECT + heal needle + leftover-1 glance including click **vFDUSD**).
- **Don’t** reopen #1290 / #1240 / #1277 unless a merged invariant is wrong.
- **Don’t** close #1300 from this ticket or grep `protocol-top-pairs` as this leftover’s marker.
- **Don’t** grep concatenated `cLUNC / USD` in hashed chunks. Never require source `'` in leftover HTTP.
- **Don’t** treat HTTP `cLUNC` (or `cUSTC` / `vFDUSD` / `cLUNC wrap`) as leftover-1 proof. Leftover-1 is operator DOM glance.
- **Don’t** merge `cac-design-issue-1300` / `1302` / `1277` as-is.
- **Don’t** expand the heal gate to USD. Do not name the optional USD-gate test I10 (use **I11** / **M1305-4**).
- **Don’t** `force_merge`.
- **Don’t** treat GitLab CI quota as leftover evidence.
- **Don’t** invent `VERIFY1305_LEFTOVER_COMPLETE` or treat HTTP PASS / green make as leftover-complete.
- **Don’t** treat `GET /health` as `_sqlx_migrations` evidence. Do **not** query heal from `DATABASE_URL`.
- **Don’t** flip Coolify auto-deploy or edit `autonomy.rs` / HMAC.
- **Don’t** treat keywords on #1305 as #297 authority or `DESIGN: APPROVE`.
- **Don’t** treat a reopen of #1305 as leftover-complete.
- **Don’t** glance leftover-1 from idle `/protocol` only (must click **vFDUSD**).
- **Don’t** paste `GET /health`, `protocol-top-pairs`, hashed-chunk `cLUNC` PASS, or green make in the leftover-complete comment.

## Regression

After implement lands the script (not on this design branch):

```bash
make verify-issue-1305
VERIFY1305_SKIP_CHILDREN=1 make verify-issue-1305
VERIFY1305_SKIP_LIVE=1 make verify-issue-1305
VERIFY1305_REQUIRE_LIVE=1 make verify-issue-1305
```
