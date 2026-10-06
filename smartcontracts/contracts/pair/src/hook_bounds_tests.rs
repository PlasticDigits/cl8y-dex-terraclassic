use super::*;
use cosmwasm_std::testing::{mock_dependencies, mock_info};
use dex_common::hook::{validate_stored_hooks, MAX_PAIR_HOOKS};

#[test]
fn hook_bounds_pair_rejects_without_mutation_and_repairs_legacy_state() {
    let mut deps = mock_dependencies();
    PAIR_INFO
        .save(
            deps.as_mut().storage,
            &PairInfoState {
                asset_infos: [
                    AssetInfo::Token {
                        contract_addr: "token_a".into(),
                    },
                    AssetInfo::Token {
                        contract_addr: "token_b".into(),
                    },
                ],
                lp_token: Addr::unchecked("lp_token"),
                factory: Addr::unchecked("factory"),
            },
        )
        .unwrap();
    let original = vec![Addr::unchecked("policy_hook")];
    HOOKS.save(deps.as_mut().storage, &original).unwrap();
    for hooks in [
        vec!["duplicate_hook".into(); 2],
        vec![String::new(); MAX_PAIR_HOOKS + 1],
    ] {
        assert!(execute_update_hooks(deps.as_mut(), mock_info("factory", &[]), hooks).is_err());
        assert_eq!(HOOKS.load(deps.as_ref().storage).unwrap(), original);
    }
    assert!(matches!(
        execute_update_hooks(deps.as_mut(), mock_info("attacker", &[]), vec![]),
        Err(ContractError::Unauthorized {})
    ));
    assert_eq!(HOOKS.load(deps.as_ref().storage).unwrap(), original);

    // A legacy oversize list freezes hook fanout until explicit complete repair.
    let legacy = vec![Addr::unchecked("policy_hook"); MAX_PAIR_HOOKS + 1];
    HOOKS.save(deps.as_mut().storage, &legacy).unwrap();
    assert!(validate_stored_hooks(&HOOKS.load(deps.as_ref().storage).unwrap()).is_err());
    assert_eq!(HOOKS.load(deps.as_ref().storage).unwrap(), legacy);
    let replacement: Vec<_> = (0..MAX_PAIR_HOOKS)
        .map(|i| format!("policy_hook_{i}"))
        .collect();
    execute_update_hooks(
        deps.as_mut(),
        mock_info("factory", &[]),
        replacement.clone(),
    )
    .unwrap();
    assert_eq!(
        HOOKS.load(deps.as_ref().storage).unwrap(),
        replacement
            .into_iter()
            .map(Addr::unchecked)
            .collect::<Vec<_>>()
    );
    execute_update_hooks(deps.as_mut(), mock_info("factory", &[]), vec![]).unwrap();
    assert!(HOOKS.load(deps.as_ref().storage).unwrap().is_empty());
}
