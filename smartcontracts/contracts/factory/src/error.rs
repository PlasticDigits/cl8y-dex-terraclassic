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
    #[error("Only one CreatePair may run per block; retry next block")]
    OnePairCreationPerBlock {},
    #[error(
        "Pair asset CW20 decimals must be ≤ {max}; got token_a={decimals_a}, token_b={decimals_b}"
    )]
    PairAssetDecimalsTooHigh {
        decimals_a: u8,
        decimals_b: u8,
        max: u8,
    },
}
