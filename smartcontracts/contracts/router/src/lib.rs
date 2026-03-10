//! # CL8Y DEX Router
//!
//! Multi-hop swap router that chains swaps across multiple pairs in a
//! single transaction. Users send input tokens via CW20 Send; the router
//! executes each hop as a SubMsg, queries its own balance to determine
//! intermediate output amounts, and delivers the final output to the
//! recipient.
//!
//! ## Reentrancy guard
//!
//! A `SWAP_STATE` item prevents concurrent multi-hop swaps. It is set
//! at the start of `execute_swap_operations` and cleared when the final
//! hop completes (or on error).
//!
//! ## Fee discount attribution
//!
//! The router passes the original user's address via the `trader` field
//! in `Cw20HookMsg::Swap` so the pair can look up the correct fee
//! discount even though the CW20 Send originates from the router contract.

pub mod contract;
pub mod error;
pub mod msg;
pub mod state;

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
    pub fn query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
        crate::contract::query(deps, env, msg)
    }

    #[entry_point]
    pub fn reply(deps: DepsMut, env: Env, msg: Reply) -> Result<Response, ContractError> {
        crate::contract::reply(deps, env, msg)
    }

    #[entry_point]
    pub fn migrate(_deps: DepsMut, _env: Env, _msg: cosmwasm_std::Empty) -> StdResult<Response> {
        Ok(Response::new().add_attribute("action", "migrate"))
    }
}
