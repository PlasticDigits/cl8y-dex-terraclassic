use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Tier {tier_id} not found")]
    TierNotFound { tier_id: u8 },

    #[error("Tier {tier_id} already exists")]
    TierAlreadyExists { tier_id: u8 },

    #[error("Tier {tier_id} is governance-only; self-registration is not allowed")]
    GovernanceOnlyTier { tier_id: u8 },

    #[error("Wallet is currently on a governance tier; only governance can change it")]
    LockedToGovernanceTier {},

    #[error("Smart contracts cannot self-register; only EOA wallets are allowed")]
    ContractNotAllowed {},

    #[error("Insufficient CL8Y balance: required {required}, got {actual}")]
    InsufficientBalance { required: String, actual: String },

    #[error("Invalid discount_bps: {value} exceeds maximum of 10000")]
    InvalidDiscountBps { value: u16 },

    #[error("Tier {tier_id} still has registered wallets; remove them first or use force")]
    TierHasRegistrations { tier_id: u8 },

    #[error("Wallet is not registered")]
    NotRegistered {},
}
