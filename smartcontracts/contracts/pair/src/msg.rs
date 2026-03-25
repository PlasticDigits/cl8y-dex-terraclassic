pub use dex_common::oracle::{ObserveResponse, OracleInfoResponse};
pub use dex_common::pair::{
    Cw20HookMsg, ExecuteMsg, FeeConfigResponse, HooksResponse, LimitOrderResponse,
    PairInstantiateMsg as InstantiateMsg, PoolResponse, QueryMsg, ReverseSimulationResponse,
    SimulationResponse,
};

use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct MigrateMsg {}
