use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Route must not be empty")]
    EmptyRoute {},

    #[error("Invalid route: {reason}")]
    InvalidRoute { reason: String },

    #[error("Pair not found for the given route segment")]
    PairNotFound {},

    #[error("Multi-hop routing is not yet supported; provide a single pair address")]
    MultiHopNotSupported {},
}
