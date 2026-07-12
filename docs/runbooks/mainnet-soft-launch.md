# Runbook: mainnet soft launch (non-economic CW20)

Ordered operator path for a **columbus-5** pool-only soft launch using **non-economic** CW20 tokens only, Coolify hosts **`https://dex.cl8y.com`** (frontend) and **`https://indexer.dex.cl8y.com`** (indexer), and a single deploy script.

**Related:** [`docs/deployment-guide.md`](../deployment-guide.md), [`launch-checklist.md`](./launch-checklist.md), [`cw20-whitelist-policy.md`](./cw20-whitelist-policy.md), [`soft-launch-faucet.md`](./soft-launch-faucet.md) (GitLab #473), agent playbook [`skills/AGENTS_MAINNET_SOFT_LAUNCH.md`](../../skills/AGENTS_MAINNET_SOFT_LAUNCH.md).

## Scope

| In scope | Out of scope |
|----------|--------------|
| Factory, pair, router, fee-discount | Wrap-mapper / USTR treasury contracts |
| CW20-base + cw20-mintable whitelist only | Terraport / GDEX / other economic CW20 code IDs |
| Gemstone non-economic tokens + seeded pairs | High-TVL LUNC/USTC markets |
| Coolify Dockerfiles (no compose) | Render-only frontend (still supported via `render.yaml`) |

Phase 5 GO may proceed without a separate staging/testnet deploy when budget-constrained; record that risk acceptance on the launch tracking issue ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)).

## Invariants

| ID | Rule |
|----|------|
| **SL1** | Factory `whitelisted_code_ids` contains **only** Terraswap **cw20-base** (**6036**) and PlasticDigits **cw20-mintable** (**10184**) by default. |
| **SL2** | No Terraport/GDEX/economic CW20 templates on the whitelist for this path. |
| **SL3** | Soft-launch trading tokens use **6** decimals; fee-discount `cl8y_token` is mainnet CL8Y (**18** decimals). |
| **SL4** | Deploy key pays gas and bootstraps admin msgs; **wasm `--admin`** + **treasury** + **final `config.governance`** = multisig [`terra1zlmv2…`](../reference/governance-multisig.md). Instantiate uses deployer as temporary `config.governance`, then hands off after tiers/registry setup. |

| **SL5** | CW20-only pairs — wrap-mapper not required. |
| **SL6** | Production indexer: `RUN_MODE=prod`, non-default `LCD_URLS`, `CORS_ORIGINS=https://dex.cl8y.com`, `VITE_INDEXER_URL=https://indexer.dex.cl8y.com` (HTTPS only). |
| **SL7** | Fee-discount tiers match [`fee-discount-tiers.md`](../reference/fee-discount-tiers.md) (drift: `make check-fee-discount-tier-docs`). |

Faucet (optional soft-launch UX) has its own invariants **F1–F12** in [`soft-launch-faucet.md`](./soft-launch-faucet.md) — does **not** change SL1–SL7 or factory whitelist.

Defaults live in [`scripts/lib/mainnet-soft-launch-defaults.sh`](../../scripts/lib/mainnet-soft-launch-defaults.sh).

## Default token / pair set

**Mintable** (mainnet code ID **10184** by default — [cw20-mintable](https://github.com/PlasticDigits/cw20-mintable)): EMBER, CORAL, JADE, ONYX, RUBY, TOPAZ.

**Standard cw20-base** (mainnet code ID **6036** by default — Terraswap cw20-base; override with `MAINNET_CW20_BASE_CODE_ID` or leave empty to store from artifacts): QUARTZ, PEARL.

**Pairs (10):** EMBER/CORAL, EMBER/JADE, EMBER/ONYX, CORAL/RUBY, JADE/TOPAZ, ONYX/QUARTZ, RUBY/PEARL, EMBER/QUARTZ, CORAL/PEARL, JADE/ONYX.

## Gas / fees

columbus-5 floor is **28.325 uluna/gas** (FCD `/v1/txs/gas_prices`). The soft-launch script uses `--gas auto --gas-prices=28.325uluna` via [`terrad-host.sh`](../../scripts/lib/terrad-host.sh) so each tx pays `gas_wanted × 28.325` (do **not** set a flat 5 or 100 LUNC `--fees` for all txs). Pair creation still attaches **100 LUNC** separately (`pair_creation_fee_uluna`). Rough total with 10 pairs: on the order of **~2–3k LUNC** (gas + pair fees), well under a typical funded `cl8ydeploy` balance.

Override only if needed: `TERRAD_HOST_GAS_PRICES=28.325uluna` or escape-hatch `TERRAD_HOST_FEES=…uluna`.

```bash
make build-optimized

# cl8ydeploy lives in ~/.terra/keyring-file (auto-detected). Encrypted file keyring:
# either run in a TTY and enter the passphrase when prompted, or once:
#   read -rs TERRAD_HOST_KEYRING_PASS; export TERRAD_HOST_KEYRING_PASS
# Key must be cl8ydeploy → terra1hu4zggf3f8yw6jw3rxrjxn2drwad675gq5k2lv
./scripts/deploy-dex-mainnet-soft-launch.sh
unset TERRAD_HOST_KEYRING_PASS
```

Dry-run (no broadcast):

```bash
DRY_RUN=1 ./scripts/deploy-dex-mainnet-soft-launch.sh
```

Outputs under `deployments/mainnet-soft-launch/`:

- `addresses.env` — code IDs + contract addresses
- `frontend.env.example` — Coolify frontend build-args
- `indexer.env.example` — Coolify indexer runtime env

Makefile: `make deploy-mainnet-soft-launch`.

## Coolify (Dockerfiles, no compose)

| Service | Dockerfile | Public URL |
|---------|------------|------------|
| Indexer | [`docker/indexer/Dockerfile`](../../docker/indexer/Dockerfile) | `https://indexer.dex.cl8y.com` |
| Frontend | [`docker/frontend/Dockerfile`](../../docker/frontend/Dockerfile) + [`nginx.conf`](../../docker/frontend/nginx.conf) | `https://dex.cl8y.com` |

**Indexer:** build context = repo root; set `DATABASE_URL`, `FACTORY_ADDRESS`, `ROUTER_ADDRESS`, `FEE_DISCOUNT_ADDRESS`, `CORS_ORIGINS=https://dex.cl8y.com`, `RUN_MODE=prod`, operator-controlled `LCD_URLS`, `API_BIND=0.0.0.0`.

**Frontend:** build-args from `frontend.env.example` including `VITE_WC_PROJECT_ID` and `VITE_INDEXER_URL=https://indexer.dex.cl8y.com`. For the Mint page ([#473](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/473)), also set `VITE_FAUCET_ADDRESS` and the six `VITE_TOKEN_{EMBER,CORAL,JADE,ONYX,RUBY,TOPAZ}_ADDRESS` values after `make deploy-soft-launch-faucet`. Do not set `VITE_DEV_MNEMONIC`.

Postgres is provisioned in Coolify separately (not via repo compose).

## Verification

```bash
make test-mainnet-soft-launch-defaults
make check-fee-discount-tier-docs
# After live deploy:
make qa-verify-deploy-config
make qa-verify-env-addresses
```

Paste deploy trace fields per [`docs/templates/deploy-trace.md`](../templates/deploy-trace.md).

## Resume / recovery

If instantiate used multisig as `config.governance` before admin setup (Unauthorized on `add_tier`), either:

1. **Reuse stored code IDs** (no re-store) and instantiate fresh with bootstrap governance (preferred after the script fix):

```bash
FACTORY_CODE_ID=11505 PAIR_CODE_ID=11506 ROUTER_CODE_ID=11507 FEE_DISCOUNT_CODE_ID=11508 \
  ./scripts/deploy-dex-mainnet-soft-launch.sh
```

2. **Or** have the multisig call fee-discount/factory `update_config` to set `governance` to `cl8ydeploy`, finish setup, then hand back to the multisig.

Orphan instances left with empty config are harmless; point Coolify at the new addresses from `addresses.env`.

### Resume pairs after RPC interrupt

If create/LP stopped mid-loop (e.g. connection reset), update `deployments/mainnet-soft-launch/addresses.env` with live addresses, then:

```bash
read -rs TERRAD_HOST_KEYRING_PASS; export TERRAD_HOST_KEYRING_PASS
./scripts/resume-mainnet-soft-launch-pairs.sh
unset TERRAD_HOST_KEYRING_PASS
```

Skips pairs that already have liquidity; finishes remaining pairs, `set_discount_registry_all`, and governance handoff. Host txs retry transient RPC errors (default 5 attempts).

Adding economic tokens later requires CW20 whitelist policy review ([`cw20-whitelist-policy.md`](./cw20-whitelist-policy.md)), governance `AddWhitelistedCodeId`, and new pairs — not a full redeploy of factory/router unless migrating.
