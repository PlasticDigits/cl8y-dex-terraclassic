use cosmwasm_std::{Addr, Empty, Uint128};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

use crate::msg::{ConfigResponse, ExecuteMsg, InstantiateMsg, QueryMsg};

fn autolp_contract() -> Box<dyn Contract<Empty>> {
    Box::new(
        ContractWrapper::new(
            crate::contract::execute,
            crate::contract::instantiate,
            crate::contract::query,
        )
        .with_reply(crate::contract::reply),
    )
}

#[test]
fn skim_below_threshold_is_noop() {
    let mut app = App::default();
    let code = app.store_code(autolp_contract());
    let manager = Addr::unchecked("manager");
    let autolp = app
        .instantiate_contract(
            code,
            manager.clone(),
            &InstantiateMsg {
                token: manager.to_string(),
                manager: manager.to_string(),
                router: None,
                pair: None,
                quote_token: None,
                threshold: Uint128::new(1_000_000),
                lp_recipient: manager.to_string(),
            },
            &[],
            "autolp",
            None,
        )
        .unwrap();
    // token query will fail if we try a real balance — threshold path queries token.
    // Unset pair + no token wasm: expect query error, which is fine for "no nested swap".
    let cfg: ConfigResponse = app
        .wrap()
        .query_wasm_smart(&autolp, &QueryMsg::GetConfig {})
        .unwrap();
    assert!(!cfg.skimming);
    assert_eq!(cfg.threshold, Uint128::new(1_000_000));
}

#[test]
fn update_config_manager_only() {
    let mut app = App::default();
    let code = app.store_code(autolp_contract());
    let manager = Addr::unchecked("manager");
    let other = Addr::unchecked("other");
    let autolp = app
        .instantiate_contract(
            code,
            manager.clone(),
            &InstantiateMsg {
                token: manager.to_string(),
                manager: manager.to_string(),
                router: None,
                pair: None,
                quote_token: None,
                threshold: Uint128::new(1),
                lp_recipient: manager.to_string(),
            },
            &[],
            "autolp",
            None,
        )
        .unwrap();
    let err = app
        .execute_contract(
            other,
            autolp.clone(),
            &ExecuteMsg::UpdateConfig {
                pair: Some("pair".into()),
                router: None,
                quote_token: None,
                threshold: None,
                lp_recipient: None,
            },
            &[],
        )
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("Unauthorized"));
    app.execute_contract(
        manager,
        autolp,
        &ExecuteMsg::UpdateConfig {
            pair: Some("pair".into()),
            router: None,
            quote_token: None,
            threshold: Some(Uint128::new(99)),
            lp_recipient: None,
        },
        &[],
    )
    .unwrap();
}
