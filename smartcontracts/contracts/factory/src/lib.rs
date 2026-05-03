//! # CL8Y DEX Factory
//!
//! Registry and governance hub for all CL8Y DEX pairs. The factory:
//!
//! - Instantiates new Pair contracts (with LP tokens) via `CreatePair`.
//! - Maintains a whitelist of allowed CW20 code IDs to prevent malicious
//!   token contracts from being used in pairs.
//! - Provides governance-gated admin operations: fee updates, hook
//!   registration, discount registry configuration, pause, and sweep.
//! - Stores a sequential pair index for paginated enumeration (discovery).
//! - Maintains `PAIR_ADDR_REGISTERED` (`state.rs`) so pair-address membership
//!   checks for governance messages are **O(1)** in storage reads (see GitLab #122).
//!
//! ## Auth model
//!
//! All admin operations require `info.sender == config.governance`.
//! `CreatePair` is permissionless but validates both tokens against the
//! code ID whitelist.

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
    pub fn migrate(
        deps: DepsMut,
        env: Env,
        msg: crate::msg::MigrateMsg,
    ) -> Result<Response, ContractError> {
        crate::contract::migrate(deps, env, msg)
    }
}
