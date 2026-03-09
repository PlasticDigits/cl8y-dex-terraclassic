# Smart Contract Reference

All message types are defined in `smartcontracts/packages/dex-common/src/`.

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
| `CreatePair`               | `token_a`, `token_b`                               | Anyone      |
| `AddWhitelistedCodeId`     | `code_id: u64`                                     | Governance  |
| `RemoveWhitelistedCodeId`  | `code_id: u64`                                     | Governance  |
| `SetPairFee`               | `pair: String`, `fee_bps: u16`                     | Governance  |
| `SetPairHooks`             | `pair: String`, `hooks: Vec<String>`               | Governance  |
| `UpdateConfig`             | `governance?`, `treasury?`, `default_fee_bps?`     | Governance  |

### QueryMsg

| Variant                  | Parameters                       | Returns            |
|--------------------------|----------------------------------|--------------------|
| `GetConfig`              | —                                | `ConfigResponse`   |
| `GetPair`                | `token_a`, `token_b`             | `PairResponse`     |
| `GetAllPairs`            | `start_after?`, `limit?`         | `PairsResponse`    |
| `GetWhitelistedCodeIds`  | `start_after?`, `limit?`         | `CodeIdsResponse`  |
| `GetPairCount`           | —                                | `PairCountResponse`|

---

## Pair

### InstantiateMsg (PairInstantiateMsg)

| Field              | Type   | Description                      |
|--------------------|--------|----------------------------------|
| `token_a`          | `Addr` | First CW20 token address         |
| `token_b`          | `Addr` | Second CW20 token address        |
| `fee_bps`          | `u16`  | Fee in basis points               |
| `treasury`         | `Addr` | Fee recipient                     |
| `factory`          | `Addr` | Factory address (for auth)        |
| `lp_token_code_id` | `u64` | Code ID for LP token instantiation|

### ExecuteMsg

| Variant          | Fields                                                    | Auth       |
|------------------|-----------------------------------------------------------|------------|
| `Receive`        | `Cw20ReceiveMsg` (wraps `Swap` or `RemoveLiquidity`)     | CW20 token |
| `AddLiquidity`   | `token_a_amount`, `token_b_amount`, `min_lp_tokens?`, `slippage_tolerance?` | Anyone |
| `UpdateFee`      | `fee_bps: u16`                                            | Factory    |
| `UpdateHooks`    | `hooks: Vec<String>`                                      | Factory    |

### Cw20HookMsg (sent via CW20 Send)

| Variant            | Fields                      |
|--------------------|-----------------------------|
| `Swap`             | `min_output?`, `to?`        |
| `RemoveLiquidity`  | `min_a?`, `min_b?`          |

### QueryMsg

| Variant          | Parameters                      | Returns                 |
|------------------|---------------------------------|-------------------------|
| `GetPairInfo`    | —                               | `PairInfoResponse`      |
| `GetReserves`    | —                               | `ReservesResponse`      |
| `GetFeeConfig`   | —                               | `FeeConfigResponse`     |
| `GetHooks`       | —                               | `HooksResponse`         |
| `SimulateSwap`   | `offer_token`, `offer_amount`   | `SimulateSwapResponse`  |

---

## Router

### InstantiateMsg

| Field     | Type     | Description            |
|-----------|----------|------------------------|
| `factory` | `String` | Factory contract address|

### ExecuteMsg

| Variant      | Fields                                           | Auth       |
|--------------|--------------------------------------------------|------------|
| `Receive`    | `Cw20ReceiveMsg` (wraps `SwapTokens`)           | CW20 token |
| `SwapTokens` | `route`, `min_output?`, `to?` (must use Receive)| —          |

### Cw20HookMsg

| Variant      | Fields                              |
|--------------|-------------------------------------|
| `SwapTokens` | `route: Vec<String>`, `min_output?`, `to?` |

### QueryMsg

| Variant          | Parameters                    | Returns                  |
|------------------|-------------------------------|--------------------------|
| `GetConfig`      | —                             | `ConfigResponse`         |
| `SimulateRoute`  | `route`, `offer_amount`       | `SimulateRouteResponse`  |

---

## Hook Interface

Any contract implementing this interface can be registered as a post-swap hook via the Factory.

### HookExecuteMsg

| Variant     | Fields                                                                    |
|-------------|---------------------------------------------------------------------------|
| `AfterSwap` | `pair`, `sender`, `input_token`, `input_amount`, `output_token`, `output_amount`, `fee_amount` |
