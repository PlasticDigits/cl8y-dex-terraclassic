//! Auto V2 LP sister for community tax tokens (GitLab #592 / #610).
//!
//! **T592-10:** `SkimToLp` is permissionless and is **never** called from token
//! `Transfer` / `Send` / pair `AfterSwap`. Tax accumulates 1:1 on this contract
//! (protocol-exempt). A later keeper tx swaps half and provides liquidity.
//!
//! **M610-1–M610-8** ([#610](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/610)):
//! `pair` must be a CL8Y factory-listed pool that includes this tax token.
//! Factory is immutable (launcher pin). Skim always sets a spread floor
//! (`max_spread` default 100 bps, cap 200 bps; optional `min_return`).
//! Binding `pair` also executes `register_listed_pair` on the tax token
//! (**#633** / **R633-3**). Re-bind of the same pair is idempotent.

pub mod contract;
pub mod error;
pub mod msg;
pub mod pair;
pub mod spread;
pub mod state;

#[cfg(test)]
mod tests;

#[cfg(not(feature = "library"))]
pub mod entry {
    use cosmwasm_std::entry_point;
    use cosmwasm_std::{Binary, Deps, DepsMut, Env, MessageInfo, Reply, Response, StdResult};

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
    pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
        crate::contract::reply(deps, env, msg)
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
