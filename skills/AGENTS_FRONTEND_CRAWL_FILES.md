# Agent playbook: DEX crawl files (#1212)

Use when changing DEX crawler endpoints, nginx HTML indexing policy, or their verification. [Issue #1212](https://git.cl8y.com/code/cl8y-dex-terraclassic/issues/1212) bundles robots, sitemap, and missing-file behavior. Read the body and comments; the September 2026 live baseline was SPA HTML 200 at both endpoints and their siblings.

## Canonical references

| Resource | Purpose |
|----------|---------|
| [docs/frontend.md § Crawl endpoints](../docs/frontend.md#crawl-endpoints) | Invariants **CRAWL-1–CRAWL-6** |
| [Issue #1212 QA checklist](../docs/qa/issue-1212/README.md) | AC1–AC7, local/live evidence, permitted skips |
| [robots.txt](../frontend-dapp/public/robots.txt) / [sitemap.xml](../frontend-dapp/public/sitemap.xml) | Vite public artifacts |
| [nginx.conf](../docker/frontend/nginx.conf) | Exact file-only routes, crawler siblings, HTML noindex |
| [verify-issue-1212.sh](../scripts/qa/verify-issue-1212.sh) | Static policy checks and local nginx HTTP regression |
| [Open Graph playbook](./AGENTS_FRONTEND_OPENGRAPH.md) | Preserve **OG-1–OG-8**, PNG and one-shell metadata |
| [Lazy chunk playbook](./AGENTS_FRONTEND_LAZY_CHUNK_LOAD.md) | Preserve **L706-7**, immutable assets and missing-hash 404 |

## Invariants

1. **CRAWL-1:** `GET /robots.txt` is `200 text/plain` with `User-agent` syntax; `GET /sitemap.xml` is `200 application/xml` with a valid sitemap-namespace `urlset`. Check the body as well as headers.
2. **CRAWL-2:** Both exact nginx locations use `try_files $uri =404`. A missing file or sibling variant never becomes HTML 200. Include `/robots.txt/`, `/robots.txt.bak`, `/sitemap.xml/`, and `/sitemap_index.xml` in HTTP checks.
3. **CRAWL-3:** The current sitemap is empty. Do not list wallet, manage, mint, trader-address, or unbounded pair paths. Future entries require an intentional crawl policy and the exact `https://dex.cl8y.com` origin.
4. **CRAWL-4:** Robots contains only `User-agent: *`, `Disallow:` (empty), and `Sitemap: https://dex.cl8y.com/sitemap.xml`. HTML returns `X-Robots-Tag: noindex, nofollow`; allowing crawlers to fetch HTML exposes the noindex instruction. Robots is not access control. This DEX policy does not apply to another content host.
5. **CRAWL-5:** Preserve real `/og-image.png`, immutable successful JS/CSS, `no-store` missing hashed assets, and revalidated HTML. Keep one static OG/title shell; no route-specific metadata, prerender, or marketing claims.
6. **CRAWL-6:** Distinguish source-fixture HTTP checks, built-dist HTTP checks, and production acceptance. A local PASS never proves Coolify deployed the change.

## Agent verification

Run checks from a worktree. This issue needs no LocalTerra, wallet, or indexer work.

```bash
make verify-issue-1212
make verify-issue-578
PLAYWRIGHT_WEB_PORT=5189 PLAYWRIGHT_HTML_OPEN=never make verify-issue-706
# Verify a real production build instead of the default source fixture:
VERIFY1212_DIST_DIR=/absolute/path/to/frontend-dapp/dist make verify-issue-1212
# Require local nginx runtime; Docker absence becomes a failure:
VERIFY1212_REQUIRE_NGINX=1 make verify-issue-1212
# Opt into post-deploy HTTP acceptance:
VERIFY1212_LIVE_URL=https://dex.cl8y.com make verify-issue-1212
```

The default #1212 verifier may skip local nginx when Docker is unavailable. An explicitly enabled runtime/live check must report an observed mismatch as FAIL. Review the diff and use the [QA checklist](../docs/qa/issue-1212/README.md) to record acceptance evidence.

Under the implementation request's verification allowance, an agent may mark an unavailable item **SKIP** with its reason and report **PASS with skips** for the completed scope. Do not label a skipped AC as completed, turn an observed failure into PASS, or infer live deployment from a local test. Include the skipped checks in the PR and final report. Attempt checks that are available before taking this allowance.
