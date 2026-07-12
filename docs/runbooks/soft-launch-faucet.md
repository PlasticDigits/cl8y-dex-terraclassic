# Runbook: soft-launch faucet (GitLab #473)

Operator path to deploy the **non-economic soft-launch CW20 faucet**, grant minter rights, and wire the dApp Mint page.

**Related:** [`mainnet-soft-launch.md`](./mainnet-soft-launch.md), [`skills/AGENTS_SOFT_LAUNCH_FAUCET.md`](../../skills/AGENTS_SOFT_LAUNCH_FAUCET.md), [`skills/AGENTS_MAINNET_SOFT_LAUNCH.md`](../../skills/AGENTS_MAINNET_SOFT_LAUNCH.md), issue [#473](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/473).

## Scope

| In scope | Out of scope |
|----------|--------------|
| `cl8y-dex-faucet` store / instantiate | Factory CW20 whitelist changes (**F7**) |
| `cl8ydeploy` `AddMinter` on six mintables | Governance / multisig primary minter handoff (**F6**) |
| Coolify `VITE_FAUCET_ADDRESS` + mintable token envs | Indexer mint-event indexing |
| LocalTerra faucet via `deploy-dex-local.sh` | Minting QUARTZ / PEARL (cw20-base) |

## Invariants (F1–F13)

| ID | Rule |
|----|------|
| **F1** | Faucet allowlist = mintable soft-launch tokens only (EMBER, CORAL, JADE, ONYX, RUBY, TOPAZ). |
| **F2** | Fixed drip `100000000` base units (100 × 10^6). Contract ignores any client amount. |
| **F3** | Global **per-wallet** cooldown **300s** across all tokens (not per-token). |
| **F4** | QUARTZ / PEARL never on allowlist or Mint UI. |
| **F5** | Faucet must be granted `AddMinter` on each allowlisted CW20 before `Drip` works. |
| **F6** | `cl8ydeploy` remains **primary** CW20 minter; faucet is an **additional** minter only. |
| **F7** | Faucet code id is **not** added to factory `whitelisted_code_ids`. Soft-launch **SL1–SL2** unchanged. |
| **F8** | UI copy must state tokens are noneconomic / demo (no claim, redemption, or backing). |
| **F9** | Emergency: faucet `Pause` + CW20 `RemoveMinter { minter: faucet }`. Pause does **not** clear cooldowns. |
| **F10** | Happy path is LCD query + execute (no indexer dependency). |
| **F11** | Frontend exposes Mint nav only when `VITE_FAUCET_ADDRESS` is set. |
| **F12** | User pays LUNC gas for `Drip` (no sponsored meta-tx). |
| **F13** | dApp fee envelope for `{ drip }` is **`FAUCET_DRIP_GAS_LIMIT` (400k)** in [`terraGas.ts`](../../frontend-dapp/src/services/terraclassic/terraGas.ts) — must not fall through to **`BASE_GAS_LIMIT` (200k)** ([#474](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/474) / [#475](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/475)). Verify: `make verify-issue-475`. |

Soft-launch **SL1–SL7** remain in force; this faucet does not change factory whitelist or fee tiers.

## Prerequisites

1. Soft-launch DEX already deployed — `deployments/mainnet-soft-launch/addresses.env` has `TOKEN_{EMBER,CORAL,JADE,ONYX,RUBY,TOPAZ}_ADDRESS`.
2. Optimized wasm: `make build-optimized` → `smartcontracts/artifacts/cl8y_dex_faucet.wasm`.
3. Host `terrad` key `cl8ydeploy` = primary minter on those CW20s (same soft-launch deploy key).

## Deploy (columbus-5)

```bash
make build-optimized
DRY_RUN=1 make deploy-soft-launch-faucet   # prints store/instantiate/AddMinter plan
make deploy-soft-launch-faucet             # broadcasts
```

Script: [`scripts/deploy-soft-launch-faucet.sh`](../../scripts/deploy-soft-launch-faucet.sh).

Writes / updates:

- `deployments/mainnet-soft-launch/addresses.env` — `FAUCET_CODE_ID`, `FAUCET_ADDRESS`, drip/cooldown
- `deployments/mainnet-soft-launch/frontend.env.example` — `VITE_FAUCET_ADDRESS`, `VITE_TOKEN_*_ADDRESS`
- `deployments/mainnet-soft-launch/faucet-trace.md` — audit record

Optional flags:

| Env | Effect |
|-----|--------|
| `SKIP_ADD_MINTER=1` | Instantiate only (operator runs `AddMinter` manually) |
| `FORCE_STORE_FAUCET=1` | Re-store wasm even if `FAUCET_CODE_ID` already in env |
| `FAUCET_DRIP_AMOUNT` / `FAUCET_COOLDOWN_SECONDS` | Override defaults (keep F2/F3 unless product changes) |

### Manual AddMinter payload

Signed by **`cl8ydeploy`** (primary minter) on each mintable CW20:

```json
{"add_minter":{"minter":"<FAUCET_ADDRESS>"}}
```

Emergency remove:

```json
{"remove_minter":{"minter":"<FAUCET_ADDRESS>"}}
```

Faucet pause (admin = deploy key):

```json
{"pause":{}}
```

## Verification

```bash
source deployments/mainnet-soft-launch/addresses.env

# Config: 6 allowlisted addrs, drip=100000000, cooldown=300, paused=false
terrad query wasm contract-state smart "$FAUCET_ADDRESS" '{"config":{}}' \
  --node https://terra-classic-rpc.publicnode.com:443 --output json

# Faucet appears under Minters; primary minter still cl8ydeploy
terrad query wasm contract-state smart "$TOKEN_EMBER_ADDRESS" '{"minters":{}}' \
  --node https://terra-classic-rpc.publicnode.com:443 --output json
terrad query wasm contract-state smart "$TOKEN_EMBER_ADDRESS" '{"minter":{}}' \
  --node https://terra-classic-rpc.publicnode.com:443 --output json

# Smoke: gas-funded wallet Drip once; immediate second Drip fails cooldown
```

Frontend Coolify: bake `VITE_FAUCET_ADDRESS` + six `VITE_TOKEN_*_ADDRESS` from `frontend.env.example`. Open `/mint` — Mint under More; QUARTZ/PEARL absent.

## LocalTerra

`scripts/deploy-dex-local.sh` stores/instantiates the faucet when `cl8y_dex_faucet.wasm` is present, allowlists the first six local tokens (EMBER…TOPAZ), runs `AddMinter`, and writes `VITE_FAUCET_ADDRESS` / `VITE_TOKEN_*` into `frontend-dapp/.env.local`.

## Residual risk (documented)

Cooldown is **per address only** — sybil wallets can drip in parallel. Acceptable for noneconomic demo tokens; do not add KYC for this path.
