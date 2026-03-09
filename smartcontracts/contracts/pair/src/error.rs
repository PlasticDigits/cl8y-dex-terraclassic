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

    #[error("Slippage exceeded")]
    SlippageExceeded {},

    #[error("Zero amount")]
    ZeroAmount {},

    #[error("Invalid fee: {reason}")]
    InvalidFee { reason: String },

    #[error("Minimum output not met: expected at least {min}, got {actual}")]
    MinimumOutputNotMet { min: String, actual: String },

    #[error("Insufficient LP tokens: expected at least {min}, got {actual}")]
    InsufficientLpTokens { min: String, actual: String },
}
