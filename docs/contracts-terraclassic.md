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
| `CreatePair`               | `asset_infos: [AssetInfo; 2]`                      | Anyone ([one create flow per block](./security-model.md#createpair-rate-limit-and-pending-state); see [#121](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/121)) |
| `AddWhitelistedCodeId`     | `code_id: u64`                                     | Governance  |
| `RemoveWhitelistedCodeId`  | `code_id: u64`                                     | Governance  |
| `SetPairFee`               | `pair: String`, `fee_bps: u16`                     | Governance  |
| `SetPairHooks`             | `pair: String`, `hooks: Vec<String>`               | Governance  |
| `SetDiscountRegistry`      | `pair: String`, `registry: Option<String>`         | Governance  |
| `SetDiscountRegistryAll` | `registry: Option<String>`                         | Governance ([gas note](#factory-discount-registry-rollout-invariants-glab-123)) |
| `SetDiscountRegistryBatch` | `registry: Option<String>`, `start_after?: u64`, `limit?: u32` | Governance — paginated rollout; see [invariants](#factory-discount-registry-rollout-invariants-glab-123) |
| `UpdateConfig`             | `governance?`, `treasury?`, `default_fee_bps?`     | Governance  |

### QueryMsg

| Variant                  | Parameters                             | Returns            |
|--------------------------|----------------------------------------|--------------------|
| `Config`                 | —                                      | `ConfigResponse`   |
| `Pair`                   | `asset_infos: [AssetInfo; 2]`          | `PairResponse`     |
| `Pairs`                  | `start_after?: [AssetInfo; 2]`, `limit?` | `PairsResponse`  |
| `GetWhitelistedCodeIds`  | `start_after?`, `limit?`               | `CodeIdsResponse`  |
| `GetPairCount`           | —                                      | `PairCountResponse`|

### Factory storage & upgrades

| Storage | Role |
|---------|------|
| `pairs` | Canonical asset-key → `PairInfo` |
| `pair_count` / `pair_index` | Sequential registry for paginated `Pairs` queries (**intentionally** iterated for discovery pagination) |
| `pair_addr_reg` | Pair contract `Addr` → `true`; **O(1)** membership for governance paths that validate a single pair address |

**Invariant:** For each index `i` in `0..pair_count`, `pair_index[i].contract_addr` has a `true` entry in `pair_addr_reg`. Maintained when pairs register in `reply_instantiate_pair`. Legacy factory instances on wasm **1.0.0** must migrate once to **1.1.0** so `pair_addr_reg` is backfilled from `pair_index` ([GitLab #122](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/122)).

**Gas / iteration:** Per-pair governance messages use **O(1)** `pair_addr_reg` where applicable ([#122](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/122)). For **broadcasting** discount-registry updates to **all** pairs at once, use **`SetDiscountRegistryBatch`** ([#123](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/123)) so each transaction carries a bounded Wasm message list; `SetDiscountRegistryAll` remains a single-tx option only for small pair counts. Other full-registry passes (e.g. propagating governance to LP admins on governance change) still iterate `pair_index` by design where no batched API exists. Indexers or LCD clients listing pairs should paginate rather than relying on unbounded queries. Off-chain operators and automation: [Indexer invariants — Factory LCD](./indexer-invariants.md#factory-lcd-pair-enumeration-vs-governance-gas-agents).

### Factory discount registry rollout (invariants, [GitLab #123](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/123))

`SetDiscountRegistryAll` attaches **one** `WasmMsg::Execute(SetDiscountRegistry …)` **per indexed pair**. With many pairs, the serialized response risks block gas limits and operator failure.

**`SetDiscountRegistryBatch`** emits at most **`limit`** such messages per transaction (omit `limit` ⇒ default **`10`**; hard cap **`30`**, same as other factory queries — see `dex_common::pagination`).

| Field / attribute | Semantics |
|-------------------|-----------|
| `start_after` | Optional **exclusive** cursor on the numeric `PAIR_INDEX` key. `null` / absent means “before index `0`”. The next index scanned is `start_after + 1` (or `0` when absent). |
| Response attrs | `pairs_updated` (messages emitted this TX), `has_more` (`true` until all indices `< PAIR_COUNT` are scanned past in order), **`next_start_after`** (omit when finished — replay this value as `start_after` on the next TX when `has_more` is true), `scanned_through_index` (last numeric index inspected this TX). |

**Invariants:**

1. Indices are contiguous from `0` to `PAIR_COUNT - 1` for normally created pairs (append-only registry).
2. If `PAIR_COUNT` grows **during** a multi-step rollout, repeat batches until **`has_more` is false**, then optionally rerun from `start_after: null` once more if new tail pairs must receive the registry (those indices were not scanned in earlier steps).
3. Failed/missing loads for an index slot are skipped (same as “all”), but contiguous indices remain the enumeration order — tooling should rely on **`has_more` / `next_start_after`**, not on `PAIR_COUNT` alone.

Canonical doc for agent automation: [`skills/AGENTS_TERRACLASSIC_GAS.md`](../skills/AGENTS_TERRACLASSIC_GAS.md).

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
| `SetDiscountRegistry`| `registry: Option<String>`                                              | Factory    |

### Cw20HookMsg (sent via CW20 Send)

| Variant              | Fields                                                    |
|----------------------|-----------------------------------------------------------|
| `Swap`               | `belief_price?`, `max_spread?`, `to?`, `deadline?`, `trader?` |
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
| `GetDiscountRegistry`| —                  | `DiscountRegistryResponse`   |

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

> **Note:** The Router passes the original sender's address as `trader` in the Pair's `Cw20HookMsg::Swap` so the Pair can look up the correct fee discount.

### QueryMsg

| Variant                          | Parameters                          | Returns                           |
|----------------------------------|-------------------------------------|-----------------------------------|
| `Config`                         | —                                   | `ConfigResponse`                  |
| `SimulateSwapOperations`         | `offer_amount`, `operations`        | `SimulateSwapOperationsResponse`  |
| `ReverseSimulateSwapOperations`  | `ask_amount`, `operations`          | `SimulateSwapOperationsResponse`  |

---

## Fee Discount

The fee-discount contract manages tiered swap fee discounts for CL8Y token holders. Traders register for a tier by holding the required CL8Y balance. The Pair contract queries this registry on each swap to determine the effective fee.

### InstantiateMsg

| Field            | Type     | Description                                          |
|------------------|----------|------------------------------------------------------|
| `governance`     | `String` | Address with admin privileges                        |
| `cl8y_token`     | `String` | CW20 address of the CL8Y token (18 decimals)        |

### ExecuteMsg

| Variant                | Fields                                                     | Auth        |
|------------------------|------------------------------------------------------------|-------------|
| `AddTier`              | `tier_id: u8`, `min_tokens: Uint128`, `discount_bps: u16` | Governance  |
| `UpdateTier`           | `tier_id: u8`, `min_tokens?: Uint128`, `discount_bps?: u16`| Governance  |
| `RemoveTier`           | `tier_id: u8`                                              | Governance  |
| `Register`             | `tier_id: u8`                                              | EOA only (self-register) |
| `RegisterWallet`       | `wallet: String`, `tier_id: u8`                            | Governance  |
| `Deregister`           | —                                                          | Self        |
| `DeregisterWallet`     | `wallet: String`                                           | Governance  |
| `AddTrustedRouter`     | `router: String`                                           | Governance  |
| `RemoveTrustedRouter`  | `router: String`                                           | Governance  |
| `UpdateConfig`         | `governance?`, `cl8y_token?`                               | Governance  |

### QueryMsg

| Variant              | Parameters                  | Returns                     |
|----------------------|-----------------------------|-----------------------------|
| `Config`             | —                           | `ConfigResponse`            |
| `GetDiscount`        | `trader: String`            | `DiscountResponse`          |
| `GetTier`            | `tier_id: u8`               | `TierResponse`              |
| `GetTiers`           | —                           | `TiersResponse`             |
| `GetRegistration`    | `wallet: String`            | `RegistrationResponse`      |
| `IsTrustedRouter`    | `router: String`            | `IsTrustedRouterResponse`   |

### Tier Table (default)

| Tier | CL8Y Required | Discount | BPS   | Notes                         |
|------|---------------|----------|-------|-------------------------------|
| 0    | 0             | 100%     | 10000 | Governance-only (market makers)|
| 1    | 1             | 2.5%     | 250   | Self-register, EOA only       |
| 2    | 5             | 10%      | 1000  | Self-register, EOA only       |
| 3    | 20            | 20%      | 2000  | Self-register, EOA only       |
| 4    | 75            | 35%      | 3500  | Self-register, EOA only       |
| 5    | 200           | 50%      | 5000  | Self-register, EOA only       |
| 6    | 500           | 60%      | 6000  | Self-register, EOA only       |
| 7    | 1,500         | 75%      | 7500  | Self-register, EOA only       |
| 8    | 3,500         | 85%      | 8500  | Self-register, EOA only       |
| 9    | 7,500         | 95%      | 9500  | Self-register, EOA only       |
| 255  | 0             | 0%       | 0     | Governance-only (blacklist)   |

### Discount Calculation

The Pair applies the discount as: `effective_fee = fee_bps * (10000 - discount_bps) / 10000`. For example, a pair with 30 bps fee and a Tier 5 trader (5000 bps discount, 50% off the fee) yields an effective fee of 15 bps.

### Balance Verification

The `GetDiscount` query checks the trader's CL8Y token balance on every call. If the balance is below the registered tier's threshold, the contract fires a deregistration message (fire-and-forget) and returns `discount_bps: 0` for that swap.

---

## Hook Interface

Any contract implementing this interface can be registered as a post-swap hook via the Factory.

### HookExecuteMsg

| Variant     | Fields                                                                                         |
|-------------|------------------------------------------------------------------------------------------------|
| `AfterSwap` | `pair`, `sender`, `offer_asset: Asset`, `return_asset: Asset`, `commission_amount`, `spread_amount` |
