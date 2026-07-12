use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Timestamp, Uint128};

#[cw_serde]
pub struct InstantiateMsg {
    /// Faucet admin (pause / allowlist / config). Soft-launch: deploy key (`cl8ydeploy`).
    pub admin: String,
    /// Allowlisted mintable CW20 addresses only (F1/F4).
    pub allowed_tokens: Vec<String>,
    /// Fixed drip in base units. Soft-launch default: `100000000` (100 × 10^6).
    pub drip_amount: Uint128,
    /// Global per-wallet cooldown seconds. Soft-launch default: `300`.
    pub cooldown_seconds: u64,
}

#[cw_serde]
pub enum ExecuteMsg {
    /// Mint `drip_amount` of `token` to the sender. Recipient is always `info.sender`.
    Drip { token: String },
    /// Admin: block all drips.
    Pause {},
    /// Admin: resume drips (preserves cooldown map — F9 pause race note).
    Unpause {},
    /// Admin: replace the full allowlist (must be non-empty).
    UpdateAllowedTokens { tokens: Vec<String> },
    /// Admin: update drip size and/or cooldown (must stay > 0 when set).
    UpdateConfig {
        drip_amount: Option<Uint128>,
        cooldown_seconds: Option<u64>,
        admin: Option<String>,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(ConfigResponse)]
    Config {},
    #[returns(CooldownResponse)]
    Cooldown { address: String },
}

#[cw_serde]
pub struct ConfigResponse {
    pub admin: Addr,
    pub drip_amount: Uint128,
    pub cooldown_seconds: u64,
    pub paused: bool,
    pub allowed_tokens: Vec<Addr>,
}

#[cw_serde]
pub struct CooldownResponse {
    pub can_claim: bool,
    pub seconds_remaining: u64,
    pub last_claim_at: Option<Timestamp>,
    pub paused: bool,
}

#[cw_serde]
pub struct MigrateMsg {}
