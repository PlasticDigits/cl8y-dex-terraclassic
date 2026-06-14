use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

use crate::types::Asset;

/// Standard query for pair swap settlement: how much of the ask-token output
/// this hook deducts from the trader's `return_amount` before `AfterSwap`.
#[cw_serde]
#[derive(QueryResponses)]
pub enum HookQueryMsg {
    #[returns(ComputeSwapFeeResponse)]
    ComputeSwapFee {
        output_token: String,
        output_amount: Uint128,
    },
}

#[cw_serde]
pub struct ComputeSwapFeeResponse {
    /// Fee denominated in `output_token` raw units (0 when this hook skips).
    pub fee_amount: Uint128,
    /// When set, the pair transfers `fee_amount` of the ask token here during
    /// swap settlement (before `AfterSwap`). Tax hooks use the recipient;
    /// burn hooks use the hook contract address.
    pub settlement_recipient: Option<String>,
}

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
