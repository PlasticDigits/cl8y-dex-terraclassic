//! Community tax CW20 — DEX-safe buy / sell / transfer tax (GitLab #592).
//!
//! ## Invariants (T592)
//!
//! - **T592-1** — Inbound `Transfer`/`Send`/`TransferFrom`/`SendFrom` to pair, router,
//!   this contract, AutoLP, or other protocol-exempt addresses credits **exactly**
//!   `amount` (no inbound FoT). Pair/router wasm is unchanged (**H-01**).
//! - **T592-2** — Sell tax is **extra-debit** on `Send` + pair `Cw20HookMsg::Swap` only.
//!   Pair is credited `amount`; seller is debited `amount + tax`.
//! - **T592-3** — Buy tax is an **outbound split** when `from` is a registered listed
//!   pair. Pair is debited `amount`; trader + sinks = `amount`.
//! - **T592-4** — SKU unlock and settings batch each cost exactly **50 UST1**, forwarded
//!   to CMM treasury. Wrong token / wrong amount / no-op / unactivated SKU → revert,
//!   fee not kept. EnableFeature is never mixed into a settings batch.
//!   Official SKU unlock is manager → launcher → token (`origin.launcher` payer
//!   is authorized for EnableFeature only; **T606-1–T606-4**).
//! - **T592-5** — Manager-only config; wasm admin is **not** an execute path. Launcher
//!   stamps CMM governance as CosmWasm admin.
//! - **T592-6** — MintControl is instantiate-only; `RevokeMint` is one-way.
//! - **T592-7** — Classification: see `tax.rs`. Provide (`TransferFrom`) and limit
//!   `PlaceLimitOrder*` stay 1:1. Pair→EOA `Transfer` (swap receive / withdraw /
//!   limit refund) uses buy tax — same primitive, documented.
//! - **T592-8** — No reflection, rebase, pause, or blacklist manager APIs.
//! - **T592-9** — Protocol exemptions cannot be removed. `RegisterListedPair` requires
//!   factory `Pair` lookup.
//! - **T592-10** — AutoLP is never invoked from `Transfer`/`Send`/`AfterSwap`.
//! - **T592-11** — Sell to a listed pair bypasses `max_wallet`. `trading_enabled=false`
//!   blocks both buy and sell.
//! - **T592-12** — Invoice token is UST1 only (#595 routes off-chain).
//!
//! Pair/router swap math is **not** in this crate.

pub mod contract;
pub mod error;
pub mod identity;
pub mod invoice;
pub mod msg;
pub mod pair_registry;
pub mod state;
pub mod tax;

#[cfg(test)]
mod multitest;

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
