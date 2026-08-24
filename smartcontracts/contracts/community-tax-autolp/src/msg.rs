use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Decimal, Uint128};

#[cw_serde]
pub struct InstantiateMsg {
    pub token: String,
    pub manager: String,
    /// Immutable CL8Y factory used to verify `pair` (GitLab #610). Launcher stamps this.
    pub factory: String,
    pub router: Option<String>,
    pub pair: Option<String>,
    pub quote_token: Option<String>,
    pub threshold: Uint128,
    pub lp_recipient: String,
    /// Optional skim `max_spread`. Omit → 100 bps. Hard cap 200 bps.
    pub skim_max_spread: Option<Decimal>,
    /// Optional absolute skim `min_return` (quote raw). Omit → `max_spread` only.
    pub skim_min_return: Option<Uint128>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Permissionless. No UST1 invoice (T592-10). Floor is config, not caller args.
    SkimToLp {},
    /// Manager: pair / threshold / LP recipient / skim floor. Omitted fields merge.
    /// Not payable here — token settings batch is the paid path; this is the
    /// sister-contract write after that batch.
    UpdateConfig {
        pair: Option<String>,
        router: Option<String>,
        quote_token: Option<String>,
        threshold: Option<Uint128>,
        lp_recipient: Option<String>,
        skim_max_spread: Option<Decimal>,
        skim_min_return: Option<Uint128>,
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
    pub factory: Addr,
    pub router: Option<Addr>,
    pub pair: Option<Addr>,
    pub quote_token: Option<Addr>,
    pub threshold: Uint128,
    pub lp_recipient: Addr,
    pub skim_max_spread: Decimal,
    pub skim_min_return: Option<Uint128>,
    pub skimming: bool,
}

#[cw_serde]
pub struct MigrateMsg {
    /// One-time factory pin when migrating pre-#610 wasm that had no factory field.
    pub factory: Option<String>,
}
