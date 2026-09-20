# Agent playbook: indexer `/health` git SHA + auto-deploy leftover (#1276)

Use when changing generic `GET /health`, `docker/indexer/Dockerfile` commit bake, or indexer Coolify auto-deploy docs. Canonical decision: [`docs/adr/0006-indexer-health-git-sha.md`](../docs/adr/0006-indexer-health-git-sha.md). Overview: [`docs/architecture.md`](../docs/architecture.md#indexer-production-attest).

**Issue:** [#1276](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1276)  
**Verify (implement):** `make verify-issue-1276` (add in the code slice)

Not CAC `/health`. Not fee-discount health. Not #1277. Do not scrape Coolify logs. Do not publish Coolify UUIDs or tokens. Do not flip the Coolify auto-deploy checkbox from an implement agent ([agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297)). Do not infer that checkbox from HTTP.

## Invariants (H1276-1–H1276-8)

| ID | Rule |
|----|------|
| **H1276-1** | `GET /health` is liveness: 200, `"status":"ok"`, no DB/LCD. Additive `git_sha` only. |
| **H1276-2** | Select `GIT_SHA` only if present **and non-empty after trim** (whitespace-only falls through); else `SOURCE_COMMIT`. No fallthrough after a **non-empty** rejected value (`HEAD`, `main` / `refs/heads/main`, `=`, secret prefix, non-hex including length 6 and 41). Parser: hex 7–40, lowercase; uppercase normalized. Trailing newline on a 40-char hex is present after trim. Omit, never echo. Required: `GIT_SHA=""` + `SOURCE_COMMIT=<hex>` → present; `GIT_SHA="   "` + hex `SOURCE_COMMIT` → present. Never copy Coolify `HEAD`/branch into `GIT_SHA`. Do not implement “parse fail → try the other env.” |
| **H1276-3** | Re-declare ARG/ENV `GIT_SHA` + `SOURCE_COMMIT` on the **runtime** stage **after** `COPY --from=builder` (after apt/`useradd`; **not** immediately after `FROM runtime`) and **before** `HEALTHCHECK`. No `git` in either stage. Not Nixpacks. |
| **H1276-4** | `GET /api/v1/health/fee-discount` unchanged. |
| **H1276-5** | No inventory, tokens, or Coolify UUIDs on `/health`. |
| **H1276-6** | Auto-deploy is the indexer Coolify protected-branch flag. CAC drain is one UUID per path — not the indexer redeploy path. Do not infer the checkbox from HTTP. |
| **H1276-7** | Auto-deploy on ⇒ expand-only sqlx migrations; dual-app skew is expected. Coolify-era rollback is **three-way** vs prior Coolify Deploys SHA (`git ls-tree --name-only <sha> indexer/migrations/`; `version`/`success`; `installed_on` inspect-only). **2(a)** unchanged → restore (auto-deploy off first, or land revert/hotfix before the next `main` webhook). **2(b)** keep schema + hotfix that still ships N **even if** a paired `down.sql` exists. **2(c)** **iff** restoring the prior image is required **and** `indexer/migrations/revert/<ahead-version>_*.down.sql` exists for version(s) **newer than that baseline** (historical `revert/` files do **not** select 2(c); no paired file for the ahead version → 2(b); do not invent a revert) → auto-deploy off → snapshot → that `down.sql` (descending if several) → `DELETE` **each** matching `_sqlx_migrations` row → restore. After 2(c), re-enable auto-deploy only when `main` no longer ships N, or re-applying N is explicit intent. Dirty: `DELETE FROM _sqlx_migrations WHERE version = <v> AND success = false` only; never `UPDATE success`. Revert files do not touch the ledger. No production `set_ignore_missing`. Inspect prod via Coolify DB / indexer `DATABASE_URL`, not `postgres-psql.sh`. Do not rewrite M573/M590 as if auto-deploy applied retroactively. |
| **H1276-8** | #1277, CAC map, #706, Nixpacks, second `/status` out of scope. |

## Do / don’t

- **Do** omit `git_sha` rather than echo `HEAD`.
- **Do** fall through empty / whitespace-only `GIT_SHA` (Dockerfile `ARG GIT_SHA=` → `ENV` `""`) to `SOURCE_COMMIT`.
- **Don’t** fall through after a non-empty rejected `GIT_SHA` (`HEAD` or `main` / `refs/heads/main` + leftover hex → omit). Do not implement “parse fail → try the other env.”
- **Don’t** map Coolify `git_commit_sha=HEAD` (or a branch) into `GIT_SHA`. Leave `GIT_SHA` unset/empty; include-source-commit fills `SOURCE_COMMIT`. Copying `HEAD` into `GIT_SHA` **and** enabling include-source-commit still omits (`HEAD` is a non-empty reject). Include-source-commit **alone** is enough because empty `GIT_SHA` falls through.
- **Do** omit a whitespace-only **selected** candidate (`parse_git_sha`); that is distinct from whitespace-only `GIT_SHA` at select time (fall through).
- **Do** keep Docker HEALTHCHECK as HTTP 200 only.
- **Do** place runtime ARG/ENV after `COPY --from=builder`, not immediately after `FROM runtime` (apt-get cache).
- **Do** read env **per request** in `health()` (helpers stay pure `&str`; no `dotenvy` in the handler).
- **Do** write parser unit tests against `&str` / `Option<&str>` (no process env). Env-mutating integration tests are `#[serial]`. Ok-only exact `{"status":"ok"}` tests `remove_var("GIT_SHA")` and `remove_var("SOURCE_COMMIT")` before the request.
- **Don’t** wait on LCD/DB in `/health`.
- **Don’t** treat leftover Coolify checkbox as a merge blocker for the code MR.
- **Don’t** scrape Coolify logs for SOURCE SHA.
- **Don’t** stamp ARG/ENV only on the builder (ARG does not cross `FROM`).
- **Don’t** close leftover on regex-only live `git_sha` (bake presence ≠ follows `main`).
- **Don’t** close leftover on `VERIFY1276_IID=1276` / `VERIFY1276_LEFTOVER_COMPLETE=1` without `VERIFY1276_EXPECT_SHA` (**FAIL before curl**; stale manual hex must not close). Sibling leftover IID is unreachable-fail, not leftover-complete.
- **Don’t** treat `VERIFY1276_REQUIRE_LIVE=1` without IID/`EXPECT_SHA` as leftover-complete (that path may PASS bake presence).
- **Don’t** enable Coolify watch paths until leftover is closed if glance uses repo `HEAD`.
- **Don’t** treat `SELECT version … LIMIT 5` or “no new row” as unchanged — compare `_sqlx_migrations` (`version`, `success`; `installed_on` inspect-only) to `git ls-tree --name-only <prior-coolify-deploys-sha> indexer/migrations/`.
- **Don’t** treat any file under `indexer/migrations/revert/` as a 2(c) selector. 2(c) **iff** `indexer/migrations/revert/<ahead-version>_*.down.sql` exists for version(s) **newer than that baseline**. Historical revert files do **not** select 2(c). No paired file for the ahead version → **2(b)**; do not invent a revert.
- **Don’t** take 2(c) just because a paired `down.sql` exists. If the next image will still ship N, **2(b)** keep schema (do not restore the prior image). 2(c) only when restoring the prior image is required.
- **Don’t** apply `down.sql` then restore without `DELETE FROM _sqlx_migrations WHERE version = <that version>` for **each** reverted ahead version (revert files do not touch the ledger; several versions: descending).
- **Don’t** `UPDATE _sqlx_migrations SET success = true`. Dirty repair is `DELETE FROM _sqlx_migrations WHERE version = <v> AND success = false` only.
- **Don’t** start 2(c) with auto-deploy still on. After 2(c), **don’t** re-enable auto-deploy while `main` still ships N (a typical hotfix still embeds N — that is **2(b)**). Re-enable only when `main` no longer ships N, or re-applying N is explicit intent.
- **Don’t** 2(a) restore with auto-deploy still on while `main` is the bad SHA without a land-before-next-webhook plan (sticky until the next webhook; same retrigger class as 2(c), without re-applying N).
- **Don’t** set `set_ignore_missing(true)` on production `sqlx::migrate!()`.

## Live leftover probe

Canonical: [ADR 0006 Tests](../docs/adr/0006-indexer-health-git-sha.md). Overview: [`docs/architecture.md`](../docs/architecture.md#indexer-production-attest) is **target after slice 1**.

`GET https://indexer.dex.cl8y.com/health` must be 200. Parse JSON with **jq** (not grep): `status=ok` and hex `git_sha` `^[0-9a-f]{7,40}$`. Missing/omitted field is **FAIL**, not SKIP. Unreachable host is FAIL when a live flag is set; SKIP without it.

**Sibling leftover scripts** (`verify-issue-701.sh` and 673/686/698/702/706): `VERIFY*_IID=<n>` / `REQUIRE_LIVE=1` means **live probe required; unreachable = FAIL not SKIP**. They do **not** implement leftover-complete. There is no repo-wide `EXPECT_SHA` outside this design. Copying `require_live()` is not the #1276 leftover gate.

**Bake presence:** `VERIFY1276_REQUIRE_LIVE=1` **without** `VERIFY1276_IID` and **without** `VERIFY1276_EXPECT_SHA` may PASS on hex `git_sha`. That is not leftover-complete.

**Leftover-complete gate:** #1276 **adds** `VERIFY1276_EXPECT_SHA` under `VERIFY1276_IID=1276` **or** `VERIFY1276_LEFTOVER_COMPLETE=1`. Unset or whitespace-only after trim **must FAIL before curl**. Bare `VERIFY1276_IID=1276` without `EXPECT_SHA` is an **intentional FAIL**, not a #701 bug. Script exit `1` when `FAIL > 0`. Do not PASS bake presence under the IID.

If **`VERIFY1276_EXPECT_SHA`** is set (leftover glance after a land: full or short hex of that merge), FAIL unless one of `git_sha` / expect is a prefix of the other (case-insensitive). Checkbox stays operator/#297; do not infer it from HTTP.

Issue AC maps as in ADR Outcome: keep the checkbox; strengthen “matching the baked commit” to `EXPECT_SHA` prefix-match; leftover does **not** HTTP-attest the Vite app.

Rollout leftover command: `VERIFY1276_REQUIRE_LIVE=1 VERIFY1276_EXPECT_SHA=$(git rev-parse HEAD) make verify-issue-1276`. Close leftover with the ADR slice 3 comment template (checkbox attested, command + jq `git_sha`, watch paths off, include-source-commit arg name).

Leave Coolify watch paths **off** until leftover is closed if glance uses repo `git rev-parse HEAD`. If include-source-commit injects a name other than `SOURCE_COMMIT`, leftover still has the hex-`GIT_SHA` path — confirm at leftover, not in the code MR.

Do not scrape Coolify.

## Slice 2 (code MR docs)

`indexer/.env.example` optional `GIT_SHA` / `SOURCE_COMMIT` (hex only; do not document copying `HEAD` into `GIT_SHA`). Makefile `verify-issue-1276`. AGENTS.md **only if** it already lists 1276. Do not rewrite M573/M590.
