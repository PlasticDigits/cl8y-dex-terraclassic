//! Community token launcher — stamps CMM wasm admin and collects SKU invoices (#592).
//!
//! **T592-5:** `WasmMsg::Instantiate { admin: cmm_governance }`. Manager cannot
//! migrate. **T592-12:** paid SKUs accept UST1 `Send` only (#595 routes off-chain).
//! **O601-3:** zero-SKU free profile uses `ExecuteMsg::CreateToken` (CW20 cannot
//! `Send` 0). Canonical columbus-5 launcher is `terra126pr5…ahzwze`
//! (code **11620**, wasm admin DEX 2-of-3). **11612** is unused.
//! **UpdateConfig** (wasm admin) rotates `token_code_id` / `autolp_code_id`.
//!
//! **T606-1 / T606-5:** official Enable Feature is UST1 `Send` here
//! (`{enable_feature:{token,sku}}`); create rejects duplicate SKUs. Playbook:
//! [`skills/AGENTS_COMMUNITY_TAX_ENABLE_FEATURE.md`](../../../../skills/AGENTS_COMMUNITY_TAX_ENABLE_FEATURE.md).

pub mod contract;
pub mod error;
pub mod msg;
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
