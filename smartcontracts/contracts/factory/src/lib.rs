//! # CL8Y DEX Factory
//!
//! Registry and governance hub for all CL8Y DEX pairs. The factory:
//!
//! - Instantiates new Pair contracts (with LP tokens) via `CreatePair`.
//! - Maintains a whitelist of allowed CW20 code IDs to prevent malicious
//!   token contracts from being used in pairs. After listing, pairs **re-check**
//!   live `code_id` against that whitelist and a per-pair pin (GitLab #582).
//! - Provides governance-gated admin operations: fee updates, pair treasury
//!   rotation, hook registration, discount registry configuration, pause, and sweep.
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
//! for LP **name** / wasm **label**. The pair keeps ASCII **alphanumeric**
//! prefixes (digits `0-9` included) and strips all other characters. New pairs
//! need `lp_token_code_id` on digit-allowing `cw20-mintable`. Governance sets
//! `pair_code_id` / `lp_token_code_id` via `UpdateConfig`. See invariant **F3**.
//!
//! ## Discount registry snapshot (GitLab #536)
//!
//! Factory `Config` stores a canonical `discount_registry`. `CreatePair` copies
//! it into pair instantiate so new listings are wired without a follow-up sweep.
//! `SetDiscountRegistryAll` / `SetDiscountRegistryBatch` write (or clear) that
//! pointer; single-pair `SetDiscountRegistry` does **not**. `UpdateConfig {
//! discount_registry }` sets the pointer without touching indexed pairs.
//! Missing field on migrate → `None`. Existing pairs are **not** fixed here
//! (GitLab #535). See invariant **F5**.
//!
//! ## Asset CW20 `code_id` pin (GitLab #582)
//!
//! `CreatePair` still gates listing on factory `WHITELISTED_CODE_IDS`. The pair
//! then snapshots each asset's live `code_id` at instantiate. Write paths
//! re-query `ContractInfo` and abort unless the live id **equals the pin**
//! **and** `IsCodeIdWhitelisted` is still true. Honest token upgrades freeze
//! those pairs until governance `RefreshPairAssetCodeIds` /
//! `RefreshPairAssetCodeIdsBatch`. Query errors fail closed. See **F6**.
//!
//! ## Community-tax autoregister (GitLab #633)
//!
//! After the pair is persisted in `reply`, the factory executes
//! `register_listed_pair` on each asset whose cw2 name is
//! `crates.io:cl8y-community-tax-token` (**R633-2**). Honest CW20s are not
//! called. Fail-closed if a tax-side register reverts (except the token's
//! idempotent `already`). Requires a factory migrate. See [`tax_register`].

pub mod contract;
pub mod error;
pub mod msg;
pub mod state;
pub mod tax_register;

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
