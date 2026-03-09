#[cfg(test)]
mod helpers {
    use cosmwasm_std::{Addr, Empty, Uint128};
    use cw20::{BalanceResponse, Cw20QueryMsg};
    use cw_multi_test::{App, ContractWrapper, Executor};
    use dex_common::types::AssetInfo;

    pub fn cw20_contract() -> Box<dyn cw_multi_test::Contract<Empty>> {
        let contract = ContractWrapper::new(
            cw20_base::contract::execute,
            cw20_base::contract::instantiate,
            cw20_base::contract::query,
        );
        Box::new(contract)
    }

    pub fn factory_contract() -> Box<dyn cw_multi_test::Contract<Empty>> {
        let contract = ContractWrapper::new(
            cl8y_dex_factory::contract::execute,
            cl8y_dex_factory::contract::instantiate,
            cl8y_dex_factory::contract::query,
        )
        .with_reply(cl8y_dex_factory::contract::reply);
        Box::new(contract)
    }

    pub fn pair_contract() -> Box<dyn cw_multi_test::Contract<Empty>> {
        let contract = ContractWrapper::new(
            cl8y_dex_pair::contract::execute,
            cl8y_dex_pair::contract::instantiate,
            cl8y_dex_pair::contract::query,
        )
        .with_reply(cl8y_dex_pair::contract::reply);
        Box::new(contract)
    }

    pub fn router_contract() -> Box<dyn cw_multi_test::Contract<Empty>> {
        let contract = ContractWrapper::new(
            cl8y_dex_router::contract::execute,
            cl8y_dex_router::contract::instantiate,
            cl8y_dex_router::contract::query,
        )
        .with_reply(cl8y_dex_router::contract::reply);
        Box::new(contract)
    }

    pub fn create_cw20_token(
        app: &mut App,
        cw20_code_id: u64,
        owner: &Addr,
        name: &str,
        symbol: &str,
        initial_amount: Uint128,
    ) -> Addr {
        app.instantiate_contract(
            cw20_code_id,
            owner.clone(),
            &cw20_base::msg::InstantiateMsg {
                name: name.to_string(),
                symbol: symbol.to_string(),
                decimals: 6,
                initial_balances: vec![cw20::Cw20Coin {
                    address: owner.to_string(),
                    amount: initial_amount,
                }],
                mint: None,
                marketing: None,
            },
            &[],
            name,
            None,
        )
        .unwrap()
    }

    pub fn query_cw20_balance(app: &App, token: &Addr, address: &Addr) -> Uint128 {
        let resp: BalanceResponse = app
            .wrap()
            .query_wasm_smart(
                token.to_string(),
                &Cw20QueryMsg::Balance {
                    address: address.to_string(),
                },
            )
            .unwrap();
        resp.balance
    }

    pub fn extract_pair_address(events: &[cosmwasm_std::Event]) -> Addr {
        let pair_addr = events
            .iter()
            .filter(|e| e.ty == "wasm")
            .flat_map(|e| &e.attributes)
            .find(|a| a.key == "pair_contract")
            .map(|a| a.value.clone())
            .expect("pair_contract attribute not found in events");
        Addr::unchecked(pair_addr)
    }

    pub fn asset_info_token(addr: &Addr) -> AssetInfo {
        AssetInfo::Token {
            contract_addr: addr.to_string(),
        }
    }

    pub struct TestEnv {
        pub factory: Addr,
        pub token_a: Addr,
        pub token_b: Addr,
        pub pair: Addr,
        pub lp_token: Addr,
        pub router: Addr,
        pub governance: Addr,
        pub treasury: Addr,
        pub user: Addr,
    }

    pub fn setup_full_env(app: &mut App) -> TestEnv {
        let governance = Addr::unchecked("governance");
        let treasury = Addr::unchecked("treasury");
        let user = Addr::unchecked("user");

        let cw20_code_id = app.store_code(cw20_contract());
        let pair_code_id = app.store_code(pair_contract());
        let factory_code_id = app.store_code(factory_contract());
        let router_code_id = app.store_code(router_contract());

        let initial_amount = Uint128::new(1_000_000_000_000);

        let token_a = create_cw20_token(
            app,
            cw20_code_id,
            &user,
            "Token A",
            "TKNA",
            initial_amount,
        );
        let token_b = create_cw20_token(
            app,
            cw20_code_id,
            &user,
            "Token B",
            "TKNB",
            initial_amount,
        );

        let factory = app
            .instantiate_contract(
                factory_code_id,
                governance.clone(),
                &dex_common::factory::InstantiateMsg {
                    governance: governance.to_string(),
                    treasury: treasury.to_string(),
                    default_fee_bps: 30,
                    pair_code_id,
                    lp_token_code_id: cw20_code_id,
                    whitelisted_code_ids: vec![cw20_code_id],
                },
                &[],
                "factory",
                None,
            )
            .unwrap();

        let resp = app
            .execute_contract(
                user.clone(),
                factory.clone(),
                &dex_common::factory::ExecuteMsg::CreatePair {
                    asset_infos: [
                        asset_info_token(&token_a),
                        asset_info_token(&token_b),
                    ],
                },
                &[],
            )
            .unwrap();

        let pair = extract_pair_address(&resp.events);

        let pair_info: dex_common::types::PairInfo = app
            .wrap()
            .query_wasm_smart(
                pair.to_string(),
                &dex_common::pair::QueryMsg::Pair {},
            )
            .unwrap();
        let lp_token = pair_info.liquidity_token;

        let router = app
            .instantiate_contract(
                router_code_id,
                governance.clone(),
                &cl8y_dex_router::msg::InstantiateMsg {
                    factory: factory.to_string(),
                },
                &[],
                "router",
                None,
            )
            .unwrap();

        TestEnv {
            factory,
            token_a,
            token_b,
            pair,
            lp_token,
            router,
            governance,
            treasury,
            user,
        }
    }

    pub fn provide_liquidity(
        app: &mut App,
        env: &TestEnv,
        provider: &Addr,
        amount_a: Uint128,
        amount_b: Uint128,
    ) {
        app.execute_contract(
            provider.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                spender: env.pair.to_string(),
                amount: amount_a,
                expires: None,
            },
            &[],
        )
        .unwrap();

        app.execute_contract(
            provider.clone(),
            env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                spender: env.pair.to_string(),
                amount: amount_b,
                expires: None,
            },
            &[],
        )
        .unwrap();

        app.execute_contract(
            provider.clone(),
            env.pair.clone(),
            &dex_common::pair::ExecuteMsg::ProvideLiquidity {
                assets: [
                    dex_common::types::Asset {
                        info: asset_info_token(&env.token_a),
                        amount: amount_a,
                    },
                    dex_common::types::Asset {
                        info: asset_info_token(&env.token_b),
                        amount: amount_b,
                    },
                ],
                slippage_tolerance: None,
                receiver: None,
                deadline: None,
            },
            &[],
        )
        .unwrap();
    }
}

#[cfg(test)]
mod factory_tests {
    use super::helpers::*;
    use cosmwasm_std::{Addr, Uint128};
    use cw_multi_test::{App, Executor};
    use dex_common::types::AssetInfo;

    #[test]
    fn test_instantiate() {
        let mut app = App::default();
        let governance = Addr::unchecked("governance");
        let treasury = Addr::unchecked("treasury");

        let cw20_code_id = app.store_code(cw20_contract());
        let pair_code_id = app.store_code(pair_contract());
        let factory_code_id = app.store_code(factory_contract());

        let factory = app
            .instantiate_contract(
                factory_code_id,
                governance.clone(),
                &dex_common::factory::InstantiateMsg {
                    governance: governance.to_string(),
                    treasury: treasury.to_string(),
                    default_fee_bps: 30,
                    pair_code_id,
                    lp_token_code_id: cw20_code_id,
                    whitelisted_code_ids: vec![cw20_code_id],
                },
                &[],
                "factory",
                None,
            )
            .unwrap();

        let config: dex_common::factory::ConfigResponse = app
            .wrap()
            .query_wasm_smart(
                factory.to_string(),
                &dex_common::factory::QueryMsg::Config {},
            )
            .unwrap();

        assert_eq!(config.governance, governance);
        assert_eq!(config.treasury, treasury);
        assert_eq!(config.default_fee_bps, 30);
        assert_eq!(config.pair_code_id, pair_code_id);
        assert_eq!(config.lp_token_code_id, cw20_code_id);

        let count: dex_common::factory::PairCountResponse = app
            .wrap()
            .query_wasm_smart(
                factory.to_string(),
                &dex_common::factory::QueryMsg::GetPairCount {},
            )
            .unwrap();
        assert_eq!(count.count, 0);

        let code_ids: dex_common::factory::CodeIdsResponse = app
            .wrap()
            .query_wasm_smart(
                factory.to_string(),
                &dex_common::factory::QueryMsg::GetWhitelistedCodeIds {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        assert_eq!(code_ids.code_ids, vec![cw20_code_id]);
    }

    #[test]
    fn test_create_pair() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let count: dex_common::factory::PairCountResponse = app
            .wrap()
            .query_wasm_smart(
                env.factory.to_string(),
                &dex_common::factory::QueryMsg::GetPairCount {},
            )
            .unwrap();
        assert_eq!(count.count, 1);

        let pair_info: dex_common::types::PairInfo = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::Pair {},
            )
            .unwrap();

        let has_both = pair_info.asset_infos.iter().any(|a| {
            matches!(a, AssetInfo::Token { contract_addr } if contract_addr == &env.token_a.to_string())
        }) && pair_info.asset_infos.iter().any(|a| {
            matches!(a, AssetInfo::Token { contract_addr } if contract_addr == &env.token_b.to_string())
        });
        assert!(has_both, "pair should contain both token_a and token_b");
        assert_ne!(pair_info.liquidity_token, Addr::unchecked(""));

        let fee_config: dex_common::pair::FeeConfigResponse = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::GetFeeConfig {},
            )
            .unwrap();
        assert_eq!(fee_config.fee_config.fee_bps, 30);
        assert_eq!(fee_config.fee_config.treasury, env.treasury);
    }

    #[test]
    fn test_create_pair_native_token_rejected() {
        let mut app = App::default();
        let governance = Addr::unchecked("governance");
        let treasury = Addr::unchecked("treasury");
        let user = Addr::unchecked("user");

        let cw20_code_id = app.store_code(cw20_contract());
        let pair_code_id = app.store_code(pair_contract());
        let factory_code_id = app.store_code(factory_contract());

        let token_a = create_cw20_token(
            &mut app,
            cw20_code_id,
            &user,
            "Token A",
            "TKNA",
            Uint128::new(1_000_000),
        );

        let factory = app
            .instantiate_contract(
                factory_code_id,
                governance.clone(),
                &dex_common::factory::InstantiateMsg {
                    governance: governance.to_string(),
                    treasury: treasury.to_string(),
                    default_fee_bps: 30,
                    pair_code_id,
                    lp_token_code_id: cw20_code_id,
                    whitelisted_code_ids: vec![cw20_code_id],
                },
                &[],
                "factory",
                None,
            )
            .unwrap();

        let err = app
            .execute_contract(
                user.clone(),
                factory.clone(),
                &dex_common::factory::ExecuteMsg::CreatePair {
                    asset_infos: [
                        asset_info_token(&token_a),
                        AssetInfo::NativeToken {
                            denom: "uluna".to_string(),
                        },
                    ],
                },
                &[],
            )
            .unwrap_err();

        assert!(
            err.root_cause()
                .to_string()
                .contains("Native tokens are not supported")
        );
    }

    #[test]
    fn test_create_pair_not_whitelisted() {
        let mut app = App::default();
        let governance = Addr::unchecked("governance");
        let treasury = Addr::unchecked("treasury");
        let user = Addr::unchecked("user");

        let cw20_code_id = app.store_code(cw20_contract());
        let pair_code_id = app.store_code(pair_contract());
        let factory_code_id = app.store_code(factory_contract());

        let token_a = create_cw20_token(
            &mut app,
            cw20_code_id,
            &user,
            "Token A",
            "TKNA",
            Uint128::new(1_000_000),
        );
        let token_b = create_cw20_token(
            &mut app,
            cw20_code_id,
            &user,
            "Token B",
            "TKNB",
            Uint128::new(1_000_000),
        );

        let factory = app
            .instantiate_contract(
                factory_code_id,
                governance.clone(),
                &dex_common::factory::InstantiateMsg {
                    governance: governance.to_string(),
                    treasury: treasury.to_string(),
                    default_fee_bps: 30,
                    pair_code_id,
                    lp_token_code_id: cw20_code_id,
                    whitelisted_code_ids: vec![],
                },
                &[],
                "factory",
                None,
            )
            .unwrap();

        let err = app
            .execute_contract(
                user.clone(),
                factory.clone(),
                &dex_common::factory::ExecuteMsg::CreatePair {
                    asset_infos: [
                        asset_info_token(&token_a),
                        asset_info_token(&token_b),
                    ],
                },
                &[],
            )
            .unwrap_err();

        assert!(
            err.root_cause()
                .to_string()
                .contains("Code ID not whitelisted")
        );
    }

    #[test]
    fn test_create_pair_duplicate() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let err = app
            .execute_contract(
                env.user.clone(),
                env.factory.clone(),
                &dex_common::factory::ExecuteMsg::CreatePair {
                    asset_infos: [
                        asset_info_token(&env.token_a),
                        asset_info_token(&env.token_b),
                    ],
                },
                &[],
            )
            .unwrap_err();

        assert!(
            err.root_cause()
                .to_string()
                .contains("Pair already exists")
        );
    }

    #[test]
    fn test_add_remove_whitelist() {
        let mut app = App::default();
        let governance = Addr::unchecked("governance");
        let treasury = Addr::unchecked("treasury");

        let cw20_code_id = app.store_code(cw20_contract());
        let pair_code_id = app.store_code(pair_contract());
        let factory_code_id = app.store_code(factory_contract());

        let factory = app
            .instantiate_contract(
                factory_code_id,
                governance.clone(),
                &dex_common::factory::InstantiateMsg {
                    governance: governance.to_string(),
                    treasury: treasury.to_string(),
                    default_fee_bps: 30,
                    pair_code_id,
                    lp_token_code_id: cw20_code_id,
                    whitelisted_code_ids: vec![],
                },
                &[],
                "factory",
                None,
            )
            .unwrap();

        app.execute_contract(
            governance.clone(),
            factory.clone(),
            &dex_common::factory::ExecuteMsg::AddWhitelistedCodeId { code_id: 42 },
            &[],
        )
        .unwrap();

        let ids: dex_common::factory::CodeIdsResponse = app
            .wrap()
            .query_wasm_smart(
                factory.to_string(),
                &dex_common::factory::QueryMsg::GetWhitelistedCodeIds {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        assert!(ids.code_ids.contains(&42));

        app.execute_contract(
            governance.clone(),
            factory.clone(),
            &dex_common::factory::ExecuteMsg::RemoveWhitelistedCodeId { code_id: 42 },
            &[],
        )
        .unwrap();

        let ids: dex_common::factory::CodeIdsResponse = app
            .wrap()
            .query_wasm_smart(
                factory.to_string(),
                &dex_common::factory::QueryMsg::GetWhitelistedCodeIds {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();
        assert!(!ids.code_ids.contains(&42));
    }

    #[test]
    fn test_update_config() {
        let mut app = App::default();
        let governance = Addr::unchecked("governance");
        let new_governance = Addr::unchecked("new_governance");
        let treasury = Addr::unchecked("treasury");
        let new_treasury = Addr::unchecked("new_treasury");

        let cw20_code_id = app.store_code(cw20_contract());
        let pair_code_id = app.store_code(pair_contract());
        let factory_code_id = app.store_code(factory_contract());

        let factory = app
            .instantiate_contract(
                factory_code_id,
                governance.clone(),
                &dex_common::factory::InstantiateMsg {
                    governance: governance.to_string(),
                    treasury: treasury.to_string(),
                    default_fee_bps: 30,
                    pair_code_id,
                    lp_token_code_id: cw20_code_id,
                    whitelisted_code_ids: vec![],
                },
                &[],
                "factory",
                None,
            )
            .unwrap();

        app.execute_contract(
            governance.clone(),
            factory.clone(),
            &dex_common::factory::ExecuteMsg::UpdateConfig {
                governance: Some(new_governance.to_string()),
                treasury: Some(new_treasury.to_string()),
                default_fee_bps: Some(50),
            },
            &[],
        )
        .unwrap();

        let config: dex_common::factory::ConfigResponse = app
            .wrap()
            .query_wasm_smart(
                factory.to_string(),
                &dex_common::factory::QueryMsg::Config {},
            )
            .unwrap();

        assert_eq!(config.governance, new_governance);
        assert_eq!(config.treasury, new_treasury);
        assert_eq!(config.default_fee_bps, 50);
    }

    #[test]
    fn test_query_pairs_pagination() {
        let mut app = App::default();
        let governance = Addr::unchecked("governance");
        let treasury = Addr::unchecked("treasury");
        let user = Addr::unchecked("user");

        let cw20_code_id = app.store_code(cw20_contract());
        let pair_code_id = app.store_code(pair_contract());
        let factory_code_id = app.store_code(factory_contract());

        let factory = app
            .instantiate_contract(
                factory_code_id,
                governance.clone(),
                &dex_common::factory::InstantiateMsg {
                    governance: governance.to_string(),
                    treasury: treasury.to_string(),
                    default_fee_bps: 30,
                    pair_code_id,
                    lp_token_code_id: cw20_code_id,
                    whitelisted_code_ids: vec![cw20_code_id],
                },
                &[],
                "factory",
                None,
            )
            .unwrap();

        let initial_amount = Uint128::new(1_000_000);
        let symbols = ["PTKNA", "PTKNB", "PTKNC", "PTKND", "PTKNE", "PTKNF"];
        let tokens: Vec<Addr> = symbols
            .iter()
            .map(|sym| {
                create_cw20_token(
                    &mut app,
                    cw20_code_id,
                    &user,
                    sym,
                    sym,
                    initial_amount,
                )
            })
            .collect();

        for i in 0..3 {
            app.execute_contract(
                user.clone(),
                factory.clone(),
                &dex_common::factory::ExecuteMsg::CreatePair {
                    asset_infos: [
                        asset_info_token(&tokens[i * 2]),
                        asset_info_token(&tokens[i * 2 + 1]),
                    ],
                },
                &[],
            )
            .unwrap();
        }

        let count: dex_common::factory::PairCountResponse = app
            .wrap()
            .query_wasm_smart(
                factory.to_string(),
                &dex_common::factory::QueryMsg::GetPairCount {},
            )
            .unwrap();
        assert_eq!(count.count, 3);

        let page1: dex_common::factory::PairsResponse = app
            .wrap()
            .query_wasm_smart(
                factory.to_string(),
                &dex_common::factory::QueryMsg::Pairs {
                    start_after: None,
                    limit: Some(2),
                },
            )
            .unwrap();
        assert_eq!(page1.pairs.len(), 2);

        let page2: dex_common::factory::PairsResponse = app
            .wrap()
            .query_wasm_smart(
                factory.to_string(),
                &dex_common::factory::QueryMsg::Pairs {
                    start_after: Some(page1.pairs[1].asset_infos.clone()),
                    limit: Some(2),
                },
            )
            .unwrap();
        assert_eq!(page2.pairs.len(), 1);
    }
}

#[cfg(test)]
mod pair_tests {
    use super::helpers::*;
    use cosmwasm_std::{to_json_binary, Uint128};
    use cw_multi_test::{App, Executor};

    #[test]
    fn test_provide_liquidity_first() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let amount_a = Uint128::new(1_000_000);
        let amount_b = Uint128::new(1_000_000);

        provide_liquidity(&mut app, &env, &env.user, amount_a, amount_b);

        let pool: dex_common::pair::PoolResponse = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::Pool {},
            )
            .unwrap();
        assert_eq!(pool.assets[0].amount, amount_a);
        assert_eq!(pool.assets[1].amount, amount_b);
        assert_eq!(pool.total_share, Uint128::new(1_000_000));

        let lp_balance = query_cw20_balance(&app, &env.lp_token, &env.user);
        assert_eq!(lp_balance, Uint128::new(999_000));
    }

    #[test]
    fn test_swap() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(
            &mut app,
            &env,
            &env.user,
            Uint128::new(1_000_000),
            Uint128::new(1_000_000),
        );

        let user_b_before = query_cw20_balance(&app, &env.token_b, &env.user);
        let treasury_b_before = query_cw20_balance(&app, &env.token_b, &env.treasury);

        let swap_amount = Uint128::new(1_000);
        let swap_msg = to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: None,
            to: None,
            deadline: None,
        })
        .unwrap();

        app.execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: swap_amount,
                msg: swap_msg,
            },
            &[],
        )
        .unwrap();

        let user_b_after = query_cw20_balance(&app, &env.token_b, &env.user);
        let treasury_b_after = query_cw20_balance(&app, &env.token_b, &env.treasury);

        let net_output = user_b_after - user_b_before;
        let fee = treasury_b_after - treasury_b_before;
        assert_eq!(net_output, Uint128::new(997));
        assert_eq!(fee, Uint128::new(3));

        let pool: dex_common::pair::PoolResponse = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::Pool {},
            )
            .unwrap();
        assert_eq!(pool.assets[0].amount, Uint128::new(1_001_000));
        assert_eq!(pool.assets[1].amount, Uint128::new(999_000));
    }

    #[test]
    fn test_swap_max_spread() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(
            &mut app,
            &env,
            &env.user,
            Uint128::new(1_000_000),
            Uint128::new(1_000_000),
        );

        let swap_msg = to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: Some(cosmwasm_std::Decimal::permille(1)),
            to: None,
            deadline: None,
        })
        .unwrap();

        let err = app
            .execute_contract(
                env.user.clone(),
                env.token_a.clone(),
                &cw20::Cw20ExecuteMsg::Send {
                    contract: env.pair.to_string(),
                    amount: Uint128::new(100_000),
                    msg: swap_msg,
                },
                &[],
            )
            .unwrap_err();

        assert!(
            err.root_cause()
                .to_string()
                .contains("Max spread assertion")
        );
    }

    #[test]
    fn test_withdraw_liquidity() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let amount = Uint128::new(1_000_000);
        provide_liquidity(&mut app, &env, &env.user, amount, amount);

        let lp_balance = query_cw20_balance(&app, &env.lp_token, &env.user);
        assert_eq!(lp_balance, Uint128::new(999_000));

        let user_a_before = query_cw20_balance(&app, &env.token_a, &env.user);
        let user_b_before = query_cw20_balance(&app, &env.token_b, &env.user);

        let remove_amount = Uint128::new(500_000);
        let remove_msg =
            to_json_binary(&dex_common::pair::Cw20HookMsg::WithdrawLiquidity {}).unwrap();

        app.execute_contract(
            env.user.clone(),
            env.lp_token.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: remove_amount,
                msg: remove_msg,
            },
            &[],
        )
        .unwrap();

        let user_a_after = query_cw20_balance(&app, &env.token_a, &env.user);
        let user_b_after = query_cw20_balance(&app, &env.token_b, &env.user);

        assert_eq!(user_a_after - user_a_before, Uint128::new(500_000));
        assert_eq!(user_b_after - user_b_before, Uint128::new(500_000));

        let lp_balance = query_cw20_balance(&app, &env.lp_token, &env.user);
        assert_eq!(lp_balance, Uint128::new(499_000));

        let pool: dex_common::pair::PoolResponse = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::Pool {},
            )
            .unwrap();
        assert_eq!(pool.assets[0].amount, Uint128::new(500_000));
        assert_eq!(pool.assets[1].amount, Uint128::new(500_000));
    }

    #[test]
    fn test_simulation() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(
            &mut app,
            &env,
            &env.user,
            Uint128::new(1_000_000),
            Uint128::new(1_000_000),
        );

        let sim: dex_common::pair::SimulationResponse = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::Simulation {
                    offer_asset: dex_common::types::Asset {
                        info: asset_info_token(&env.token_a),
                        amount: Uint128::new(1_000),
                    },
                },
            )
            .unwrap();

        assert_eq!(sim.return_amount, Uint128::new(997));
        assert_eq!(sim.commission_amount, Uint128::new(3));
    }

    #[test]
    fn test_reverse_simulation() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(
            &mut app,
            &env,
            &env.user,
            Uint128::new(1_000_000),
            Uint128::new(1_000_000),
        );

        let rsim: dex_common::pair::ReverseSimulationResponse = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::ReverseSimulation {
                    ask_asset: dex_common::types::Asset {
                        info: asset_info_token(&env.token_b),
                        amount: Uint128::new(997),
                    },
                },
            )
            .unwrap();

        assert!(
            rsim.offer_amount >= Uint128::new(1_000),
            "reverse simulation offer_amount should be >= 1000, got {}",
            rsim.offer_amount
        );
    }

    #[test]
    fn test_update_fee() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        app.execute_contract(
            env.governance.clone(),
            env.factory.clone(),
            &dex_common::factory::ExecuteMsg::SetPairFee {
                pair: env.pair.to_string(),
                fee_bps: 100,
            },
            &[],
        )
        .unwrap();

        let fee_config: dex_common::pair::FeeConfigResponse = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::GetFeeConfig {},
            )
            .unwrap();
        assert_eq!(fee_config.fee_config.fee_bps, 100);
    }

    #[test]
    fn test_unauthorized_update() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let random_user = cosmwasm_std::Addr::unchecked("random_user");

        let err = app
            .execute_contract(
                random_user,
                env.pair.clone(),
                &dex_common::pair::ExecuteMsg::UpdateFee { fee_bps: 100 },
                &[],
            )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }
}

#[cfg(test)]
mod router_tests {
    use super::helpers::*;
    use cosmwasm_std::{to_json_binary, Uint128};
    use cw_multi_test::{App, Executor};

    #[test]
    fn test_single_hop_swap() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(
            &mut app,
            &env,
            &env.user,
            Uint128::new(1_000_000),
            Uint128::new(1_000_000),
        );

        let user_b_before = query_cw20_balance(&app, &env.token_b, &env.user);

        let swap_amount = Uint128::new(1_000);
        let hook_msg = to_json_binary(&cl8y_dex_router::msg::Cw20HookMsg::ExecuteSwapOperations {
            operations: vec![cl8y_dex_router::msg::SwapOperation::TerraSwap {
                offer_asset_info: asset_info_token(&env.token_a),
                ask_asset_info: asset_info_token(&env.token_b),
            }],
            minimum_receive: None,
            to: None,
            deadline: None,
        })
        .unwrap();

        app.execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.router.to_string(),
                amount: swap_amount,
                msg: hook_msg,
            },
            &[],
        )
        .unwrap();

        let user_b_after = query_cw20_balance(&app, &env.token_b, &env.user);
        assert!(
            user_b_after > user_b_before,
            "user should have received output tokens via router"
        );

        let net_output = user_b_after - user_b_before;
        assert_eq!(net_output, Uint128::new(997));
    }

    #[test]
    fn test_native_swap_rejected() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(
            &mut app,
            &env,
            &env.user,
            Uint128::new(1_000_000),
            Uint128::new(1_000_000),
        );

        let hook_msg = to_json_binary(&cl8y_dex_router::msg::Cw20HookMsg::ExecuteSwapOperations {
            operations: vec![cl8y_dex_router::msg::SwapOperation::NativeSwap {
                offer_denom: "uluna".to_string(),
                ask_denom: "uusd".to_string(),
            }],
            minimum_receive: None,
            to: None,
            deadline: None,
        })
        .unwrap();

        let err = app
            .execute_contract(
                env.user.clone(),
                env.token_a.clone(),
                &cw20::Cw20ExecuteMsg::Send {
                    contract: env.router.to_string(),
                    amount: Uint128::new(1_000),
                    msg: hook_msg,
                },
                &[],
            )
            .unwrap_err();

        assert!(
            err.root_cause()
                .to_string()
                .contains("Native token swaps are not supported")
        );
    }
}
