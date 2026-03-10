use cosmwasm_schema::cw_serde;
use cosmwasm_std::Addr;
use cw_storage_plus::{Item, Map};

/// Configuration for the LP burn hook.
///
/// On each swap, the hook burns LP tokens it holds for `target_pair`,
/// proportional to the swap output amount. Governance or the treasury
/// pre-funds the hook with LP tokens; the hook gradually burns them,
/// permanently locking the underlying reserves in the pool and
/// increasing the value of all remaining LP shares.
#[cw_serde]
pub struct LpBurnHookConfig {
    /// The pair whose LP tokens this hook burns.
    pub target_pair: Addr,
    /// The CW20 LP token contract for `target_pair`.
    pub lp_token: Addr,
    /// Fraction of each swap's output amount (in bps) to burn as LP tokens.
    pub percentage_bps: u16,
    pub admin: Addr,
}

pub const CONFIG: Item<LpBurnHookConfig> = Item::new("config");
pub const ALLOWED_PAIRS: Map<&str, bool> = Map::new("allowed_pairs");
