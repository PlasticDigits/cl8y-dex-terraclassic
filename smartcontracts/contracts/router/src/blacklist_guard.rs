use cosmwasm_std::{Addr, Deps};

use crate::error::ContractError;
use crate::msg::SwapOperation;
use dex_common::blacklist::BlacklistCheck;
use dex_common::factory::QueryMsg as FactoryQueryMsg;

pub fn assert_router_swap_not_blacklisted(
    deps: Deps,
    factory: &Addr,
    sender: &Addr,
    input_token: &Addr,
    operations: &[SwapOperation],
) -> Result<(), ContractError> {
    let mut tokens = vec![input_token.to_string()];
    let mut pairs = Vec::new();

    for op in operations {
        match op {
            SwapOperation::NativeSwap { .. } => {
                return Err(ContractError::NativeSwapNotSupported {});
            }
            SwapOperation::TerraSwap {
                offer_asset_info,
                ask_asset_info,
                ..
            } => {
                let offer = offer_asset_info
                    .assert_is_token()
                    .map_err(|_| ContractError::NativeTokenNotSupported {})?;
                let ask = ask_asset_info
                    .assert_is_token()
                    .map_err(|_| ContractError::NativeTokenNotSupported {})?;
                if !tokens.contains(&offer.to_string()) {
                    tokens.push(offer.to_string());
                }
                if !tokens.contains(&ask.to_string()) {
                    tokens.push(ask.to_string());
                }

                let pair_response: dex_common::factory::PairResponse = deps
                    .querier
                    .query_wasm_smart(
                        factory.to_string(),
                        &FactoryQueryMsg::Pair {
                            asset_infos: [offer_asset_info.clone(), ask_asset_info.clone()],
                        },
                    )
                    .map_err(|_| ContractError::PairNotFound {})?;
                let pair_addr = pair_response.pair.contract_addr.to_string();
                if !pairs.contains(&pair_addr) {
                    pairs.push(pair_addr);
                }
            }
        }
    }

    let resp: dex_common::blacklist::BlacklistCheckResponse = match deps.querier.query_wasm_smart(
        factory.to_string(),
        &FactoryQueryMsg::BlacklistCheck(BlacklistCheck {
            wallet: Some(sender.to_string()),
            tokens,
            pair: None,
            pairs,
        }),
    ) {
        Ok(r) => r,
        // GitLab #456 (SEC-I03 F02): default deny — any factory BlacklistCheck query error
        // blocks the swap instead of silently disabling blacklist enforcement.
        Err(_) => return Err(ContractError::BlacklistGuardUnavailable {}),
    };

    if resp.blocked {
        return Err(ContractError::Blacklisted {});
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::assert_router_swap_not_blacklisted;
    use crate::error::ContractError;
    use crate::msg::SwapOperation;
    use cosmwasm_std::testing::{mock_dependencies, MockApi, MockQuerier, MockStorage};
    use cosmwasm_std::{
        from_json, to_json_binary, Addr, ContractResult, OwnedDeps, QuerierResult, SystemResult,
        WasmQuery,
    };
    use dex_common::blacklist::BlacklistCheckResponse;
    use dex_common::factory::{PairResponse, QueryMsg as FactoryQueryMsg};
    use dex_common::types::{AssetInfo, PairInfo};

    fn token(addr: &str) -> AssetInfo {
        AssetInfo::Token {
            contract_addr: addr.to_string(),
        }
    }

    fn one_hop_ops() -> Vec<SwapOperation> {
        vec![SwapOperation::TerraSwap {
            offer_asset_info: token("token_a"),
            ask_asset_info: token("token_b"),
            hybrid: None,
            greedy: None,
            min_return: None,
        }]
    }

    fn pair_response() -> PairResponse {
        PairResponse {
            pair: PairInfo {
                asset_infos: [token("token_a"), token("token_b")],
                contract_addr: Addr::unchecked("pair1"),
                liquidity_token: Addr::unchecked("lp1"),
            },
        }
    }

    fn healthy_blacklist_check(blocked: bool) -> QuerierResult {
        let resp = BlacklistCheckResponse {
            blocked,
            wallet_blacklisted: false,
            blacklisted_tokens: vec![],
            pair_blacklisted: false,
            blacklisted_pairs: vec![],
        };
        SystemResult::Ok(ContractResult::Ok(to_json_binary(&resp).unwrap()))
    }

    /// Deps whose factory answers `Pair` normally but routes `BlacklistCheck` through
    /// `blacklist_check`, so each test can inject a healthy response, a blocked response, or a
    /// query error (stale / pre-guard factory pointer).
    fn deps_with_blacklist(
        blacklist_check: impl Fn() -> QuerierResult + 'static,
    ) -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies();
        deps.querier.update_wasm(move |query| match query {
            WasmQuery::Smart { msg, .. } => match from_json::<FactoryQueryMsg>(msg.as_slice()) {
                Ok(FactoryQueryMsg::Pair { .. }) => SystemResult::Ok(ContractResult::Ok(
                    to_json_binary(&pair_response()).unwrap(),
                )),
                Ok(FactoryQueryMsg::BlacklistCheck(_)) => blacklist_check(),
                _ => SystemResult::Ok(ContractResult::Err("unexpected factory query".to_string())),
            },
            _ => SystemResult::Ok(ContractResult::Err("unexpected query type".to_string())),
        });
        deps
    }

    fn run_guard(deps: &OwnedDeps<MockStorage, MockApi, MockQuerier>) -> Result<(), ContractError> {
        assert_router_swap_not_blacklisted(
            deps.as_ref(),
            &Addr::unchecked("factory"),
            &Addr::unchecked("trader"),
            &Addr::unchecked("token_a"),
            &one_hop_ops(),
        )
    }

    // GitLab #456 (SEC-I03 F02): the router guard must default-deny — any factory BlacklistCheck
    // query error blocks the swap instead of silently disabling blacklist enforcement. Mirrors the
    // pair-side `factory_blacklist_query_error_blocks_swap` coverage for the router path.
    #[test]
    fn router_guard_blocks_swap_when_factory_blacklist_query_errors() {
        // Factory answers Pair but errors on BlacklistCheck (stale / pre-1.5 factory pointer).
        let deps = deps_with_blacklist(|| {
            SystemResult::Ok(ContractResult::Err(
                "unknown variant `blacklist_check`".to_string(),
            ))
        });
        let err = run_guard(&deps).unwrap_err();
        assert!(
            matches!(err, ContractError::BlacklistGuardUnavailable {}),
            "expected BlacklistGuardUnavailable, got {err:?}"
        );
    }

    #[test]
    fn router_guard_allows_swap_when_factory_healthy_and_not_blacklisted() {
        let deps = deps_with_blacklist(|| healthy_blacklist_check(false));
        run_guard(&deps).expect("healthy factory with no blacklist hit should allow the swap");
    }

    #[test]
    fn router_guard_blocks_swap_when_blacklist_hits() {
        let deps = deps_with_blacklist(|| healthy_blacklist_check(true));
        let err = run_guard(&deps).unwrap_err();
        assert!(
            matches!(err, ContractError::Blacklisted {}),
            "expected Blacklisted, got {err:?}"
        );
    }
}
