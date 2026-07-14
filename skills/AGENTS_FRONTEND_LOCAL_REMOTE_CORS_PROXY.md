# Agent playbook: local Vite ↔ remote indexer/LCD CORS proxy

Use when running the dApp on **`http://127.0.0.1:5173`** (or localhost) against **remote** soft-launch / mainnet backends such as `https://indexer.dex.cl8y.com` and `https://terra-classic-lcd.publicnode.com` ([GitLab #488](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/488) QA).

## Problem

Production indexer `CORS_ORIGINS` allows **`https://dex.cl8y.com` only**. Browser fetches from the Vite origin fail with:

- `CORS header ‘Access-Control-Allow-Origin’ missing` (indexer)
- `CORS request did not succeed` (public LCD from some networks)

Do **not** open production CORS to `*` to “fix” local QA.

## Fix (same-origin Vite proxy)

| Piece | Role |
|-------|------|
| [`viteDevProxy.ts`](../frontend-dapp/src/dev/viteDevProxy.ts) | Detect remote URLs; plan `/__dev/indexer` + `/__dev/lcd` |
| [`vite.config.ts`](../frontend-dapp/vite.config.ts) | `server.proxy` + `define` rewrite of `VITE_*_URL` during `vite` serve |
| Docs | [`docs/frontend.md` § Local CORS proxy](../docs/frontend.md#local-dev-remote-cors-proxy) |

**Invariant:** Keep real HTTPS URLs in `.env.local`. During serve, the browser only talks to same-origin `/__dev/*`; Vite forwards with `changeOrigin: true`.

## Rules

1. Soft-launch env example: [`deployments/mainnet-soft-launch/frontend.env.example`](../deployments/mainnet-soft-launch/frontend.env.example) — copy into `frontend-dapp/.env.local`, then `make dev` / `bash scripts/dev-frontend-local.sh`.
2. Startup logs should include `[vite-dev-proxy] indexer …` and `[vite-dev-proxy] lcd …`.
3. Verify: `curl -sS -o /dev/null -w '%{http_code}\n' http://127.0.0.1:5173/__dev/lcd/cosmos/base/tendermint/v1beta1/blocks/latest` → **200**.
4. Indexer **503** from Coolify is an upstream outage — proxy removes CORS but cannot invent a healthy indexer.
5. LocalTerra loopback (`127.0.0.1:3001` / `:1317`) must **not** be rewritten — leave direct.
6. Escape hatch: `VITE_DEV_PROXY=0` in `.env.local`.
7. Never enable this rewrite for `vite build` production artifacts.

## Related

- LocalTerra indexer CORS (`localhost` vs `127.0.0.1`): [`docs/frontend.md` § Local dev indexer CORS](../docs/frontend.md#local-dev-indexer-cors), [#131](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/131)
- Soft launch: [`AGENTS_MAINNET_SOFT_LAUNCH.md`](./AGENTS_MAINNET_SOFT_LAUNCH.md)
- Design system (#488): [`AGENTS_FRONTEND_DESIGN_SYSTEM.md`](./AGENTS_FRONTEND_DESIGN_SYSTEM.md)
