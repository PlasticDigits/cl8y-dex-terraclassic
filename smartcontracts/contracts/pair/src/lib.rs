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
//! - **LP ticker alphanumeric sanitize (#518):** the LP CW20 `symbol` is derived
//!   via [`dex_common::lp_symbol`] — keep `[0-9A-Za-z]`, drop all other prefix
//!   chars, join `{a}-{b}-LP` (`[a-zA-Z0-9\-]{3,12}`). Requires factory
//!   `lp_token_code_id` to be digit-allowing `cw20-mintable` (classic Terraswap
//!   LP code rejects digits). Name/label keep unsanitized symbols. See **F3**.
//! - **Discount registry snapshot (#536):** `CreatePair` copies factory
//!   `config.discount_registry` into instantiate. `GetDiscountRegistry` returns
//!   the stored `Option<Addr>` (`None` = unwired, full pair fee). Existing pairs
//!   are not retroactively wired — that is GitLab #535. See **F5**.
//! - **Asset CW20 `code_id` pin (#582):** instantiate snapshots each asset's
//!   live `ContractInfo.code_id`. Swap / provide / withdraw / limit place+fill
//!   (and cancel/claim CW20 refunds) abort unless live id **equals the pin**
//!   **and** remains factory-whitelisted. Honest token upgrades freeze the pair
//!   until factory `RefreshPairAssetCodeIds`. Do **not** add FoT balance-delta
//!   math. See **F6**.
//!
//! ## Limit book and escrow (FIFO hybrid)
//!
//! - **Escrow:** Bids lock **token1**; asks lock **token0**. Pending amounts
//!   live in `PENDING_ESCROW_TOKEN0` / `PENDING_ESCROW_TOKEN1` and are
//!   excluded from `RESERVES` and from sweepable “excess” (see `execute_sweep`).
//!   Orders parked when `expires_at` is reached during a match walk are tracked in
//!   `EXPIRED_LIMIT_CLAIMS` until `ClaimExpiredLimitOrder`; until then their `remaining`
//!   stays in pending escrow (see `docs/limit-orders.md`, GitLab #120).
//! - **Cancel / claim:** Only the stored `owner` may cancel an **active** book row or claim a parked
//!   **`EXPIRED_LIMIT_CLAIMS`** row; cancel does not apply after a park, preventing double CW20 return.
//!   **`ClaimExpiredLimitOrder`** is pause-gated like cancel (GitLab #120).
//! - **Blacklisted maker (GitLab #468):** hybrid match walks never fill resting orders whose `owner`
//!   is on the factory trading blacklist; eligible rows park into `EXPIRED_LIMIT_CLAIMS` (same escrow
//!   path as expiry) so makers can claim after `UnblacklistWallet`.
//! - **Bounded work:** Placement insert position is found by a linear walk
//!   from the book head capped by `max_adjust_steps` (see `orderbook`).
//!   Hybrid swaps cap distinct makers per tx via `max_maker_fills`.
//! - **Queries:** `HybridSimulation` / `HybridReverseSimulation` are the only
//!   swap quote paths. Use [`dex_common::pair::pool_only_hybrid_params`] for
//!   pool-only quotes (`book_input = 0`). Optional `trader` / `sender` apply
//!   CL8Y fee-tier discounts (same math as execute; read-only — GitLab #238).
//!   Execute/limit paths cache registry lookups for [`dex_common::pair::DISCOUNT_CACHE_TTL_SECONDS`]
//!   (GitLab #251); simulation reads cache but does not write.
//!   See `docs/limit-orders.md` and invariant L8 in `docs/contracts-security-audit.md`.
//!   Reverse quotes seed from pool CP math and cap full sims at 32 per query (GitLab #257).
//!
//! ## Auth model
//!
//! | Action            | Authorized caller        |
//! |-------------------|--------------------------|
//! | Swap              | Any CW20 holder (via Send) |
//! | ProvideLiquidity   | Anyone                   |
//! | WithdrawLiquidity  | LP token holder (via Send) |
//! | UpdateFee / UpdateTreasury / Hooks / Pause / Sweep / DiscountRegistry / RefreshAssetCodeIds | Factory only |

pub mod asset_code_id_guard;
pub mod asset_decimals;
pub mod blacklist_guard;
pub mod contract;
pub mod discount_cache;
pub mod error;
pub mod greedy;
pub mod hybrid_reverse;
pub mod limit_batch_withdraw;
pub mod limit_book_clean;
pub mod limit_placement;
pub mod msg;
pub mod orderbook;
pub mod state;
pub mod tx_swap_index;

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
