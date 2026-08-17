# Agent playbook: frontend trust boundaries (#378)

Use when hardening or reviewing **off-chain** security for the React dApp: indexer trust, build guards, CSP, token logos, expert mode, deploy address surfaces.

Parent remediation: GitLab [#376](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/376) · implementation [#378](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/work_items/378).

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/security-posture.md](../docs/security-posture.md) | Public launch posture; footer link (SEC-A01, [#387](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/387)) |
| [docs/security-model.md § Off-chain trust](../docs/security-model.md#off-chain-trust-boundaries-frontend--indexer) | Risks, mitigations, out-of-scope items |
| [docs/frontend.md § Trust boundaries](../docs/frontend.md#frontend-trust-boundaries) | Invariant table |
| [docs/runbooks/launch-checklist.md § Phase 4](../docs/runbooks/launch-checklist.md) | HTTPS indexer, WC ID, CSP deploy checklist |
| `frontend-dapp/vite.config.ts` | Mnemonic + WC build guards; production CSP transform |
| `frontend-dapp/src/viteConfig.build.test.ts` | Regression for guards and CSP shape |

## Verification commands

```bash
make test-frontend
make lint-frontend
cd frontend-dapp && npm run test:run -- src/viteConfig.build.test.ts src/utils/tokenLogoAllowlist.test.ts src/components/swap/ExpertModeModal.test.tsx src/components/ui/TokenLogo.test.tsx src/components/legal/__tests__/LegalFooterNotice.test.tsx
```

Attack/abuse spot checks:

```bash
# Staging mnemonic guard (must fail)
cd frontend-dapp && VITE_DEV_MNEMONIC='test test test' npx vite build --mode staging

# Production WC guard (must fail without ID)
cd frontend-dapp && npx vite build --mode production
```

## Rules of thumb

1. **Do not** add client-side BFS route fallback or on-chain hop graph cross-check in the browser (RPC/LCD limits; rejected in #378).
2. **Do not** show factory/router on swap confirmation — `/protocol` only.
3. **Do not** add more token detail to retail UI beyond the compact [#541](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/541) identity row ([`AGENTS_FRONTEND_TOKEN_IDENTITY.md`](./AGENTS_FRONTEND_TOKEN_IDENTITY.md)); logo allowlist + blockie fallback is sufficient. Identity ≠ endorsement.
4. Expert mode: typed confirm on **enable only**; keep **30%** block and **50%** settings max.
5. Production bundles must not contain dev mnemonic or the shared WC default project ID.

## Related

- Production build maps: [`AGENTS_FRONTEND_PRODUCTION_BUILD.md`](./AGENTS_FRONTEND_PRODUCTION_BUILD.md)
- Dev wallet: [`AGENTS_BUNDLE_DEV_WALLET.md`](./AGENTS_BUNDLE_DEV_WALLET.md)
- Swap route display: [`AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md`](./AGENTS_FRONTEND_SWAP_ROUTE_DISPLAY.md)
- Swap signing confirmation: [`AGENTS_FRONTEND_SWAP_SIGNING_CONFIRMATION.md`](./AGENTS_FRONTEND_SWAP_SIGNING_CONFIRMATION.md) (#409 / SEC-D11)
- Legal clickwrap CSP hosts (#517): [`AGENTS_FRONTEND_CLICKWRAP.md`](./AGENTS_FRONTEND_CLICKWRAP.md) — add Legal API/portal to `connect-src`, never blanket `https:`
