use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use dex_common::hook::HookExecuteMsg;

#[cw_serde]
pub struct InstantiateMsg {
    pub target_pair: String,
    pub percentage_bps: u16,
    pub admin: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Hook(HookExecuteMsg),
    UpdateConfig {
        target_pair: Option<String>,
        percentage_bps: Option<u16>,
    },
    UpdateAllowedPairs {
        add: Vec<String>,
        remove: Vec<String>,
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
    pub target_pair: Addr,
    pub percentage_bps: u16,
    pub admin: Addr,
}
