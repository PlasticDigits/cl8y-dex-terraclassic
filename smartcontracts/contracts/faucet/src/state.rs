use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Timestamp, Uint128};
use cw_storage_plus::{Item, Map};

/// Soft-launch faucet invariants (GitLab #473):
/// - **F1** — drip only allowlisted mintable CW20 addresses
/// - **F2** — fixed drip size (default 100 tokens at 6 decimals = `100_000_000`)
/// - **F3** — global per-wallet cooldown across all tokens (default 300s)
/// - **F4** — QUARTZ/PEARL (cw20-base) must never appear on the allowlist
/// - **F6** — primary CW20 minter remains the deploy key; faucet is an additional minter
/// - **F7** — faucet code id is not added to the factory CW20 whitelist
#[cw_serde]
pub struct Config {
    /// Operator admin (pause / config / allowlist). Soft-launch: `cl8ydeploy`.
    pub admin: Addr,
    /// Fixed drip amount in base units (6 decimals → 100 human = 100_000_000).
    pub drip_amount: Uint128,
    /// Seconds between drips for a given wallet (global across tokens).
    pub cooldown_seconds: u64,
    pub paused: bool,
}

pub const CONFIG: Item<Config> = Item::new("config");

/// Allowlisted CW20 token addresses (exact `Addr` match; no symbol trust).
pub const ALLOWED_TOKENS: Map<&Addr, bool> = Map::new("allowed_tokens");

/// Last successful drip wall-clock time per claimant wallet.
pub const LAST_CLAIM: Map<&Addr, Timestamp> = Map::new("last_claim");
