# Agent playbook: frontend production build hygiene

Use when changing **Vite build output**, **source maps**, or reviewing PRs that touch `frontend-dapp/vite.config.ts` for security-sensitive defaults.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Production build — Vite source maps](../docs/frontend.md#vite-production-sourcemaps) | Invariants, prod vs non-prod `mode`, checklist pointer ([GitLab #117](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/117)) |
| `frontend-dapp/vite.config.ts` | `build.sourcemap` — must stay **disabled for `mode === 'production'`** unless product/security explicitly approves a different strategy (e.g. hidden maps + upload-only tooling). |
| `frontend-dapp/src/viteConfig.build.test.ts` | Vitest guard: `loadConfigFromFile` asserts prod `sourcemap === false` and non-prod remains enabled. |

## Rules of thumb

1. **Default `npm run build`** must not emit browser-served `*.js.map` for the production bundle (verify with a smoke build and `find dist -name '*.js.map'`).
2. **Staging-only maps** belong behind `vite build --mode <non-production>` or explicit env gates — not unconditional `sourcemap: true`.
3. **Gas / swap work** is unrelated; use [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md) for `out of gas` and fee constants.
