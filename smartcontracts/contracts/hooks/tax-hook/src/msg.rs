use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;
use dex_common::hook::HookExecuteMsg;

#[cw_serde]
pub struct InstantiateMsg {
    pub recipient: String,
    pub tax_percentage_bps: u16,
    pub tax_token: String,
    pub admin: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    Hook(HookExecuteMsg),
    UpdateConfig {
        recipient: Option<String>,
        tax_percentage_bps: Option<u16>,
        tax_token: Option<String>,
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
    pub recipient: Addr,
    pub tax_percentage_bps: u16,
    pub tax_token: Addr,
    pub admin: Addr,
}
