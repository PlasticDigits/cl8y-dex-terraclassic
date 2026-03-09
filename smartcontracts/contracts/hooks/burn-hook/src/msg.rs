use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use dex_common::hook::HookExecuteMsg;

#[cw_serde]
pub struct InstantiateMsg {
    pub burn_token: String,
    pub burn_percentage_bps: u16,
    pub admin: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Hook(HookExecuteMsg),
    UpdateConfig {
        burn_token: Option<String>,
        burn_percentage_bps: Option<u16>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    GetConfig {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub burn_token: Addr,
    pub burn_percentage_bps: u16,
    pub admin: Addr,
}
