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

    #[error("Unauthorized hook caller: {sender} is not a registered pair")]
    UnauthorizedHookCaller { sender: String },

    #[error("Hook `pair` field must match caller: expected {expected}, got {actual}")]
    SpoofedPair { expected: String, actual: String },

    #[error("Pair LP token {pair_lp} does not match hook config {configured_lp}")]
    LpTokenMismatch {
        pair_lp: String,
        configured_lp: String,
    },

    #[error("Invalid BPS: {value} exceeds maximum of 10000")]
    InvalidBps { value: u16 },
}
