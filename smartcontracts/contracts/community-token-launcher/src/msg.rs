use cl8y_community_tax_token::msg::{LaunchGuardsConfig, MintInit, Sink, Sku};
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};
use cw20::{Cw20Coin, Cw20ReceiveMsg};

#[cw_serde]
pub struct InstantiateMsg {
    pub token_code_id: u64,
    pub autolp_code_id: Option<u64>,
    pub ust1: String,
    pub cmm_treasury: String,
    pub cmm_governance: String,
    pub factory: String,
    pub router: Option<String>,
}

#[cw_serde]
pub enum ExecuteMsg {
    Receive(Cw20ReceiveMsg),
    /// 0-SKU create only. CW20 Send of 0 UST1 is rejected by UST1, so free create
    /// cannot use `Receive` (GitLab #593). Paid SKUs still require the invoice hook.
    CreateToken(Box<CreateTokenMsg>),
}

#[cw_serde]
pub struct CreateTokenMsg {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub initial_balances: Vec<Cw20Coin>,
    pub manager: String,
    pub treasury: String,
    pub buy_bps: u16,
    pub sell_bps: u16,
    pub max_buy_bps: u16,
    pub max_sell_bps: u16,
    pub max_transfer_bps: u16,
    pub features: Vec<Sku>,
    pub mint: Option<MintInit>,
    pub transfer_bps: Option<u16>,
    pub sinks: Option<Vec<Sink>>,
    pub launch_guards: Option<LaunchGuardsConfig>,
    pub autolp_threshold: Option<Uint128>,
    pub autolp_lp_recipient: Option<String>,
}

#[cw_serde]
pub enum InvoiceHookMsg {
    CreateToken(Box<CreateTokenMsg>),
    EnableFeature { token: String, sku: Sku },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    GetConfig {},
}

#[cw_serde]
pub struct ConfigResponse {
    pub token_code_id: u64,
    pub autolp_code_id: Option<u64>,
    pub ust1: Addr,
    pub cmm_treasury: Addr,
    pub cmm_governance: Addr,
    pub factory: Addr,
    pub router: Option<Addr>,
}

#[cw_serde]
pub struct MigrateMsg {}
