# Deployment Guide

## Prerequisites

- Rust stable with `wasm32-unknown-unknown` target
- [cosmwasm/optimizer](https://github.com/CosmWasm/optimizer) Docker image (for production builds)
- `terrad` CLI or equivalent Terra Classic CLI
- A funded wallet with sufficient LUNC for gas

## 1. Build Optimized WASM

```bash
cd smartcontracts
docker run --rm -v "$(pwd)":/code \
  --mount type=volume,source="$(basename "$(pwd)")_cache",target=/target \
  --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
  cosmwasm/optimizer:0.16.1
```

Optimized artifacts are placed in `smartcontracts/artifacts/`.

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

Set up the standard tier table:

```bash
# Tier 0 — market makers (governance-only, 100% discount)
terrad tx wasm execute <fee_discount_addr> '{"add_tier":{"tier_id":0,"min_tokens":"0","discount_bps":10000}}' ...

# Tier 1 — 1 CL8Y, 10% discount
terrad tx wasm execute <fee_discount_addr> '{"add_tier":{"tier_id":1,"min_tokens":"1000000000000000000","discount_bps":1000}}' ...

# Tier 2 — 50 CL8Y, 25% discount
terrad tx wasm execute <fee_discount_addr> '{"add_tier":{"tier_id":2,"min_tokens":"50000000000000000000","discount_bps":2500}}' ...

# Tier 3 — 200 CL8Y, 35% discount
terrad tx wasm execute <fee_discount_addr> '{"add_tier":{"tier_id":3,"min_tokens":"200000000000000000000","discount_bps":3500}}' ...

# Tier 4 — 1000 CL8Y, 50% discount
terrad tx wasm execute <fee_discount_addr> '{"add_tier":{"tier_id":4,"min_tokens":"1000000000000000000000","discount_bps":5000}}' ...

# Tier 5 — 15000 CL8Y, 80% discount
terrad tx wasm execute <fee_discount_addr> '{"add_tier":{"tier_id":5,"min_tokens":"15000000000000000000000","discount_bps":8000}}' ...

# Tier 255 — blacklist (governance-only, 0% discount)
terrad tx wasm execute <fee_discount_addr> '{"add_tier":{"tier_id":255,"min_tokens":"0","discount_bps":0}}' ...
```

> **Note:** CL8Y is a CW20 token with 18 decimals. `min_tokens` values are in the smallest unit (1 CL8Y = 10^18).

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

- [ ] Verify Factory config via `GetConfig` query
- [ ] Create a test pair and verify it appears in `GetAllPairs`
- [ ] Execute a test swap and confirm balances
- [ ] Verify treasury received fees
- [ ] Verify fee-discount tiers via `GetTiers` query
- [ ] Register a test wallet for a tier, execute a swap, and confirm reduced fee
- [ ] Verify Router is a trusted router via `IsTrustedRouter` query
- [ ] Verify discount registry is set on pairs via `GetDiscountRegistry` query
- [ ] Frontend loads and connects wallet
- [ ] Swap and pool flows work end-to-end
- [ ] Tier registration page works end-to-end
