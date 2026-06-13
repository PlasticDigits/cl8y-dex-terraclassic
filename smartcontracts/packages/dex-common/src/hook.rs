use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{Addr, Uint128};

use crate::types::Asset;

/// Fee a post-swap hook deducts from the pair's ask-token output during settlement.
/// Tax and burn hooks implement [`HookFeeQueryMsg::OutputFee`]; LP-burn hooks do not.
#[cw_serde]
pub struct HookOutputFeeResponse {
    pub fee_token: String,
    pub fee_amount: Uint128,
    /// CW20 recipient of the fee transfer from the pair (tax recipient or hook for burn).
    pub fee_recipient: String,
}

/// Standard query for hooks that charge a percentage of swap output (invariant I-02).
#[cw_serde]
#[derive(QueryResponses)]
pub enum HookFeeQueryMsg {
    #[returns(HookOutputFeeResponse)]
    OutputFee {
        output_token: String,
        output_amount: Uint128,
    },
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
