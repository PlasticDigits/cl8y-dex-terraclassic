# Deployment Guide

See also: **[Pool-only v2 launch runbook](runbooks/launch-checklist.md)** — governance, treasury, hooks, trusted router, verification, and **[Phase 5 go/no-go sign-off](runbooks/launch-checklist.md#phase-5--go--no-go-decision-required-before-production-mainnet)** (required before production mainnet — [GitLab #391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)).

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

**Wasm policy (local automation, [GitLab #234](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/234)):** Dev checks may use fast `cargo build --target wasm32-unknown-unknown` (see reference job `contracts-terra` in [`.github/workflows/test.yml`](../.github/workflows/test.yml)). **Do not upload that wasm to mainnet.** Release uploads use **`make build-optimized`** (workspace-optimizer; same as reference spec [`.github/workflows/contracts-wasm-optimizer.yml`](../.github/workflows/contracts-wasm-optimizer.yml)) and `wasm-checksums.txt` under `smartcontracts/artifacts/`.

## 2. Upload Code

Upload each contract and note the returned code IDs.

```bash
terrad tx wasm store artifacts/cl8y_dex_factory.wasm \
  --from <wallet> --gas auto --gas-adjustment 1.4 \
  --fees 500000uluna --chain-id <chain-id> --node <rpc-url>

# Repeat for pair, router, fee_discount, and cw20_base
```

## 3. Instantiate Factory

Canonical governance / admin / upgrade address: **`terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7`** ([`docs/reference/governance-multisig.md`](reference/governance-multisig.md)).

```bash
terrad tx wasm instantiate <factory_code_id> '{
  "governance": "terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7",
  "treasury": "terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7",
  "default_fee_bps": 30,
  "pair_code_id": <pair_code_id>,
  "lp_token_code_id": <cw20_code_id>,
  "whitelisted_code_ids": [<cw20_code_id>]
}' --label "cl8y-dex-factory" --admin terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7 \
  --from <wallet> --gas auto --gas-adjustment 1.4 \
  --fees 500000uluna --chain-id <chain-id> --node <rpc-url>
```

## 4. Instantiate Router

```bash
terrad tx wasm instantiate <router_code_id> '{
  "factory": "<factory_contract_addr>"
}' --label "cl8y-dex-router" --admin terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7 \
  --from <wallet> --gas auto --gas-adjustment 1.4 \
  --fees 500000uluna --chain-id <chain-id> --node <rpc-url>
```

## 5. Instantiate Fee Discount Registry

```bash
terrad tx wasm instantiate <fee_discount_code_id> '{
  "governance": "terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7",
  "cl8y_token": "<cl8y_cw20_addr>"
}' --label "cl8y-dex-fee-discount" --admin terra1zlmv2xydxcusurtr6rl78wsvytdc6mfex6hep7 \
  --from <wallet> --gas auto --gas-adjustment 1.4 \
  --fees 500000uluna --chain-id <chain-id> --node <rpc-url>
```

### 5a. Add Default Tiers

Use the **authoritative** tier ladder, `min_cl8y_balance` strings, `governance_only` flags, and copy-paste `terrad` examples in **[`docs/reference/fee-discount-tiers.md`](reference/fee-discount-tiers.md)** ([GitLab #198](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/198)). Wire format must match `ExecuteMsg::AddTier` (`min_cl8y_balance`, not `min_tokens`). Do not duplicate tier numbers in this guide — run `make check-fee-discount-tier-docs` after edits. Agent playbook: [`skills/AGENTS_FEE_DISCOUNT_TIERS.md`](../skills/AGENTS_FEE_DISCOUNT_TIERS.md).

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
# Attach uluna >= factory Config.pair_creation_fee_uluna (default 100 LUNC = 100000000uluna; GitLab #276).
terrad tx wasm execute <factory_addr> '{
  "create_pair": {
    "asset_infos": [
      { "token": { "contract_addr": "<token_a_addr>" } },
      { "token": { "contract_addr": "<token_b_addr>" } }
    ]
  }
}' --amount 100000000uluna \
  --from <wallet> --gas auto --gas-adjustment 1.4 \
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

## Deploy trace (audit record)

Before closing a deploy or migration, record an auditable trace on the **launch tracking issue** ([#391](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/391)) using [`docs/templates/deploy-trace.md`](templates/deploy-trace.md) (SEC-D12, [GitLab #410](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/410)):

- [ ] **Git SHA:** `git rev-parse HEAD`
- [ ] **Terra Classic chain version:** `terrad version` or `terrad status --node <rpc> | jq -r .node_info.version`
- [ ] **Contract code IDs** (factory, pair, router, fee-discount)
- [ ] **`wasm-checksums.txt`** from `smartcontracts/artifacts/wasm-checksums.txt`
- [ ] **Post-deploy verification output** (queries + smoke script)

See also: [Pool-only launch runbook § Phase 1 deploy trace](runbooks/launch-checklist.md#deploy-trace-audit-record--required-before-leaving-phase-1), [wasm admin migration Pre-flight](runbooks/wasm-admin-migration.md#pre-flight), [`skills/AGENTS_DEPLOY_TRACE.md`](../skills/AGENTS_DEPLOY_TRACE.md).

## Post-Deployment Checklist

- [ ] **Pre-deploy test evidence (SEC-H08):** paste or link passing output for `make test-contracts`, `make test-indexer-integration`, and `make test-frontend` at the deployed commit SHA on the release issue — or link the GitLab pipeline URL when deploying CI-built artifacts ([#444](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/444)). See [launch checklist Phase 0](runbooks/launch-checklist.md#phase-0--preconditions) and [`skills/AGENTS_TEST_EVIDENCE_GATE.md`](../skills/AGENTS_TEST_EVIDENCE_GATE.md).
- [ ] **IBC-hooks chain exposure (SEC-D02):** record Terra Classic chain binary/SDK version and IBC-hooks module status at deploy time; attest that app contracts do not expose IBC receive/ack/timeout entry points (`make verify-no-ibc-hooks-in-contracts`). Re-run after chain upgrades or new contract modules — [launch checklist Phase 0](runbooks/launch-checklist.md#phase-0--preconditions), [security model § IBC hooks](security-model.md#ibc-hooks-chain-dependency-sec-d02) ([#407](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/407)).
- [ ] **Deploy trace** posted on the launch tracking issue (see [Deploy trace](#deploy-trace-audit-record) above)
- [ ] Run scripted config verification: [`scripts/qa/verify-deploy-config.sh`](../scripts/qa/verify-deploy-config.sh) (`make qa-verify-deploy-config`) — asserts governance, treasury, default fee, whitelisted CW20 code IDs, fee-discount tiers, trusted router, pair hooks, and blacklist state; paste output on the release issue (SEC-H03, [#441](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/441))
- [ ] Confirm indexer **`FACTORY_ADDRESS` is non-empty** before starting the indexer in any environment (SEC-I02, [#451](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/451)) — startup rejects empty/whitespace values; **`make qa-verify-deploy`** also fails when the env var is missing
- [ ] Run env/chain address cross-check: [`scripts/qa/verify-env-addresses.sh`](../scripts/qa/verify-env-addresses.sh) (`make qa-verify-env-addresses`) — compares indexer and frontend env addresses and asserts router on-chain `factory` matches env; paste output on the release issue (SEC-H04, [#442](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/442))
- [ ] Run read-only pool checks: [`scripts/smoke-pool-swap.sh`](../scripts/smoke-pool-swap.sh) (`PAIR_ADDR`, optional `OFFER_TOKEN` / `TERRA_LCD_URL`)
- [ ] Verify Factory config via `GetConfig` query (covered by `qa-verify-deploy-config`; manual `terrad query` optional)
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
