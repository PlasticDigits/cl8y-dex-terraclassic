use cosmwasm_schema::cw_serde;
use cosmwasm_std::{Addr, Uint128};

use crate::types::Asset;

#[cw_serde]
pub enum HookExecuteMsg {
    AfterSwap {
        pair: Addr,
        sender: Addr,
        offer_asset: Asset,
        return_asset: Asset,
        /// Total protocol commission in the ask asset: pool leg + book taker fees
        /// (treasury transfers). Matches [`HybridSimulationResponse::commission_amount`].
        /// Invariant **L7** — [GitLab #196](https://gitlab.com/PlasticDigits/cl8y-dex-terraclassic/-/issues/196).
        commission_amount: Uint128,
        /// Constant-product spread metric from the pool leg only.
        spread_amount: Uint128,
    },
}

/// Wrapper used by the pair contract when calling hooks.
/// Serializes identically to each hook's `ExecuteMsg::Hook(...)` variant.
#[cw_serde]
pub enum HookCallMsg {
    Hook(HookExecuteMsg),
}
