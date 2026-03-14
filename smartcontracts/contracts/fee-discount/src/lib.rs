//! # CL8Y DEX Fee Discount Registry
//!
//! Tiered fee discount system for CL8Y token holders. Traders who hold
//! sufficient CL8Y tokens can self-register for discount tiers that
//! reduce their swap fees across all pairs.
//!
//! ## Tier system
//!
//! Each tier specifies a minimum CL8Y balance and a discount in basis points.
//! Balances are checked on every swap via the `GetDiscount` query. If a
//! trader's balance drops below the threshold, the discount is revoked and
//! a fire-and-forget deregistration is triggered.
//!
//! ## Security
//!
//! - **EOA-only self-registration:** Smart contracts cannot self-register,
//!   preventing MEV bots from gaming the discount system.
//! - **Governance-only tiers:** Tier 0 (100% discount, market makers) and
//!   Tier 255 (blacklist) can only be assigned by governance.
//! - **Trusted routers:** Only routers registered as trusted can attribute
//!   fee discounts to the original trader address.

pub mod contract;
pub mod error;
pub mod msg;
pub mod state;

#[cfg(not(feature = "library"))]
pub mod entry {
    use cosmwasm_std::entry_point;
    use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Response, StdResult};

    use crate::error::ContractError;
    use crate::msg::{ExecuteMsg, InstantiateMsg, QueryMsg};

    #[entry_point]
    pub fn instantiate(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: InstantiateMsg,
    ) -> Result<Response, ContractError> {
        crate::contract::instantiate(deps, env, info, msg)
    }

    #[entry_point]
    pub fn execute(
        deps: DepsMut,
        env: Env,
        info: MessageInfo,
        msg: ExecuteMsg,
    ) -> Result<Response, ContractError> {
        crate::contract::execute(deps, env, info, msg)
    }

    #[entry_point]
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        crate::contract::query(deps, env, msg)
    }

    #[entry_point]
    pub fn migrate(
        deps: DepsMut,
        env: Env,
        msg: crate::msg::MigrateMsg,
    ) -> Result<Response, ContractError> {
        crate::contract::migrate(deps, env, msg)
    }
}
