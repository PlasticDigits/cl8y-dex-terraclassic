//! Live CW20 `token_info` decimals for the pair's two assets (GitLab #529).
//!
//! Pair state does not persist decimals. Placement / price-update validation queries both
//! CW20s so the human-scale limit-price band can use `10^(dec0 − dec1)`.

use cosmwasm_std::Deps;
use cw20::Cw20QueryMsg;
use dex_common::types::AssetInfo;

use crate::error::ContractError;

fn token_addr(info: &AssetInfo) -> &str {
    match info {
        AssetInfo::Token { contract_addr } => contract_addr.as_str(),
        AssetInfo::NativeToken { .. } => unreachable!("native tokens not supported"),
    }
}

/// Query both pair assets' CW20 `decimals`.
pub fn query_pair_asset_decimals(
    deps: Deps,
    asset_infos: &[AssetInfo; 2],
) -> Result<(u8, u8), ContractError> {
    let info0: cw20::TokenInfoResponse = deps.querier.query_wasm_smart(
        token_addr(&asset_infos[0]).to_string(),
        &Cw20QueryMsg::TokenInfo {},
    )?;
    let info1: cw20::TokenInfoResponse = deps.querier.query_wasm_smart(
        token_addr(&asset_infos[1]).to_string(),
        &Cw20QueryMsg::TokenInfo {},
    )?;
    Ok((info0.decimals, info1.decimals))
}
