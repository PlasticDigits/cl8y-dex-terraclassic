use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;

use crate::oracle::{ObserveResponse, OracleInfoResponse};
use crate::types::{Asset, AssetInfo, FeeConfig};

// ---------------------------------------------------------------------------
// Limit orders (hybrid AMM + FIFO doubly-linked book)
// ---------------------------------------------------------------------------

/// On-chain caps (pair contract enforces the same upper bounds).
pub const MAX_ADJUST_STEPS_HARD_CAP: u32 = 256;
pub const MAX_MAKER_FILLS_HARD_CAP: u32 = 256;

/// Pattern C: explicit split between constant-product pool and limit book.
/// `pool_input + book_input` must equal the CW20 `amount` on the swap hook.
#[cw_serde]
pub struct HybridSwapParams {
    /// Amount routed to the AMM after the book leg (includes any book remainder
    /// rolled forward when the book cannot fully fill `book_input`).
    pub pool_input: Uint128,
    /// Amount allocated to match against the limit book first (offer token units).
    pub book_input: Uint128,
    /// Stop after touching this many distinct maker orders (each order counts once per tx).
    pub max_maker_fills: u32,
    /// Optional order id to start linear insert/match adjustment (indexer hint).
    pub book_start_hint: Option<u64>,
}

/// Which side of the book the maker is on.
///
/// **Price** is always **token1 per token0** (output token1 per 1 unit of token0), matching pool pricing.
///
/// - **Bid**: maker escrows **token1** and buys token0 from incoming takers. Matched when a taker
///   swaps **token0 → token1** (taker sells token0, receives token1 from bids’ escrow).
/// - **Ask**: maker escrows **token0** and sells for token1. Matched when a taker swaps
///   **token1 → token0** (taker pays token1, receives token0 from asks’ escrow).
///
/// **Composite sort key** (total order, no duplicate keys):
/// - Bids: descending `price`, then ascending `order_id` (better bids first; FIFO at same price).
/// - Asks: ascending `price`, then ascending `order_id` (better asks first; FIFO at same price).
#[cw_serde]
pub enum LimitOrderSide {
    Bid,
    Ask,
}

/// Resting limit order returned by queries.
#[cw_serde]
pub struct LimitOrderResponse {
    pub order_id: u64,
    pub owner: Addr,
    pub side: LimitOrderSide,
    /// Token1 per token0 (minimum acceptable when the order executes).
    pub price: Decimal,
    /// Remaining escrow: token1 for bids, token0 for asks.
    pub remaining: Uint128,
    /// Unix seconds when the order stops being matchable (`None` = no expiry).
    #[serde(default)]
    pub expires_at: Option<u64>,
    pub prev: Option<u64>,
    pub next: Option<u64>,
}

#[cw_serde]
pub struct PairInstantiateMsg {
    pub asset_infos: [AssetInfo; 2],
    pub fee_bps: u16,
    pub treasury: Addr,
    pub factory: Addr,
    pub lp_token_code_id: u64,
    pub token_symbols: Option<[String; 2]>,
    pub governance: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    /// TerraSwap-compatible: provide liquidity using Asset pairs.
    ProvideLiquidity {
        assets: [Asset; 2],
        slippage_tolerance: Option<Decimal>,
        receiver: Option<String>,
        deadline: Option<u64>,
    },
    /// TerraSwap-compatible: swap with an offer_asset.
    /// For CW20 tokens, callers should use CW20 Send with `Cw20HookMsg::Swap` instead.
    Swap {
        offer_asset: Asset,
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
        to: Option<String>,
        deadline: Option<u64>,
    },
    UpdateFee {
        fee_bps: u16,
    },
    UpdateHooks {
        hooks: Vec<String>,
    },
    /// Grow the TWAP observation ring buffer. Larger cardinality supports
    /// longer TWAP windows. Anyone may call this (pays the gas for storage).
    IncreaseObservationCardinality {
        new_cardinality: u16,
    },
    /// Set the fee discount registry contract address. Factory only.
    SetDiscountRegistry {
        registry: Option<String>,
    },
    /// Emergency pause — only callable by the factory contract.
    SetPaused {
        paused: bool,
    },
    /// Recover tokens sent directly to the pair contract (donations,
    /// accidental transfers). Sends the excess (actual CW20 balance minus
    /// internal reserves) to `recipient`. Factory only.
    /// Does NOT modify internal reserves — pool pricing is unaffected.
    Sweep {
        token: String,
        recipient: String,
    },
    /// Update the LP token's CosmWasm admin. Factory only.
    SetLpAdmin {
        admin: String,
    },
    /// Cancel a resting limit order and refund remaining escrow to `owner`.
    CancelLimitOrder {
        order_id: u64,
    },
}

/// TerraSwap-compatible hook messages sent inside CW20 Send.
#[cw_serde]
pub enum Cw20HookMsg {
    Swap {
        belief_price: Option<Decimal>,
        max_spread: Option<Decimal>,
        to: Option<String>,
        deadline: Option<u64>,
        /// Original trader address for fee discount lookup.
        /// Set by trusted routers; the pair verifies the CW20 sender
        /// is a trusted router before honoring this field.
        trader: Option<String>,
        /// Pattern C: optional split between limit book and pool (see [`HybridSwapParams`]).
        hybrid: Option<HybridSwapParams>,
    },
    /// Post a limit order using the escrowed CW20 as the correct side asset (token0 for Ask, token1 for Bid).
    PlaceLimitOrder {
        side: LimitOrderSide,
        /// Minimum token1 per token0 for maker fills.
        price: Decimal,
        /// Reserved for future indexer-assisted insertion. **Current pair
        /// implementations ignore this field** and locate the insert position by
        /// walking from the book head (capped by `max_adjust_steps`).
        hint_after_order_id: Option<u64>,
        /// Max steps when walking the book from the **head** to find the insert position.
        max_adjust_steps: u32,
        /// Unix seconds after which the order is no longer matched (must be `> now` if set).
        #[serde(default)]
        expires_at: Option<u64>,
    },
    /// Burn LP tokens and receive underlying assets pro-rata.
    /// Optional `min_assets` protects against sandwich attacks by reverting
    /// if either returned amount falls below the specified minimum.
    WithdrawLiquidity { min_assets: Option<[Uint128; 2]> },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    /// TerraSwap-compatible: returns PairInfo.
    #[returns(crate::types::PairInfo)]
    Pair {},
    /// TerraSwap-compatible: returns pool reserves and total LP share.
    #[returns(PoolResponse)]
    Pool {},
    /// TerraSwap-compatible: simulate a swap.
    #[returns(SimulationResponse)]
    Simulation { offer_asset: Asset },
    /// TerraSwap-compatible: reverse-simulate a swap.
    #[returns(ReverseSimulationResponse)]
    ReverseSimulation { ask_asset: Asset },
    #[returns(FeeConfigResponse)]
    GetFeeConfig {},
    #[returns(HooksResponse)]
    GetHooks {},

    // ---- TWAP oracle queries ----
    /// Return cumulative price sums at the requested `seconds_ago` offsets.
    /// Consumers compute the arithmetic-mean TWAP as:
    ///   `twap = (cum[0] - cum[1]) / (seconds_ago[1] - seconds_ago[0])`
    ///
    /// **SECURITY WARNING:** This TWAP should NOT be the sole price feed for
    /// liquidations, mark prices, or collateral valuation. Always validate
    /// against a secondary oracle (Band, off-chain relay, governance
    /// reference) and apply deviation / staleness checks.
    #[returns(ObserveResponse)]
    Observe { seconds_ago: Vec<u32> },
    /// Metadata about the oracle ring buffer.
    #[returns(OracleInfoResponse)]
    OracleInfo {},

    /// Limit order by id (if it exists).
    #[returns(LimitOrderResponse)]
    LimitOrder { order_id: u64 },
    /// Head order id for bid or ask list (empty book = none).
    #[returns(Option<u64>)]
    OrderBookHead { side: LimitOrderSide },
}

/// TerraSwap-compatible pool response.
#[cw_serde]
pub struct PoolResponse {
    pub assets: [Asset; 2],
    pub total_share: Uint128,
}

#[cw_serde]
pub struct FeeConfigResponse {
    pub fee_config: FeeConfig,
}

#[cw_serde]
pub struct HooksResponse {
    pub hooks: Vec<Addr>,
}

/// TerraSwap-compatible simulation response.
#[cw_serde]
pub struct SimulationResponse {
    pub return_amount: Uint128,
    pub spread_amount: Uint128,
    pub commission_amount: Uint128,
}

/// TerraSwap-compatible reverse simulation response.
#[cw_serde]
pub struct ReverseSimulationResponse {
    pub offer_amount: Uint128,
    pub spread_amount: Uint128,
    pub commission_amount: Uint128,
}
