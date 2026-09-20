# Agent playbook: indexer `/health` git SHA + auto-deploy leftover (#1276)

Use when changing generic `GET /health`, `docker/indexer/Dockerfile` commit bake, or indexer Coolify auto-deploy docs. Canonical decision: [`docs/adr/0006-indexer-health-git-sha.md`](../docs/adr/0006-indexer-health-git-sha.md). Overview: [`docs/architecture.md`](../docs/architecture.md#indexer-production-attest).

**Issue:** [#1276](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1276)  
**Verify (implement):** `make verify-issue-1276` (add in the code slice)

Not CAC `/health`. Not fee-discount health. Not #1277. Do not scrape Coolify logs. Do not publish Coolify UUIDs or tokens. Do not flip the Coolify auto-deploy checkbox from an implement agent ([agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297)). Do not infer that checkbox from HTTP.

## Invariants (H1276-1–H1276-8)

| ID | Rule |
|----|------|
| **H1276-1** | `GET /health` is liveness: 200, `"status":"ok"`, no DB/LCD. Additive `git_sha` only. |
| **H1276-2** | Select `GIT_SHA` only if present **and non-empty after trim** (whitespace-only falls through); else `SOURCE_COMMIT`. No fallthrough after a **non-empty** rejected value (`HEAD`, `=`, secret prefix, non-hex). Parser: hex 7–40, lowercase; uppercase normalized. Omit, never echo. Required: `GIT_SHA=""` + `SOURCE_COMMIT=<hex>` → present; `GIT_SHA="   "` + hex `SOURCE_COMMIT` → present. Never copy Coolify `HEAD`/branch into `GIT_SHA`. |
| **H1276-3** | Re-declare ARG/ENV `GIT_SHA` + `SOURCE_COMMIT` on the **runtime** stage **after** `COPY --from=builder` (after apt/`useradd`; **not** immediately after `FROM runtime`) and **before** `HEALTHCHECK`. No `git` in either stage. Not Nixpacks. |
| **H1276-4** | `GET /api/v1/health/fee-discount` unchanged. |
| **H1276-5** | No inventory, tokens, or Coolify UUIDs on `/health`. |
| **H1276-6** | Auto-deploy is the indexer Coolify protected-branch flag. CAC drain is one UUID per path — not the indexer redeploy path. Do not infer the checkbox from HTTP. |
| **H1276-7** | Auto-deploy on ⇒ expand-only sqlx migrations; schema-ahead ⇒ forward-fix; Vite vs Rust skew is expected. Do not rewrite M573/M590 as if auto-deploy applied retroactively. |
| **H1276-8** | #1277, CAC map, #706, Nixpacks, second `/status` out of scope. |

## Do / don’t

- **Do** omit `git_sha` rather than echo `HEAD`.
- **Do** fall through empty / whitespace-only `GIT_SHA` (Dockerfile `ARG GIT_SHA=` → `ENV` `""`) to `SOURCE_COMMIT`.
- **Don’t** fall through after a non-empty rejected `GIT_SHA` (`HEAD` + leftover hex → omit).
- **Don’t** map Coolify `git_commit_sha=HEAD` (or a branch) into `GIT_SHA`. Leave `GIT_SHA` unset/empty; include-source-commit fills `SOURCE_COMMIT`.
- **Do** keep Docker HEALTHCHECK as HTTP 200 only.
- **Do** place runtime ARG/ENV after `COPY --from=builder`, not immediately after `FROM runtime` (apt-get cache).
- **Do** read env **per request** in `health()` (helpers stay pure `&str`; no `dotenvy` in the handler).
- **Do** write parser unit tests against `&str` / `Option<&str>` (no process env). Env-mutating integration tests are `#[serial]`. Ok-only exact `{"status":"ok"}` tests `remove_var("GIT_SHA")` and `remove_var("SOURCE_COMMIT")` before the request.
- **Don’t** wait on LCD/DB in `/health`.
- **Don’t** treat leftover Coolify checkbox as a merge blocker for the code MR.
- **Don’t** scrape Coolify logs for SOURCE SHA.
- **Don’t** stamp ARG/ENV only on the builder (ARG does not cross `FROM`).
- **Don’t** close leftover on regex-only live `git_sha` (bake presence ≠ follows `main`).

## Live leftover probe (`VERIFY1276_REQUIRE_LIVE=1` or `VERIFY1276_IID=1276`)

`GET https://indexer.dex.cl8y.com/health` must be 200. Parse JSON with **jq** (not grep): `status=ok` and hex `git_sha` `^[0-9a-f]{7,40}$`. Missing/omitted field is **FAIL**, not SKIP. Unreachable host is FAIL when the flag is set; SKIP without it.

If **`VERIFY1276_EXPECT_SHA`** is set (leftover glance after a land: full or short hex of that merge), FAIL unless one of `git_sha` / expect is a prefix of the other (case-insensitive). Checkbox stays operator/#297; do not infer it from HTTP.

Leftover-complete must list **both**: checkbox **on** **and** tip-match after an indexer-touching land. Regex-only is bake presence, not “follows `main`.” A stale hex from a one-off manual bake would PASS regex-only while auto-deploy is still off.

Do not scrape Coolify.

## Slice 2 (code MR docs)

`indexer/.env.example` optional `GIT_SHA` / `SOURCE_COMMIT` (hex only; do not document copying `HEAD` into `GIT_SHA`). Makefile `verify-issue-1276`. AGENTS.md **only if** it already lists 1276. Do not rewrite M573/M590.
