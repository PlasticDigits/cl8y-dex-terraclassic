//! #1390 compatibility: older router contexts never grant the refund exception.
use super::*;
use cosmwasm_std::{
    from_json,
    testing::{mock_dependencies, mock_env, mock_info},
    ContractResult, SystemResult,
};

#[test]
fn issue1390_old_context_without_owned_refund_keeps_max_wallet_cap() {
    let mut deps = mock_dependencies();
    let env = mock_env();
    let router = Addr::unchecked("router");
    let payer = Addr::unchecked("payer");
    CONFIG
        .save(
            deps.as_mut().storage,
            &Config {
                manager: Addr::unchecked("manager"),
                treasury: Addr::unchecked("treasury"),
                buy_bps: 450,
                sell_bps: 100,
                transfer_bps: 0,
                max_buy_bps: 450,
                max_sell_bps: 100,
                max_transfer_bps: 0,
                factory: Addr::unchecked("factory"),
                router: Some(router.clone()),
                ust1: Addr::unchecked("ustone"),
                cmm_treasury: Addr::unchecked("cmm"),
                autolp: None,
                launcher: None,
                mint_revoked: true,
            },
        )
        .unwrap();
    FEATURES
        .save(
            deps.as_mut().storage,
            &Features::from_skus(&[crate::msg::Sku::LaunchGuards]),
        )
        .unwrap();
    LAUNCH_GUARDS
        .save(
            deps.as_mut().storage,
            &LaunchGuards {
                max_wallet: Some(Uint128::new(100)),
                cooldown_blocks: 1,
                trading_enabled: true,
            },
        )
        .unwrap();
    BALANCES
        .save(deps.as_mut().storage, &payer, &Uint128::new(1_000))
        .unwrap();
    BALANCES
        .save(deps.as_mut().storage, &router, &Uint128::one())
        .unwrap();
    // Actual pre-1390 JSON: the transfer is authorized and the token is acquired,
    // but neither condition identifies original-input restoration.
    let legacy_json = br#"{
        "funding_version":1,"trader":"payer","pair":"pair",
        "offer_token":"offer","ask_token":"ask","gross":"100","economic":"99",
        "transfers":[{"token":"cosmos2contract","recipient":"payer","amount":"1"}],
        "acquired_tokens":["cosmos2contract"]
    }"#;
    let ctx: dex_common::route_tax::RouteTaxContextResponse = from_json(legacy_json).unwrap();
    assert!(ctx.original_input_refund.is_none());
    assert_eq!(env.contract.address, "cosmos2contract");
    deps.querier.update_wasm(move |_| {
        SystemResult::Ok(ContractResult::Ok(Binary::from(legacy_json.as_slice())))
    });
    let err = execute_route_transfer(
        deps.as_mut(),
        env.clone(),
        mock_info("router", &[]),
        0,
        payer.to_string(),
        Uint128::one(),
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::RouteTaxContext {}));
    // Once the mandatory single-use nonce is present, missing refund metadata
    // still cannot confer the max-wallet exception.
    let mut settlement_ctx = ctx;
    settlement_ctx.route_id = 1;
    let settlement_json = cosmwasm_std::to_json_binary(&settlement_ctx).unwrap();
    deps.querier
        .update_wasm(move |_| SystemResult::Ok(ContractResult::Ok(settlement_json.clone())));
    let err = execute_route_transfer(
        deps.as_mut(),
        env,
        mock_info("router", &[]),
        0,
        payer.to_string(),
        Uint128::one(),
    )
    .unwrap_err();
    assert!(matches!(err, ContractError::MaxWallet {}));
    assert_eq!(
        BALANCES.load(deps.as_ref().storage, &payer).unwrap(),
        Uint128::new(1_000)
    );
    assert_eq!(
        BALANCES.load(deps.as_ref().storage, &router).unwrap(),
        Uint128::one()
    );
}
