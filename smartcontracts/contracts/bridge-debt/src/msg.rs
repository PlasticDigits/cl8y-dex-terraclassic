use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use cw20::Cw20ReceiveMsg;

#[cw_serde]
pub struct InstantiateMsg {
    pub owner: String,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Owner: wasm code id allowed for `XXX-Bridge-Debt` tokens.
    SetDebtCodeId {
        code_id: u64,
    },
    /// Owner: bind a bridged CW20 to its Bridge-Debt receipt.
    ///
    /// The debt token's debtor (the CMM, for USDT-Bridge-Debt) is read from
    /// the token. This contract must already be that token's sole minter.
    RegisterMarket {
        underlying: String,
        debt_token: String,
    },
    /// Owner: mint `amount` of the bridged token to `recipient` (the LP
    /// seeding address) and the same amount of Bridge-Debt to the debtor.
    Issue {
        underlying: String,
        recipient: String,
        amount: Uint128,
    },
    /// Bridged-token send hook. Burns the received tokens and the same amount
    /// of Bridge-Debt from the debtor.
    Receive(Cw20ReceiveMsg),
    /// Owner: return bridged tokens that arrived without a repay hook.
    /// Does not change outstanding debt.
    Recover {
        underlying: String,
        recipient: String,
        amount: Uint128,
    },
    UpdateOwner {
        owner: String,
    },
}

/// Payload wrapped in the bridged CW20 `Send` message.
#[cw_serde]
pub enum Cw20HookMsg {
    Repay {},
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(MarketResponse)]
    Market { underlying: String },
    #[returns(MarketsResponse)]
    Markets {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct ConfigResponse {
    pub owner: Addr,
    pub debt_code_id: Option<u64>,
}

#[cw_serde]
pub struct MarketResponse {
    pub underlying: Addr,
    pub debt_token: Addr,
    pub debtor: Addr,
    pub decimals: u8,
    pub outstanding: Uint128,
}

#[cw_serde]
pub struct MarketsResponse {
    pub markets: Vec<MarketResponse>,
}

#[cw_serde]
pub struct MigrateMsg {}
