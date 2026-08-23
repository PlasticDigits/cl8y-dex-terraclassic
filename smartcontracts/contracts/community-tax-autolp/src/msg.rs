use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

#[cw_serde]
pub struct InstantiateMsg {
    pub token: String,
    pub manager: String,
    pub router: Option<String>,
    pub pair: Option<String>,
    pub quote_token: Option<String>,
    pub threshold: Uint128,
    pub lp_recipient: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Permissionless. No UST1 invoice (T592-10).
    SkimToLp {},
    /// Manager: pair / threshold / LP recipient. Not payable here — token settings
    /// batch is the paid path; this is the sister-contract write after that batch.
    UpdateConfig {
        pair: Option<String>,
        router: Option<String>,
        quote_token: Option<String>,
        threshold: Option<Uint128>,
        lp_recipient: Option<String>,
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
    pub token: Addr,
    pub manager: Addr,
    pub router: Option<Addr>,
    pub pair: Option<Addr>,
    pub quote_token: Option<Addr>,
    pub threshold: Uint128,
    pub lp_recipient: Addr,
    pub skimming: bool,
}

#[cw_serde]
pub struct MigrateMsg {}
