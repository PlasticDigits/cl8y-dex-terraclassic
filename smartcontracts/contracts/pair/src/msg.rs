pub use dex_common::oracle::{ObserveResponse, OracleInfoResponse};
pub use dex_common::pair::{
    Cw20HookMsg, DiscountRegistryResponse, ExecuteMsg, ExpiredLimitRefundResponse,
    FeeConfigResponse, HooksResponse, LimitOrderResponse, OrderStatus, OrderStatusResponse,
    PairInstantiateMsg as InstantiateMsg, PoolResponse, QueryMsg,
};

use cosmwasm_schema::cw_serde;

#[cw_serde]
pub struct MigrateMsg {}
