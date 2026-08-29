# Agent playbook: frontend production build hygiene

Use when changing **Vite build output**, **source maps**, or reviewing PRs that touch `frontend-dapp/vite.config.ts` for security-sensitive defaults.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Production build — Vite source maps](../docs/frontend.md#vite-production-sourcemaps) | Invariants, prod vs non-prod `mode`, checklist pointer ([GitLab #117](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/117)) |
| `frontend-dapp/vite.config.ts` | `build.sourcemap` — must stay **disabled for `mode === 'production'`** unless product/security explicitly approves a different strategy (e.g. hidden maps + upload-only tooling). |
| `frontend-dapp/viteCsp.ts` | Production CSP `connect-src` / `img-src` builders — no blanket `https:` in prod ([#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/378)). |
| `frontend-dapp/src/viteConfig.build.test.ts` | Vitest guard: `loadConfigFromFile` asserts prod `sourcemap === false`, staging mnemonic rejection, missing `VITE_WC_PROJECT_ID` on production builds, and production `VITE_DEV_MODE=true` rejection ([#695](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/695)). Production-*allow* cases pin `process.env.VITE_DEV_MODE` so `.env.local` cannot re-inject `true` ([#698](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/698)). |
| `frontend-dapp/src/utils/__tests__/viteCsp.test.ts` | Production CSP meta content, env host narrowing, and `render.yaml` static-header omission ([#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/378)). |
| [GitLab #139](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/139) | Connect-modal QA checklist includes **`npm run build`** / **`npx vitest run`** gates (see [docs/frontend.md § Connect modal](../docs/frontend.md#connect-modal-extension-install)). |

## Rules of thumb

1. **Default `npm run build`** must not emit browser-served `*.js.map` for the production bundle (verify with a smoke build and `find dist -name '*.js.map'`).
2. **`tsc -b` is part of build** — test mocks and helpers are type-checked. Use **`as unknown as MediaQueryList`** for `matchMedia` stubs (see [`useMediaQuery.test.tsx`](../frontend-dapp/src/hooks/__tests__/useMediaQuery.test.tsx)); honor optional params in mocked API signatures; prefer **type predicates** (e.g. **`isKnownFactoryTradePair`**) when guards narrow route params for `setState`.
3. **Staging-only maps** belong behind `vite build --mode <non-production>` or explicit env gates — not unconditional `sourcemap: true`.
4. **Gas / swap work** is unrelated; use [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md) for `out of gas` and fee constants. **Swap max-spread / price-impact UX** (LCD preflight, error copy) lives in [`docs/swap-max-spread-ux.md`](../docs/swap-max-spread-ux.md) ([GitLab **#134**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/134)).

## Related (local dev only)

- Production `VITE_DEV_MODE` reject (#695 / FE-01): [`AGENTS_FRONTEND_DEV_MODE_GUARD.md`](./AGENTS_FRONTEND_DEV_MODE_GUARD.md)
- **Indexer CORS vs Vite hostname** affects browser `fetch` to `VITE_INDEXER_URL` (e.g. limit-order cancel-ID polling). Not a production-build concern, but agents touching env templates should know: [GitLab **#131**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/131), [`docs/frontend.md` § Local dev indexer CORS](../docs/frontend.md#local-dev-indexer-cors), [`AGENTS_LOCALNET_TRADING_SWARM.md`](./AGENTS_LOCALNET_TRADING_SWARM.md).

## Related (layout / navigation)

- **Header breakpoints & tablet More overflow:** [`docs/frontend.md` § Responsive shell & header navigation](../docs/frontend.md#responsive-header-navigation), [`AGENTS_FRONTEND_RESPONSIVE_HEADER.md`](./AGENTS_FRONTEND_RESPONSIVE_HEADER.md) ([GitLab **#136**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/136)).
- **Risk / NFA / first-visit modal:** [`docs/frontend.md` § Risk surfacing, NFA copy, and first-visit acknowledgement](../docs/frontend.md#legal-risk-surfacing), [`AGENTS_FRONTEND_RISK_DISCLAIMERS.md`](./AGENTS_FRONTEND_RISK_DISCLAIMERS.md) ([GitLab **#138**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/138)).

## Related (form accessibility)

- **Programmatic labels for inputs:** [`docs/frontend.md` § Form inputs — programmatic labels](../docs/frontend.md#form-inputs-programmatic-labels), [`AGENTS_FRONTEND_A11Y_FORM_LABELS.md`](./AGENTS_FRONTEND_A11Y_FORM_LABELS.md) ([GitLab **#143**](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/143)).
