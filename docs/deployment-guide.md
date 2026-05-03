# Deployment Guide

See also: **[Pool-only v2 launch runbook](runbooks/launch-checklist.md)** — governance, treasury, hooks, trusted router, verification.

## Prerequisites

- Rust stable with `wasm32-unknown-unknown` target (optional if you only use Docker for wasm)
- Docker, for **workspace-optimizer** production builds ([`cosmwasm/workspace-optimizer`](https://github.com/CosmWasm/optimizer) — same image as `make build-optimized`)
- `terrad` CLI or equivalent Terra Classic CLI
- A funded wallet with sufficient LUNC for gas

## 1. Build Optimized WASM

**Canonical production wasm** is produced by **CosmWasm workspace-optimizer** (same as `make build-optimized` / `smartcontracts/scripts/optimize.sh`). Artifacts land in `smartcontracts/artifacts/`.

```bash
make build-optimized
```

CI uses a **hybrid** policy: pull requests run fast `cargo build --target wasm32-unknown-unknown` in [`.github/workflows/test.yml`](../.github/workflows/test.yml) only. **Do not upload PR wasm to mainnet** — run the **[Contracts WASM (workspace-optimizer)](../.github/workflows/contracts-wasm-optimizer.yml)** workflow (`workflow_dispatch` or on `main` / version tags) and use its artifacts plus `wasm-checksums.txt` for release uploads.

## 2. Upload Code

Upload each contract and note the returned code IDs.

```bash
terrad tx wasm store artifacts/cl8y_dex_factory.wasm \
  --from <wallet> --gas auto --gas-adjustment 1.4 \
  --fees 500000uluna --chain-id <chain-id> --node <rpc-url>

# Repeat for pair, router, fee_discount, and cw20_base
```

## 3. Instantiate Factory

```bash
terrad tx wasm instantiate <factory_code_id> '{
  "governance": "<governance_addr>",
  "treasury": "<treasury_addr>",
  "default_fee_bps": 30,
  "pair_code_id": <pair_code_id>,
  "lp_token_code_id": <cw20_code_id>,
  "whitelisted_code_ids": [<cw20_code_id>]
}' --label "cl8y-dex-factory" --admin <governance_addr> \
  --from <wallet> --gas auto --gas-adjustment 1.4 \
  --fees 500000uluna --chain-id <chain-id> --node <rpc-url>
```

## 4. Instantiate Router

```bash
terrad tx wasm instantiate <router_code_id> '{
  "factory": "<factory_contract_addr>"
}' --label "cl8y-dex-router" --admin <governance_addr> \
  --from <wallet> --gas auto --gas-adjustment 1.4 \
  --fees 500000uluna --chain-id <chain-id> --node <rpc-url>
```

## 5. Instantiate Fee Discount Registry

```bash
terrad tx wasm instantiate <fee_discount_code_id> '{
  "governance": "<governance_addr>",
  "cl8y_token": "<cl8y_cw20_addr>"
}' --label "cl8y-dex-fee-discount" --admin <governance_addr> \
  --from <wallet> --gas auto --gas-adjustment 1.4 \
  --fees 500000uluna --chain-id <chain-id> --node <rpc-url>
```

### 5a. Add Default Tiers

Use the **authoritative** tier ladder, `min_cl8y_balance` strings, `governance_only` flags, and copy-paste `terrad` examples in **[`docs/reference/fee-discount-tiers.md`](reference/fee-discount-tiers.md)**. Wire format must match `ExecuteMsg::AddTier` (`min_cl8y_balance`, not `min_tokens`).

After tiers exist, complete **§5b** (trusted router) before expecting router-originated `trader` discounts.

### 5b. Register Router as Trusted

```bash
terrad tx wasm execute <fee_discount_addr> '{
  "add_trusted_router": { "router": "<router_contract_addr>" }
}' --from <wallet> --gas auto --gas-adjustment 1.4 \
  --fees 500000uluna --chain-id <chain-id> --node <rpc-url>
```

### 5c. Set Discount Registry on All Pairs

```bash
terrad tx wasm execute <factory_addr> '{
  "set_discount_registry_all": { "registry": "<fee_discount_addr>" }
}' --from <wallet> --gas auto --gas-adjustment 1.4 \
  --fees 500000uluna --chain-id <chain-id> --node <rpc-url>
```

## 6. Create Pairs

Pairs are created through the Factory:

```bash
terrad tx wasm execute <factory_addr> '{
  "create_pair": {
    "asset_infos": [
      { "token": { "contract_addr": "<token_a_addr>" } },
      { "token": { "contract_addr": "<token_b_addr>" } }
    ]
  }
}' --from <wallet> --gas auto --gas-adjustment 1.4 \
  --fees 500000uluna --chain-id <chain-id> --node <rpc-url>
```

## 7. Deploy Frontend

Update `.env.production` with the contract addresses:

```env
VITE_NETWORK=mainnet
VITE_FACTORY_ADDRESS=terra1...
VITE_ROUTER_ADDRESS=terra1...
VITE_FEE_DISCOUNT_ADDRESS=terra1...
```

Push to the deployment branch. Render.sh builds and serves the static site automatically (see `render.yaml`).

## Testnet Deployment

Follow the same steps above, substituting:
- Chain ID: `rebel-2` (or current testnet)
- RPC: testnet RPC endpoint
- `VITE_NETWORK=testnet`

## Post-Deployment Checklist

- [ ] Run read-only pool checks: [`scripts/smoke-pool-swap.sh`](../scripts/smoke-pool-swap.sh) (`PAIR_ADDR`, optional `OFFER_TOKEN` / `TERRA_LCD_URL`)
- [ ] Verify Factory config via `GetConfig` query
- [ ] Create a test pair and verify it appears in `GetAllPairs`
- [ ] Execute a test swap and confirm balances
- [ ] Verify treasury received fees
- [ ] Verify fee-discount tiers via `GetTiers` query
- [ ] Register a test wallet for a tier, execute a swap, and confirm reduced fee
- [ ] Verify Router is a trusted router via `IsTrustedRouter` query
- [ ] Verify discount registry is set on pairs via pair/factory policy (historically documented as `GetDiscountRegistry`; confirm against your pair schema). For **many pairs**, use factory `set_discount_registry_batch` with pagination (see [Contract Reference — Factory discount registry rollout](./contracts-terraclassic.md#factory-discount-registry-rollout-invariants-glab-123), [GitLab #123](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/123)) instead of a single `set_discount_registry_all`.
- [ ] Frontend loads and connects wallet
- [ ] Swap and pool flows work end-to-end
- [ ] Tier registration page works end-to-end
