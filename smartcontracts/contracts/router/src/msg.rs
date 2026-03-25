use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;
use dex_common::pair::HybridSwapParams;
use dex_common::types::AssetInfo;

#[cw_serde]
pub struct InstantiateMsg {
    pub factory: String,
}

/// TerraSwap-compatible swap operation.
#[cw_serde]
pub enum SwapOperation {
    NativeSwap {
        offer_denom: String,
        ask_denom: String,
    },
    TerraSwap {
        offer_asset_info: AssetInfo,
        ask_asset_info: AssetInfo,
        /// Pattern C: per-hop book vs pool split (None = 100% pool, TerraSwap-compatible default).
        hybrid: Option<HybridSwapParams>,
    },
}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    /// TerraSwap-compatible multi-hop swap.
    ExecuteSwapOperations {
        operations: Vec<SwapOperation>,
        max_spread: Decimal,
        minimum_receive: Option<Uint128>,
        to: Option<String>,
        deadline: Option<u64>,
        unwrap_output: Option<bool>,
    },
    /// Set the wrap-mapper contract address (governance-only via factory).
    SetWrapMapper {
        wrap_mapper: String,
    },
}

/// TerraSwap-compatible CW20 hook message for the router.
#[cw_serde]
pub enum Cw20HookMsg {
    ExecuteSwapOperations {
        operations: Vec<SwapOperation>,
        max_spread: Decimal,
        minimum_receive: Option<Uint128>,
        to: Option<String>,
        deadline: Option<u64>,
        unwrap_output: Option<bool>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(SimulateSwapOperationsResponse)]
    SimulateSwapOperations {
        offer_amount: Uint128,
        operations: Vec<SwapOperation>,
    },
    #[returns(SimulateSwapOperationsResponse)]
    ReverseSimulateSwapOperations {
        ask_amount: Uint128,
        operations: Vec<SwapOperation>,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    pub factory: Addr,
    pub wrap_mapper: Option<Addr>,
}

#[cw_serde]
pub struct SimulateSwapOperationsResponse {
    pub amount: Uint128,
}

#[cw_serde]
pub struct MigrateMsg {}
