use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),
    #[error("Unauthorized")]
    Unauthorized {},
    #[error("Pair already exists")]
    PairAlreadyExists {},
    #[error("Code ID not whitelisted")]
    CodeIdNotWhitelisted {},
    #[error("Pair not found in factory registry: {pair}")]
    PairNotInRegistry { pair: String },
    #[error("Invalid fee")]
    InvalidFee {},
    #[error("Invalid tokens")]
    InvalidTokens {},
    #[error("Invalid code ID")]
    InvalidCodeId {},
    #[error("Native tokens are not supported; use CW20 wrapped tokens instead")]
    NativeTokenNotSupported {},
}
