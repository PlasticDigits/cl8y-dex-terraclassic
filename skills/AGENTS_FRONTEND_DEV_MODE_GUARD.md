# Agent playbook: production `VITE_DEV_MODE` reject (#695)

Use when changing Vite build guards, Simulated Wallet env, Coolify frontend build-args, or reviewing PRs that touch `frontend-dapp/vite.config.ts` `assertBuildEnvGuards`.

Parent audit: **FE-01** (`INTERNAL_GROK46_1787908099` / `INTERNAL_KIMIK3_1785897304`). Implementation: GitLab [#695](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/695). Do **not** reopen mnemonic (#118 / #378) or WalletConnect project-id guards.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Simulated wallet](../docs/frontend.md#simulated-dev-wallet-and-vite_dev_mnemonic) | Production `VITE_DEV_MODE` reject invariant |
| [docs/frontend.md § Trust boundaries](../docs/frontend.md#frontend-trust-boundaries) | Build-guard row includes #695 |
| [docs/security-model.md § Off-chain trust](../docs/security-model.md#off-chain-trust-boundaries-frontend) | Build-env surface + FE-01 row |
| [docs/operator-secrets.md](../docs/operator-secrets.md) | Coolify: leave `VITE_DEV_MODE` unset |
| `frontend-dapp/vite.config.ts` | `assertBuildEnvGuards` production reject |
| `frontend-dapp/src/viteConfig.build.test.ts` | `rejects production build when VITE_DEV_MODE is true` |
| `frontend-dapp/src/utils/constants.ts` | Runtime `DEV_MODE` — do not change the gate here |
| `frontend-dapp/src/services/terraclassic/devWallet.ts` | Runtime Simulated Wallet — leave unchanged |

## Invariants (D695-1–D695-8)

1. **D695-1 Production fail-closed** — `vite build --mode production` with `VITE_DEV_MODE=true` throws. No bundle. Error text names `VITE_DEV_MODE` and tells the operator to unset it in Coolify / `.env.production`.
2. **D695-2 Unconditional** — `VITE_ALLOW_DEV_MNEMONIC=local-only` does **not** permit `VITE_DEV_MODE` in production mode.
3. **D695-3 Local / QA keep the flag** — `development`, LocalTerra, Vitest (`vitest.config.ts`), and Playwright (`playwright.config.ts`) keep `VITE_DEV_MODE=true`. `vite build --mode staging` may keep it for Simulated Wallet QA.
4. **D695-4 Serve is not a build** — `command !== 'build'` skips all `assertBuildEnvGuards` (unchanged). Do not add a serve-time reject.
5. **D695-5 Mnemonic stays first** — Do not weaken or reorder the #118 / #378 mnemonic guard. When both flags are set, the mnemonic error still fires.
6. **D695-6 No seed logging** — Do not inline or log mnemonics in the new error path or tests.
7. **D695-7 Runtime gate unchanged** — `DEV_MODE` / `devWallet.ts` stay as-is. This ticket is a **build** reject, not a new runtime check.
8. **D695-8 Docs + verify** — This playbook + `docs/frontend.md` + `make verify-issue-695`.

## Rules of thumb

1. **Do not** treat a copied `.env.example` (`VITE_DEV_MODE=true`) as a Coolify production env. Production images must leave the variable unset.
2. **Do not** change CSP, WalletConnect, or clickwrap in this ticket.
3. Staging QA that needs Simulated Wallet uses **non-production** Vite mode. Production stays blocked.
4. A production bundle with the flag and **no** mnemonic still ships Simulated Wallet chrome — that is the phishing / wrong-network surface this guard closes.

## Verification

```bash
make verify-issue-695
```

No LocalTerra, indexer, or wallet work. Manual Coolify check (record on #695): production env has `VITE_DEV_MODE` unset.

Attack-path smoke (must fail):

```bash
cd frontend-dapp && VITE_DEV_MODE=true VITE_WC_PROJECT_ID=x npx vite build --mode production
```

## Cross-links

- Dev wallet / mnemonic: [`AGENTS_BUNDLE_DEV_WALLET.md`](./AGENTS_BUNDLE_DEV_WALLET.md) ([#118](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/118))
- Trust boundaries / WC id: [`AGENTS_FRONTEND_TRUST_BOUNDARIES.md`](./AGENTS_FRONTEND_TRUST_BOUNDARIES.md) ([#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/378))
- Production build hygiene: [`AGENTS_FRONTEND_PRODUCTION_BUILD.md`](./AGENTS_FRONTEND_PRODUCTION_BUILD.md)
