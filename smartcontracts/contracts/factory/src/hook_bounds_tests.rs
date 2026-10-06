use super::*;
use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
use cosmwasm_std::{from_json, Addr};
use dex_common::hook::MAX_PAIR_HOOKS;

#[test]
fn hook_bounds_factory_rejects_before_forwarding_and_keeps_config() {
    let mut deps = mock_dependencies();
    instantiate(
        deps.as_mut(),
        mock_env(),
        mock_info("gov", &[]),
        InstantiateMsg {
            governance: "gov".into(),
            treasury: "treasury".into(),
            default_fee_bps: 30,
            pair_code_id: 1,
            lp_token_code_id: 2,
            whitelisted_code_ids: vec![],
            default_limit_batch_max_rungs: 10,
            pair_creation_fee_uluna: Uint128::zero(),
        },
    )
    .unwrap();
    let pair = Addr::unchecked("registered_pair");
    PAIR_ADDR_REGISTERED
        .save(deps.as_mut().storage, pair.clone(), &true)
        .unwrap();
    let before = CONFIG.load(deps.as_ref().storage).unwrap();
    for hooks in [
        vec!["same_hook".into(); 2],
        vec![String::new(); MAX_PAIR_HOOKS + 1],
    ] {
        assert!(execute_set_pair_hooks(
            deps.as_mut(),
            mock_info("gov", &[]),
            pair.to_string(),
            hooks
        )
        .is_err());
        assert_eq!(CONFIG.load(deps.as_ref().storage).unwrap(), before);
        assert!(PAIR_ADDR_REGISTERED
            .load(deps.as_ref().storage, pair.clone())
            .unwrap());
    }
    let hooks: Vec<_> = (0..MAX_PAIR_HOOKS).map(|i| format!("hook{i}")).collect();
    let response = execute_set_pair_hooks(
        deps.as_mut(),
        mock_info("gov", &[]),
        pair.to_string(),
        hooks.clone(),
    )
    .unwrap();
    assert_eq!(response.messages.len(), 1);
    let CosmosMsg::Wasm(WasmMsg::Execute { msg, .. }) = &response.messages[0].msg else {
        panic!("expected pair execute")
    };
    assert_eq!(
        from_json::<dex_common::pair::ExecuteMsg>(msg).unwrap(),
        dex_common::pair::ExecuteMsg::UpdateHooks { hooks }
    );
}

#[test]
fn hook_bounds_factory_keeps_governance_and_registry_gates() {
    let mut deps = mock_dependencies();
    CONFIG
        .save(
            deps.as_mut().storage,
            &Config {
                governance: Addr::unchecked("gov"),
                treasury: Addr::unchecked("treasury"),
                default_fee_bps: 30,
                pair_code_id: 1,
                lp_token_code_id: 2,
                default_limit_batch_max_rungs: 10,
                pair_creation_fee_uluna: Uint128::zero(),
                discount_registry: None,
            },
        )
        .unwrap();
    assert!(matches!(
        execute_set_pair_hooks(
            deps.as_mut(),
            mock_info("other", &[]),
            "registered_pair".into(),
            vec![]
        ),
        Err(ContractError::Unauthorized {})
    ));
    assert!(matches!(
        execute_set_pair_hooks(
            deps.as_mut(),
            mock_info("gov", &[]),
            "unregistered_pair".into(),
            vec![]
        ),
        Err(ContractError::PairNotInRegistry { .. })
    ));
}
