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

    #[error("Invalid BPS: {value} exceeds maximum of 10000")]
    InvalidBps { value: u16 },

    #[error("AfterSwap pair address {pair} does not match caller {sender}")]
    PairSenderMismatch { pair: String, sender: String },

    #[error("Caller {sender} is not a pair contract")]
    InvalidPairContract { sender: String },

    #[error(
        "Pair {pair} liquidity token {pair_lp} does not match configured lp_token {config_lp}"
    )]
    PairLpTokenMismatch {
        pair: String,
        pair_lp: String,
        config_lp: String,
    },
}
