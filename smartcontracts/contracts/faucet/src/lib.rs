//! # CL8Y DEX Soft-Launch Faucet
//!
//! Rate-limited CW20 drip for **non-economic** soft-launch mintable tokens (GitLab #473).
//!
//! ## Invariants (F1–F12)
//!
//! - **F1** — Only allowlisted token addresses may be dripped.
//! - **F2** — Fixed drip amount (no user-chosen amount); soft-launch default `100_000_000`.
//! - **F3** — Global per-wallet cooldown across all tokens (default 300s).
//! - **F4** — QUARTZ/PEARL (cw20-base) must not be allowlisted.
//! - **F5** — Faucet must be granted `AddMinter` on each allowlisted CW20 before drips work.
//! - **F6** — Deploy key remains primary CW20 minter; faucet is an additional minter only.
//! - **F7** — Faucet is **not** added to the factory CW20 whitelist.
//! - **F9** — Admin can `Pause` / `Unpause`; pause does not clear the cooldown map.
//! - **F12** — Caller pays gas; contract holds no native funds.
//!
//! Recipient is always `info.sender` (no drip-to-other).

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
