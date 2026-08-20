//! Listing-time CW20 `code_id` pin + factory whitelist re-check (GitLab #582).
//!
//! Factory `CreatePair` already requires each asset's live `code_id` to be
//! whitelisted. After listing, Terra Classic `MsgMigrateContract` can move that
//! **instance** onto any stored wasm. Pair write paths therefore:
//!
//! 1. **Pin (B)** — live `ContractInfo.code_id` must equal the snapshot taken
//!    at instantiate (or last governance `RefreshAssetCodeIds`).
//! 2. **Whitelist (A)** — that live id must still be on factory
//!    `WHITELISTED_CODE_IDS` (O(1) `IsCodeIdWhitelisted`). Governance can
//!    `RemoveWhitelistedCodeId` to freeze every pair whose pin still matches
//!    a buggy template until tokens migrate and pins are refreshed.
//!
//! Factory query errors fail closed (same posture as [`crate::blacklist_guard`]).

use cosmwasm_std::{Addr, Deps, QuerierWrapper};
use dex_common::factory::{CodeIdWhitelistedResponse, QueryMsg as FactoryQueryMsg};
use dex_common::types::AssetInfo;

use crate::error::ContractError;
use crate::state::{PairInfoState, ASSET_CODE_IDS, PAIR_INFO};

fn token_addr(asset: &AssetInfo) -> Result<Addr, ContractError> {
    asset
        .assert_is_token()
        .map(Addr::unchecked)
        .map_err(|_| ContractError::NativeTokenNotSupported {})
}

fn live_code_id(querier: &QuerierWrapper, token: &Addr) -> Result<u64, ContractError> {
    querier
        .query_wasm_contract_info(token.to_string())
        .map(|info| info.code_id)
        .map_err(|_| ContractError::AssetCodeIdGuardUnavailable {})
}

fn probe_factory_whitelist(
    querier: &QuerierWrapper,
    factory: &Addr,
    code_id: u64,
) -> Result<bool, ContractError> {
    let resp: CodeIdWhitelistedResponse = querier
        .query_wasm_smart(
            factory.to_string(),
            &FactoryQueryMsg::IsCodeIdWhitelisted { code_id },
        )
        .map_err(|_| ContractError::AssetCodeIdGuardUnavailable {})?;
    Ok(resp.whitelisted)
}

/// Query live `code_id`s for both pair assets (order matches `asset_infos`).
pub fn snapshot_asset_code_ids(
    querier: &QuerierWrapper,
    pair_info: &PairInfoState,
) -> Result<[u64; 2], ContractError> {
    let token0 = token_addr(&pair_info.asset_infos[0])?;
    let token1 = token_addr(&pair_info.asset_infos[1])?;
    Ok([
        live_code_id(querier, &token0)?,
        live_code_id(querier, &token1)?,
    ])
}

/// Abort unless each asset's live `code_id` equals the pin **and** remains
/// factory-whitelisted.
pub fn assert_asset_code_ids(deps: Deps) -> Result<(), ContractError> {
    let pair_info = PAIR_INFO.load(deps.storage)?;
    let pinned = ASSET_CODE_IDS
        .may_load(deps.storage)?
        .ok_or(ContractError::AssetCodeIdUnpinned {})?;
    assert_pins_and_whitelist(&deps.querier, &pair_info, pinned)
}

fn assert_pins_and_whitelist(
    querier: &QuerierWrapper,
    pair_info: &PairInfoState,
    pinned: [u64; 2],
) -> Result<(), ContractError> {
    let tokens = [
        token_addr(&pair_info.asset_infos[0])?,
        token_addr(&pair_info.asset_infos[1])?,
    ];
    for (token, pinned_id) in tokens.iter().zip(pinned) {
        let live = live_code_id(querier, token)?;
        if live != pinned_id {
            return Err(ContractError::AssetCodeIdDrift {
                token: token.to_string(),
                pinned: pinned_id,
                live,
            });
        }
        if !probe_factory_whitelist(querier, &pair_info.factory, live)? {
            return Err(ContractError::AssetCodeIdNotWhitelisted {
                token: token.to_string(),
                code_id: live,
            });
        }
    }
    Ok(())
}

/// Governance refresh: pin live ids only when both are still whitelisted.
pub fn refresh_asset_code_ids(deps: Deps) -> Result<[u64; 2], ContractError> {
    let pair_info = PAIR_INFO.load(deps.storage)?;
    let live = snapshot_asset_code_ids(&deps.querier, &pair_info)?;
    for (asset, code_id) in pair_info.asset_infos.iter().zip(live) {
        let token = token_addr(asset)?;
        if !probe_factory_whitelist(&deps.querier, &pair_info.factory, code_id)? {
            return Err(ContractError::AssetCodeIdNotWhitelisted {
                token: token.to_string(),
                code_id,
            });
        }
    }
    Ok(live)
}
