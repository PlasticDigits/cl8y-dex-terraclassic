use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Faucet is paused")]
    Paused {},

    #[error("Token is not on the faucet allowlist")]
    TokenNotAllowed {},

    #[error("Cooldown active: {seconds_remaining} seconds remaining")]
    CooldownActive { seconds_remaining: u64 },

    #[error("drip_amount must be greater than zero")]
    InvalidDripAmount {},

    #[error("cooldown_seconds must be greater than zero")]
    InvalidCooldown {},

    #[error("allowed_tokens must not be empty")]
    EmptyAllowlist {},
}
