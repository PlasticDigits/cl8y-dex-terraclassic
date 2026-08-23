use cosmwasm_std::{OverflowError, StdError};
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("{0}")]
    Overflow(#[from] OverflowError),

    #[error("{0}")]
    Cw20(#[from] cw20_base::ContractError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("Fee required: exact {required} UST1")]
    FeeRequired { required: String },

    #[error("Invoice amount must be exactly {required} UST1, got {got}")]
    InvoiceAmount { required: String, got: String },

    #[error("Invoice must be paid in the configured UST1 token")]
    InvoiceToken {},

    #[error("Settings batch is empty or identical to on-chain state")]
    NoOpSettings {},

    #[error("SKU {sku} is not unlocked")]
    SkuNotUnlocked { sku: String },

    #[error("SKU {sku} is already enabled")]
    FeatureAlreadyEnabled { sku: String },

    #[error("MintControl can only be purchased at instantiate")]
    MintControlInstantiateOnly {},

    #[error("Mint has been revoked")]
    MintRevoked {},

    #[error("Mint is not enabled on this token")]
    MintDisabled {},

    #[error("Tax bps {bps} exceeds cap {cap}")]
    TaxBpsCap { bps: u16, cap: u16 },

    #[error("Combined max tax bps {combined} exceeds {cap}")]
    CombinedTaxCap { combined: u16, cap: u16 },

    #[error("Sink ratios must sum to 10000, got {sum}")]
    SinkRatio { sum: u32 },

    #[error("Too many sinks (max {max})")]
    TooManySinks { max: usize },

    #[error("Wallet sink requires addr")]
    WalletSinkAddr {},

    #[error("Pair is not factory-listed or does not hold this token")]
    PairNotListed {},

    #[error("Cannot remove a protocol exemption")]
    CannotRemoveProtocolExempt {},

    #[error("Insufficient balance for extra-debit sell tax")]
    InsufficientForSellTax {},

    #[error("Trading is not enabled")]
    TradingDisabled {},

    #[error("Max wallet exceeded")]
    MaxWallet {},

    #[error("Transfer cooldown active")]
    Cooldown {},

    #[error("Decimals must be <= {max}")]
    DecimalsCap { max: u8 },

    #[error("Do not mix EnableFeature into a settings batch")]
    MixedInvoice {},
}
