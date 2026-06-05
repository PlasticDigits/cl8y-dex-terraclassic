use cosmwasm_std::{Addr, Deps, QuerierWrapper};
use dex_common::blacklist::BlacklistCheck;
use dex_common::factory::QueryMsg as FactoryQueryMsg;
use dex_common::types::AssetInfo;

use crate::error::ContractError;
use crate::state::PairInfoState;

fn token_addr(asset: &AssetInfo) -> Result<Addr, ContractError> {
    asset
        .assert_is_token()
        .map(|s| Addr::unchecked(s))
        .map_err(|_| ContractError::NativeTokenNotSupported {})
}

/// Reject user-facing actions when the factory trading blacklist applies (GitLab #308).
pub fn assert_trade_not_blacklisted(
    querier: &QuerierWrapper,
    pair_info: &PairInfoState,
    pair_contract: &Addr,
    wallets: &[&Addr],
    extra_tokens: &[Addr],
) -> Result<(), ContractError> {
    let token0 = token_addr(&pair_info.asset_infos[0])?;
    let token1 = token_addr(&pair_info.asset_infos[1])?;

    let mut tokens = vec![token0.to_string(), token1.to_string()];
    for t in extra_tokens {
        let s = t.to_string();
        if !tokens.contains(&s) {
            tokens.push(s);
        }
    }

    for wallet in wallets {
        let resp: dex_common::blacklist::BlacklistCheckResponse = querier
            .query_wasm_smart(
                pair_info.factory.to_string(),
                &FactoryQueryMsg::BlacklistCheck(BlacklistCheck {
                    wallet: Some(wallet.to_string()),
                    tokens: tokens.clone(),
                    pair: Some(pair_contract.to_string()),
                    pairs: vec![],
                }),
            )
            .map_err(|e| ContractError::Std(cosmwasm_std::StdError::generic_err(e.to_string())))?;

        if resp.blocked {
            return Err(ContractError::Blacklisted {
                wallet_blacklisted: resp.wallet_blacklisted,
                pair_blacklisted: resp.pair_blacklisted,
                blacklisted_tokens: resp
                    .blacklisted_tokens
                    .iter()
                    .map(|a| a.to_string())
                    .collect(),
                blacklisted_pairs: resp
                    .blacklisted_pairs
                    .iter()
                    .map(|a| a.to_string())
                    .collect(),
            });
        }
    }

    Ok(())
}

pub fn assert_trade_not_blacklisted_deps(
    deps: Deps,
    pair_info: &PairInfoState,
    pair_contract: &Addr,
    wallets: &[&Addr],
    extra_tokens: &[Addr],
) -> Result<(), ContractError> {
    assert_trade_not_blacklisted(
        &deps.querier,
        pair_info,
        pair_contract,
        wallets,
        extra_tokens,
    )
}
