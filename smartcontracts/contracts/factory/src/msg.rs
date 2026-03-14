pub use dex_common::factory::{
    CodeIdsResponse, ConfigResponse, ExecuteMsg, InstantiateMsg, PairCountResponse, PairResponse,
    PairsResponse, QueryMsg,
};

use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct MigrateMsg {}
