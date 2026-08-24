use cosmwasm_std::StdError;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ContractError {
    #[error("{0}")]
    Std(#[from] StdError),

    #[error("Invoice must be paid in the configured UST1 token")]
    InvoiceToken {},

    #[error("Invoice amount must be exactly {required} UST1, got {got}")]
    InvoiceAmount { required: String, got: String },

    #[error("MintControl can only be purchased at instantiate")]
    MintControlInstantiateOnly {},

    #[error("AutoLP code id is not configured")]
    AutolpCodeNotSet {},

    /// Paid SKUs must go through UST1 `Send` so the invoice is collected.
    #[error("CreateToken execute is free-profile only (empty features); paid SKUs use UST1 Send")]
    FreeProfileOnly {},

    #[error("{0}")]
    Token(#[from] cl8y_community_tax_token::error::ContractError),

    #[error("Unauthorized")]
    Unauthorized {},

    #[error("token_code_id {code_id} is not factory-whitelisted")]
    TokenCodeNotWhitelisted { code_id: u64 },

    #[error("code_id must be non-zero")]
    InvalidCodeId {},

    #[error("UpdateConfig requires token_code_id and/or autolp_code_id")]
    NothingToUpdate {},

    #[error("Duplicate SKU {sku} in create invoice")]
    DuplicateSku { sku: String },
}
