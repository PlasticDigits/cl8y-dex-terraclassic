use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;

use crate::oracle::{ObserveResponse, OracleInfoResponse};
use crate::types::{Asset, AssetInfo, FeeConfig};

pub use crate::limit_clean::{
    clamp_max_clean_orders, clamp_max_clean_scan_steps, LimitCleanConfigResponse,
    MAX_CLEAN_SCAN_STEPS, MAX_LIMIT_CLEAN_ORDERS_HARD_CAP,
};
pub use crate::limit_placement::{
    clamp_max_batch_rungs, expand_limit_ladder, LimitLadderDistribution, LimitOrderConfigResponse,
    LimitOrderLadderSpec, LimitOrderPlacementItem, DEFAULT_LIMIT_BATCH_MAX_RUNGS,
    MAX_LIMIT_BATCH_RUNGS_HARD_CAP, SUGGESTED_FACTORY_DEFAULT_LIMIT_BATCH_MAX_RUNGS,
};

// ---------------------------------------------------------------------------
// Limit orders (hybrid AMM + FIFO doubly-linked book)
// ---------------------------------------------------------------------------

/// On-chain caps (pair contract enforces the same upper bounds).
pub const MAX_ADJUST_STEPS_HARD_CAP: u32 = 256;
/// Maximum distinct maker orders matched per hybrid book walk (execute + sim).
/// Clamps caller `max_maker_fills` ([GitLab #262](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/262)).
pub const MAX_MAKER_FILLS_HARD_CAP: u32 = 100;
/// Hard ceiling on total book-walk iterations per side per call (fills + parks + skips + continues).
/// Decoupled from [`MAX_MAKER_FILLS_HARD_CAP`] so taker walks can traverse deep expired prefixes
/// without raising the distinct-maker fill cap
/// ([GitLab #254](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/254),
/// [#262](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/262)).
/// Sized for ~19k gas per list iteration vs a 15M dApp gas envelope (~789 steps); see GitLab #262.
pub const MAX_SCAN_STEPS: u32 = 500;
/// Maximum expired limit orders parked into `EXPIRED_LIMIT_CLAIMS` per book walk during hybrid swap
/// ([GitLab #250](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/250),
/// raised in [#254](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/254)).
/// Additional expired orders at the book head are skipped without storage writes until a later tx.
/// Kept well below [`MAX_SCAN_STEPS`] — parking is write-heavy (unlink + claim row + event).
pub const MAX_EXPIRED_PARKS_PER_SWAP: u32 = 15;

/// Post-fill rounding can leave 1–9 smallest-unit remainders on limit orders. Match walks
/// auto-park sub-threshold dust into `EXPIRED_LIMIT_CLAIMS` (GitLab #264); governance
/// `CleanLimitBook` thresholds ([#263](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/263))
/// cover larger notionals separately.
pub const LIMIT_ORDER_DUST_FLUSH_THRESHOLD: Uint128 = Uint128::new(10);

/// TTL for on-pair CL8Y fee-discount cache entries ([GitLab #251](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/251)).
///
/// The discount is a 300s **snapshot**, not a live per-trade balance check: a tier holder must
/// transact at least once per window to keep it fresh, and a wallet dropping below its tier keeps
/// the cached discount until the entry expires. Accepted design — the cache exists to avoid a
/// per-swap registry query (#251); see [GitLab #275](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/275).
pub const DISCOUNT_CACHE_TTL_SECONDS: u64 = 300;

/// Decimals configured on CW20 mintable LP tokens instantiated by pairs (gitlab #124).
pub const LP_TOKEN_DECIMALS: u8 = 18;

/// Maximum CW20 `decimals` allowed on either pool asset when bootstrapping empty reserves
/// (`provide_liquidity` first mint) or when creating a pair via the factory. Higher decimals are
/// rejected because realistic deposits can overflow `Uint128` in `amount_a * amount_b`.
pub const MAX_PAIR_ASSET_DECIMALS_BOOTSTRAP: u8 = 18;

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

/// Pool-only quote/execute shape: entire offer on the AMM, no book leg (invariant L8).
pub fn pool_only_hybrid_params(offer_amount: Uint128) -> HybridSwapParams {
    HybridSwapParams {
        pool_input: offer_amount,
        book_input: Uint128::zero(),
        max_maker_fills: 1,
        book_start_hint: None,
    }
}

/// Ratio template for [`QueryMsg::HybridReverseSimulation`] when the offer is pool-only.
pub fn pool_only_hybrid_template() -> HybridSwapParams {
    HybridSwapParams {
        pool_input: Uint128::one(),
        book_input: Uint128::zero(),
        max_maker_fills: 1,
        book_start_hint: None,
    }
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

/// Claimable refund for an order parked on expiry (see `ClaimExpiredLimitOrder`).
#[cw_serde]
pub struct ExpiredLimitRefundResponse {
    pub order_id: u64,
    pub owner: Addr,
    pub side: LimitOrderSide,
    /// Same units as `LimitOrderResponse::remaining`: token1 for bids, token0 for asks.
    pub remaining: Uint128,
    #[serde(default)]
    pub expires_at: Option<u64>,
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

/// Pair governance pause flag (readable without attempting a swap).
#[cw_serde]
pub struct PausedResponse {
    pub paused: bool,
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
    /// Max rungs per batch/ladder placement (clamped to [`MAX_LIMIT_BATCH_RUNGS_HARD_CAP`]).
    pub max_batch_rungs: u32,
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
    /// Cancel multiple resting limit orders in one tx (≤ [`MAX_LIMIT_BATCH_RUNGS_HARD_CAP`] ids;
    /// same per-pair cap as batch placement). Owner-only for every id; whole tx reverts on any
    /// failure. Refunds aggregate into at most two CW20 transfers (token0 + token1). GitLab #246.
    CancelLimitOrders {
        order_ids: Vec<u64>,
    },
    /// Claim escrow for an order removed from the book because it had **expired** when a taker’s
    /// match walk processed that price level. The refund row is stored until claimed (`ExpiredLimitRefund`
    /// query). Owner-only. **Blocked while the pair is paused** (same `assert_not_paused` gate as
    /// `CancelLimitOrder` — emergency pause freezes all maker withdrawal paths; GitLab #120).
    ClaimExpiredLimitOrder {
        order_id: u64,
    },
    /// Claim multiple parked-expiry refund rows in one tx (same cap and all-or-nothing rules as
    /// [`CancelLimitOrders`]). GitLab #246.
    ClaimExpiredLimitOrders {
        order_ids: Vec<u64>,
    },
    /// Change the limit price of an existing order (same `order_id`, same
    /// remaining size). Does not charge the maker placement fee again.
    UpdateLimitOrderPrice {
        order_id: u64,
        price: Decimal,
        hint_after_order_id: Option<u64>,
        max_adjust_steps: u32,
    },
    /// Update batch/ladder rung cap. Factory (governance) only.
    UpdateLimitOrderConfig {
        max_batch_rungs: u32,
    },
    /// Permissionless: park time-expired and/or governance dust orders from the limit book
    /// into `EXPIRED_LIMIT_CLAIMS` (no CW20 movement; makers claim later). Distinct from
    /// factory-only CW20 excess recovery [`Sweep`](ExecuteMsg::Sweep) (GitLab #263).
    CleanLimitBook {
        side: LimitOrderSide,
        max_orders: u32,
        /// Optional order id to start the walk; when absent or not on-book, walk from head.
        start_hint: Option<u64>,
        /// Optional traversal budget (nodes visited before exiting). Absent or `0` →
        /// `MAX_CLEAN_SCAN_STEPS`; clamped to that hard cap. Bounds scan gas independently of
        /// `max_orders`, so a keeper can resume from the emitted `resume_cursor` (GitLab #274).
        #[serde(default)]
        max_steps: Option<u32>,
    },
    /// Set per-side min remaining notionals for force-clean (factory only). `0` disables force-clean
    /// on that side; only time-expired orders are eligible.
    UpdateLimitCleanConfig {
        min_remaining_token0: Uint128,
        min_remaining_token1: Uint128,
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
    /// Place multiple limits in one CW20 `send` (same side; amounts must sum to `send` amount).
    PlaceLimitOrderBatch {
        side: LimitOrderSide,
        orders: Vec<LimitOrderPlacementItem>,
    },
    /// Place a price ladder; expanded on-chain to [`PlaceLimitOrderBatch`].
    PlaceLimitOrderLadder { ladder: LimitOrderLadderSpec },
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

    /// Whether the pair is paused (no swap, no new limits, no cancel / claim / price update per L6).
    #[returns(PausedResponse)]
    IsPaused {},

    /// Limit order by id (if it exists).
    #[returns(LimitOrderResponse)]
    LimitOrder { order_id: u64 },
    /// Refund row for an order removed from the book due to expiry during a match walk (`None` if none).
    #[returns(Option<ExpiredLimitRefundResponse>)]
    ExpiredLimitRefund { order_id: u64 },
    /// Head order id for bid or ask list (empty book = none).
    #[returns(Option<u64>)]
    OrderBookHead { side: LimitOrderSide },

    /// Max rungs allowed per batch/ladder placement on this pair.
    #[returns(LimitOrderConfigResponse)]
    LimitOrderConfig {},
    /// Min remaining notionals for permissionless force-clean per token side (GitLab #263).
    #[returns(LimitCleanConfigResponse)]
    LimitCleanConfig {},

    #[returns(HybridSimulationResponse)]
    HybridSimulation {
        offer_asset: Asset,
        hybrid: HybridSwapParams,
        /// When set, CL8Y fee-tier discount is applied (same math as execute). Omit for undiscounted quotes.
        trader: Option<String>,
        /// CW20 sender for discount lookup when `trader` differs (e.g. trusted router). Defaults to `trader`.
        sender: Option<String>,
        /// When set, skip the no-belief material pool-leg guard (same as execute `belief_price`).
        #[serde(default)]
        belief_price: Option<Decimal>,
    },
    #[returns(HybridReverseSimulationResponse)]
    HybridReverseSimulation {
        ask_asset: Asset,
        hybrid: HybridSwapParams,
        trader: Option<String>,
        sender: Option<String>,
        /// When set, skip the no-belief material pool-leg guard (same as execute `belief_price`).
        #[serde(default)]
        belief_price: Option<Decimal>,
    },
}

/// Undiscounted forward hybrid quote (`trader` / `sender` omitted — full pair fee).
pub fn hybrid_simulation_undiscounted(offer_asset: Asset, hybrid: HybridSwapParams) -> QueryMsg {
    QueryMsg::HybridSimulation {
        offer_asset,
        hybrid,
        trader: None,
        sender: None,
        belief_price: None,
    }
}

/// Undiscounted reverse hybrid quote.
pub fn hybrid_reverse_simulation_undiscounted(
    ask_asset: Asset,
    hybrid: HybridSwapParams,
) -> QueryMsg {
    QueryMsg::HybridReverseSimulation {
        ask_asset,
        hybrid,
        trader: None,
        sender: None,
        belief_price: None,
    }
}

/// Forward hybrid quote with CL8Y fee-tier discount for `trader` (optional distinct `sender`).
pub fn hybrid_simulation_with_trader(
    offer_asset: Asset,
    hybrid: HybridSwapParams,
    trader: String,
    sender: Option<String>,
) -> QueryMsg {
    QueryMsg::HybridSimulation {
        offer_asset,
        hybrid,
        trader: Some(trader),
        sender,
        belief_price: None,
    }
}

/// Reverse hybrid quote with CL8Y fee-tier discount for `trader`.
pub fn hybrid_reverse_simulation_with_trader(
    ask_asset: Asset,
    hybrid: HybridSwapParams,
    trader: String,
    sender: Option<String>,
) -> QueryMsg {
    QueryMsg::HybridReverseSimulation {
        ask_asset,
        hybrid,
        trader: Some(trader),
        sender,
        belief_price: None,
    }
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

#[cw_serde]
pub struct HybridSimulationResponse {
    pub return_amount: Uint128,
    pub spread_amount: Uint128,
    /// Total protocol commission in the ask asset (pool + book taker fees). Same basis as
    /// `AfterSwap.commission_amount` in post-swap hooks (invariant **L7**, GitLab #196).
    pub commission_amount: Uint128,
    /// Pool-leg treasury commission (ask asset).
    pub pool_commission_amount: Uint128,
    /// Book taker commission total (ask asset); zero when `book_input = 0`.
    pub book_commission_amount: Uint128,
    pub book_return_amount: Uint128,
    pub pool_return_amount: Uint128,
}

#[cw_serde]
pub struct HybridReverseSimulationResponse {
    pub offer_amount: Uint128,
    pub spread_amount: Uint128,
    /// Total protocol commission in the ask asset (pool + book taker fees).
    pub commission_amount: Uint128,
    pub pool_commission_amount: Uint128,
    pub book_commission_amount: Uint128,
    pub book_return_amount: Uint128,
    pub pool_return_amount: Uint128,
}
