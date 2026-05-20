# Agent playbook: LCD / RPC outage UX (frontend)

Use when the Terra **LCD** is unavailable and the dApp shows frozen spinners, missing retry affordances, or fails to auto-recover after the node returns ([GitLab **#171**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/171) — W11-C2).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § LCD / RPC connectivity](../docs/frontend.md#lcd-rpc-connectivity) | Invariant table |
| [`frontend-dapp/src/utils/lcdConnectivity.ts`](../frontend-dapp/src/utils/lcdConnectivity.ts) | **`isLcdConnectivityError`**, **`probeLcdReachability`**, **`LCD_CONNECTIVITY_OUTAGE_MESSAGE`** |
| [`frontend-dapp/src/hooks/useLcdConnectivityRecovery.ts`](../frontend-dapp/src/hooks/useLcdConnectivityRecovery.ts) | Health probe + **`invalidateQueries`** on recovery |
| [`frontend-dapp/src/components/common/LcdConnectivityBanner.tsx`](../frontend-dapp/src/components/common/LcdConnectivityBanner.tsx) | Global layout banner |
| [`frontend-dapp/src/components/common/LcdQueryGate.tsx`](../frontend-dapp/src/components/common/LcdQueryGate.tsx) | Per-section loading / **`RetryError`** gate |
| [`frontend-dapp/src/App.tsx`](../frontend-dapp/src/App.tsx) | `QueryClient` retry / reconnect defaults |

## Rules of thumb

1. **Blocking LCD queries** — wrap factory pair loads and other must-have LCD data with **`LcdQueryGate`** or an explicit **`RetryError`** when `isError` (see Swap, Trade, Limits, Pool).
2. **Do not rely on tab focus** — recovery is the **5s LCD probe** + query invalidation, not `refetchOnWindowFocus` (global default remains **false**).
3. **Indexer vs LCD** — indexer outage banners ([GitLab **#164**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/164)) are separate; this playbook is for **chain LCD / wallet RPC** transport.
4. **New patterns** — extend **`isLcdConnectivityError`** and [`lcdConnectivity.test.ts`](../frontend-dapp/src/utils/__tests__/lcdConnectivity.test.ts); keep retail copy in **`LCD_CONNECTIVITY_OUTAGE_MESSAGE`** unless product asks for a variant.

## Manual QA (local)

1. `make start` + `make deploy-local` + `npm run dev` with `VITE_NETWORK=local`.
2. `docker compose stop localterra` → reload `/swap`, `/pool`, `/trade` → expect banner + **`RetryError`** (no infinite skeleton).
3. `docker compose start localterra` → wait ~5–10s on the same tab → data reloads without tab switch.

## Related

- Retail error funnel: [`AGENTS_FRONTEND_USER_ERRORS.md`](./AGENTS_FRONTEND_USER_ERRORS.md)
- Trade indexer outage: [docs/frontend.md § Trade page — indexer outage banner](../docs/frontend.md#trade-page-indexer-outage-banner)
