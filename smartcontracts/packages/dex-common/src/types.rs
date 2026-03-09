use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;

#[cw_serde]
pub struct PairInfo {
    pub token_a: Addr,
    pub token_b: Addr,
    pub pair_contract: Addr,
    pub lp_token: Addr,
}

#[cw_serde]
pub struct FeeConfig {
    pub fee_bps: u16,
    pub treasury: Addr,
}
