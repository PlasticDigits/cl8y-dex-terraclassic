use super::*;
use crate::hook_settlement::collect_fee_hook_deductions;
use cosmwasm_std::testing::mock_dependencies;

#[test]
fn hook_bounds_accept_empty_and_exact_max_preserving_order() {
    let deps = mock_dependencies();
    assert!(validate_pair_hooks(&deps.api, &[]).unwrap().is_empty());
    let hooks: Vec<_> = (0..MAX_PAIR_HOOKS).map(|i| format!("hook{i}")).collect();
    let accepted = validate_pair_hooks(&deps.api, &hooks).unwrap();
    assert_eq!(
        accepted.iter().map(Addr::to_string).collect::<Vec<_>>(),
        hooks
    );
}

#[test]
fn hook_bounds_reject_oversize_before_invalid_addresses_and_duplicates() {
    let deps = mock_dependencies();
    let too_many = vec![String::new(); MAX_PAIR_HOOKS + 1];
    let error = validate_pair_hooks(&deps.api, &too_many).unwrap_err();
    assert!(error.to_string().contains("maximum 8"));
    let error =
        validate_pair_hooks(&deps.api, &["same_hook".into(), "same_hook".into()]).unwrap_err();
    assert!(error.to_string().contains("Duplicate pair hook"));
}

#[test]
fn hook_bounds_legacy_rejection_runs_before_config_queries() {
    let mut deps = mock_dependencies();
    deps.querier
        .update_wasm(|_| panic!("invalid hook lists must never query hook contracts"));
    for hooks in [
        vec![Addr::unchecked("legacy"); MAX_PAIR_HOOKS + 1],
        vec![Addr::unchecked("legacy"); 2],
    ] {
        let error =
            collect_fee_hook_deductions(deps.as_ref(), &hooks, "ask_token", Uint128::new(100))
                .unwrap_err();
        assert!(error.to_string().contains("pair hook"));
    }
}
