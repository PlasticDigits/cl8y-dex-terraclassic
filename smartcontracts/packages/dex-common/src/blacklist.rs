use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::Addr;

/// Combined blacklist probe for pair/router guards and dApp UX (GitLab #308).
#[cw_serde]
pub struct BlacklistCheck {
    pub wallet: Option<String>,
    pub tokens: Vec<String>,
    /// Single pair probe (legacy / pair contract guard).
    #[serde(default)]
    pub pair: Option<String>,
    /// Multihop router: any listed pair may trigger `blocked`.
    #[serde(default)]
    pub pairs: Vec<String>,
}

#[cw_serde]
pub struct BlacklistCheckResponse {
    pub blocked: bool,
    pub wallet_blacklisted: bool,
    pub blacklisted_tokens: Vec<Addr>,
    pub pair_blacklisted: bool,
    pub blacklisted_pairs: Vec<Addr>,
}

/// Paginated wallet blacklist for indexer / dashboards.
#[cw_serde]
#[derive(QueryResponses)]
pub enum BlacklistQueryMsg {
    #[returns(BlacklistCheckResponse)]
    BlacklistCheck(BlacklistCheck),
    #[returns(BlacklistedWalletsResponse)]
    BlacklistedWallets {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(BlacklistedTokensResponse)]
    BlacklistedTokens {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(BlacklistedPairsResponse)]
    BlacklistedPairs {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[cw_serde]
pub struct BlacklistedWalletsResponse {
    pub wallets: Vec<Addr>,
    pub next: Option<String>,
}

#[cw_serde]
pub struct BlacklistedTokensResponse {
    pub tokens: Vec<Addr>,
    pub next: Option<String>,
}

#[cw_serde]
pub struct BlacklistedPairsResponse {
    pub pairs: Vec<Addr>,
    pub next: Option<String>,
}
