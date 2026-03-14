pub use dex_common::pair::{
    Cw20HookMsg, ExecuteMsg, FeeConfigResponse, HooksResponse, PairInstantiateMsg as InstantiateMsg,
    PoolResponse, QueryMsg, ReverseSimulationResponse, SimulationResponse,
};
pub use dex_common::oracle::{ObserveResponse, OracleInfoResponse};

use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct MigrateMsg {}
