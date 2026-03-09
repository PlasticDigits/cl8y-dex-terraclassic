# Terraport Contract Interface Reference

Terraport is the dominant DEX on Terra Classic (Columbus-5). Vyntrex integrates Terraport
and needs to parse its on-chain messages, queries, and events. This document is the
authoritative reference for what Terraport's contracts accept and emit.

## Origin and Source Code

Terraport's DEX core (factory, pair, router) is a **TerraSwap fork**. The source code is
**not publicly available** in Terraport's own repositories, but the upstream TerraSwap
codebase is open source:

- Upstream: <https://github.com/terraswap/classic-terraswap>
- Docs: <https://docs.terraswap.io>
- Terraport developer docs: <https://terraport.gitbook.io/terraport-docs/developers/query>

Astroport (`astroport-core`) is a **separate project** with a different message schema. While
the concepts overlap (factory/pair/router), Astroport uses `Vec<AssetInfo>` instead of
`[AssetInfo; 2]`, adds `PairType` enums, TWAP oracles, fee-share configs, and many query
variants that Terraport does not have. Terraport/TerraSwap messages are **not
Astroport-compatible**.

---

## Contract Addresses (Columbus-5 Mainnet)

| Contract | Address |
|---|---|
| Token code | `terra1ex0hjv3wurhj4wgup4jzlzaqj4av6xqd8le4etml7rg9rs207y4s8cdvrp` |
| Vesting | `terra19v3vkpxsxeach4tpdklxaxc9wuwx65jqfs6jzm5cu5yz457hhgmsp4a48n` |
| Staking | `terra134ummlrj2rnv8h8rjhs6a54fng0xlg8wk7a2gwu6vj42pznkf6xs95966d` |
| Farm LUNC-TERRA | `terra17etqn03dx94w5vcgk0yux03ce9ns443j0pjyf407edhf0v4z26fqpdvv6y` |
| Farm USTC-TERRA | `terra17ddfyml64hjm57vpn2ah03e5nk4ugqk66rp37832g883pkn9ma4sfkxfyf` |
| Treasury | `terra187cnk4ae0ynets4f638398zhehfhkuefrljgn0tyewlkpmulxq7sr6a3lr` |
| Burn address | `terra1rd8f2mhdhgtsfk5afcpfj78wzkljc4lns6t2rhwftvtwzdz95cuqmt7rmm` |
| Factory | `terra1n75fgfc8clsssrm2k0fswgtzsvstdaah7la6sfu96szdu22xta0q57rqqr` |
| Router | `terra1vrqd7fkchyc7wjumn8fxly88z7kath4djjls3yc5th5g76f3543salu48s` |

Source: <https://terraport.gitbook.io/terraport-docs/resources/contracts>

---

## Shared Types

These types are used across all Terraport contracts and match the TerraSwap type system.

```rust
pub enum AssetInfo {
    Token { contract_addr: String },       // CW20 token
    NativeToken { denom: String },         // native/IBC coin (uluna, uusd, etc.)
}

pub struct Asset {
    pub info: AssetInfo,
    pub amount: Uint128,
}

pub struct PairInfo {
    pub asset_infos: [AssetInfo; 2],
    pub contract_addr: String,
    pub liquidity_token: String,
}
```

JSON representation:

```json
// CW20 token
{ "token": { "contract_addr": "terra1..." } }

// Native token
{ "native_token": { "denom": "uluna" } }

// Asset (token + amount)
{
  "info": { "native_token": { "denom": "uluna" } },
  "amount": "1000000"
}
```

> **Key difference from our contracts:** Our DEX uses only CW20 tokens identified by
> `Addr` directly, with no `AssetInfo` enum. Terraport supports both CW20 and native
> tokens via the `AssetInfo` discriminated union.

---

## Factory Contract

### Execute Messages

#### `create_pair`

Creates a new trading pair. Terraport uses `asset_infos` (not `assets` with amounts like
base TerraSwap).

```json
{
  "create_pair": {
    "asset_infos": [
      { "token": { "contract_addr": "terra1..." } },
      { "native_token": { "denom": "uluna" } }
    ]
  }
}
```

### Query Messages

#### `config`

Returns factory configuration.

```json
{ "config": {} }
```

#### `pair`

Returns info for a specific pair.

```json
{
  "pair": {
    "asset_infos": [
      { "token": { "contract_addr": "terra1..." } },
      { "native_token": { "denom": "uluna" } }
    ]
  }
}
```

Response:

```json
{
  "asset_infos": [
    { "token": { "contract_addr": "terra1..." } },
    { "native_token": { "denom": "uluna" } }
  ],
  "contract_addr": "terra1...",
  "liquidity_token": "terra1..."
}
```

#### `pairs`

Paginated list of all pairs.

```json
{
  "pairs": {
    "start_after": [
      { "token": { "contract_addr": "terra1..." } },
      { "native_token": { "denom": "uluna" } }
    ],
    "limit": 10
  }
}
```

Response:

```json
{
  "pairs": [
    {
      "asset_infos": [ ... ],
      "contract_addr": "terra1...",
      "liquidity_token": "terra1..."
    }
  ]
}
```

#### `native_token_decimals`

```json
{ "native_token_decimals": { "denom": "uluna" } }
```

### Events

| Action | Attributes |
|---|---|
| `create_pair` | `pair` (asset pair description) |

---

## Pair Contract

### Execute Messages

#### `provide_liquidity`

Add liquidity to the pool.

```json
{
  "provide_liquidity": {
    "assets": [
      {
        "info": { "token": { "contract_addr": "terra1..." } },
        "amount": "1000000"
      },
      {
        "info": { "native_token": { "denom": "uluna" } },
        "amount": "1000000"
      }
    ],
    "receiver": "terra1...",
    "deadline": 1686903051,
    "slippage_tolerance": "0.005"
  }
}
```

All fields except `assets` are optional. When providing native tokens, the corresponding
coins must be attached to the transaction. For CW20 tokens, the sender must first set an
allowance on the pair contract.

#### `swap` (native token offer)

Direct swap when offering a native token.

```json
{
  "swap": {
    "offer_asset": {
      "info": { "native_token": { "denom": "uluna" } },
      "amount": "1000000"
    },
    "belief_price": "0.1",
    "max_spread": "0.01",
    "to": "terra1...",
    "deadline": 1686903051
  }
}
```

All fields except `offer_asset` are optional.

#### `swap` (CW20 token offer, via CW20 `send`)

When offering a CW20 token, the swap message is wrapped inside a CW20 `send`:

```json
{
  "send": {
    "contract": "<pair_contract_addr>",
    "amount": "1000000",
    "msg": "<base64 encoded swap message>"
  }
}
```

The inner `msg` (before base64 encoding):

```json
{
  "swap": {
    "belief_price": "0.1",
    "max_spread": "0.01",
    "to": "terra1...",
    "deadline": 1686903051
  }
}
```

#### `withdraw_liquidity` (via CW20 `send`)

Withdraw liquidity by sending LP tokens back to the pair contract:

```json
{
  "send": {
    "contract": "<pair_contract_addr>",
    "amount": "1000000",
    "msg": "<base64 encoded withdraw message>"
  }
}
```

The inner `msg` (before base64 encoding):

```json
{
  "withdraw_liquidity": {}
}
```

### Query Messages

#### `pool`

Returns current pool reserves and total LP supply.

```json
{ "pool": {} }
```

Response:

```json
{
  "assets": [
    { "info": { "token": { "contract_addr": "terra1..." } }, "amount": "1000000" },
    { "info": { "native_token": { "denom": "uluna" } }, "amount": "2000000" }
  ],
  "total_share": "1414213"
}
```

#### `pair`

Returns the pair info (same as factory `pair` query response).

```json
{ "pair": {} }
```

#### `simulation`

Simulate a swap to preview output amount, spread, and commission.

```json
{
  "simulation": {
    "offer_asset": {
      "info": { "token": { "contract_addr": "terra1..." } },
      "amount": "1000000"
    }
  }
}
```

Response:

```json
{
  "return_amount": "990000",
  "spread_amount": "5000",
  "commission_amount": "5000"
}
```

#### `reverse_simulation`

Calculate how much input is needed to receive a desired output.

```json
{
  "reverse_simulation": {
    "ask_asset": {
      "info": { "native_token": { "denom": "uluna" } },
      "amount": "1000000"
    }
  }
}
```

Response:

```json
{
  "offer_amount": "1010000",
  "spread_amount": "5000",
  "commission_amount": "5000"
}
```

### Events

| Action | Attributes |
|---|---|
| `provide_liquidity` | `sender`, `receiver`, `assets` (stringified), `share` (LP minted) |
| `withdraw_liquidity` | `sender`, `withdrawn_share`, `refund_assets` (stringified) |
| `swap` | `sender`, `receiver`, `offer_asset`, `ask_asset`, `offer_amount`, `return_amount`, `spread_amount`, `commission_amount` |

---

## Router Contract

The router enables multi-hop swaps across multiple pairs.

### Execute Messages

#### `execute_swap_operations` (native token start)

When the first token in the route is a native token, call the router directly:

```json
{
  "execute_swap_operations": {
    "operations": [
      {
        "native_swap": {
          "offer_denom": "ukrw",
          "ask_denom": "uusd"
        }
      },
      {
        "terra_swap": {
          "offer_asset_info": { "native_token": { "denom": "uusd" } },
          "ask_asset_info": { "token": { "contract_addr": "terra1..." } }
        }
      }
    ],
    "minimum_receive": "88000",
    "to": "terra1...",
    "deadline": 1686903051
  }
}
```

#### `execute_swap_operations` (CW20 token start, via CW20 `send`)

When the first token is a CW20 token, wrap the message in a CW20 `send`:

```json
{
  "send": {
    "amount": "100000000",
    "contract": "<router_contract_addr>",
    "msg": "<base64 encoded execute_swap_operations message>"
  }
}
```

#### Swap Operation Types

```rust
pub enum SwapOperation {
    NativeSwap {
        offer_denom: String,
        ask_denom: String,
    },
    TerraSwap {
        offer_asset_info: AssetInfo,
        ask_asset_info: AssetInfo,
    },
}
```

- `native_swap` -- on-chain native token market swap (e.g. LUNC <-> USTC)
- `terra_swap` -- AMM pool swap through a TerraSwap/Terraport pair

### Query Messages

#### `simulate_swap_operations`

Preview multi-hop swap output.

```json
{
  "simulate_swap_operations": {
    "offer_amount": "10000000",
    "operations": [
      {
        "terra_swap": {
          "offer_asset_info": { "native_token": { "denom": "uluna" } },
          "ask_asset_info": { "token": { "contract_addr": "terra1..." } }
        }
      }
    ]
  }
}
```

Response:

```json
{ "amount": "9800000" }
```

#### `reverse_simulate_swap_operations`

Calculate required input for a desired output across a multi-hop route. Note: `native_swap`
operations are **not supported** in reverse simulation.

```json
{
  "reverse_simulate_swap_operations": {
    "ask_amount": "10000000",
    "operations": [
      {
        "terra_swap": {
          "offer_asset_info": { "token": { "contract_addr": "terra1..." } },
          "ask_asset_info": { "native_token": { "denom": "uusd" } }
        }
      }
    ]
  }
}
```

### Events

The router delegates to individual pair contracts via submessages. Swap events are emitted
by each pair's `swap` handler, not by the router itself.

---

## Terraport-Specific Contracts (Non-TerraSwap)

Terraport adds staking, farming, and governance contracts that are **not** part of the
TerraSwap base. Their source code is not publicly available. Known operations from the UI:

| Contract | Operations |
|---|---|
| Staking | `Stake`, `Unstake`, `UnstakeAll`, `Claim` |
| Farming | Stake LP tokens, claim TERRA rewards |
| Vesting | Token vesting schedules |
| Treasury | Fee collection and distribution |

---

## Comparison: Terraport vs Our DEX vs Astroport

### Asset Model

| Feature | Terraport (TerraSwap) | Our DEX | Astroport |
|---|---|---|---|
| Asset types | CW20 + native tokens | CW20 only | CW20 + native tokens |
| Asset identifier | `AssetInfo` enum (Token/NativeToken) | Direct `Addr` | `AssetInfo` enum (Token/NativeToken) |
| Pair arity | Fixed `[AssetInfo; 2]` | Fixed 2 tokens | Variable `Vec<AssetInfo>` |
| Pair types | XYK only | XYK only | XYK, Stable, Custom |

### Factory Messages

| Operation | Terraport | Our DEX | Astroport |
|---|---|---|---|
| Create pair | `create_pair { asset_infos }` | `CreatePair { token_a, token_b }` | `CreatePair { pair_type, asset_infos, init_params }` |
| Query pair | `pair { asset_infos }` | `GetPair { token_a, token_b }` | `Pair { asset_infos }` |
| List pairs | `pairs { start_after, limit }` | `GetAllPairs { start_after, limit }` | `Pairs { start_after, limit }` |
| Update config | Not documented | `UpdateConfig { ... }` | `UpdateConfig { ... }` |
| Set fees | Not documented (built into pair) | `SetPairFee { pair, fee_bps }` | `UpdatePairConfig { config }` |
| Set hooks | N/A | `SetPairHooks { pair, hooks }` | N/A |

### Pair Messages

| Operation | Terraport | Our DEX | Astroport |
|---|---|---|---|
| Swap | `swap { offer_asset, belief_price, max_spread, to, deadline }` | CW20 hook: `Swap { min_output, to }` | `Swap { offer_asset, ask_asset_info, belief_price, max_spread, to }` |
| Add liquidity | `provide_liquidity { assets, slippage_tolerance, receiver, deadline }` | `AddLiquidity { token_a_amount, token_b_amount, min_lp_tokens, slippage_tolerance }` | `ProvideLiquidity { assets, slippage_tolerance, auto_stake, receiver, min_lp_to_receive }` |
| Remove liquidity | CW20 hook: `withdraw_liquidity {}` | CW20 hook: `RemoveLiquidity { min_a, min_b }` | `WithdrawLiquidity { assets, min_assets_to_receive }` |
| Simulate swap | `simulation { offer_asset }` | `SimulateSwap { offer_token, offer_amount }` | `Simulation { offer_asset, ask_asset_info }` |
| Reverse simulate | `reverse_simulation { ask_asset }` | N/A | `ReverseSimulation { offer_asset_info, ask_asset }` |
| Query pool | `pool {}` | `GetReserves {}` | `Pool {}` |
| Query pair info | `pair {}` | `GetPairInfo {}` | `Pair {}` |

### Router Messages

| Operation | Terraport | Our DEX | Astroport |
|---|---|---|---|
| Multi-hop swap | `execute_swap_operations { operations, minimum_receive, to, deadline }` | CW20 hook: `SwapTokens { route, min_output, to }` | `ExecuteSwapOperations { operations, minimum_receive, to, max_spread }` |
| Route representation | `Vec<SwapOperation>` (NativeSwap/TerraSwap) | `Vec<String>` (pair addresses) | `Vec<SwapOperation>` (NativeSwap/AstroSwap) |
| Simulate route | `simulate_swap_operations { offer_amount, operations }` | `SimulateRoute { route, offer_amount }` | `SimulateSwapOperations { offer_amount, operations }` |
| Reverse simulate | `reverse_simulate_swap_operations { ask_amount, operations }` | N/A | `ReverseSimulateSwapOperations { ask_amount, operations }` |

### Swap Events

| Attribute | Terraport | Our DEX | Astroport |
|---|---|---|---|
| `action` | `"swap"` | `"swap"` | `"swap"` |
| `sender` | Yes | Yes | Yes |
| `receiver` | Yes | N/A | Yes |
| `offer_asset` | Yes (stringified) | `input_token` | Yes (stringified) |
| `ask_asset` | Yes (stringified) | `output_token` | Yes (stringified) |
| `offer_amount` | Yes | `input_amount` | Yes |
| `return_amount` | Yes | `output_amount` | Yes |
| `spread_amount` | Yes | N/A | Yes |
| `commission_amount` | Yes | `fee_amount` | Yes |
| `maker_fee_amount` | N/A | N/A | Yes |

---

## Vyntrex Integration Notes

For Vyntrex to correctly parse Terraport transactions, it needs to handle:

1. **The `AssetInfo` enum** -- assets can be either `{ "token": { "contract_addr": "..." } }`
   or `{ "native_token": { "denom": "..." } }`. This is fundamentally different from our
   CW20-only model.

2. **CW20 `send` wrapping** -- CW20 token swaps and LP withdrawals are wrapped in a CW20
   `send` message with a base64-encoded inner message. The actual operation is in the
   decoded `msg` field.

3. **Swap event parsing** -- look for `wasm` events with `action = "swap"` and extract
   `offer_asset`, `ask_asset`, `offer_amount`, `return_amount`, `spread_amount`,
   `commission_amount`.

4. **Router operations** -- multi-hop swaps use `SwapOperation` variants (`native_swap` or
   `terra_swap`), not simple pair address lists like our router.

5. **No hooks** -- Terraport does not have the post-swap hook system that our DEX uses
   (burn-hook, tax-hook, lp-burn-hook). Fee handling is built into the pair contract
   directly.
