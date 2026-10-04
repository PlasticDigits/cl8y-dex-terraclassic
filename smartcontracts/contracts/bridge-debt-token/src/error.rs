use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Native funds are not accepted")]
    UnexpectedFunds {},

    #[error("Amount must be greater than zero")]
    ZeroAmount {},

    #[error("Name is not in the expected format (3-50 bytes)")]
    InvalidName {},

    #[error("Symbol is not in the expected format [A-Za-z0-9-]{{3,32}}")]
    InvalidSymbol {},

    #[error("Decimals must not exceed 18")]
    InvalidDecimals {},

    #[error("Minter and debtor must be different addresses")]
    MinterIsDebtor {},

    #[error("Mint would exceed the debt cap")]
    CannotExceedCap {},

    #[error("Debt tokens stay with the debtor")]
    NotTransferable {},

    #[error("Insufficient debt balance")]
    InsufficientBalance {},
}
