//! Community tax CW20 — DEX-safe buy / sell / transfer tax (GitLab #592).
//!
//! ## Invariants (T592)
//!
//! - **T592-1** — Pair inbound credit equals its CW20 hook declared economic
//!   amount exactly. Authenticated routed buys credit net proceeds to the router.
//! - **T592-2** — Direct pair `Send`/`SendFrom` debit amount + sell tax; allowance
//!   covers the actual debit. Route-local v1 debits the official router's funded
//!   economic amount + tax, never the trader wallet. The router sizes the maximal
//!   economic amount within the explicitly authorized gross route budget.
//! - **T592-3** — Buy tax applies once at the active pair → official-router ask
//!   boundary. Authenticated route settlement/refunds avoid a second buy charge.
//!   Legacy pair → wallet and unrelated router → wallet retain their classification.
//! - **T592-4** — SKU unlock and settings batch each cost exactly **50 UST1**, forwarded
//!   to CMM treasury. Wrong token / wrong amount / no-op / unactivated SKU → revert,
//!   fee not kept. AutoLP no-op is sister `GetConfig` identity (not "key present").
//!   Identical `minter` is also a no-op. EnableFeature is never mixed into a settings batch.
//!   Official SKU unlock is manager → launcher → token (`origin.launcher` payer
//!   is authorized for EnableFeature only; **T606-1–T606-4**).
//! - **T592-5** — Manager-only economic config; live wasm admin or configured factory governance
//!   can rotate/revoke router trust (#1388). Launcher
//!   stamps CMM governance as CosmWasm admin.
//! - **T592-6** — MintControl is instantiate-only; `RevokeMint` is one-way.
//! - **T592-7** — Classification: see `tax.rs`. Provide (`TransferFrom`) and limit
//!   `PlaceLimitOrder*` stay 1:1. Pair→EOA `Transfer` (swap receive / withdraw /
//!   limit refund) uses buy tax — same primitive, documented. Official router hops
//!   tax the original trader (**T592-13** / #607 improved option 2). Manager-directory
//!   wallets (including hop `trader`) skip **buy, sell, and transfer** tax
//!   (**#609** / **E609-1**). `config.manager` is the same skip without a
//!   `MANAGER_EXEMPT` row (**#633** / **R633-1**). Launch guards still use the
//!   economic kind (**T592-11**). The manager is **not** protocol-exempt.
//! - **T592-13** — Route-local v1 binds pair, assets, gross/economic amount and
//!   trader to the configured router's active context. `Swap.trader` alone cannot
//!   attest funding. Missing context rejects; wallet extra-debit fallback is absent.
//!   See `docs/qa/issue-1381/README.md` and shared `dex-common::route_tax` protocol.
//! - **T592-14** — Governance router recovery uses live wasm-admin or configured factory
//!   governance authority and
//!   reviewed live code ID/checksum plus factory/funding/settlement capabilities.
//!   Candidates must be immutable or owned by configured factory governance.
//!   Settlement consumes (router, monotonic route ID, slot, token) once; a bounded
//!   token record preserves the latest route and at most five slots. Failed routes
//!   revert both consumption and balances. See `skills/AGENTS_ROUTER_TRUST_LIFECYCLE.md`.
//! - **T592-8** — No reflection, rebase, pause, or blacklist manager APIs.
//! - **T592-9** — Active protocol role exemptions cannot be removed; governance rotation removes an obsolete router exemption only when no other active role overlaps. `RegisterListedPair` requires
//!   factory `Pair` lookup. Manager directory is a full tax skip for those three
//!   kinds, not a transfer-only list, and does not grant listed-pair status.
//! - **T592-10** — AutoLP is never invoked from `Transfer`/`Send`/`AfterSwap`.
//! - **T592-11** — Sell to a listed pair bypasses `max_wallet`. `trading_enabled=false`
//!   blocks both buy and sell.
//! - **T592-12** — Invoice token is UST1 only (#595 routes off-chain).
//!
//! Pair/router swap math is **not** in this crate.

pub mod adopt;
pub mod autolp_recovery;
pub mod contract;
pub mod error;
pub mod identity;
pub mod invoice;
pub mod msg;
pub mod pair_registry;
pub mod router_trust;
pub mod state;
pub mod tax;

#[cfg(test)]
mod adopt_multitest;
#[cfg(test)]
mod multitest;
#[cfg(test)]
mod router_trust_multitest;

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
