# Issue #1212: honest DEX crawl endpoints

[Issue and comments](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1212) reported robots, sitemap, and sibling variants returning the SPA shell in September 2026. The fix ships real files, exact nginx file-only locations, sibling rejection, an empty sitemap, and `X-Robots-Tag: noindex, nofollow` on HTML. Vite copies the public files into the frontend image; the shared OG/title shell remains unchanged.

Invariants: [docs/frontend.md § Crawl endpoints](../../frontend.md#crawl-endpoints) (**CRAWL-1–CRAWL-6**). Third-party agent instructions: [AGENTS_FRONTEND_CRAWL_FILES.md](../../../skills/AGENTS_FRONTEND_CRAWL_FILES.md). Implementation: [robots.txt](../../../frontend-dapp/public/robots.txt), [sitemap.xml](../../../frontend-dapp/public/sitemap.xml), [nginx.conf](../../../docker/frontend/nginx.conf), [verifier](../../../scripts/qa/verify-issue-1212.sh).

## Implementation evidence — 2026-10-04

- [x] **Local AC1/AC2 behavior:** source fixture and production-built dist both returned robots `200 text/plain` with the fixed directives and sitemap `200 application/xml` with an empty namespaced `urlset`.
- [x] **AC3:** removing both files from a disposable nginx root returned 404; all six tested sibling variants also returned 404.
- [x] **AC4:** empty sitemap omits every transaction route; HTTP checks confirmed the shared shell and noindex/nofollow for `/`, `/trade`, `/portfolio`, `/trader/qa-1212`, `/token/qa-1212/manage`, and `/mint`.
- [x] **AC5:** #1212 PNG/hash checks passed. `make verify-issue-578` passed (9 unit tests); `PLAYWRIGHT_WEB_PORT=5189 PLAYWRIGHT_HTML_OPEN=never make verify-issue-706` passed (53 unit tests, nginx smoke, live cache checks, and 3 browser tests).
- [x] **AC6:** fixed first-party robots directives and an empty sitemap contain no claims or external URLs. Independent review also checked rejected HTML, malformed/nonempty XML, foreign sitemap origins, and invalid verifier origins.
- [x] **AC7 local verifier, build, and retest:** production Vite build passed with a synthetic `VITE_WC_PROJECT_ID=qa1212-static-build` (test artifact only). `VERIFY1212_REQUIRE_NGINX=1 make verify-issue-1212` passed; the built-dist variant passed twice, including after final checker edits. Bash syntax, documentation links, independent review, and `git diff --check` passed.
- [ ] **Production AC1/AC2 and post-deploy AC7:** at 01:22 UTC, `https://dex.cl8y.com/robots.txt` still returned `200 text/html` with the old SPA shell. The strict live checker correctly failed on that media type. This branch has not been deployed; rerun live acceptance after the frontend deployment. Local verification is **PASS with production acceptance pending**, not a live PASS.

The initial #706 browser run reused Grafana on port 3000 and timed out. Its verifier now preserves an explicit `PLAYWRIGHT_WEB_PORT`; rerunning with a dedicated Vite port passed all three tests. The adjacent #578 cache assertion was also updated to the existing `no-cache, must-revalidate` policy. No application route metadata or asset caching policy changed.

## Review and verification checklist

Copy this checklist into the PR or verification report. Mark each item PASS, FAIL, or SKIP and attach command/output evidence. Check off an acceptance criterion only for the scope actually exercised; local AC1/AC2 behavior does not complete production AC1/AC2.

- [ ] **AC1:** Run `make verify-issue-1212`; robots is 200 `text/plain`, contains `User-agent`, and is not HTML. After deployment, run `VERIFY1212_LIVE_URL=https://dex.cl8y.com make verify-issue-1212` to complete the production criterion.
- [ ] **AC2:** The same verifier parses sitemap XML, checks its sitemap-namespace `urlset` and media type, and rejects HTML. Record local and production results separately.
- [ ] **AC3:** The local nginx verifier removes crawler files and checks both exact URLs return 404. Also check sibling variants `/robots.txt/`, `/robots.txt.bak`, `/sitemap.xml/`, and `/sitemap_index.xml` never return SPA HTML 200.
- [ ] **AC4:** Review the empty `urlset`; the local HTTP checks assert `/`, `/trade`, and `/portfolio` still serve HTML with `X-Robots-Tag: noindex, nofollow`. Wallet/manage/mint/trader/pair routes are omitted.
- [ ] **AC5:** Run `make verify-issue-578` and `PLAYWRIGHT_WEB_PORT=5189 PLAYWRIGHT_HTML_OPEN=never make verify-issue-706`; #1212 HTTP checks also verify PNG bytes/type, successful JS/CSS caching, and missing JS/CSS 404 with `no-store`.
- [ ] **AC6:** Review `User-agent: *`, empty `Disallow:`, the DEX `Sitemap:` directive, and empty sitemap. The static verifier rejects changed origin/directives, nonempty sitemap entries, and HTML. There are no product claims or third-party URLs.
- [ ] **AC7:** Confirm `make verify-issue-1212` exists and passes. Complete the distinct post-Coolify status/type/body check with the live verifier; retain its output and timestamp.
- [ ] **Build artifact:** Run the frontend production build, then `VERIFY1212_DIST_DIR=/absolute/path/to/frontend-dapp/dist make verify-issue-1212`. This verifies copied artifacts rather than the default source fixture.
- [ ] **Retest:** Rerun `make verify-issue-1212` after final edits and review `git diff --check`.

## Commands and evidence scope

```bash
make verify-issue-1212
make verify-issue-578
PLAYWRIGHT_WEB_PORT=5189 PLAYWRIGHT_HTML_OPEN=never make verify-issue-706
VERIFY1212_DIST_DIR=/absolute/path/to/frontend-dapp/dist make verify-issue-1212
VERIFY1212_REQUIRE_NGINX=1 make verify-issue-1212
VERIFY1212_LIVE_URL=https://dex.cl8y.com make verify-issue-1212
```

The default #1212 runtime uses a source fixture and the repository nginx configuration. Docker absence may produce a runtime SKIP; `VERIFY1212_REQUIRE_NGINX=1` requires that runtime. A production build is checked only when its absolute dist path is supplied. Opting into the live URL performs strict checks against the deployed server; production still serving the old shell is a deployment failure, not evidence against the locally tested fix.

Choose an unused `PLAYWRIGHT_WEB_PORT` for #706; the example port isolates Vite from other worktrees and services.

An agent may skip any checklist item it cannot complete, state the concrete reason, and report **PASS with skips** for completed verification under this implementation request. Attempt available checks first. Keep skipped criteria unchecked and list them separately; **PASS with skips does not mean every AC or live acceptance passed**. An observed test failure remains FAIL until resolved and successfully rerun. Do not claim deployment, wallet/device work, or production acceptance from static inspection or local HTTP evidence.

No LocalTerra, wallet, indexer, Playwright, or new per-route SEO surface is required by this change. A relevant follow-up, when production is not yet updated, is to deploy the frontend image and rerun the live HTTP checks.
