use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use cw20::Cw20ReceiveMsg;

#[cw_serde]
pub struct InstantiateMsg {
    pub factory: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    SwapTokens {
        route: Vec<String>,
        min_output: Option<Uint128>,
        to: Option<String>,
    },
}

#[cw_serde]
pub enum Cw20HookMsg {
    SwapTokens {
        route: Vec<String>,
        min_output: Option<Uint128>,
        to: Option<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    GetConfig {},
    #[returns(SimulateRouteResponse)]
    SimulateRoute {
        route: Vec<String>,
        offer_amount: Uint128,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    pub factory: Addr,
}

#[cw_serde]
pub struct SimulateRouteResponse {
    pub return_amount: Uint128,
    pub fee_amounts: Vec<Uint128>,
}
