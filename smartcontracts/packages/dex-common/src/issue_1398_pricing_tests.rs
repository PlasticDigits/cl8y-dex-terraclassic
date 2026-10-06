use super::*;
use cosmwasm_std::testing::mock_dependencies;
use cosmwasm_std::{to_json_binary, ContractResult, SystemResult, WasmQuery};
use std::{cell::Cell, rc::Rc};

fn tax_config(bps: u16) -> HookConfigResponse {
    HookConfigResponse {
        admin: Some(Addr::unchecked("governance")),
        recipient: Some(Addr::unchecked("recipient")),
        tax_percentage_bps: Some(bps),
        tax_token: Some(Addr::unchecked("ask_token")),
        ..HookConfigResponse::default()
    }
}

#[test]
fn issue1398_pricing_max_hooks_query_each_config_once_and_apply_exact_floors() {
    let mut deps = mock_dependencies();
    let calls = Rc::new(Cell::new(0));
    let seen = Rc::clone(&calls);
    let response = to_json_binary(&tax_config(100)).unwrap();
    deps.querier.update_wasm(move |query| {
        let WasmQuery::Smart { msg, .. } = query else {
            panic!("expected smart query")
        };
        assert_eq!(msg.as_slice(), br#"{"get_config":{}}"#);
        seen.set(seen.get() + 1);
        SystemResult::Ok(ContractResult::Ok(response.clone()))
    });
    let hooks: Vec<_> = (0..crate::hook::MAX_PAIR_HOOKS)
        .map(|i| Addr::unchecked(format!("hook_{i}")))
        .collect();
    let gross = Uint128::new(12_345);
    let deductions =
        collect_fee_hook_deductions(deps.as_ref(), &hooks, "ask_token", gross).unwrap();
    assert_eq!(calls.get(), 8);
    assert_eq!(deductions.len(), 8);
    assert!(deductions.iter().all(|d| d.amount == Uint128::new(123)));
    assert_eq!(
        net_return_after_hooks(gross, &deductions).unwrap(),
        Uint128::new(11_361)
    );
}

#[test]
fn issue1398_pricing_invalid_legacy_lists_issue_zero_queries() {
    let mut deps = mock_dependencies();
    deps.querier
        .update_wasm(|_| panic!("invalid stored lists must reject before config queries"));
    for hooks in [
        (0..=crate::hook::MAX_PAIR_HOOKS)
            .map(|i| Addr::unchecked(format!("hook_{i}")))
            .collect::<Vec<_>>(),
        vec![Addr::unchecked("policy_hook"); 2],
    ] {
        assert!(
            collect_fee_hook_deductions(deps.as_ref(), &hooks, "ask_token", Uint128::new(100))
                .is_err()
        );
    }
}

#[test]
fn issue1398_pricing_unavailable_or_unknown_config_fails_closed() {
    let mut deps = mock_dependencies();
    deps.querier
        .update_wasm(|_| SystemResult::Ok(ContractResult::Err("unavailable".into())));
    let hooks = [Addr::unchecked("fee_hook")];
    let error = collect_fee_hook_deductions(deps.as_ref(), &hooks, "ask_token", Uint128::new(100))
        .unwrap_err();
    assert!(error.to_string().contains("GetConfig unavailable"));
    deps.querier.update_wasm(|_| {
        SystemResult::Ok(ContractResult::Ok(
            to_json_binary(&HookConfigResponse::default()).unwrap(),
        ))
    });
    let error = collect_fee_hook_deductions(deps.as_ref(), &hooks, "ask_token", Uint128::new(100))
        .unwrap_err();
    assert!(error.to_string().contains("unsupported hook pricing"));
}

#[test]
fn issue1398_pricing_partial_and_ambiguous_configs_fail_closed() {
    for cfg in [
        HookConfigResponse {
            tax_percentage_bps: Some(100),
            ..HookConfigResponse::default()
        },
        HookConfigResponse {
            burn_token: Some(Addr::unchecked("ask_token")),
            ..tax_config(100)
        },
        HookConfigResponse {
            percentage_bps: Some(100),
            ..tax_config(100)
        },
    ] {
        let mut deps = mock_dependencies();
        let response = to_json_binary(&cfg).unwrap();
        deps.querier
            .update_wasm(move |_| SystemResult::Ok(ContractResult::Ok(response.clone())));
        let error = collect_fee_hook_deductions(
            deps.as_ref(),
            &[Addr::unchecked("fee_hook")],
            "ask_token",
            Uint128::new(100),
        )
        .unwrap_err();
        assert!(error.to_string().contains("unsupported hook pricing"));
    }
}

#[test]
fn issue1398_pricing_full_tax_and_uint128_max_do_not_overflow() {
    assert_eq!(bps_of_amount(Uint128::MAX, 10_000).unwrap(), Uint128::MAX);
    assert_eq!(
        bps_of_amount(Uint128::MAX, 3333).unwrap(),
        Uint128::new(113_416_112_894_748_789_872_342_756_657_008_344_877)
    );
    assert!(bps_of_amount(Uint128::MAX, 10_001).is_err());
    let mut deps = mock_dependencies();
    let response = to_json_binary(&tax_config(10_000)).unwrap();
    deps.querier
        .update_wasm(move |_| SystemResult::Ok(ContractResult::Ok(response.clone())));
    let deductions = collect_fee_hook_deductions(
        deps.as_ref(),
        &[Addr::unchecked("fee_hook")],
        "ask_token",
        Uint128::MAX,
    )
    .unwrap();
    assert_eq!(
        net_return_after_hooks(Uint128::MAX, &deductions).unwrap(),
        Uint128::zero()
    );
}

#[test]
fn issue1398_pricing_sum_above_full_output_rejects_even_when_each_fee_rounds_to_zero() {
    let mut deps = mock_dependencies();
    let response = to_json_binary(&tax_config(6000)).unwrap();
    deps.querier
        .update_wasm(move |_| SystemResult::Ok(ContractResult::Ok(response.clone())));
    let error = collect_fee_hook_deductions(
        deps.as_ref(),
        &[Addr::unchecked("tax_a"), Addr::unchecked("tax_b")],
        "ask_token",
        Uint128::new(1),
    )
    .unwrap_err();
    assert!(error.to_string().contains("total bps exceeds 10000"));
}

#[test]
fn issue1398_pricing_burn_wrong_token_and_lp_policy_configs_have_explicit_semantics() {
    let mut deps = mock_dependencies();
    deps.querier.update_wasm(|query| {
        let WasmQuery::Smart { contract_addr, .. } = query else {
            panic!("expected smart query")
        };
        let cfg = match contract_addr.as_str() {
            "burn_hook" => HookConfigResponse {
                burn_token: Some(Addr::unchecked("ask_token")),
                burn_percentage_bps: Some(2500),
                ..HookConfigResponse::default()
            },
            "other_token_tax" => HookConfigResponse {
                tax_token: Some(Addr::unchecked("other_token")),
                ..tax_config(5000)
            },
            "lp_hook" => HookConfigResponse {
                target_pair: Some(Addr::unchecked("pair")),
                lp_token: Some(Addr::unchecked("lp_token")),
                percentage_bps: Some(100),
                ..HookConfigResponse::default()
            },
            _ => panic!("unexpected hook"),
        };
        SystemResult::Ok(ContractResult::Ok(to_json_binary(&cfg).unwrap()))
    });
    let hooks = [
        Addr::unchecked("burn_hook"),
        Addr::unchecked("other_token_tax"),
        Addr::unchecked("lp_hook"),
    ];
    let deductions =
        collect_fee_hook_deductions(deps.as_ref(), &hooks, "ask_token", Uint128::new(101)).unwrap();
    assert_eq!(deductions.len(), 1);
    assert_eq!(deductions[0].kind, HookFeeKind::Burn);
    assert_eq!(deductions[0].amount, Uint128::new(25));
    assert_eq!(
        net_return_after_hooks(Uint128::new(101), &deductions).unwrap(),
        Uint128::new(76)
    );
}
