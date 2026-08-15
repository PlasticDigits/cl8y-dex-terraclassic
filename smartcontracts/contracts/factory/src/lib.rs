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
//! - Maintains `PAIR_KEY_INDEX` for **O(1)** `Pairs` query cursor resolve (GitLab #258).
//! - Maintains `PAIR_ADDR_REGISTERED` (`state.rs`) so pair-address membership
//!   checks for governance messages are **O(1)** in storage reads (see GitLab #122).
//!
//! ## Auth model
//!
//! All admin operations require `info.sender == config.governance`.
//! `CreatePair` is permissionless but validates both tokens against the
//! code ID whitelist.
//!
//! ## `CreatePair` concurrency (Terra / CosmWasm)
//!
//! Each Cosmos transaction runs **atomically**: the factory `execute` entry,
//! any submessages (pair `WasmMsg::Instantiate`), and their `reply` handlers
//! for that transaction finish before another transaction touching the same
//! contract runs. So two different `CreatePair` transactions do **not**
//! interleave mid-flight—the hypothetical “overwrite `PENDING_PAIR` before the
//! first reply” race from a second tx in the same block does **not** apply on
//! standard Cosmos blockchains.
//!
//! The factory still enforces **at most one `CreatePair` instantiate flow per
//! block height** via [`PAIR_CREATION_BLOCK`](crate::state::PAIR_CREATION_BLOCK).
//! That is a deliberate rate limit and documents the “single pending slot”
//! invariant for reviewers and third-party agents. Integrators that batch more
//! than one new pair in the same block must advance the block (or split across
//! heights).
//!
//! ## LP ticker (GitLab #518)
//!
//! Factory passes truncated/uppercased asset symbols into pair `token_symbols`
//! for LP **name** / wasm **label**. The pair sanitizes the LP CW20 `symbol` via
//! [`dex_common::lp_symbol`] so classic `[a-zA-Z\-]{3,12}` LP code (columbus-5)
//! accepts UST1 / CL8Y pairs. See invariant **F3**.

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
