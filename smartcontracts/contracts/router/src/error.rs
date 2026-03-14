use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Operations must not be empty")]
    EmptyOperations {},

    #[error("Native token swaps are not supported; use CW20 wrapped tokens instead")]
    NativeSwapNotSupported {},

    #[error("Native tokens are not supported in swap operations; use CW20 wrapped tokens")]
    NativeTokenNotSupported {},

    #[error("Pair not found for the given swap operation")]
    PairNotFound {},

    #[error("Minimum receive assertion: expected at least {minimum}, got {actual}")]
    MinimumReceiveAssertion { minimum: String, actual: String },

    #[error("Deadline exceeded: transaction expired at {deadline}, current time is {current}")]
    DeadlineExceeded { deadline: u64, current: u64 },

    #[error("Swap in progress — cannot start another")]
    SwapInProgress {},

    #[error("Too many hops: {actual} exceeds maximum of {max}")]
    TooManyHops { max: usize, actual: usize },
}
