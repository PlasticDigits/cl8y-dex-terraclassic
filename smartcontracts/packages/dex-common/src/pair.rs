use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;

use crate::oracle::{ObserveResponse, OracleInfoResponse};
use crate::types::{Asset, AssetInfo, FeeConfig};

#[cw_serde]
pub struct PairInstantiateMsg {
    pub asset_infos: [AssetInfo; 2],
    pub fee_bps: u16,
    pub treasury: Addr,
    pub factory: Addr,
    pub lp_token_code_id: u64,
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
    },
    WithdrawLiquidity {},
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

    /// Return cumulative ticks at the requested `seconds_ago` offsets.
    /// Consumers compute TWAP as:
    ///   `avg_tick = (tick[0] - tick[1]) / (seconds_ago[1] - seconds_ago[0])`
    ///   `price = 2^(avg_tick / 2^64)`
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
