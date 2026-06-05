use cosmwasm_std::{DivideByZeroError, OverflowError, StdError};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{0}")]
    DivideByZero(#[from] DivideByZeroError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Invalid token")]
    InvalidToken {},

    #[error("Insufficient liquidity")]
    InsufficientLiquidity {},

    #[error("Slippage exceeded: expected at least {min_lp} LP tokens, would receive {actual_lp}")]
    SlippageExceeded { min_lp: String, actual_lp: String },

    #[error("Zero amount")]
    ZeroAmount {},

    #[error("Invalid fee: {reason}")]
    InvalidFee { reason: String },

    #[error("Max spread assertion: actual spread ({actual}) exceeds max allowed ({max})")]
    MaxSpreadAssertion { max: String, actual: String },

    #[error("Insufficient LP tokens: expected at least {min}, got {actual}")]
    InsufficientLpTokens { min: String, actual: String },

    #[error("Native tokens are not supported; use CW20 wrapped tokens instead")]
    NativeTokenNotSupported {},

    #[error("Asset mismatch: provided asset does not match pair assets")]
    AssetMismatch {},

    #[error("Oracle: {reason}")]
    Oracle { reason: String },

    #[error("Deadline exceeded: transaction expired at {deadline}, current time is {current}")]
    DeadlineExceeded { deadline: u64, current: u64 },

    #[error("Contract is paused")]
    Paused {},

    #[error(
        "Trading blacklist: wallet_blacklisted={wallet_blacklisted}, pair_blacklisted={pair_blacklisted}, tokens={blacklisted_tokens:?}, pairs={blacklisted_pairs:?}"
    )]
    Blacklisted {
        wallet_blacklisted: bool,
        pair_blacklisted: bool,
        blacklisted_tokens: Vec<String>,
        blacklisted_pairs: Vec<String>,
    },

    #[error("Invariant violation: {reason}")]
    InvariantViolation { reason: String },

    #[error("Nothing to sweep: contract balance equals internal reserves for {token}")]
    NothingToSweep { token: String },

    #[error("Withdraw slippage exceeded: asset {asset} returned {actual} but minimum is {min}")]
    WithdrawSlippageExceeded {
        asset: String,
        actual: String,
        min: String,
    },

    #[error("Limit order insert exceeded max adjust steps ({max})")]
    LimitInsertStepsExceeded { max: u32 },

    #[error("Hybrid swap split mismatch: pool_input + book_input must equal offer amount")]
    HybridSplitMismatch {},

    #[error("Invalid hybrid parameters")]
    InvalidHybridParams { reason: String },

    #[error("Limit order amount too small after maker fee")]
    LimitOrderMakerFeeExceedsAmount {},

    #[error("Limit batch exceeds max_batch_rungs ({max}); got {actual} orders")]
    LimitBatchTooLarge { max: u32, actual: u32 },

    #[error("Limit batch CW20 amount mismatch: expected {expected}, got {actual}")]
    LimitBatchAmountMismatch { expected: String, actual: String },

    #[error("Limit batch must contain at least one order")]
    LimitBatchEmpty {},

    #[error("Duplicate order id {order_id} in limit batch")]
    LimitBatchDuplicateOrderId { order_id: u64 },

    #[error("Limit batch placed no rungs ({skipped} skipped due to book-walk cap)")]
    LimitBatchNoRungsPlaced { skipped: u32 },

    #[error("Invalid limit ladder: {reason}")]
    LimitLadderInvalid { reason: String },

    #[error("Invalid limit batch config: {reason}")]
    LimitBatchConfigInvalid { reason: String },

    #[error("No claimable expired-limit refund for order id {order_id}")]
    NoExpiredLimitClaim { order_id: u64 },

    #[error("CleanLimitBook max_orders exceeds hard cap ({max}); got {actual}")]
    LimitCleanMaxOrdersExceeded { max: u32, actual: u32 },

    #[error(
        "Pair asset CW20 decimals must be ≤ {max}; got token0={decimals0}, token1={decimals1}"
    )]
    PairAssetDecimalsTooHigh {
        decimals0: u8,
        decimals1: u8,
        max: u8,
    },
}
