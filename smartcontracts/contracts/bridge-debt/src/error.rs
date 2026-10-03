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

    #[error("Debt code id is not set")]
    DebtCodeUnset {},

    #[error("Debt token code id does not match")]
    UnexpectedDebtCode {},

    #[error("Market is already registered")]
    MarketExists {},

    #[error("Unknown market")]
    UnknownMarket {},

    #[error("Debt token is already registered to a market")]
    DebtTokenInUse {},

    #[error("Underlying and debt token must be different contracts")]
    InvalidMarket {},

    #[error("Underlying token does not look like a CW20")]
    InvalidUnderlying {},

    #[error("Debt token is missing debtor metadata")]
    NotDebtToken {},

    #[error("Underlying and debt token decimals differ")]
    DecimalsMismatch {},

    #[error("Debt token must start with zero supply")]
    DebtSupplyNotZero {},

    #[error("Debt token supply does not match outstanding borrows")]
    DebtSupplyDiverged {},

    #[error("Debt token minter must be this contract")]
    DebtMinterMismatch {},

    #[error("Debt token debtor does not match the registered debtor")]
    DebtorMismatch {},

    #[error("Debtor cannot be this contract or the debt token")]
    InvalidDebtor {},

    #[error("Debt token has additional minters")]
    AdditionalMinters {},

    #[error("Invalid CW20 hook message")]
    InvalidHook {},

    #[error("Repay amount exceeds outstanding debt")]
    RepayExceedsDebt {},

    #[error("Amount overflow")]
    Overflow {},
}
