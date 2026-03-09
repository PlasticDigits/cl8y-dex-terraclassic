use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};
use cw20::Cw20ReceiveMsg;

use crate::types::{FeeConfig, PairInfo};

#[cw_serde]
pub struct PairInstantiateMsg {
    pub token_a: Addr,
    pub token_b: Addr,
    pub fee_bps: u16,
    pub treasury: Addr,
    pub factory: Addr,
    pub lp_token_code_id: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    AddLiquidity {
        token_a_amount: Uint128,
        token_b_amount: Uint128,
        min_lp_tokens: Option<Uint128>,
        slippage_tolerance: Option<Decimal>,
    },
    UpdateFee {
        fee_bps: u16,
    },
    UpdateHooks {
        hooks: Vec<String>,
    },
}

#[cw_serde]
pub enum Cw20HookMsg {
    Swap {
        min_output: Option<Uint128>,
        to: Option<String>,
    },
    RemoveLiquidity {
        min_a: Option<Uint128>,
        min_b: Option<Uint128>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(PairInfoResponse)]
    GetPairInfo {},
    #[returns(ReservesResponse)]
    GetReserves {},
    #[returns(FeeConfigResponse)]
    GetFeeConfig {},
    #[returns(HooksResponse)]
    GetHooks {},
    #[returns(SimulateSwapResponse)]
    SimulateSwap {
        offer_token: String,
        offer_amount: Uint128,
    },
}

#[cw_serde]
pub struct PairInfoResponse {
    pub pair: PairInfo,
}

#[cw_serde]
pub struct ReservesResponse {
    pub reserve_a: Uint128,
    pub reserve_b: Uint128,
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
pub struct SimulateSwapResponse {
    pub return_amount: Uint128,
    pub fee_amount: Uint128,
    pub spread_amount: Uint128,
}
