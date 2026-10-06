use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Binary, Uint128};
#[allow(unused_imports)]
use cw20::{BalanceResponse, MinterResponse, TokenInfoResponse};

#[cw_serde]
pub struct InstantiateMsg {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    /// Sole address allowed to mint and to burn the debtor's balance.
    pub minter: String,
    /// Receipts are minted only to this address and cannot be transferred.
    pub debtor: String,
    /// Hard cap on total supply. `None` means the minter decides each mint.
    pub cap: Option<Uint128>,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Rejected. The debtor's balance is the borrow record.
    Transfer { recipient: String, amount: Uint128 },
    /// Rejected. The debtor's balance is the borrow record.
    Send {
        contract: String,
        amount: Uint128,
        msg: Binary,
    },
    /// Rejected. Only the minter can reduce supply, via [`ExecuteMsg::BurnFrom`].
    Burn { amount: Uint128 },
    /// Minter: mint `amount` to the debtor. Any other recipient is rejected.
    Mint { recipient: String, amount: Uint128 },
    /// Minter: burn `amount` from the debtor. This is the repay path.
    BurnFrom { owner: String, amount: Uint128 },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(BalanceResponse)]
    Balance { address: String },
    #[returns(TokenInfoResponse)]
    TokenInfo {},
    #[returns(MinterResponse)]
    Minter {},
    #[returns(DebtorResponse)]
    Debtor {},
    /// Always empty. Present so an issuer can reject tokens that add extra minters.
    #[returns(MintersResponse)]
    Minters {},
}

#[cw_serde]
pub struct DebtorResponse {
    pub debtor: String,
}

#[cw_serde]
pub struct MintersResponse {
    pub minters: Vec<String>,
}

#[cw_serde]
/// Lossless code replacement from bridge-debt-token 1.0.0 or 1.1.0.
/// Identity and receipt accounting must already satisfy all invariants.
pub struct MigrateMsg {}
