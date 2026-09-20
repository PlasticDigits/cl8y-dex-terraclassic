# Agent playbook: indexer `/health` git SHA + auto-deploy leftover (#1276)

Use when changing generic `GET /health`, `docker/indexer/Dockerfile` commit bake, or indexer Coolify auto-deploy docs. Canonical decision: [`docs/adr/0006-indexer-health-git-sha.md`](../docs/adr/0006-indexer-health-git-sha.md). Overview: [`docs/architecture.md`](../docs/architecture.md#indexer-production-attest).

**Issue:** [#1276](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1276)  
**Verify (implement):** `make verify-issue-1276` (add in the code slice; live Coolify SKIP unless `VERIFY1276_REQUIRE_LIVE=1`)

Not CAC `/health`. Not fee-discount health. Not #1277. Do not scrape Coolify logs. Do not publish Coolify UUIDs or tokens. Do not flip the Coolify auto-deploy checkbox from an implement agent ([agent-control #297](https://git.cl8y.com/PlasticDigits/cl8y-agent-control/issues/297)).

## Invariants (H1276-1–H1276-8)

| ID | Rule |
|----|------|
| **H1276-1** | `GET /health` is liveness: 200, `"status":"ok"`, no DB/LCD. Additive `git_sha` only. |
| **H1276-2** | Parser: hex 7–40, lowercase; uppercase normalized; omit `HEAD` / whitespace / `=` / secret prefixes (`Bearer `, `sk-`, `glpat-`, `N|`, …). |
| **H1276-3** | Dockerfile ARG/ENV `GIT_SHA` + `SOURCE_COMMIT` required. No `git` in the image. Not Nixpacks. |
| **H1276-4** | `GET /api/v1/health/fee-discount` unchanged. |
| **H1276-5** | No inventory, tokens, or Coolify UUIDs on `/health`. |
| **H1276-6** | Auto-deploy is the indexer Coolify protected-branch flag. CAC drain is one UUID per path — not the indexer redeploy path. |
| **H1276-7** | Auto-deploy on ⇒ expand-only sqlx migrations; schema-ahead ⇒ forward-fix; Vite vs Rust skew is expected. |
| **H1276-8** | #1277, CAC map, #706, Nixpacks, second `/status` out of scope. |

## Do / don’t

- **Do** omit `git_sha` rather than echo `HEAD`.
- **Do** read `GIT_SHA` then `SOURCE_COMMIT`; no fallthrough after a rejected first value.
- **Do** keep Docker HEALTHCHECK as HTTP 200 only.
- **Don’t** wait on LCD/DB in `/health`.
- **Don’t** treat leftover Coolify checkbox as a merge blocker for the code MR.
- **Don’t** scrape Coolify logs for SOURCE SHA.
