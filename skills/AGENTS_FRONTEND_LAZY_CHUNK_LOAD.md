# Agent playbook: stale lazy-chunk reload after Coolify deploy (#706)

Use when a **long-lived tab** hits **Page unavailable** after a frontend roll, when changing [`LazyRoute`](../frontend-dapp/src/components/common/LazyRoute.tsx) / [`chunkLoadError.ts`](../frontend-dapp/src/utils/chunkLoadError.ts), or when reviewing nginx cache headers on hashed assets.

**Issue:** [GitLab **#706**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/706)  
**Offline Try Again (closed, UX only):** [#172](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/172) — do **not** reopen #172 for stale Coolify hashes.  
**Do not un-split routes:** [#179](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/179). LCD outage is a different banner: [#171](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/171).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Lazy route chunks](../docs/frontend.md#lazy-route-chunks) | Offline #172 + stale-deploy #706 invariant table |
| [`chunkLoadError.ts`](../frontend-dapp/src/utils/chunkLoadError.ts) | `isChunkLoadError`, `reloadOnceOnStaleChunk`, URL scrub |
| [`LazyRoute.tsx`](../frontend-dapp/src/components/common/LazyRoute.tsx) | `loadAttempt` + fresh `lazy(loader)` on Try Again |
| [`ErrorBoundary.tsx`](../frontend-dapp/src/components/common/ErrorBoundary.tsx) | Updating… then one-shot reload; guarded **Reload app** + **Try Again** |
| [`RouteContentReadyMarker.tsx`](../frontend-dapp/src/components/common/RouteContentReadyMarker.tsx) | Clears the sessionStorage guard on successful lazy mount |
| [`docker/frontend/nginx.conf`](../docker/frontend/nginx.conf) | HTML `no-cache, must-revalidate`; hashed 200 `immutable`; 404 JS `no-store` |
| [docs/runbooks/rollback-decision.md](../docs/runbooks/rollback-decision.md) | Stale `index.html` / CDN cache-key |

## Invariants (L706-1–L706-8)

| ID | Rule |
|----|------|
| **L706-1** | Online `isChunkLoadError` on a lazy route **one-shot** `window.location.reload()` after setting `sessionStorage` key `cl8y-dex-stale-chunk-reload`. First paint is **Updating…** (`stale-chunk-updating`), not **Page unavailable**. Successful lazy mount **clears** the key so a *later* deploy in the same tab can recover again. |
| **L706-2** | If the chunk 404s **again** after that reload (broken deploy), **do not** loop-reload. Show the route card with **Reload app** (`location.reload()`) **and** **Try Again** (`loadAttempt++` re-import). |
| **L706-3** | `navigator.onLine === false`: **no** auto-reload and **no** storage write. **Page unavailable** + **Try Again** re-import stays the [#172](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/172) path when hashes still exist. |
| **L706-4** | Non-chunk React / contract / indexer errors **must not** `location.reload()`. App-level chunk path still **Reload App**. |
| **L706-5** | `isChunkLoadError` is true for Chrome `Failed to fetch dynamically imported module`, Firefox `error loading dynamically imported module`, Safari `Importing a module script failed`, and `ChunkLoadError: Loading chunk N failed`. False for indexer `Failed to fetch`, LCD timeouts, `Max spread`, user reject. Technical details never echo `https://dex.cl8y.com/assets/…`. Residual: an untrusted `error.message` that copies a browser chunk phrase could one-shot reload — same UI-trust caveat as [#145](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/145). Match **browser chunk patterns only**. |
| **L706-6** | Route fallback stays inside [`Layout`](../frontend-dapp/src/components/common/Layout.tsx) (header/nav). Reload is **same-origin document only** — never assign the failed module URL (`javascript:`, `https://evil.example/…`). No service worker / Workbox. Do **not** eagerly `import()` every page. CSP unchanged (`script-src 'self'`). No secrets in the reload key. |
| **L706-7** | nginx: hashed **200** JS/CSS stay `public, immutable`. HTML / SPA `try_files` → `/index.html` is `no-cache, must-revalidate` (not `immutable`). Missing `*.js` is **404** with `Cache-Control: no-store` (not SPA HTML, not immutable). |
| **L706-8** | This playbook + `docs/frontend.md` + `make verify-issue-706`. Live Coolify leftover (tab across a roll) is operator QA after the next frontend rebuild — do not wait on GitLab CI quota. |

## Stale deploy vs offline

| Cause | Network | Hashes on disk | Recovery |
|-------|---------|----------------|----------|
| Coolify replaced `/assets/PoolPage-<oldhash>.js` | Online | Old hash **gone** | One-shot document reload → new `index.html` names the new chunk |
| Transient miss / DevTools Offline | Offline or blip | **Same** deploy | **Try Again** re-`import()` (#172). Reload cannot invent a network |
| Broken live deploy (new shell 404s its own chunks) | Online | New hash **missing** | Guard stops after one reload; **Reload app** + **Try Again** |

Do **not** special-case PoolPage. The failing URL is whichever lazy chunk the old shell requested (`/charts`, `/trade`, `/protocol`, `/token/create`, …).

## nginx / CDN

- Hashed **200** stay cache-bust-by-filename (`immutable`).
- A CDN that ignores `no-cache` on `/` / `/index.html` keeps serving the old shell after reload — purge HTML (not JS `no-store`). See [rollback-decision.md](../docs/runbooks/rollback-decision.md).
- Deploy-race 404 of a **new** hash must not be cached as immutable (**L706-7**).

## Do / don’t

- **Do** run `make verify-issue-706` from a git worktree after pulling `main`.
- **Do** link `frontend-dapp/node_modules` from the primary checkout in a git worktree. Do **not** `npm install` over a worktree symlink.
- **Don’t** add a service worker or keep N generations of hashed files in the Docker image as the primary fix.
- **Don’t** treat LCD/indexer `Failed to fetch` as a chunk error.
- **Don’t** parse the TypeError string into `window.location.href`.
- **Don’t** reopen #172 for Coolify hash 404s.

## Attack notes

| Vector | Control |
|--------|---------|
| Infinite reload DoS | One reload per session until successful mount (**L706-2**) |
| Open redirect | `reloadSameOriginDocument()` only (**L706-6**) |
| sessionStorage throw | Skip auto-reload, show fallback, no uncaught exception |
| CSP bypass | No new `'unsafe-eval'` / `'unsafe-inline'` handlers |
| Mixed-content / MIME | Missing JS is **404**, not `index.html` as `application/javascript` |

## Verification

```bash
make verify-issue-706
VERIFY706_SKIP_E2E=1 make verify-issue-706
VERIFY706_REQUIRE_LIVE=1 make verify-issue-706
VERIFY706_REQUIRE_NGINX=1 make verify-issue-706
```

No LocalTerra, indexer schema, or wallet work for the default gate. Optional Playwright is `e2e/stale-chunk-reload-706.spec.ts` at **5 workers** (`e2e-smoke`). Optional docker nginx header smoke needs Docker. Live `curl -sI https://dex.cl8y.com/` SKIP unless reachable (FAIL when `VERIFY706_REQUIRE_LIVE=1`).

## Related

- Offline Try Again: this file historically (#172); stale-deploy path is #706.
- Trade LCP / do not un-split: [`AGENTS_FRONTEND_TRADE_INITIAL_LOAD.md`](./AGENTS_FRONTEND_TRADE_INITIAL_LOAD.md)
- LCD outage: [`AGENTS_FRONTEND_LCD_CONNECTIVITY.md`](./AGENTS_FRONTEND_LCD_CONNECTIVITY.md)
- User-facing errors: [`AGENTS_FRONTEND_USER_ERRORS.md`](./AGENTS_FRONTEND_USER_ERRORS.md)
- Production CSP / no SW: [`AGENTS_FRONTEND_TRUST_BOUNDARIES.md`](./AGENTS_FRONTEND_TRUST_BOUNDARIES.md)
- Rollback / stale HTML: [`AGENTS_ROLLBACK_DECISION.md`](./AGENTS_ROLLBACK_DECISION.md)
