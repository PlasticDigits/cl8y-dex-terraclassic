use cosmwasm_std::{Addr, Deps, QuerierWrapper};
use dex_common::blacklist::BlacklistCheck;
use dex_common::factory::QueryMsg as FactoryQueryMsg;
use dex_common::types::AssetInfo;

use crate::error::ContractError;
use crate::state::PairInfoState;

fn token_addr(asset: &AssetInfo) -> Result<Addr, ContractError> {
    asset
        .assert_is_token()
        .map(Addr::unchecked)
        .map_err(|_| ContractError::NativeTokenNotSupported {})
}

fn probe_factory_blacklist(
    querier: &QuerierWrapper,
    factory: &Addr,
    check: BlacklistCheck,
) -> Result<Option<dex_common::blacklist::BlacklistCheckResponse>, ContractError> {
    match querier.query_wasm_smart(factory.to_string(), &FactoryQueryMsg::BlacklistCheck(check)) {
        Ok(resp) => Ok(Some(resp)),
        // GitLab #456 (SEC-I03 F02): default deny — any factory BlacklistCheck query error
        // blocks the trade instead of silently disabling blacklist enforcement.
        Err(_) => Err(ContractError::BlacklistGuardUnavailable {}),
    }
}

fn blacklist_err(resp: dex_common::blacklist::BlacklistCheckResponse) -> Result<(), ContractError> {
    Err(ContractError::Blacklisted {
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
    })
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

    let base = BlacklistCheck {
        wallet: None,
        tokens: tokens.clone(),
        pair: Some(pair_contract.to_string()),
        pairs: vec![],
    };

    if wallets.is_empty() {
        if let Some(resp) = probe_factory_blacklist(querier, &pair_info.factory, base)? {
            if resp.blocked {
                return blacklist_err(resp);
            }
        }
        return Ok(());
    }

    for wallet in wallets {
        let check = BlacklistCheck {
            wallet: Some(wallet.to_string()),
            ..base.clone()
        };
        if let Some(resp) = probe_factory_blacklist(querier, &pair_info.factory, check)? {
            if resp.blocked {
                return blacklist_err(resp);
            }
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

/// Factory trading blacklist probe for a single wallet (GitLab #468).
pub fn wallet_is_trade_blacklisted(
    querier: &QuerierWrapper,
    pair_info: &PairInfoState,
    pair_contract: &Addr,
    wallet: &Addr,
    extra_tokens: &[Addr],
) -> Result<bool, ContractError> {
    let token0 = token_addr(&pair_info.asset_infos[0])?;
    let token1 = token_addr(&pair_info.asset_infos[1])?;

    let mut tokens = vec![token0.to_string(), token1.to_string()];
    for t in extra_tokens {
        let s = t.to_string();
        if !tokens.contains(&s) {
            tokens.push(s);
        }
    }

    let check = BlacklistCheck {
        wallet: Some(wallet.to_string()),
        tokens,
        pair: Some(pair_contract.to_string()),
        pairs: vec![],
    };
    let resp = probe_factory_blacklist(querier, &pair_info.factory, check)?
        .ok_or(ContractError::BlacklistGuardUnavailable {})?;
    Ok(resp.blocked)
}

/// Read-only blacklist gate for limit-book match walks (GitLab #468).
pub struct TradeBlacklistGate<'a> {
    querier: QuerierWrapper<'a>,
    pair_info: &'a PairInfoState,
    pair_contract: &'a Addr,
    extra_tokens: &'a [Addr],
}

impl<'a> TradeBlacklistGate<'a> {
    pub fn from_parts(
        querier: QuerierWrapper<'a>,
        pair_info: &'a PairInfoState,
        pair_contract: &'a Addr,
    ) -> Self {
        Self {
            querier,
            pair_info,
            pair_contract,
            extra_tokens: &[],
        }
    }

    pub fn from_deps(
        deps: Deps<'a>,
        pair_info: &'a PairInfoState,
        pair_contract: &'a Addr,
    ) -> Self {
        Self::from_parts(deps.querier, pair_info, pair_contract)
    }

    pub fn is_wallet_blacklisted(&self, wallet: &Addr) -> Result<bool, ContractError> {
        wallet_is_trade_blacklisted(
            &self.querier,
            self.pair_info,
            self.pair_contract,
            wallet,
            self.extra_tokens,
        )
    }
}
