//! Discovery reuse must preserve amount-specific, pair-specific trader checks.
use super::*;
use cosmwasm_std::{
    from_json, testing::mock_dependencies, ContractResult, StdError, SystemResult, WasmQuery,
};
use dex_common::{route_tax as tax, types::AssetInfo};
use std::{cell::RefCell, rc::Rc};

type MockDeps = cosmwasm_std::OwnedDeps<
    cosmwasm_std::MemoryStorage,
    cosmwasm_std::testing::MockApi,
    cosmwasm_std::testing::MockQuerier,
>;
type PreviewCall = (String, tax::RouteTaxKind, Uint128, Option<String>, String);
#[derive(Default)]
struct Calls {
    capability: usize,
    ordinary_probes: usize,
    token_identity: usize,
    factory: usize,
    reverse: usize,
    forward: usize,
    previews: Vec<PreviewCall>,
}
#[derive(Clone, Copy, PartialEq)]
enum Fault {
    None,
    FreshPreview,
    InsufficientForward,
    UntrustedRouter,
    UnknownOrdinary,
    LegacyTax,
    FundingVersion,
    BadAccounting,
}
fn asset(index: usize) -> AssetInfo {
    AssetInfo::Token {
        contract_addr: format!("token{index}"),
    }
}
fn operations() -> Vec<SwapOperation> {
    (0..3)
        .map(|hop| SwapOperation::TerraSwap {
            offer_asset_info: asset(hop),
            ask_asset_info: asset(hop + 1),
            hybrid: None,
            greedy: None,
            min_return: None,
        })
        .collect()
}
fn rates(token: &str) -> Option<(u16, u16)> {
    match token {
        "token1" => Some((125, 275)),
        "token2" => Some((700, 350)),
        _ => None,
    }
}
fn fixture(fault: Fault) -> (MockDeps, Rc<RefCell<Calls>>) {
    let mut deps = mock_dependencies();
    FACTORY
        .save(deps.as_mut().storage, &Addr::unchecked("factory"))
        .unwrap();
    let calls = Rc::new(RefCell::new(Calls::default()));
    let recorded = calls.clone();
    deps.querier.update_wasm(move |query| {
        let result = (|| -> StdResult<cosmwasm_std::Binary> {
            let mut calls = recorded.borrow_mut();
            match query {
                WasmQuery::Raw { contract_addr, key } => {
                    assert_eq!(key.as_slice(), b"contract_info");
                    let (contract, version) = if contract_addr.starts_with("pair") {
                        ("cl8y-dex-pair", "1.20.0")
                    } else {
                        calls.token_identity += 1;
                        (
                            if fault == Fault::UnknownOrdinary {
                                "untrusted-token"
                            } else {
                                "crates.io:cw20-base"
                            },
                            "2.0.0",
                        )
                    };
                    to_json_binary(&cw2::ContractVersion {
                        contract: contract.into(),
                        version: version.into(),
                    })
                }
                WasmQuery::Smart { contract_addr, msg } if contract_addr == "factory" => {
                    calls.factory += 1;
                    let dex_common::factory::QueryMsg::Pair { asset_infos } = from_json(msg)?
                    else {
                        panic!("unexpected factory query")
                    };
                    let pair = format!(
                        "pair{}{}",
                        asset_infos[0].assert_is_token()?,
                        asset_infos[1].assert_is_token()?
                    );
                    to_json_binary(&dex_common::factory::PairResponse {
                        pair: dex_common::types::PairInfo {
                            asset_infos,
                            contract_addr: Addr::unchecked(pair),
                            liquidity_token: Addr::unchecked("lp"),
                        },
                    })
                }
                WasmQuery::Smart { contract_addr, msg } if contract_addr.starts_with("pair") => {
                    match from_json::<pair::QueryMsg>(msg)? {
                        pair::QueryMsg::HybridReverseSimulation {
                            ask_asset,
                            trader,
                            sender,
                            ..
                        } => {
                            assert!(matches!(
                                trader.as_deref(),
                                Some("trader" | "protocol-exempt")
                            ));
                            assert_eq!(sender.as_deref(), Some("sender"));
                            calls.reverse += 1;
                            to_json_binary(&pair::HybridReverseSimulationResponse {
                                offer_amount: ask_asset.amount,
                                spread_amount: Uint128::zero(),
                                commission_amount: Uint128::zero(),
                                pool_commission_amount: Uint128::zero(),
                                book_commission_amount: Uint128::zero(),
                                book_return_amount: Uint128::zero(),
                                pool_return_amount: ask_asset.amount,
                            })
                        }
                        pair::QueryMsg::HybridSimulation {
                            offer_asset,
                            hybrid,
                            trader,
                            sender,
                            ..
                        } => {
                            assert!(matches!(
                                trader.as_deref(),
                                Some("trader" | "protocol-exempt")
                            ));
                            assert_eq!(sender.as_deref(), Some("sender"));
                            assert_eq!(hybrid.unwrap().pool_input, offer_asset.amount);
                            calls.forward += 1;
                            let amount = if fault == Fault::InsufficientForward {
                                Uint128::zero()
                            } else {
                                offer_asset.amount
                            };
                            to_json_binary(&pair::HybridSimulationResponse {
                                return_amount: amount,
                                spread_amount: Uint128::zero(),
                                commission_amount: Uint128::zero(),
                                pool_commission_amount: Uint128::zero(),
                                book_commission_amount: Uint128::zero(),
                                book_return_amount: Uint128::zero(),
                                pool_return_amount: amount,
                                limit_book_offer_consumed: Uint128::zero(),
                                greedy_stop: None,
                            })
                        }
                        _ => panic!("unexpected pair query"),
                    }
                }
                WasmQuery::Smart { contract_addr, msg } => {
                    let query = match from_json::<tax::TokenQueryMsg>(msg) {
                        Ok(query) => query,
                        Err(_) => {
                            calls.ordinary_probes += 1;
                            if fault == Fault::LegacyTax {
                                return Ok(cosmwasm_std::Binary::from(
                                    br#"{"legacy":true}"#.as_slice(),
                                ));
                            }
                            return Err(StdError::generic_err("unsupported ordinary query"));
                        }
                    };
                    match query {
                        tax::TokenQueryMsg::RouteTaxCapability {} => {
                            calls.capability += 1;
                            let Some((buy_bps, sell_bps)) = rates(contract_addr) else {
                                return Err(StdError::generic_err("ordinary CW20"));
                            };
                            to_json_binary(&tax::RouteTaxCapabilityResponse {
                                funding_version: if fault == Fault::FundingVersion { 2 } else { 1 },
                                router: Some(Addr::unchecked(if fault == Fault::UntrustedRouter {
                                    "other-router"
                                } else {
                                    "router"
                                })),
                                buy_bps,
                                sell_bps,
                            })
                        }
                        tax::TokenQueryMsg::RouteTaxPreview {
                            kind,
                            amount,
                            trader,
                            pair,
                        } => {
                            calls.previews.push((
                                contract_addr.clone(),
                                kind.clone(),
                                amount,
                                trader.clone(),
                                pair,
                            ));
                            if trader.as_deref() == Some("protocol-exempt") {
                                return Err(StdError::generic_err("invalid routed trader"));
                            }
                            assert_eq!(trader.as_deref(), Some("trader"));
                            let (buy_bps, sell_bps) = rates(contract_addr).unwrap();
                            if fault == Fault::FreshPreview
                                && kind == tax::RouteTaxKind::Sell
                                && amount != Uint128::new(10_000 + u128::from(sell_bps))
                            {
                                return Err(StdError::generic_err("fresh amount launch guard"));
                            }
                            let mut preview = match kind {
                                tax::RouteTaxKind::Buy => tax::buy_from_gross(amount, buy_bps)?,
                                tax::RouteTaxKind::Sell => tax::sell_from_gross(amount, sell_bps)?,
                            };
                            if fault == Fault::BadAccounting {
                                preview.net = amount;
                            }
                            to_json_binary(&preview)
                        }
                    }
                }
                _ => panic!("unexpected query"),
            }
        })();
        SystemResult::Ok(match result {
            Ok(bytes) => ContractResult::Ok(bytes),
            Err(error) => ContractResult::Err(error.to_string()),
        })
    });
    (deps, calls)
}
fn reverse(deps: Deps, target: Uint128, trader: &str) -> StdResult<SimulateSwapOperationsResponse> {
    query_reverse_simulate_swap_operations(
        deps,
        &Addr::unchecked("router"),
        target,
        operations(),
        Some(trader.into()),
        Some("sender".into()),
    )
}
fn expected_forward(mut gross: Uint128) -> Uint128 {
    for hop in 0..3 {
        if let Some((_, sell)) = rates(&format!("token{hop}")) {
            gross = tax::sell_from_gross(gross, sell).unwrap().economic;
        }
        if let Some((buy, _)) = rates(&format!("token{}", hop + 1)) {
            gross = tax::buy_from_gross(gross, buy).unwrap().net;
        }
    }
    gross
}
#[test]
fn issue1422_reverse_discovers_once_but_previews_and_forward_stay_fresh() {
    for target in [1, 99, 10_000, 1_000_000, 100_000_000_000_000_000_000u128] {
        let (deps, calls) = fixture(Fault::None);
        let target = Uint128::new(target);
        let gross = reverse(deps.as_ref(), target, "trader").unwrap().amount;
        assert!(expected_forward(gross) >= target);
        assert!(expected_forward(gross - Uint128::one()) < target);
        let recorded = calls.borrow();
        assert_eq!(
            (
                recorded.capability,
                recorded.ordinary_probes,
                recorded.token_identity
            ),
            (4, 6, 2)
        );
        assert_eq!(
            (recorded.factory, recorded.reverse, recorded.forward),
            (3, 3, 3)
        );
        assert_eq!(recorded.previews.len(), 8); // Reverse rates AND actual forward amounts.
        for (token, kind, _, trader, pair) in &recorded.previews {
            assert_eq!(trader.as_deref(), Some("trader"));
            match (token.as_str(), kind) {
                ("token1", tax::RouteTaxKind::Buy) => assert_eq!(pair, "pairtoken0token1"),
                ("token1", tax::RouteTaxKind::Sell) | ("token2", tax::RouteTaxKind::Buy) => {
                    assert_eq!(pair, "pairtoken1token2")
                }
                ("token2", tax::RouteTaxKind::Sell) => assert_eq!(pair, "pairtoken2token3"),
                _ => panic!("unexpected tax boundary"),
            }
        }
    }
}
#[test]
fn issue1422_fresh_preview_guards_and_insufficient_forward_are_rejected() {
    for (fault, message) in [
        (Fault::FreshPreview, "fresh amount launch guard"),
        (Fault::InsufficientForward, "REVERSE_FORWARD_INSUFFICIENT"),
    ] {
        let (deps, calls) = fixture(fault);
        let error = reverse(deps.as_ref(), Uint128::new(1_000_000), "trader").unwrap_err();
        assert!(error.to_string().contains(message), "{error}");
        assert_eq!(calls.borrow().reverse, 3);
        assert!(calls.borrow().forward > 0);
    }
}
#[test]
fn issue1422_discovery_and_preview_errors_remain_fail_closed() {
    for (fault, message) in [
        (
            Fault::UntrustedRouter,
            "token does not trust this route-local router",
        ),
        (
            Fault::UnknownOrdinary,
            "unsupported token trading capability",
        ),
        (
            Fault::LegacyTax,
            "token route-local tax funding capability unavailable",
        ),
        (
            Fault::FundingVersion,
            "unsupported route-local tax funding version",
        ),
        (
            Fault::BadAccounting,
            "inconsistent route-local tax accounting",
        ),
    ] {
        let (deps, _) = fixture(fault);
        let error = reverse(deps.as_ref(), Uint128::new(1_000_000), "trader").unwrap_err();
        assert!(error.to_string().contains(message), "{error}");
    }
}
#[test]
fn issue1422_new_request_discovers_again_and_checks_its_original_trader() {
    let (deps, calls) = fixture(Fault::None);
    reverse(deps.as_ref(), Uint128::new(1_000_000), "trader").unwrap();
    assert_eq!(calls.borrow().capability, 4);
    let error = reverse(deps.as_ref(), Uint128::new(1_000_000), "protocol-exempt").unwrap_err();
    assert!(error.to_string().contains("invalid routed trader"));
    assert_eq!(calls.borrow().capability, 6); // USTR and BETA freshly rediscovered.
}
#[test]
fn issue1422_repeated_capable_path_rejects_before_pair_work() {
    let (deps, calls) = fixture(Fault::None);
    let mut repeated = operations();
    let SwapOperation::TerraSwap { ask_asset_info, .. } = &mut repeated[2] else {
        unreachable!()
    };
    *ask_asset_info = asset(1);
    let error = query_reverse_simulate_swap_operations(
        deps.as_ref(),
        &Addr::unchecked("router"),
        Uint128::one(),
        repeated,
        Some("trader".into()),
        Some("sender".into()),
    )
    .unwrap_err();
    assert!(error
        .to_string()
        .contains("repeated route-tax-capable asset"));
    assert_eq!(calls.borrow().factory, 0);
}
