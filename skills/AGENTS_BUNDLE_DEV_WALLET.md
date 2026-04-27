# Agent playbook: dev wallet and client-bundle secrets (Terra Classic dApp)

Use this when working on the simulated (dev) wallet, Vite env inlining, or anything that could put seeds or private keys in the production JavaScript bundle.

## Canonical references

| Doc / code | Purpose |
|------------|---------|
| [docs/frontend.md § Simulated (dev) wallet and VITE_DEV_MNEMONIC](../docs/frontend.md#simulated-dev-wallet-and-vite_dev_mnemonic) | Invariants, env layout, `deploy-dex-local.sh` (GitLab [#118](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/118)) |
| `frontend-dapp/src/services/terraclassic/devWallet.ts` | No default mnemonic; requires `VITE_DEV_MNEMONIC` in dev only |
| `docker/init-chain.sh` | Canonical `TEST_MNEMONIC` for LocalTerra (public test vector, not a production secret) |
| [`.gitleaks.toml`](../.gitleaks.toml) | Custom rule `bip39-like-phrase-frontend-src` to catch BIP39-like quoted phrases in `frontend-dapp/src` |

## Rules of thumb

1. **Never** ship a BIP39 string literal in `frontend-dapp/src` — Vite inlines `VITE_*` at build time; a runtime `if (!dev)` does not remove the string from the bundle.
2. **Development only:** put `VITE_DEV_MNEMONIC` in **`.env.development`** (gitignored) or shell for `vite` / `npm run dev`. `deploy-dex-local.sh` writes it to `.env.development` from `init-chain.sh`.
3. **Production `npm run build`:** `vite.config.ts` **fails the build** if `VITE_DEV_MNEMONIC` is set in any file or env that Vite’s `loadEnv` merges for `mode=production` (see GitLab #118).
4. **E2E / Playwright:** `playwright.config.ts` injects the mnemonic from `docker/init-chain.sh` into the `webServer` process env (not from frontend source).
5. **Gitleaks** default rules do not reliably flag multi-word BIP39 phrases; this repo adds a path-scoped custom rule (`bip39-like-phrase-frontend-src`). The pre-commit hook uses `gitleaks protect --staged -c .gitleaks.toml` (blocks new violations). A full `gitleaks detect` over **all git history** may still list old commits that predated the fix; use `gitleaks detect --no-git` for a working-tree-only check. Do not allowlist `frontend-dapp/src` for this pattern without a security review.

## Cross-links

- Production source maps: [`AGENTS_FRONTEND_PRODUCTION_BUILD.md`](./AGENTS_FRONTEND_PRODUCTION_BUILD.md) · [GitLab #117](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/117)  
- Gas / swap limits: [`AGENTS_TERRACLASSIC_GAS.md`](./AGENTS_TERRACLASSIC_GAS.md)
- Localnet trading swarm (separate bot mnemonics, never commit seeds): [`AGENTS_LOCALNET_TRADING_SWARM.md`](./AGENTS_LOCALNET_TRADING_SWARM.md) · [GitLab #119](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/119)
