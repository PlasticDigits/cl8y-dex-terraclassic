# Smart Contract Reference

All message types are defined in `smartcontracts/packages/dex-common/src/`.

Message names follow TerraSwap/Terraport conventions for Vyntrex compatibility.

## Shared Types

### AssetInfo

```json
{ "token": { "contract_addr": "terra1..." } }
// or
{ "native_token": { "denom": "uluna" } }
```

> **Note:** `NativeToken` is accepted in the type system for wire compatibility with TerraSwap but is rejected at runtime. Only CW20 tokens (`Token` variant) are supported.

### Asset

```json
{ "info": <AssetInfo>, "amount": "1000000" }
```

### PairInfo (query response)

| Field             | Type             | Description                      |
|-------------------|------------------|----------------------------------|
| `asset_infos`     | `[AssetInfo; 2]` | The two assets in the pair       |
| `contract_addr`   | `Addr`           | Pair contract address            |
| `liquidity_token` | `Addr`           | CW20 LP token address            |

---

## Factory

### InstantiateMsg

| Field                  | Type       | Description                          |
|------------------------|------------|--------------------------------------|
| `governance`           | `String`   | Address with admin privileges        |
| `treasury`             | `String`   | Address that receives swap fees      |
| `default_fee_bps`      | `u16`      | Default fee in basis points (≤10000) |
| `pair_code_id`         | `u64`      | Stored code ID for Pair contract     |
| `lp_token_code_id`     | `u64`      | Stored code ID for CW20 LP token     |
| `whitelisted_code_ids` | `Vec<u64>` | Initial CW20 code IDs allowed        |

### ExecuteMsg

| Variant                    | Fields                                             | Auth        |
|----------------------------|----------------------------------------------------|-------------|
| `CreatePair`               | `asset_infos: [AssetInfo; 2]`                      | Anyone      |
| `AddWhitelistedCodeId`     | `code_id: u64`                                     | Governance  |
| `RemoveWhitelistedCodeId`  | `code_id: u64`                                     | Governance  |
| `SetPairFee`               | `pair: String`, `fee_bps: u16`                     | Governance  |
| `SetPairHooks`             | `pair: String`, `hooks: Vec<String>`               | Governance  |
| `UpdateConfig`             | `governance?`, `treasury?`, `default_fee_bps?`     | Governance  |

### QueryMsg

| Variant                  | Parameters                             | Returns            |
|--------------------------|----------------------------------------|--------------------|
| `Config`                 | —                                      | `ConfigResponse`   |
| `Pair`                   | `asset_infos: [AssetInfo; 2]`          | `PairResponse`     |
| `Pairs`                  | `start_after?: [AssetInfo; 2]`, `limit?` | `PairsResponse`  |
| `GetWhitelistedCodeIds`  | `start_after?`, `limit?`               | `CodeIdsResponse`  |
| `GetPairCount`           | —                                      | `PairCountResponse`|

---

## Pair

### InstantiateMsg (PairInstantiateMsg)

| Field              | Type             | Description                       |
|--------------------|------------------|-----------------------------------|
| `asset_infos`      | `[AssetInfo; 2]` | The two assets for the pair       |
| `fee_bps`          | `u16`            | Fee in basis points               |
| `treasury`         | `Addr`           | Fee recipient                     |
| `factory`          | `Addr`           | Factory address (for auth)        |
| `lp_token_code_id` | `u64`           | Code ID for LP token instantiation|

### ExecuteMsg

| Variant              | Fields                                                                   | Auth       |
|----------------------|--------------------------------------------------------------------------|------------|
| `Receive`            | `Cw20ReceiveMsg` (wraps `Swap` or `WithdrawLiquidity`)                  | CW20 token |
| `ProvideLiquidity`   | `assets: [Asset; 2]`, `slippage_tolerance?`, `receiver?`, `deadline?`   | Anyone     |
| `Swap`               | `offer_asset`, `belief_price?`, `max_spread?`, `to?`, `deadline?`       | (rejected for CW20 -- use CW20 Send) |
| `UpdateFee`          | `fee_bps: u16`                                                          | Factory    |
| `UpdateHooks`        | `hooks: Vec<String>`                                                     | Factory    |

### Cw20HookMsg (sent via CW20 Send)

| Variant              | Fields                                                    |
|----------------------|-----------------------------------------------------------|
| `Swap`               | `belief_price?`, `max_spread?`, `to?`, `deadline?`        |
| `WithdrawLiquidity`  | (no fields)                                               |

### QueryMsg

| Variant              | Parameters         | Returns                      |
|----------------------|--------------------|------------------------------|
| `Pair`               | —                  | `PairInfo`                   |
| `Pool`               | —                  | `PoolResponse`               |
| `Simulation`         | `offer_asset`      | `SimulationResponse`         |
| `ReverseSimulation`  | `ask_asset`        | `ReverseSimulationResponse`  |
| `GetFeeConfig`       | —                  | `FeeConfigResponse`          |
| `GetHooks`           | —                  | `HooksResponse`              |

### Event Attributes (swap)

| Attribute           | Description                          |
|---------------------|--------------------------------------|
| `action`            | `"swap"`                             |
| `sender`            | User who initiated the swap          |
| `receiver`          | Recipient of output tokens           |
| `offer_asset`       | Input token identifier               |
| `ask_asset`         | Output token identifier              |
| `offer_amount`      | Amount of input tokens               |
| `return_amount`     | Amount of output tokens (net of fee) |
| `spread_amount`     | Price impact amount                  |
| `commission_amount` | Fee amount taken                     |

---

## Router

### InstantiateMsg

| Field     | Type     | Description            |
|-----------|----------|------------------------|
| `factory` | `String` | Factory contract address|

### SwapOperation

```json
{ "terra_swap": { "offer_asset_info": <AssetInfo>, "ask_asset_info": <AssetInfo> } }
// or (rejected at runtime)
{ "native_swap": { "offer_denom": "uluna", "ask_denom": "uusd" } }
```

### ExecuteMsg

| Variant                    | Fields                                                            | Auth       |
|----------------------------|-------------------------------------------------------------------|------------|
| `Receive`                  | `Cw20ReceiveMsg` (wraps `ExecuteSwapOperations`)                 | CW20 token |
| `ExecuteSwapOperations`    | `operations`, `minimum_receive?`, `to?`, `deadline?`             | (rejected -- use CW20 Send) |

### Cw20HookMsg

| Variant                    | Fields                                                   |
|----------------------------|----------------------------------------------------------|
| `ExecuteSwapOperations`    | `operations: Vec<SwapOperation>`, `minimum_receive?`, `to?`, `deadline?` |

### QueryMsg

| Variant                          | Parameters                          | Returns                           |
|----------------------------------|-------------------------------------|-----------------------------------|
| `Config`                         | —                                   | `ConfigResponse`                  |
| `SimulateSwapOperations`         | `offer_amount`, `operations`        | `SimulateSwapOperationsResponse`  |
| `ReverseSimulateSwapOperations`  | `ask_amount`, `operations`          | `SimulateSwapOperationsResponse`  |

---

## Hook Interface

Any contract implementing this interface can be registered as a post-swap hook via the Factory.

### HookExecuteMsg

| Variant     | Fields                                                                                         |
|-------------|------------------------------------------------------------------------------------------------|
| `AfterSwap` | `pair`, `sender`, `offer_asset: Asset`, `return_asset: Asset`, `commission_amount`, `spread_amount` |
