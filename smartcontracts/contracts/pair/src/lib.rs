//! # CL8Y DEX Pair Contract
//!
//! Constant-product AMM (x × y = k) that manages a single trading pair of
//! CW20 tokens. Handles swaps, liquidity provision/withdrawal, fee
//! collection, and TWAP oracle observations.
//!
//! ## Key invariants
//!
//! - **k monotonicity:** After every swap, k' ≥ k. The pool never loses
//!   value from rounding — `ceil_div` ensures the pool retains any
//!   fractional remainder. (Applies to the **constant-product leg**; the
//!   limit book matches against escrow and does not change `k` directly.)
//! - **Token conservation:** Tokens enter and leave the pool only through
//!   explicit `ProvideLiquidity`, `WithdrawLiquidity`, or `Swap` paths.
//!   Direct CW20 transfers do not affect tracked reserves.
//! - **MINIMUM_LIQUIDITY:** The first 1000 LP tokens are permanently
//!   burned to prevent share-inflation attacks on the first depositor.
//! - **Bootstrap decimals gate (#124):** The first liquidity mint is rejected
//!   unless both reserve CW20 tokens have `decimals` ≤
//!   `dex_common::pair::MAX_PAIR_ASSET_DECIMALS_BOOTSTRAP`. LP share tokens use
//!   `dex_common::pair::LP_TOKEN_DECIMALS`.
//!
//! ## Limit book and escrow (FIFO hybrid)
//!
//! - **Escrow:** Bids lock **token1**; asks lock **token0**. Pending amounts
//!   live in `PENDING_ESCROW_TOKEN0` / `PENDING_ESCROW_TOKEN1` and are
//!   excluded from `RESERVES` and from sweepable “excess” (see `execute_sweep`).
//! - **Cancel:** Only the stored `owner` may cancel and receive the refund.
//! - **Bounded work:** Placement insert position is found by a linear walk
//!   from the book head capped by `max_adjust_steps` (see `orderbook`).
//!   Hybrid swaps cap distinct makers per tx via `max_maker_fills`.
//! - **Queries:** `Simulation` / `ReverseSimulation` are **pool-only** and do
//!   not walk the on-chain limit book. See repository docs (`limit-orders.md`,
//!   `contracts-security-audit.md` invariant L8).
//!
//! ## Auth model
//!
//! | Action            | Authorized caller        |
//! |-------------------|--------------------------|
//! | Swap              | Any CW20 holder (via Send) |
//! | ProvideLiquidity   | Anyone                   |
//! | WithdrawLiquidity  | LP token holder (via Send) |
//! | UpdateFee / Hooks / Pause / Sweep / DiscountRegistry | Factory only |

pub mod contract;
pub mod error;
pub mod msg;
pub mod orderbook;
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
