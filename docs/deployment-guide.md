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

# Repeat for pair, router, and cw20_base
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

## 5. Create Pairs

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

## 6. Deploy Frontend

Update `.env.production` with the contract addresses:

```env
VITE_NETWORK=mainnet
VITE_FACTORY_ADDRESS=terra1...
VITE_ROUTER_ADDRESS=terra1...
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
- [ ] Frontend loads and connects wallet
- [ ] Swap and pool flows work end-to-end
