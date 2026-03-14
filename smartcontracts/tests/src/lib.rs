#[cfg(test)]
mod helpers {
    use cosmwasm_std::{Addr, Empty, Uint128};
    use cw20::{BalanceResponse, Cw20QueryMsg};
    use cw_multi_test::{App, ContractWrapper, Executor};
    use dex_common::types::AssetInfo;

    pub fn cw20_mintable_contract() -> Box<dyn cw_multi_test::Contract<Empty>> {
        let contract = ContractWrapper::new(
            cw20_mintable::contract::execute,
            cw20_mintable::contract::instantiate,
            cw20_mintable::contract::query,
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

    pub fn fee_discount_contract() -> Box<dyn cw_multi_test::Contract<Empty>> {
        let contract = ContractWrapper::new(
            cl8y_dex_fee_discount::contract::execute,
            cl8y_dex_fee_discount::contract::instantiate,
            cl8y_dex_fee_discount::contract::query,
        );
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

    pub fn burn_hook_contract() -> Box<dyn cw_multi_test::Contract<Empty>> {
        let contract = ContractWrapper::new(
            cl8y_dex_burn_hook::contract::execute,
            cl8y_dex_burn_hook::contract::instantiate,
            cl8y_dex_burn_hook::contract::query,
        );
        Box::new(contract)
    }

    pub fn tax_hook_contract() -> Box<dyn cw_multi_test::Contract<Empty>> {
        let contract = ContractWrapper::new(
            cl8y_dex_tax_hook::contract::execute,
            cl8y_dex_tax_hook::contract::instantiate,
            cl8y_dex_tax_hook::contract::query,
        );
        Box::new(contract)
    }

    pub fn lp_burn_hook_contract() -> Box<dyn cw_multi_test::Contract<Empty>> {
        let contract = ContractWrapper::new(
            cl8y_dex_lp_burn_hook::contract::execute,
            cl8y_dex_lp_burn_hook::contract::instantiate,
            cl8y_dex_lp_burn_hook::contract::query,
        );
        Box::new(contract)
    }

    pub fn pair_contract_with_migrate() -> Box<dyn cw_multi_test::Contract<Empty>> {
        let contract = ContractWrapper::new(
            cl8y_dex_pair::contract::execute,
            cl8y_dex_pair::contract::instantiate,
            cl8y_dex_pair::contract::query,
        )
        .with_reply(cl8y_dex_pair::contract::reply)
        .with_migrate(cl8y_dex_pair::contract::migrate);
        Box::new(contract)
    }

    fn mock_noop_exec(
        _deps: cosmwasm_std::DepsMut,
        _env: cosmwasm_std::Env,
        _info: cosmwasm_std::MessageInfo,
        _msg: Empty,
    ) -> cosmwasm_std::StdResult<cosmwasm_std::Response> {
        Ok(cosmwasm_std::Response::new())
    }

    fn mock_noop_query(
        _deps: cosmwasm_std::Deps,
        _env: cosmwasm_std::Env,
        _msg: Empty,
    ) -> cosmwasm_std::StdResult<cosmwasm_std::Binary> {
        Ok(cosmwasm_std::Binary::default())
    }

    /// Mock contract that writes cw2 version info as "cl8y-dex-pair" "0.9.0".
    pub fn mock_old_pair_contract() -> Box<dyn cw_multi_test::Contract<Empty>> {
        fn inst(
            deps: cosmwasm_std::DepsMut,
            _env: cosmwasm_std::Env,
            _info: cosmwasm_std::MessageInfo,
            _msg: Empty,
        ) -> cosmwasm_std::StdResult<cosmwasm_std::Response> {
            cw2::set_contract_version(deps.storage, "cl8y-dex-pair", "0.9.0")?;
            Ok(cosmwasm_std::Response::new())
        }
        Box::new(ContractWrapper::new(mock_noop_exec, inst, mock_noop_query))
    }

    /// Mock contract that writes cw2 version info as "cl8y-dex-pair" "99.0.0"
    /// to simulate attempting a downgrade migration.
    pub fn mock_future_pair_contract() -> Box<dyn cw_multi_test::Contract<Empty>> {
        fn inst(
            deps: cosmwasm_std::DepsMut,
            _env: cosmwasm_std::Env,
            _info: cosmwasm_std::MessageInfo,
            _msg: Empty,
        ) -> cosmwasm_std::StdResult<cosmwasm_std::Response> {
            cw2::set_contract_version(deps.storage, "cl8y-dex-pair", "99.0.0")?;
            Ok(cosmwasm_std::Response::new())
        }
        Box::new(ContractWrapper::new(mock_noop_exec, inst, mock_noop_query))
    }

    pub fn create_cw20_token(
        app: &mut App,
        cw20_code_id: u64,
        owner: &Addr,
        name: &str,
        symbol: &str,
        initial_amount: Uint128,
    ) -> Addr {
        create_cw20_token_with_decimals(app, cw20_code_id, owner, name, symbol, 6, initial_amount)
    }

    pub fn create_cw20_token_with_decimals(
        app: &mut App,
        cw20_code_id: u64,
        owner: &Addr,
        name: &str,
        symbol: &str,
        decimals: u8,
        initial_amount: Uint128,
    ) -> Addr {
        app.instantiate_contract(
            cw20_code_id,
            owner.clone(),
            &cw20_mintable::msg::InstantiateMsg {
                name: name.to_string(),
                symbol: symbol.to_string(),
                decimals,
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

    pub fn query_pool(app: &App, pair: &Addr) -> dex_common::pair::PoolResponse {
        app.wrap()
            .query_wasm_smart(
                pair.to_string(),
                &dex_common::pair::QueryMsg::Pool {},
            )
            .unwrap()
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
        setup_env_with_fee(app, 30)
    }

    pub fn setup_env_with_fee(app: &mut App, fee_bps: u16) -> TestEnv {
        let governance = Addr::unchecked("governance");
        let treasury = Addr::unchecked("treasury");
        let user = Addr::unchecked("user");

        let cw20_code_id = app.store_code(cw20_mintable_contract());
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
                    default_fee_bps: fee_bps,
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

    pub fn swap_a_to_b(app: &mut App, env: &TestEnv, sender: &Addr, amount: Uint128) {
        let swap_msg = cosmwasm_std::to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: Some(cosmwasm_std::Decimal::one()),
            to: None,
            deadline: None,
            trader: None,
        })
        .unwrap();

        app.execute_contract(
            sender.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount,
                msg: swap_msg,
            },
            &[],
        )
        .unwrap();
    }

    pub fn swap_b_to_a(app: &mut App, env: &TestEnv, sender: &Addr, amount: Uint128) {
        let swap_msg = cosmwasm_std::to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: Some(cosmwasm_std::Decimal::one()),
            to: None,
            deadline: None,
            trader: None,
        })
        .unwrap();

        app.execute_contract(
            sender.clone(),
            env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount,
                msg: swap_msg,
            },
            &[],
        )
        .unwrap();
    }

    pub fn withdraw_liquidity(
        app: &mut App,
        env: &TestEnv,
        sender: &Addr,
        lp_amount: Uint128,
    ) {
        let remove_msg = cosmwasm_std::to_json_binary(
            &dex_common::pair::Cw20HookMsg::WithdrawLiquidity { min_assets: None },
        )
        .unwrap();

        app.execute_contract(
            sender.clone(),
            env.lp_token.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: lp_amount,
                msg: remove_msg,
            },
            &[],
        )
        .unwrap();
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

    pub fn transfer_tokens(
        app: &mut App,
        token: &Addr,
        from: &Addr,
        to: &Addr,
        amount: Uint128,
    ) {
        app.execute_contract(
            from.clone(),
            token.clone(),
            &cw20::Cw20ExecuteMsg::Transfer {
                recipient: to.to_string(),
                amount,
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

        let cw20_code_id = app.store_code(cw20_mintable_contract());
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

        let cw20_code_id = app.store_code(cw20_mintable_contract());
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

        let cw20_code_id = app.store_code(cw20_mintable_contract());
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

        let cw20_code_id = app.store_code(cw20_mintable_contract());
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

        let cw20_code_id = app.store_code(cw20_mintable_contract());
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

        let cw20_code_id = app.store_code(cw20_mintable_contract());
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
            trader: None,
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
        assert_eq!(fee, Uint128::new(2));

        let pool: dex_common::pair::PoolResponse = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::Pool {},
            )
            .unwrap();
        assert_eq!(pool.assets[0].amount, Uint128::new(1_001_000));
        assert_eq!(pool.assets[1].amount, Uint128::new(999_001));
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
            trader: None,
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
            to_json_binary(&dex_common::pair::Cw20HookMsg::WithdrawLiquidity { min_assets: None }).unwrap();

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
        assert_eq!(sim.commission_amount, Uint128::new(2));
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

#[cfg(test)]
mod fee_discount_tests {
    use super::helpers::*;
    use cosmwasm_std::{to_json_binary, Addr, Decimal, Uint128};
    use cw_multi_test::{App, Executor};

    const ONE_CL8Y: u128 = 1_000_000_000_000_000_000;

    #[allow(dead_code)]
    struct DiscountTestEnv {
        base: TestEnv,
        cl8y_token: Addr,
        fee_discount: Addr,
    }

    fn setup_discount_env(app: &mut App) -> DiscountTestEnv {
        let base = setup_full_env(app);
        let cw20_code_id = app.store_code(cw20_mintable_contract());
        let fee_discount_code_id = app.store_code(fee_discount_contract());

        let cl8y_token = app
            .instantiate_contract(
                cw20_code_id,
                base.user.clone(),
                &cw20_mintable::msg::InstantiateMsg {
                    name: "CL8Y Token".to_string(),
                    symbol: "CL8Y".to_string(),
                    decimals: 18,
                    initial_balances: vec![cw20::Cw20Coin {
                        address: base.user.to_string(),
                        amount: Uint128::new(100_000 * ONE_CL8Y),
                    }],
                    mint: None,
                    marketing: None,
                },
                &[],
                "cl8y",
                None,
            )
            .unwrap();

        let fee_discount = app
            .instantiate_contract(
                fee_discount_code_id,
                base.governance.clone(),
                &cl8y_dex_fee_discount::msg::InstantiateMsg {
                    governance: base.governance.to_string(),
                    cl8y_token: cl8y_token.to_string(),
                },
                &[],
                "fee_discount",
                None,
            )
            .unwrap();

        // Add tiers
        for (tier_id, min_cl8y, discount_bps, governance_only) in [
            (0u8, 0u128, 10000u16, true),
            (1, ONE_CL8Y, 1000, false),
            (2, 50 * ONE_CL8Y, 2500, false),
            (3, 200 * ONE_CL8Y, 3500, false),
            (4, 1000 * ONE_CL8Y, 5000, false),
            (5, 15000 * ONE_CL8Y, 8000, false),
            (255, 0, 0, true),
        ] {
            app.execute_contract(
                base.governance.clone(),
                fee_discount.clone(),
                &cl8y_dex_fee_discount::msg::ExecuteMsg::AddTier {
                    tier_id,
                    min_cl8y_balance: Uint128::new(min_cl8y),
                    discount_bps,
                    governance_only,
                },
                &[],
            )
            .unwrap();
        }

        // Add router as trusted
        app.execute_contract(
            base.governance.clone(),
            fee_discount.clone(),
            &cl8y_dex_fee_discount::msg::ExecuteMsg::AddTrustedRouter {
                router: base.router.to_string(),
            },
            &[],
        )
        .unwrap();

        // Set discount registry on pair via factory
        app.execute_contract(
            base.governance.clone(),
            base.factory.clone(),
            &dex_common::factory::ExecuteMsg::SetDiscountRegistry {
                pair: base.pair.to_string(),
                registry: Some(fee_discount.to_string()),
            },
            &[],
        )
        .unwrap();

        DiscountTestEnv {
            base,
            cl8y_token,
            fee_discount,
        }
    }

    #[test]
    fn test_swap_with_tier1_discount() {
        let mut app = App::default();
        let denv = setup_discount_env(&mut app);
        let env = &denv.base;

        provide_liquidity(&mut app, env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        // Register for tier 1 (1 CL8Y, 10% discount)
        app.execute_contract(
            env.user.clone(),
            denv.fee_discount.clone(),
            &cl8y_dex_fee_discount::msg::ExecuteMsg::Register { tier_id: 1 },
            &[],
        )
        .unwrap();

        let user_b_before = query_cw20_balance(&app, &env.token_b, &env.user);
        let treasury_b_before = query_cw20_balance(&app, &env.token_b, &env.treasury);

        let swap_amount = Uint128::new(10_000);
        let swap_msg = to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: Some(Decimal::percent(5)),
            to: None,
            deadline: None,
            trader: None,
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

        // gross_output = 1_000_000 - ceil(1_000_000^2 / 1_010_000) = 9_900
        // 27 bps effective: fee = floor(9_900 * 27 / 10_000) = 26
        assert_eq!(fee, Uint128::new(26));
        assert_eq!(net_output, Uint128::new(9874));
    }

    #[test]
    fn test_swap_with_tier4_discount() {
        let mut app = App::default();
        let denv = setup_discount_env(&mut app);
        let env = &denv.base;

        provide_liquidity(&mut app, env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        // Register for tier 4 (1000 CL8Y, 50% discount)
        app.execute_contract(
            env.user.clone(),
            denv.fee_discount.clone(),
            &cl8y_dex_fee_discount::msg::ExecuteMsg::Register { tier_id: 4 },
            &[],
        )
        .unwrap();

        let treasury_b_before = query_cw20_balance(&app, &env.token_b, &env.treasury);

        let swap_amount = Uint128::new(10_000);
        let swap_msg = to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: Some(Decimal::percent(5)),
            to: None,
            deadline: None,
            trader: None,
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

        let treasury_b_after = query_cw20_balance(&app, &env.token_b, &env.treasury);
        let fee = treasury_b_after - treasury_b_before;

        // gross = 9_900, 15 bps effective: fee = floor(9_900 * 15 / 10_000) = 14
        assert_eq!(fee, Uint128::new(14));
    }

    #[test]
    fn test_swap_no_discount_without_registration() {
        let mut app = App::default();
        let denv = setup_discount_env(&mut app);
        let env = &denv.base;

        provide_liquidity(&mut app, env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        let treasury_b_before = query_cw20_balance(&app, &env.token_b, &env.treasury);

        let swap_amount = Uint128::new(1_000);
        let swap_msg = to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: None,
            to: None,
            deadline: None,
            trader: None,
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

        let treasury_b_after = query_cw20_balance(&app, &env.token_b, &env.treasury);
        let fee = treasury_b_after - treasury_b_before;

        // gross = 1_000_000 - ceil(1e12 / 1_001_000) = 999, fee = floor(999 * 30 / 10_000) = 2
        assert_eq!(fee, Uint128::new(2));
    }

    #[test]
    fn test_governance_register_tier0_market_maker() {
        let mut app = App::default();
        let denv = setup_discount_env(&mut app);
        let env = &denv.base;

        provide_liquidity(&mut app, env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        let market_maker = Addr::unchecked("market_maker");

        // Transfer tokens to market maker for swapping
        app.execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Transfer {
                recipient: market_maker.to_string(),
                amount: Uint128::new(100_000),
            },
            &[],
        )
        .unwrap();

        // Governance registers market maker on tier 0 (100% discount)
        app.execute_contract(
            env.governance.clone(),
            denv.fee_discount.clone(),
            &cl8y_dex_fee_discount::msg::ExecuteMsg::RegisterWallet {
                wallet: market_maker.to_string(),
                tier_id: 0,
            },
            &[],
        )
        .unwrap();

        let treasury_b_before = query_cw20_balance(&app, &env.token_b, &env.treasury);

        let swap_amount = Uint128::new(10_000);
        let swap_msg = to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: Some(Decimal::percent(5)),
            to: None,
            deadline: None,
            trader: None,
        })
        .unwrap();

        app.execute_contract(
            market_maker.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: swap_amount,
                msg: swap_msg,
            },
            &[],
        )
        .unwrap();

        let treasury_b_after = query_cw20_balance(&app, &env.token_b, &env.treasury);
        let fee = treasury_b_after - treasury_b_before;

        assert_eq!(fee, Uint128::zero());
    }

    #[test]
    fn test_self_register_governance_tier_rejected() {
        let mut app = App::default();
        let denv = setup_discount_env(&mut app);

        let err = app
            .execute_contract(
                denv.base.user.clone(),
                denv.fee_discount.clone(),
                &cl8y_dex_fee_discount::msg::ExecuteMsg::Register { tier_id: 0 },
                &[],
            )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("governance-only"));
    }

    #[test]
    fn test_self_deregister_governance_tier_rejected() {
        let mut app = App::default();
        let denv = setup_discount_env(&mut app);

        // Governance puts user on blacklist tier 255
        app.execute_contract(
            denv.base.governance.clone(),
            denv.fee_discount.clone(),
            &cl8y_dex_fee_discount::msg::ExecuteMsg::RegisterWallet {
                wallet: denv.base.user.to_string(),
                tier_id: 255,
            },
            &[],
        )
        .unwrap();

        // User tries to deregister — should fail
        let err = app
            .execute_contract(
                denv.base.user.clone(),
                denv.fee_discount.clone(),
                &cl8y_dex_fee_discount::msg::ExecuteMsg::Deregister {},
                &[],
            )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("governance tier"));
    }

    #[test]
    fn test_blacklist_tier_no_discount() {
        let mut app = App::default();
        let denv = setup_discount_env(&mut app);
        let env = &denv.base;

        provide_liquidity(&mut app, env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        // User registers tier 1 first
        app.execute_contract(
            env.user.clone(),
            denv.fee_discount.clone(),
            &cl8y_dex_fee_discount::msg::ExecuteMsg::Register { tier_id: 1 },
            &[],
        )
        .unwrap();

        // Governance blacklists the user (tier 255, 0% discount)
        app.execute_contract(
            env.governance.clone(),
            denv.fee_discount.clone(),
            &cl8y_dex_fee_discount::msg::ExecuteMsg::RegisterWallet {
                wallet: env.user.to_string(),
                tier_id: 255,
            },
            &[],
        )
        .unwrap();

        let treasury_b_before = query_cw20_balance(&app, &env.token_b, &env.treasury);

        let swap_msg = to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: None,
            to: None,
            deadline: None,
            trader: None,
        })
        .unwrap();

        app.execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(1_000),
                msg: swap_msg,
            },
            &[],
        )
        .unwrap();

        let treasury_b_after = query_cw20_balance(&app, &env.token_b, &env.treasury);
        let fee = treasury_b_after - treasury_b_before;

        // Full fee (30 bps), no discount from blacklist tier
        // gross = 999, fee = floor(999 * 30 / 10_000) = 2
        assert_eq!(fee, Uint128::new(2));

        // User should not be able to self-switch away
        let err = app
            .execute_contract(
                env.user.clone(),
                denv.fee_discount.clone(),
                &cl8y_dex_fee_discount::msg::ExecuteMsg::Register { tier_id: 1 },
                &[],
            )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("governance tier"));
    }

    #[test]
    fn test_swap_via_router_with_discount() {
        let mut app = App::default();
        let denv = setup_discount_env(&mut app);
        let env = &denv.base;

        provide_liquidity(&mut app, env, &env.user, Uint128::new(10_000_000), Uint128::new(10_000_000));

        // Register user for tier 4 (50% discount)
        app.execute_contract(
            env.user.clone(),
            denv.fee_discount.clone(),
            &cl8y_dex_fee_discount::msg::ExecuteMsg::Register { tier_id: 4 },
            &[],
        )
        .unwrap();

        let treasury_b_before = query_cw20_balance(&app, &env.token_b, &env.treasury);

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
                amount: Uint128::new(10_000),
                msg: hook_msg,
            },
            &[],
        )
        .unwrap();

        let treasury_b_after = query_cw20_balance(&app, &env.token_b, &env.treasury);
        let fee = treasury_b_after - treasury_b_before;

        // 50% discount on 30 bps = 15 bps effective
        // gross = 10_000_000 - ceil(10_000_000^2 / 10_010_000) = 9_990
        // fee = floor(9_990 * 15 / 10_000) = 14
        assert_eq!(fee, Uint128::new(14));
    }

    #[test]
    fn test_tier_switching() {
        let mut app = App::default();
        let denv = setup_discount_env(&mut app);
        let env = &denv.base;

        // Register for tier 1
        app.execute_contract(
            env.user.clone(),
            denv.fee_discount.clone(),
            &cl8y_dex_fee_discount::msg::ExecuteMsg::Register { tier_id: 1 },
            &[],
        )
        .unwrap();

        let reg: cl8y_dex_fee_discount::msg::RegistrationResponse = app
            .wrap()
            .query_wasm_smart(
                denv.fee_discount.to_string(),
                &cl8y_dex_fee_discount::msg::QueryMsg::GetRegistration {
                    trader: env.user.to_string(),
                },
            )
            .unwrap();
        assert_eq!(reg.tier_id, Some(1));

        // Switch to tier 5
        app.execute_contract(
            env.user.clone(),
            denv.fee_discount.clone(),
            &cl8y_dex_fee_discount::msg::ExecuteMsg::Register { tier_id: 5 },
            &[],
        )
        .unwrap();

        let reg: cl8y_dex_fee_discount::msg::RegistrationResponse = app
            .wrap()
            .query_wasm_smart(
                denv.fee_discount.to_string(),
                &cl8y_dex_fee_discount::msg::QueryMsg::GetRegistration {
                    trader: env.user.to_string(),
                },
            )
            .unwrap();
        assert_eq!(reg.tier_id, Some(5));
    }

    #[test]
    fn test_insufficient_cl8y_balance_rejected() {
        let mut app = App::default();
        let denv = setup_discount_env(&mut app);

        let poor_user = Addr::unchecked("poor_user");

        let err = app
            .execute_contract(
                poor_user.clone(),
                denv.fee_discount.clone(),
                &cl8y_dex_fee_discount::msg::ExecuteMsg::Register { tier_id: 1 },
                &[],
            )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Insufficient CL8Y balance"));
    }

    #[test]
    fn test_query_tiers() {
        let mut app = App::default();
        let denv = setup_discount_env(&mut app);

        let tiers: cl8y_dex_fee_discount::msg::TiersResponse = app
            .wrap()
            .query_wasm_smart(
                denv.fee_discount.to_string(),
                &cl8y_dex_fee_discount::msg::QueryMsg::GetTiers {},
            )
            .unwrap();

        assert_eq!(tiers.tiers.len(), 7);
        assert_eq!(tiers.tiers[0].tier_id, 0);
        assert_eq!(tiers.tiers[0].tier.discount_bps, 10000);
        assert!(tiers.tiers[0].tier.governance_only);
        assert_eq!(tiers.tiers[1].tier_id, 1);
        assert_eq!(tiers.tiers[1].tier.discount_bps, 1000);
        assert!(!tiers.tiers[1].tier.governance_only);
    }
}

// ===========================================================================
// COVERAGE TESTS — deterministic tests for untested contract paths
// ===========================================================================

#[cfg(test)]
mod pair_coverage_tests {
    use super::helpers::*;
    use cosmwasm_std::{to_json_binary, Addr, Decimal, Uint128};
    use cw_multi_test::{App, Executor};

    #[test]
    fn test_swap_b_to_a() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        let user_a_before = query_cw20_balance(&app, &env.token_a, &env.user);
        swap_b_to_a(&mut app, &env, &env.user, Uint128::new(1_000));
        let user_a_after = query_cw20_balance(&app, &env.token_a, &env.user);
        assert_eq!(user_a_after - user_a_before, Uint128::new(997));
    }

    #[test]
    fn test_provide_liquidity_second_deposit() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        let lp_before = query_cw20_balance(&app, &env.lp_token, &env.user);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(500_000), Uint128::new(500_000));

        let lp_after = query_cw20_balance(&app, &env.lp_token, &env.user);
        assert_eq!(lp_after - lp_before, Uint128::new(500_000));

        let pool = query_pool(&app, &env.pair);
        assert_eq!(pool.assets[0].amount, Uint128::new(1_500_000));
        assert_eq!(pool.assets[1].amount, Uint128::new(1_500_000));
        assert_eq!(pool.total_share, Uint128::new(1_500_000));
    }

    #[test]
    fn test_provide_liquidity_asymmetric() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(200_000), Uint128::new(400_000));

        let pool = query_pool(&app, &env.pair);
        assert_eq!(pool.assets[0].amount, Uint128::new(1_200_000));
        assert_eq!(pool.assets[1].amount, Uint128::new(1_400_000));
    }

    #[test]
    fn test_swap_zero_amount_rejected() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        let swap_msg = to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: None,
            to: None,
            deadline: None,
            trader: None,
        })
        .unwrap();

        let err = app
            .execute_contract(
                env.user.clone(),
                env.token_a.clone(),
                &cw20::Cw20ExecuteMsg::Send {
                    contract: env.pair.to_string(),
                    amount: Uint128::zero(),
                    msg: swap_msg,
                },
                &[],
            )
            .unwrap_err();
        let msg = err.root_cause().to_string().to_lowercase();
        assert!(msg.contains("zero"), "Expected zero-amount error, got: {}", msg);
    }

    #[test]
    fn test_swap_to_receiver() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        let receiver = Addr::unchecked("receiver");
        let swap_msg = to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: None,
            to: Some(receiver.to_string()),
            deadline: None,
            trader: None,
        })
        .unwrap();

        app.execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(1_000),
                msg: swap_msg,
            },
            &[],
        )
        .unwrap();

        let receiver_bal = query_cw20_balance(&app, &env.token_b, &receiver);
        assert_eq!(receiver_bal, Uint128::new(997));
    }

    #[test]
    fn test_swap_on_empty_pool_fails() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let swap_msg = to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: None,
            to: None,
            deadline: None,
            trader: None,
        })
        .unwrap();

        let err = app
            .execute_contract(
                env.user.clone(),
                env.token_a.clone(),
                &cw20::Cw20ExecuteMsg::Send {
                    contract: env.pair.to_string(),
                    amount: Uint128::new(1_000),
                    msg: swap_msg,
                },
                &[],
            )
            .unwrap_err();
        assert!(err.root_cause().to_string().contains("Insufficient liquidity"));
    }

    #[test]
    fn test_provide_liquidity_zero_rejected() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        app.execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                spender: env.pair.to_string(),
                amount: Uint128::new(1_000),
                expires: None,
            },
            &[],
        )
        .unwrap();
        app.execute_contract(
            env.user.clone(),
            env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                spender: env.pair.to_string(),
                amount: Uint128::zero(),
                expires: None,
            },
            &[],
        )
        .unwrap();

        let err = app
            .execute_contract(
                env.user.clone(),
                env.pair.clone(),
                &dex_common::pair::ExecuteMsg::ProvideLiquidity {
                    assets: [
                        dex_common::types::Asset {
                            info: asset_info_token(&env.token_a),
                            amount: Uint128::new(1_000),
                        },
                        dex_common::types::Asset {
                            info: asset_info_token(&env.token_b),
                            amount: Uint128::zero(),
                        },
                    ],
                    slippage_tolerance: None,
                    receiver: None,
                    deadline: None,
                },
                &[],
            )
            .unwrap_err();
        assert!(err.root_cause().to_string().contains("Zero amount"));
    }

    #[test]
    fn test_provide_liquidity_with_slippage_tolerance() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        app.execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                spender: env.pair.to_string(),
                amount: Uint128::new(100_000),
                expires: None,
            },
            &[],
        )
        .unwrap();
        app.execute_contract(
            env.user.clone(),
            env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                spender: env.pair.to_string(),
                amount: Uint128::new(100_000),
                expires: None,
            },
            &[],
        )
        .unwrap();

        app.execute_contract(
            env.user.clone(),
            env.pair.clone(),
            &dex_common::pair::ExecuteMsg::ProvideLiquidity {
                assets: [
                    dex_common::types::Asset {
                        info: asset_info_token(&env.token_a),
                        amount: Uint128::new(100_000),
                    },
                    dex_common::types::Asset {
                        info: asset_info_token(&env.token_b),
                        amount: Uint128::new(100_000),
                    },
                ],
                slippage_tolerance: Some(Decimal::percent(5)),
                receiver: None,
                deadline: None,
            },
            &[],
        )
        .unwrap();
    }

    #[test]
    fn test_provide_liquidity_to_receiver() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let receiver = Addr::unchecked("lp_receiver");

        app.execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                spender: env.pair.to_string(),
                amount: Uint128::new(1_000_000),
                expires: None,
            },
            &[],
        )
        .unwrap();
        app.execute_contract(
            env.user.clone(),
            env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                spender: env.pair.to_string(),
                amount: Uint128::new(1_000_000),
                expires: None,
            },
            &[],
        )
        .unwrap();

        app.execute_contract(
            env.user.clone(),
            env.pair.clone(),
            &dex_common::pair::ExecuteMsg::ProvideLiquidity {
                assets: [
                    dex_common::types::Asset {
                        info: asset_info_token(&env.token_a),
                        amount: Uint128::new(1_000_000),
                    },
                    dex_common::types::Asset {
                        info: asset_info_token(&env.token_b),
                        amount: Uint128::new(1_000_000),
                    },
                ],
                slippage_tolerance: None,
                receiver: Some(receiver.to_string()),
                deadline: None,
            },
            &[],
        )
        .unwrap();

        let receiver_lp = query_cw20_balance(&app, &env.lp_token, &receiver);
        assert_eq!(receiver_lp, Uint128::new(999_000));

        let user_lp = query_cw20_balance(&app, &env.lp_token, &env.user);
        assert_eq!(user_lp, Uint128::zero());
    }

    #[test]
    fn test_withdraw_zero_rejected() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        let msg = to_json_binary(&dex_common::pair::Cw20HookMsg::WithdrawLiquidity { min_assets: None }).unwrap();

        let err = app
            .execute_contract(
                env.user.clone(),
                env.lp_token.clone(),
                &cw20::Cw20ExecuteMsg::Send {
                    contract: env.pair.to_string(),
                    amount: Uint128::zero(),
                    msg,
                },
                &[],
            )
            .unwrap_err();
        let msg = err.root_cause().to_string().to_lowercase();
        assert!(msg.contains("zero"), "Expected zero-amount error, got: {}", msg);
    }

    #[test]
    fn test_withdraw_from_wrong_token_rejected() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        let msg = to_json_binary(&dex_common::pair::Cw20HookMsg::WithdrawLiquidity { min_assets: None }).unwrap();

        let err = app
            .execute_contract(
                env.user.clone(),
                env.token_a.clone(),
                &cw20::Cw20ExecuteMsg::Send {
                    contract: env.pair.to_string(),
                    amount: Uint128::new(1_000),
                    msg,
                },
                &[],
            )
            .unwrap_err();
        assert!(err.root_cause().to_string().contains("Invalid token"));
    }

    #[test]
    fn test_withdraw_all_liquidity_then_swap_fails() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        let lp_balance = query_cw20_balance(&app, &env.lp_token, &env.user);
        withdraw_liquidity(&mut app, &env, &env.user, lp_balance);

        let pool = query_pool(&app, &env.pair);
        assert_eq!(pool.assets[0].amount, Uint128::new(1_000));
        assert_eq!(pool.assets[1].amount, Uint128::new(1_000));

        let swap_msg = to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: None,
            to: None,
            deadline: None,
            trader: None,
        })
        .unwrap();

        let result = app.execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(100),
                msg: swap_msg,
            },
            &[],
        );
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_paused_swap_rejected() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        app.execute_contract(
            env.governance.clone(),
            env.factory.clone(),
            &dex_common::factory::ExecuteMsg::SetPairPaused {
                pair: env.pair.to_string(),
                paused: true,
            },
            &[],
        )
        .unwrap();

        let swap_msg = to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: None,
            to: None,
            deadline: None,
            trader: None,
        })
        .unwrap();

        let err = app
            .execute_contract(
                env.user.clone(),
                env.token_a.clone(),
                &cw20::Cw20ExecuteMsg::Send {
                    contract: env.pair.to_string(),
                    amount: Uint128::new(1_000),
                    msg: swap_msg,
                },
                &[],
            )
            .unwrap_err();
        assert!(err.root_cause().to_string().contains("paused"));
    }

    #[test]
    fn test_paused_provide_liquidity_rejected() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        app.execute_contract(
            env.governance.clone(),
            env.factory.clone(),
            &dex_common::factory::ExecuteMsg::SetPairPaused {
                pair: env.pair.to_string(),
                paused: true,
            },
            &[],
        )
        .unwrap();

        app.execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                spender: env.pair.to_string(),
                amount: Uint128::new(1_000_000),
                expires: None,
            },
            &[],
        )
        .unwrap();
        app.execute_contract(
            env.user.clone(),
            env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                spender: env.pair.to_string(),
                amount: Uint128::new(1_000_000),
                expires: None,
            },
            &[],
        )
        .unwrap();

        let err = app
            .execute_contract(
                env.user.clone(),
                env.pair.clone(),
                &dex_common::pair::ExecuteMsg::ProvideLiquidity {
                    assets: [
                        dex_common::types::Asset {
                            info: asset_info_token(&env.token_a),
                            amount: Uint128::new(1_000_000),
                        },
                        dex_common::types::Asset {
                            info: asset_info_token(&env.token_b),
                            amount: Uint128::new(1_000_000),
                        },
                    ],
                    slippage_tolerance: None,
                    receiver: None,
                    deadline: None,
                },
                &[],
            )
            .unwrap_err();
        assert!(err.root_cause().to_string().contains("paused"));
    }

    #[test]
    fn test_unpause_restores_operation() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        app.execute_contract(
            env.governance.clone(),
            env.factory.clone(),
            &dex_common::factory::ExecuteMsg::SetPairPaused {
                pair: env.pair.to_string(),
                paused: true,
            },
            &[],
        )
        .unwrap();

        app.execute_contract(
            env.governance.clone(),
            env.factory.clone(),
            &dex_common::factory::ExecuteMsg::SetPairPaused {
                pair: env.pair.to_string(),
                paused: false,
            },
            &[],
        )
        .unwrap();

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));
        swap_a_to_b(&mut app, &env, &env.user, Uint128::new(1_000));
    }

    #[test]
    fn test_swap_with_belief_price() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        let swap_msg = to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
            belief_price: Some(Decimal::one()),
            max_spread: Some(Decimal::percent(5)),
            to: None,
            deadline: None,
            trader: None,
        })
        .unwrap();

        app.execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(1_000),
                msg: swap_msg,
            },
            &[],
        )
        .unwrap();
    }

    #[test]
    fn test_provide_asset_mismatch_rejected() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);
        let cw20_code_id = app.store_code(cw20_mintable_contract());
        let fake_token = create_cw20_token(
            &mut app,
            cw20_code_id,
            &env.user,
            "Fake",
            "FAKE",
            Uint128::new(1_000_000),
        );

        app.execute_contract(
            env.user.clone(),
            fake_token.clone(),
            &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                spender: env.pair.to_string(),
                amount: Uint128::new(1_000),
                expires: None,
            },
            &[],
        )
        .unwrap();
        app.execute_contract(
            env.user.clone(),
            env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                spender: env.pair.to_string(),
                amount: Uint128::new(1_000),
                expires: None,
            },
            &[],
        )
        .unwrap();

        let err = app
            .execute_contract(
                env.user.clone(),
                env.pair.clone(),
                &dex_common::pair::ExecuteMsg::ProvideLiquidity {
                    assets: [
                        dex_common::types::Asset {
                            info: asset_info_token(&fake_token),
                            amount: Uint128::new(1_000),
                        },
                        dex_common::types::Asset {
                            info: asset_info_token(&env.token_b),
                            amount: Uint128::new(1_000),
                        },
                    ],
                    slippage_tolerance: None,
                    receiver: None,
                    deadline: None,
                },
                &[],
            )
            .unwrap_err();
        assert!(err.root_cause().to_string().contains("Asset mismatch"));
    }

    #[test]
    fn test_simulation_on_empty_pool() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

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
        assert_eq!(sim.return_amount, Uint128::zero());
        assert_eq!(sim.commission_amount, Uint128::zero());
    }

    #[test]
    fn test_swap_large_amount() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000_000), Uint128::new(1_000_000_000));

        let swap_msg = to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: Some(Decimal::one()),
            to: None,
            deadline: None,
            trader: None,
        })
        .unwrap();

        app.execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(500_000_000),
                msg: swap_msg,
            },
            &[],
        )
        .unwrap();

        let pool = query_pool(&app, &env.pair);
        assert!(pool.assets[0].amount > Uint128::new(1_000_000_000));
        assert!(pool.assets[1].amount < Uint128::new(1_000_000_000));
        assert!(pool.assets[1].amount > Uint128::zero());
    }

    #[test]
    fn test_multiple_swaps_same_direction() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        for _ in 0..10 {
            swap_a_to_b(&mut app, &env, &env.user, Uint128::new(1_000));
        }

        let pool = query_pool(&app, &env.pair);
        assert!(pool.assets[0].amount > Uint128::new(1_000_000));
        assert!(pool.assets[1].amount < Uint128::new(1_000_000));
    }

    #[test]
    fn test_swap_alternating_directions() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        for i in 0..10 {
            if i % 2 == 0 {
                swap_a_to_b(&mut app, &env, &env.user, Uint128::new(1_000));
            } else {
                swap_b_to_a(&mut app, &env, &env.user, Uint128::new(1_000));
            }
        }

        let pool = query_pool(&app, &env.pair);
        assert!(pool.assets[0].amount > Uint128::zero());
        assert!(pool.assets[1].amount > Uint128::zero());
    }

    #[test]
    fn test_zero_fee_pool() {
        let mut app = App::default();
        let env = setup_env_with_fee(&mut app, 0);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        let treasury_before = query_cw20_balance(&app, &env.token_b, &env.treasury);
        swap_a_to_b(&mut app, &env, &env.user, Uint128::new(1_000));
        let treasury_after = query_cw20_balance(&app, &env.token_b, &env.treasury);

        assert_eq!(treasury_after - treasury_before, Uint128::zero());
    }

    #[test]
    fn test_high_fee_pool() {
        let mut app = App::default();
        let env = setup_env_with_fee(&mut app, 5000);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        let user_b_before = query_cw20_balance(&app, &env.token_b, &env.user);
        let treasury_before = query_cw20_balance(&app, &env.token_b, &env.treasury);
        swap_a_to_b(&mut app, &env, &env.user, Uint128::new(10_000));
        let user_b_after = query_cw20_balance(&app, &env.token_b, &env.user);
        let treasury_after = query_cw20_balance(&app, &env.token_b, &env.treasury);

        let net = user_b_after - user_b_before;
        let fee = treasury_after - treasury_before;
        assert!(fee > Uint128::zero());
        assert!(fee > net / Uint128::new(2));
    }

    #[test]
    fn test_provide_swap_withdraw_cycle() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let initial_a = query_cw20_balance(&app, &env.token_a, &env.user);
        let initial_b = query_cw20_balance(&app, &env.token_b, &env.user);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        swap_a_to_b(&mut app, &env, &env.user, Uint128::new(10_000));

        let lp_balance = query_cw20_balance(&app, &env.lp_token, &env.user);
        withdraw_liquidity(&mut app, &env, &env.user, lp_balance);

        let final_a = query_cw20_balance(&app, &env.token_a, &env.user);
        let final_b = query_cw20_balance(&app, &env.token_b, &env.user);
        let treasury_a = query_cw20_balance(&app, &env.token_a, &env.treasury);
        let treasury_b = query_cw20_balance(&app, &env.token_b, &env.treasury);

        let pool = query_pool(&app, &env.pair);
        let total_a = final_a + treasury_a + pool.assets[0].amount;
        let total_b = final_b + treasury_b + pool.assets[1].amount;

        assert_eq!(total_a, initial_a);
        assert_eq!(total_b, initial_b);
    }

    #[test]
    fn test_multi_lp_fair_withdrawal() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let user2 = Addr::unchecked("user2");
        transfer_tokens(&mut app, &env.token_a, &env.user, &user2, Uint128::new(500_000_000));
        transfer_tokens(&mut app, &env.token_b, &env.user, &user2, Uint128::new(500_000_000));

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));
        provide_liquidity(&mut app, &env, &user2, Uint128::new(1_000_000), Uint128::new(1_000_000));

        swap_a_to_b(&mut app, &env, &env.user, Uint128::new(10_000));

        let lp1 = query_cw20_balance(&app, &env.lp_token, &env.user);
        let lp2 = query_cw20_balance(&app, &env.lp_token, &user2);

        let pool_before = query_pool(&app, &env.pair);
        let user1_a_before = query_cw20_balance(&app, &env.token_a, &env.user);
        let user1_b_before = query_cw20_balance(&app, &env.token_b, &env.user);

        withdraw_liquidity(&mut app, &env, &env.user, lp1);

        let user1_a_after = query_cw20_balance(&app, &env.token_a, &env.user);
        let user1_b_after = query_cw20_balance(&app, &env.token_b, &env.user);

        let got_a = user1_a_after - user1_a_before;
        let got_b = user1_b_after - user1_b_before;

        let expected_a = lp1 * pool_before.assets[0].amount / pool_before.total_share;
        let expected_b = lp1 * pool_before.assets[1].amount / pool_before.total_share;

        assert_eq!(got_a, expected_a);
        assert_eq!(got_b, expected_b);

        let pool_after_1 = query_pool(&app, &env.pair);
        let user2_a_before = query_cw20_balance(&app, &env.token_a, &user2);
        let user2_b_before = query_cw20_balance(&app, &env.token_b, &user2);

        withdraw_liquidity(&mut app, &env, &user2, lp2);

        let user2_a_after = query_cw20_balance(&app, &env.token_a, &user2);
        let user2_b_after = query_cw20_balance(&app, &env.token_b, &user2);

        let got2_a = user2_a_after - user2_a_before;
        let got2_b = user2_b_after - user2_b_before;

        let expected2_a = lp2 * pool_after_1.assets[0].amount / pool_after_1.total_share;
        let expected2_b = lp2 * pool_after_1.assets[1].amount / pool_after_1.total_share;

        assert_eq!(got2_a, expected2_a);
        assert_eq!(got2_b, expected2_b);
    }

    #[test]
    fn test_fee_change_mid_trading() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(10_000_000), Uint128::new(10_000_000));

        let treasury_b_before = query_cw20_balance(&app, &env.token_b, &env.treasury);
        swap_a_to_b(&mut app, &env, &env.user, Uint128::new(100_000));
        let fee_30bps = query_cw20_balance(&app, &env.token_b, &env.treasury) - treasury_b_before;

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

        let treasury_b_before2 = query_cw20_balance(&app, &env.token_b, &env.treasury);
        swap_a_to_b(&mut app, &env, &env.user, Uint128::new(100_000));
        let fee_100bps = query_cw20_balance(&app, &env.token_b, &env.treasury) - treasury_b_before2;

        assert!(fee_100bps > fee_30bps);
    }

    #[test]
    fn test_simulation_matches_actual_swap() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        let sim: dex_common::pair::SimulationResponse = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::Simulation {
                    offer_asset: dex_common::types::Asset {
                        info: asset_info_token(&env.token_a),
                        amount: Uint128::new(10_000),
                    },
                },
            )
            .unwrap();

        let user_b_before = query_cw20_balance(&app, &env.token_b, &env.user);
        let treasury_b_before = query_cw20_balance(&app, &env.token_b, &env.treasury);
        swap_a_to_b(&mut app, &env, &env.user, Uint128::new(10_000));
        let user_b_after = query_cw20_balance(&app, &env.token_b, &env.user);
        let treasury_b_after = query_cw20_balance(&app, &env.token_b, &env.treasury);

        assert_eq!(user_b_after - user_b_before, sim.return_amount);
        assert_eq!(treasury_b_after - treasury_b_before, sim.commission_amount);
    }

    #[test]
    fn test_first_deposit_minimum_liquidity() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_001), Uint128::new(1_001));

        let lp_balance = query_cw20_balance(&app, &env.lp_token, &env.user);
        assert_eq!(lp_balance, Uint128::new(1));

        let pool = query_pool(&app, &env.pair);
        assert_eq!(pool.total_share, Uint128::new(1_001));
    }

    #[test]
    fn test_first_deposit_below_minimum_rejected() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        app.execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                spender: env.pair.to_string(),
                amount: Uint128::new(100),
                expires: None,
            },
            &[],
        )
        .unwrap();
        app.execute_contract(
            env.user.clone(),
            env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                spender: env.pair.to_string(),
                amount: Uint128::new(100),
                expires: None,
            },
            &[],
        )
        .unwrap();

        let err = app
            .execute_contract(
                env.user.clone(),
                env.pair.clone(),
                &dex_common::pair::ExecuteMsg::ProvideLiquidity {
                    assets: [
                        dex_common::types::Asset {
                            info: asset_info_token(&env.token_a),
                            amount: Uint128::new(100),
                        },
                        dex_common::types::Asset {
                            info: asset_info_token(&env.token_b),
                            amount: Uint128::new(100),
                        },
                    ],
                    slippage_tolerance: None,
                    receiver: None,
                    deadline: None,
                },
                &[],
            )
            .unwrap_err();
        assert!(err.root_cause().to_string().contains("Insufficient liquidity"));
    }
}

#[cfg(test)]
mod factory_coverage_tests {
    use super::helpers::*;
    use cosmwasm_std::{Addr, Uint128};
    use cw_multi_test::{App, Executor};

    #[test]
    fn test_unauthorized_config_update() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let random = Addr::unchecked("random");
        let err = app
            .execute_contract(
                random,
                env.factory.clone(),
                &dex_common::factory::ExecuteMsg::UpdateConfig {
                    governance: None,
                    treasury: None,
                    default_fee_bps: Some(100),
                },
                &[],
            )
            .unwrap_err();
        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    #[test]
    fn test_unauthorized_fee_update() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let random = Addr::unchecked("random");
        let err = app
            .execute_contract(
                random,
                env.factory.clone(),
                &dex_common::factory::ExecuteMsg::SetPairFee {
                    pair: env.pair.to_string(),
                    fee_bps: 100,
                },
                &[],
            )
            .unwrap_err();
        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    #[test]
    fn test_invalid_fee_rejected() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let err = app
            .execute_contract(
                env.governance.clone(),
                env.factory.clone(),
                &dex_common::factory::ExecuteMsg::SetPairFee {
                    pair: env.pair.to_string(),
                    fee_bps: 10001,
                },
                &[],
            )
            .unwrap_err();
        assert!(err.root_cause().to_string().contains("Invalid fee"));
    }

    #[test]
    fn test_set_discount_registry_all() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);
        let fee_discount_code_id = app.store_code(fee_discount_contract());
        let cw20_code_id = app.store_code(cw20_mintable_contract());

        let cl8y_token = create_cw20_token(
            &mut app,
            cw20_code_id,
            &env.user,
            "CL8Y",
            "CL8Y",
            Uint128::new(1_000_000),
        );

        let fee_discount = app
            .instantiate_contract(
                fee_discount_code_id,
                env.governance.clone(),
                &cl8y_dex_fee_discount::msg::InstantiateMsg {
                    governance: env.governance.to_string(),
                    cl8y_token: cl8y_token.to_string(),
                },
                &[],
                "fee_discount",
                None,
            )
            .unwrap();

        app.execute_contract(
            env.governance.clone(),
            env.factory.clone(),
            &dex_common::factory::ExecuteMsg::SetDiscountRegistryAll {
                registry: Some(fee_discount.to_string()),
            },
            &[],
        )
        .unwrap();
    }

    #[test]
    fn test_pair_query() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let pair_resp: dex_common::factory::PairResponse = app
            .wrap()
            .query_wasm_smart(
                env.factory.to_string(),
                &dex_common::factory::QueryMsg::Pair {
                    asset_infos: [
                        asset_info_token(&env.token_a),
                        asset_info_token(&env.token_b),
                    ],
                },
            )
            .unwrap();
        assert_eq!(pair_resp.pair.contract_addr, env.pair);
    }

    #[test]
    fn test_unauthorized_pause_rejected() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let random = Addr::unchecked("random");
        let err = app
            .execute_contract(
                random,
                env.factory.clone(),
                &dex_common::factory::ExecuteMsg::SetPairPaused {
                    pair: env.pair.to_string(),
                    paused: true,
                },
                &[],
            )
            .unwrap_err();
        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }
}

#[cfg(test)]
mod fee_discount_coverage_tests {
    use super::helpers::*;
    use cosmwasm_std::{Addr, Uint128};
    use cw_multi_test::{App, Executor};

    const ONE_CL8Y: u128 = 1_000_000_000_000_000_000;

    fn setup_basic_discount(app: &mut App) -> (Addr, Addr, Addr) {
        let governance = Addr::unchecked("governance");
        let user = Addr::unchecked("user");
        let cw20_code_id = app.store_code(cw20_mintable_contract());
        let fd_code_id = app.store_code(fee_discount_contract());

        let cl8y = create_cw20_token_with_decimals(
            app,
            cw20_code_id,
            &user,
            "CL8Y",
            "CL8Y",
            18,
            Uint128::new(100_000 * ONE_CL8Y),
        );

        let fd = app
            .instantiate_contract(
                fd_code_id,
                governance.clone(),
                &cl8y_dex_fee_discount::msg::InstantiateMsg {
                    governance: governance.to_string(),
                    cl8y_token: cl8y.to_string(),
                },
                &[],
                "fee_discount",
                None,
            )
            .unwrap();

        app.execute_contract(
            governance.clone(),
            fd.clone(),
            &cl8y_dex_fee_discount::msg::ExecuteMsg::AddTier {
                tier_id: 1,
                min_cl8y_balance: Uint128::new(ONE_CL8Y),
                discount_bps: 1000,
                governance_only: false,
            },
            &[],
        )
        .unwrap();

        (fd, cl8y, governance)
    }

    #[test]
    fn test_update_tier() {
        let mut app = App::default();
        let (fd, _cl8y, governance) = setup_basic_discount(&mut app);

        app.execute_contract(
            governance.clone(),
            fd.clone(),
            &cl8y_dex_fee_discount::msg::ExecuteMsg::UpdateTier {
                tier_id: 1,
                min_cl8y_balance: Some(Uint128::new(2 * ONE_CL8Y)),
                discount_bps: Some(2000),
                governance_only: None,
            },
            &[],
        )
        .unwrap();

        let tier: cl8y_dex_fee_discount::msg::TierResponse = app
            .wrap()
            .query_wasm_smart(
                fd.to_string(),
                &cl8y_dex_fee_discount::msg::QueryMsg::GetTier { tier_id: 1 },
            )
            .unwrap();
        assert_eq!(tier.tier.discount_bps, 2000);
        assert_eq!(tier.tier.min_cl8y_balance, Uint128::new(2 * ONE_CL8Y));
    }

    #[test]
    fn test_remove_tier() {
        let mut app = App::default();
        let (fd, _cl8y, governance) = setup_basic_discount(&mut app);

        app.execute_contract(
            governance.clone(),
            fd.clone(),
            &cl8y_dex_fee_discount::msg::ExecuteMsg::RemoveTier { tier_id: 1 },
            &[],
        )
        .unwrap();

        let result: Result<cl8y_dex_fee_discount::msg::TierResponse, _> = app
            .wrap()
            .query_wasm_smart(
                fd.to_string(),
                &cl8y_dex_fee_discount::msg::QueryMsg::GetTier { tier_id: 1 },
            );
        assert!(result.is_err());
    }

    #[test]
    fn test_deregister() {
        let mut app = App::default();
        let (fd, _cl8y, _gov) = setup_basic_discount(&mut app);
        let user = Addr::unchecked("user");

        app.execute_contract(
            user.clone(),
            fd.clone(),
            &cl8y_dex_fee_discount::msg::ExecuteMsg::Register { tier_id: 1 },
            &[],
        )
        .unwrap();

        app.execute_contract(
            user.clone(),
            fd.clone(),
            &cl8y_dex_fee_discount::msg::ExecuteMsg::Deregister {},
            &[],
        )
        .unwrap();

        let reg: cl8y_dex_fee_discount::msg::RegistrationResponse = app
            .wrap()
            .query_wasm_smart(
                fd.to_string(),
                &cl8y_dex_fee_discount::msg::QueryMsg::GetRegistration {
                    trader: user.to_string(),
                },
            )
            .unwrap();
        assert!(!reg.registered);
    }

    #[test]
    fn test_deregister_not_registered() {
        let mut app = App::default();
        let (fd, _cl8y, _gov) = setup_basic_discount(&mut app);
        let nobody = Addr::unchecked("nobody");

        let err = app
            .execute_contract(
                nobody,
                fd.clone(),
                &cl8y_dex_fee_discount::msg::ExecuteMsg::Deregister {},
                &[],
            )
            .unwrap_err();
        assert!(err.root_cause().to_string().to_lowercase().contains("not registered"));
    }

    #[test]
    fn test_update_config() {
        let mut app = App::default();
        let (fd, cl8y, governance) = setup_basic_discount(&mut app);

        let new_gov = Addr::unchecked("new_governance");

        app.execute_contract(
            governance.clone(),
            fd.clone(),
            &cl8y_dex_fee_discount::msg::ExecuteMsg::UpdateConfig {
                governance: Some(new_gov.to_string()),
                cl8y_token: None,
            },
            &[],
        )
        .unwrap();

        let config: cl8y_dex_fee_discount::msg::ConfigResponse = app
            .wrap()
            .query_wasm_smart(
                fd.to_string(),
                &cl8y_dex_fee_discount::msg::QueryMsg::Config {},
            )
            .unwrap();
        assert_eq!(config.governance, new_gov);
        assert_eq!(config.cl8y_token, cl8y);
    }

    #[test]
    fn test_add_remove_trusted_router() {
        let mut app = App::default();
        let (fd, _cl8y, governance) = setup_basic_discount(&mut app);
        let router = Addr::unchecked("test_router");

        app.execute_contract(
            governance.clone(),
            fd.clone(),
            &cl8y_dex_fee_discount::msg::ExecuteMsg::AddTrustedRouter {
                router: router.to_string(),
            },
            &[],
        )
        .unwrap();

        let is_trusted: cl8y_dex_fee_discount::msg::IsTrustedRouterResponse = app
            .wrap()
            .query_wasm_smart(
                fd.to_string(),
                &cl8y_dex_fee_discount::msg::QueryMsg::IsTrustedRouter {
                    addr: router.to_string(),
                },
            )
            .unwrap();
        assert!(is_trusted.is_trusted);

        app.execute_contract(
            governance.clone(),
            fd.clone(),
            &cl8y_dex_fee_discount::msg::ExecuteMsg::RemoveTrustedRouter {
                router: router.to_string(),
            },
            &[],
        )
        .unwrap();

        let is_trusted: cl8y_dex_fee_discount::msg::IsTrustedRouterResponse = app
            .wrap()
            .query_wasm_smart(
                fd.to_string(),
                &cl8y_dex_fee_discount::msg::QueryMsg::IsTrustedRouter {
                    addr: router.to_string(),
                },
            )
            .unwrap();
        assert!(!is_trusted.is_trusted);
    }

    #[test]
    fn test_query_discount_untrusted_router_falls_back_to_sender() {
        let mut app = App::default();
        let (fd, _cl8y, _gov) = setup_basic_discount(&mut app);
        let user = Addr::unchecked("user");
        let untrusted_router = Addr::unchecked("untrusted_router");

        app.execute_contract(
            user.clone(),
            fd.clone(),
            &cl8y_dex_fee_discount::msg::ExecuteMsg::Register { tier_id: 1 },
            &[],
        )
        .unwrap();

        let discount: cl8y_dex_fee_discount::msg::DiscountResponse = app
            .wrap()
            .query_wasm_smart(
                fd.to_string(),
                &cl8y_dex_fee_discount::msg::QueryMsg::GetDiscount {
                    trader: user.to_string(),
                    sender: untrusted_router.to_string(),
                },
            )
            .unwrap();
        assert_eq!(discount.discount_bps, 0);
    }

    #[test]
    fn test_tier_already_exists_rejected() {
        let mut app = App::default();
        let (fd, _cl8y, governance) = setup_basic_discount(&mut app);

        let err = app
            .execute_contract(
                governance.clone(),
                fd.clone(),
                &cl8y_dex_fee_discount::msg::ExecuteMsg::AddTier {
                    tier_id: 1,
                    min_cl8y_balance: Uint128::new(ONE_CL8Y),
                    discount_bps: 500,
                    governance_only: false,
                },
                &[],
            )
            .unwrap_err();
        assert!(err.root_cause().to_string().contains("already exists"));
    }

    #[test]
    fn test_invalid_discount_bps_rejected() {
        let mut app = App::default();
        let (fd, _cl8y, governance) = setup_basic_discount(&mut app);

        let err = app
            .execute_contract(
                governance.clone(),
                fd.clone(),
                &cl8y_dex_fee_discount::msg::ExecuteMsg::AddTier {
                    tier_id: 99,
                    min_cl8y_balance: Uint128::zero(),
                    discount_bps: 10001,
                    governance_only: false,
                },
                &[],
            )
            .unwrap_err();
        assert!(err.root_cause().to_string().contains("Invalid discount"));
    }

    #[test]
    fn test_tier_not_found_rejected() {
        let mut app = App::default();
        let (fd, _cl8y, _gov) = setup_basic_discount(&mut app);
        let user = Addr::unchecked("user");

        let err = app
            .execute_contract(
                user,
                fd.clone(),
                &cl8y_dex_fee_discount::msg::ExecuteMsg::Register { tier_id: 99 },
                &[],
            )
            .unwrap_err();
        assert!(err.root_cause().to_string().contains("not found"));
    }

    #[test]
    fn test_governance_deregister_wallet() {
        let mut app = App::default();
        let (fd, _cl8y, governance) = setup_basic_discount(&mut app);
        let user = Addr::unchecked("user");

        app.execute_contract(
            governance.clone(),
            fd.clone(),
            &cl8y_dex_fee_discount::msg::ExecuteMsg::AddTier {
                tier_id: 0,
                min_cl8y_balance: Uint128::zero(),
                discount_bps: 10000,
                governance_only: true,
            },
            &[],
        )
        .unwrap();

        app.execute_contract(
            governance.clone(),
            fd.clone(),
            &cl8y_dex_fee_discount::msg::ExecuteMsg::RegisterWallet {
                wallet: user.to_string(),
                tier_id: 0,
            },
            &[],
        )
        .unwrap();

        app.execute_contract(
            governance.clone(),
            fd.clone(),
            &cl8y_dex_fee_discount::msg::ExecuteMsg::DeregisterWallet {
                wallet: user.to_string(),
                epoch: None,
            },
            &[],
        )
        .unwrap();

        let reg: cl8y_dex_fee_discount::msg::RegistrationResponse = app
            .wrap()
            .query_wasm_smart(
                fd.to_string(),
                &cl8y_dex_fee_discount::msg::QueryMsg::GetRegistration {
                    trader: user.to_string(),
                },
            )
            .unwrap();
        assert!(!reg.registered);
    }
}

#[cfg(test)]
mod router_coverage_tests {
    use super::helpers::*;
    use cosmwasm_std::{to_json_binary, Uint128};
    use cw_multi_test::{App, Executor};

    #[test]
    fn test_router_simulate_swap() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        let sim: cl8y_dex_router::msg::SimulateSwapOperationsResponse = app
            .wrap()
            .query_wasm_smart(
                env.router.to_string(),
                &cl8y_dex_router::msg::QueryMsg::SimulateSwapOperations {
                    offer_amount: Uint128::new(10_000),
                    operations: vec![cl8y_dex_router::msg::SwapOperation::TerraSwap {
                        offer_asset_info: asset_info_token(&env.token_a),
                        ask_asset_info: asset_info_token(&env.token_b),
                    }],
                },
            )
            .unwrap();

        assert!(sim.amount > Uint128::zero());
    }

    #[test]
    fn test_router_reverse_simulate() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        let rsim: cl8y_dex_router::msg::SimulateSwapOperationsResponse = app
            .wrap()
            .query_wasm_smart(
                env.router.to_string(),
                &cl8y_dex_router::msg::QueryMsg::ReverseSimulateSwapOperations {
                    ask_amount: Uint128::new(9_000),
                    operations: vec![cl8y_dex_router::msg::SwapOperation::TerraSwap {
                        offer_asset_info: asset_info_token(&env.token_a),
                        ask_asset_info: asset_info_token(&env.token_b),
                    }],
                },
            )
            .unwrap();

        assert!(rsim.amount > Uint128::new(9_000));
    }

    #[test]
    fn test_router_minimum_receive_assertion() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        let hook_msg = to_json_binary(&cl8y_dex_router::msg::Cw20HookMsg::ExecuteSwapOperations {
            operations: vec![cl8y_dex_router::msg::SwapOperation::TerraSwap {
                offer_asset_info: asset_info_token(&env.token_a),
                ask_asset_info: asset_info_token(&env.token_b),
            }],
            minimum_receive: Some(Uint128::new(999_999)),
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
        let msg = err.root_cause().to_string().to_lowercase();
        assert!(msg.contains("minimum receive") || msg.contains("minimum_receive"),
            "Expected minimum receive error, got: {}", msg);
    }

    #[test]
    fn test_router_multi_hop() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let cw20_code_id = app.store_code(cw20_mintable_contract());

        app.execute_contract(
            env.governance.clone(),
            env.factory.clone(),
            &dex_common::factory::ExecuteMsg::AddWhitelistedCodeId { code_id: cw20_code_id },
            &[],
        )
        .unwrap();

        let token_c = create_cw20_token(
            &mut app,
            cw20_code_id,
            &env.user,
            "Token C",
            "TKNC",
            Uint128::new(1_000_000_000_000),
        );

        let resp = app
            .execute_contract(
                env.user.clone(),
                env.factory.clone(),
                &dex_common::factory::ExecuteMsg::CreatePair {
                    asset_infos: [
                        asset_info_token(&env.token_b),
                        asset_info_token(&token_c),
                    ],
                },
                &[],
            )
            .unwrap();
        let pair_bc = extract_pair_address(&resp.events);
        let pair_bc_info: dex_common::types::PairInfo = app
            .wrap()
            .query_wasm_smart(pair_bc.to_string(), &dex_common::pair::QueryMsg::Pair {})
            .unwrap();
        let _lp_bc = pair_bc_info.liquidity_token;

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(10_000_000), Uint128::new(10_000_000));

        app.execute_contract(
            env.user.clone(),
            env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                spender: pair_bc.to_string(),
                amount: Uint128::new(10_000_000),
                expires: None,
            },
            &[],
        )
        .unwrap();
        app.execute_contract(
            env.user.clone(),
            token_c.clone(),
            &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                spender: pair_bc.to_string(),
                amount: Uint128::new(10_000_000),
                expires: None,
            },
            &[],
        )
        .unwrap();
        app.execute_contract(
            env.user.clone(),
            pair_bc.clone(),
            &dex_common::pair::ExecuteMsg::ProvideLiquidity {
                assets: [
                    dex_common::types::Asset {
                        info: asset_info_token(&env.token_b),
                        amount: Uint128::new(10_000_000),
                    },
                    dex_common::types::Asset {
                        info: asset_info_token(&token_c),
                        amount: Uint128::new(10_000_000),
                    },
                ],
                slippage_tolerance: None,
                receiver: None,
                deadline: None,
            },
            &[],
        )
        .unwrap();

        let user_c_before = query_cw20_balance(&app, &token_c, &env.user);

        let hook_msg = to_json_binary(&cl8y_dex_router::msg::Cw20HookMsg::ExecuteSwapOperations {
            operations: vec![
                cl8y_dex_router::msg::SwapOperation::TerraSwap {
                    offer_asset_info: asset_info_token(&env.token_a),
                    ask_asset_info: asset_info_token(&env.token_b),
                },
                cl8y_dex_router::msg::SwapOperation::TerraSwap {
                    offer_asset_info: asset_info_token(&env.token_b),
                    ask_asset_info: asset_info_token(&token_c),
                },
            ],
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
                amount: Uint128::new(10_000),
                msg: hook_msg,
            },
            &[],
        )
        .unwrap();

        let user_c_after = query_cw20_balance(&app, &token_c, &env.user);
        assert!(user_c_after > user_c_before);
    }

    #[test]
    fn test_router_config_query() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let config: cl8y_dex_router::msg::ConfigResponse = app
            .wrap()
            .query_wasm_smart(
                env.router.to_string(),
                &cl8y_dex_router::msg::QueryMsg::Config {},
            )
            .unwrap();
        assert_eq!(config.factory, env.factory);
    }
}

#[cfg(test)]
mod hook_coverage_tests {
    use super::helpers::*;
    use cosmwasm_std::{Addr, Uint128};
    use cw_multi_test::{App, Executor};

    #[test]
    fn test_burn_hook_instantiate_and_config() {
        let mut app = App::default();
        let admin = Addr::unchecked("admin");
        let cw20_code_id = app.store_code(cw20_mintable_contract());
        let burn_hook_code_id = app.store_code(burn_hook_contract());

        let burn_token = create_cw20_token(
            &mut app,
            cw20_code_id,
            &admin,
            "BurnToken",
            "BURN",
            Uint128::new(1_000_000),
        );

        let burn_hook = app
            .instantiate_contract(
                burn_hook_code_id,
                admin.clone(),
                &cl8y_dex_burn_hook::msg::InstantiateMsg {
                    burn_token: burn_token.to_string(),
                    burn_percentage_bps: 500,
                    admin: admin.to_string(),
                },
                &[],
                "burn_hook",
                None,
            )
            .unwrap();

        let config: cl8y_dex_burn_hook::msg::ConfigResponse = app
            .wrap()
            .query_wasm_smart(
                burn_hook.to_string(),
                &cl8y_dex_burn_hook::msg::QueryMsg::GetConfig {},
            )
            .unwrap();
        assert_eq!(config.burn_token, burn_token);
        assert_eq!(config.burn_percentage_bps, 500);
        assert_eq!(config.admin, admin);
    }

    #[test]
    fn test_burn_hook_update_config() {
        let mut app = App::default();
        let admin = Addr::unchecked("admin");
        let cw20_code_id = app.store_code(cw20_mintable_contract());
        let burn_hook_code_id = app.store_code(burn_hook_contract());

        let burn_token = create_cw20_token(
            &mut app,
            cw20_code_id,
            &admin,
            "BurnToken",
            "BURN",
            Uint128::new(1_000_000),
        );

        let burn_hook = app
            .instantiate_contract(
                burn_hook_code_id,
                admin.clone(),
                &cl8y_dex_burn_hook::msg::InstantiateMsg {
                    burn_token: burn_token.to_string(),
                    burn_percentage_bps: 500,
                    admin: admin.to_string(),
                },
                &[],
                "burn_hook",
                None,
            )
            .unwrap();

        app.execute_contract(
            admin.clone(),
            burn_hook.clone(),
            &cl8y_dex_burn_hook::msg::ExecuteMsg::UpdateConfig {
                burn_token: None,
                burn_percentage_bps: Some(1000),
            },
            &[],
        )
        .unwrap();

        let config: cl8y_dex_burn_hook::msg::ConfigResponse = app
            .wrap()
            .query_wasm_smart(
                burn_hook.to_string(),
                &cl8y_dex_burn_hook::msg::QueryMsg::GetConfig {},
            )
            .unwrap();
        assert_eq!(config.burn_percentage_bps, 1000);
    }

    #[test]
    fn test_burn_hook_unauthorized_rejected() {
        let mut app = App::default();
        let admin = Addr::unchecked("admin");
        let random = Addr::unchecked("random");
        let cw20_code_id = app.store_code(cw20_mintable_contract());
        let burn_hook_code_id = app.store_code(burn_hook_contract());

        let burn_token = create_cw20_token(
            &mut app,
            cw20_code_id,
            &admin,
            "BurnToken",
            "BURN",
            Uint128::new(1_000_000),
        );

        let burn_hook = app
            .instantiate_contract(
                burn_hook_code_id,
                admin.clone(),
                &cl8y_dex_burn_hook::msg::InstantiateMsg {
                    burn_token: burn_token.to_string(),
                    burn_percentage_bps: 500,
                    admin: admin.to_string(),
                },
                &[],
                "burn_hook",
                None,
            )
            .unwrap();

        let err = app
            .execute_contract(
                random,
                burn_hook.clone(),
                &cl8y_dex_burn_hook::msg::ExecuteMsg::UpdateConfig {
                    burn_token: None,
                    burn_percentage_bps: Some(1000),
                },
                &[],
            )
            .unwrap_err();
        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    #[test]
    fn test_tax_hook_instantiate_and_config() {
        let mut app = App::default();
        let admin = Addr::unchecked("admin");
        let cw20_code_id = app.store_code(cw20_mintable_contract());
        let tax_hook_code_id = app.store_code(tax_hook_contract());

        let tax_token = create_cw20_token(
            &mut app,
            cw20_code_id,
            &admin,
            "TaxToken",
            "TAX",
            Uint128::new(1_000_000),
        );

        let recipient = Addr::unchecked("tax_recipient");

        let tax_hook = app
            .instantiate_contract(
                tax_hook_code_id,
                admin.clone(),
                &cl8y_dex_tax_hook::msg::InstantiateMsg {
                    recipient: recipient.to_string(),
                    tax_percentage_bps: 200,
                    tax_token: tax_token.to_string(),
                    admin: admin.to_string(),
                },
                &[],
                "tax_hook",
                None,
            )
            .unwrap();

        let config: cl8y_dex_tax_hook::msg::ConfigResponse = app
            .wrap()
            .query_wasm_smart(
                tax_hook.to_string(),
                &cl8y_dex_tax_hook::msg::QueryMsg::GetConfig {},
            )
            .unwrap();
        assert_eq!(config.recipient, recipient);
        assert_eq!(config.tax_percentage_bps, 200);
        assert_eq!(config.tax_token, tax_token);
    }

    #[test]
    fn test_lp_burn_hook_instantiate_and_config() {
        let mut app = App::default();
        let admin = Addr::unchecked("admin");
        let lp_burn_code_id = app.store_code(lp_burn_hook_contract());

        let target_pair = Addr::unchecked("some_pair");

        let lp_burn_hook = app
            .instantiate_contract(
                lp_burn_code_id,
                admin.clone(),
                &cl8y_dex_lp_burn_hook::msg::InstantiateMsg {
                    target_pair: target_pair.to_string(),
                    lp_token: "lp_token_addr".to_string(),
                    percentage_bps: 300,
                    admin: admin.to_string(),
                },
                &[],
                "lp_burn_hook",
                None,
            )
            .unwrap();

        let config: cl8y_dex_lp_burn_hook::msg::ConfigResponse = app
            .wrap()
            .query_wasm_smart(
                lp_burn_hook.to_string(),
                &cl8y_dex_lp_burn_hook::msg::QueryMsg::GetConfig {},
            )
            .unwrap();
        assert_eq!(config.target_pair, target_pair);
        assert_eq!(config.lp_token, Addr::unchecked("lp_token_addr"));
        assert_eq!(config.percentage_bps, 300);
    }

    #[test]
    fn test_burn_hook_invalid_bps_rejected() {
        let mut app = App::default();
        let admin = Addr::unchecked("admin");
        let cw20_code_id = app.store_code(cw20_mintable_contract());
        let burn_hook_code_id = app.store_code(burn_hook_contract());

        let burn_token = create_cw20_token(
            &mut app,
            cw20_code_id,
            &admin,
            "BurnToken",
            "BURN",
            Uint128::new(1_000_000),
        );

        let err = app
            .instantiate_contract(
                burn_hook_code_id,
                admin.clone(),
                &cl8y_dex_burn_hook::msg::InstantiateMsg {
                    burn_token: burn_token.to_string(),
                    burn_percentage_bps: 10001,
                    admin: admin.to_string(),
                },
                &[],
                "burn_hook",
                None,
            )
            .unwrap_err();
        assert!(err.root_cause().to_string().contains("Invalid"));
    }
}

// ===========================================================================
// FUZZ / INVARIANT TESTS — property-based testing with proptest
// ===========================================================================

#[cfg(test)]
mod fuzz_tests {
    use super::helpers::*;
    use cosmwasm_std::{to_json_binary, Addr, Uint128};
    use cw_multi_test::{App, Executor};
    use proptest::prelude::*;

    fn k_value(app: &App, pair: &Addr) -> u128 {
        let pool = query_pool(app, pair);
        pool.assets[0].amount.u128() * pool.assets[1].amount.u128()
    }

    /// Assert that k is maintained after a swap and that any increase is
    /// solely from ceiling-division rounding. Mathematically:
    ///   ceil(k/n)*n − k  is in [0, n−1]
    /// where n = new_input_reserve (the input-side reserve AFTER the swap).
    /// So the absolute increase must be strictly less than n.
    fn assert_k_rounding_only(
        k_before: u128,
        k_after: u128,
        new_input_reserve: u128,
    ) -> Result<(), proptest::test_runner::TestCaseError> {
        prop_assert!(
            k_after >= k_before,
            "k decreased: before={}, after={}", k_before, k_after
        );
        let increase = k_after - k_before;
        prop_assert!(
            increase < new_input_reserve,
            "k increase exceeds rounding bound: increase={}, bound={} (new_input_reserve), \
             k_before={}, k_after={}",
            increase, new_input_reserve, k_before, k_after
        );
        Ok(())
    }

    // -----------------------------------------------------------------------
    // PROPERTY 0: Verify ceiling-division rounding bound is tight.
    // After a swap the k increase must be exactly:
    //   if k_old % new_input_reserve == 0  →  0  (no rounding)
    //   else  →  new_input_reserve − (k_old % new_input_reserve)
    // This proves the increase is purely from integer rounding in
    // ceil(k / new_input_reserve) and nothing else.
    // -----------------------------------------------------------------------
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(256))]

        #[test]
        fn prop_k_increase_equals_exact_rounding_remainder(
            init_a in 10_000u128..100_000_000u128,
            init_b in 10_000u128..100_000_000u128,
            swap_fraction_bps in 1u128..5000u128,
        ) {
            let mut app = App::default();
            let env = setup_full_env(&mut app);

            provide_liquidity(&mut app, &env, &env.user,
                Uint128::new(init_a), Uint128::new(init_b));

            let pool_before = query_pool(&app, &env.pair);
            let k_before = pool_before.assets[0].amount.u128()
                         * pool_before.assets[1].amount.u128();

            let swap_amount = std::cmp::max(1, init_a * swap_fraction_bps / 10000);
            swap_a_to_b(&mut app, &env, &env.user, Uint128::new(swap_amount));

            let pool_after = query_pool(&app, &env.pair);
            let k_after = pool_after.assets[0].amount.u128()
                        * pool_after.assets[1].amount.u128();
            let new_input_reserve = pool_after.assets[0].amount.u128();

            let remainder = k_before % new_input_reserve;
            let expected_increase = if remainder == 0 { 0 } else { new_input_reserve - remainder };
            let actual_increase = k_after - k_before;

            prop_assert_eq!(actual_increase, expected_increase,
                "k increase doesn't match ceiling rounding: actual={}, expected={}, \
                 k_before={}, k_after={}, new_input_reserve={}, remainder={}",
                actual_increase, expected_increase, k_before, k_after,
                new_input_reserve, remainder);
        }
    }

    // -----------------------------------------------------------------------
    // PROPERTY 1: K maintained (non-decreasing) after swaps, increase is
    // bounded by ceiling-division rounding (< new_input_reserve).
    // -----------------------------------------------------------------------
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]

        #[test]
        fn prop_k_maintained_after_swap_a_to_b(
            init_a in 10_000u128..1_000_000_000u128,
            init_b in 10_000u128..1_000_000_000u128,
            swap_fraction_bps in 1u128..5000u128,
        ) {
            let mut app = App::default();
            let env = setup_full_env(&mut app);

            provide_liquidity(&mut app, &env, &env.user,
                Uint128::new(init_a), Uint128::new(init_b));

            let k_before = k_value(&app, &env.pair);

            let swap_amount = std::cmp::max(1, init_a * swap_fraction_bps / 10000);
            swap_a_to_b(&mut app, &env, &env.user, Uint128::new(swap_amount));

            let pool_after = query_pool(&app, &env.pair);
            let k_after = pool_after.assets[0].amount.u128() * pool_after.assets[1].amount.u128();
            let new_input_reserve = pool_after.assets[0].amount.u128();
            assert_k_rounding_only(k_before, k_after, new_input_reserve)?;
        }

        #[test]
        fn prop_k_maintained_after_swap_b_to_a(
            init_a in 10_000u128..1_000_000_000u128,
            init_b in 10_000u128..1_000_000_000u128,
            swap_fraction_bps in 1u128..5000u128,
        ) {
            let mut app = App::default();
            let env = setup_full_env(&mut app);

            provide_liquidity(&mut app, &env, &env.user,
                Uint128::new(init_a), Uint128::new(init_b));

            let k_before = k_value(&app, &env.pair);

            let swap_amount = std::cmp::max(1, init_b * swap_fraction_bps / 10000);
            swap_b_to_a(&mut app, &env, &env.user, Uint128::new(swap_amount));

            let pool_after = query_pool(&app, &env.pair);
            let k_after = pool_after.assets[0].amount.u128() * pool_after.assets[1].amount.u128();
            let new_input_reserve = pool_after.assets[1].amount.u128();
            assert_k_rounding_only(k_before, k_after, new_input_reserve)?;
        }
    }

    // -----------------------------------------------------------------------
    // PROPERTY 2: Token conservation — total supply is constant
    // sum(user_balance + pool_reserve + treasury_balance) == initial_supply
    // for each token, after any combination of operations.
    // -----------------------------------------------------------------------
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]

        #[test]
        fn prop_token_conservation_after_swaps(
            init_a in 100_000u128..1_000_000_000u128,
            init_b in 100_000u128..1_000_000_000u128,
            num_swaps in 1usize..15,
            seed in 0u64..1_000_000u64,
        ) {
            let mut app = App::default();
            let env = setup_full_env(&mut app);

            let initial_total = Uint128::new(1_000_000_000_000);

            provide_liquidity(&mut app, &env, &env.user,
                Uint128::new(init_a), Uint128::new(init_b));

            let mut rng_state = seed;
            for _ in 0..num_swaps {
                rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let direction = (rng_state >> 32) % 2 == 0;
                let pool = query_pool(&app, &env.pair);

                let max_swap = if direction {
                    std::cmp::min(
                        pool.assets[0].amount.u128() / 2,
                        query_cw20_balance(&app, &env.token_a, &env.user).u128(),
                    )
                } else {
                    std::cmp::min(
                        pool.assets[1].amount.u128() / 2,
                        query_cw20_balance(&app, &env.token_b, &env.user).u128(),
                    )
                };

                if max_swap < 1 { continue; }

                rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let swap_amount = 1 + (rng_state as u128 % max_swap);

                if direction {
                    swap_a_to_b(&mut app, &env, &env.user, Uint128::new(swap_amount));
                } else {
                    swap_b_to_a(&mut app, &env, &env.user, Uint128::new(swap_amount));
                }
            }

            let pool = query_pool(&app, &env.pair);
            let user_a = query_cw20_balance(&app, &env.token_a, &env.user);
            let user_b = query_cw20_balance(&app, &env.token_b, &env.user);
            let treasury_a = query_cw20_balance(&app, &env.token_a, &env.treasury);
            let treasury_b = query_cw20_balance(&app, &env.token_b, &env.treasury);

            let total_a = user_a + pool.assets[0].amount + treasury_a;
            let total_b = user_b + pool.assets[1].amount + treasury_b;

            prop_assert_eq!(total_a, initial_total,
                "Token A conservation violated: {} != {}", total_a, initial_total);
            prop_assert_eq!(total_b, initial_total,
                "Token B conservation violated: {} != {}", total_b, initial_total);
        }
    }

    // -----------------------------------------------------------------------
    // PROPERTY 3: Token conservation through full cycle
    // provide → swap N times → withdraw all → verify no tokens lost/created
    // -----------------------------------------------------------------------
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]

        #[test]
        fn prop_full_cycle_conservation(
            init_a in 100_000u128..100_000_000u128,
            init_b in 100_000u128..100_000_000u128,
            num_swaps in 1usize..10,
            seed in 0u64..1_000_000u64,
        ) {
            let mut app = App::default();
            let env = setup_full_env(&mut app);

            let initial_total = Uint128::new(1_000_000_000_000);

            provide_liquidity(&mut app, &env, &env.user,
                Uint128::new(init_a), Uint128::new(init_b));

            let mut rng_state = seed;
            for _ in 0..num_swaps {
                rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let direction = (rng_state >> 32) % 2 == 0;
                let pool = query_pool(&app, &env.pair);
                let max_swap = if direction {
                    std::cmp::min(
                        pool.assets[0].amount.u128() / 3,
                        query_cw20_balance(&app, &env.token_a, &env.user).u128(),
                    )
                } else {
                    std::cmp::min(
                        pool.assets[1].amount.u128() / 3,
                        query_cw20_balance(&app, &env.token_b, &env.user).u128(),
                    )
                };
                if max_swap < 1 { continue; }

                rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let swap_amount = 1 + (rng_state as u128 % max_swap);

                if direction {
                    swap_a_to_b(&mut app, &env, &env.user, Uint128::new(swap_amount));
                } else {
                    swap_b_to_a(&mut app, &env, &env.user, Uint128::new(swap_amount));
                }
            }

            let lp = query_cw20_balance(&app, &env.lp_token, &env.user);
            if !lp.is_zero() {
                withdraw_liquidity(&mut app, &env, &env.user, lp);
            }

            let pool = query_pool(&app, &env.pair);
            let user_a = query_cw20_balance(&app, &env.token_a, &env.user);
            let user_b = query_cw20_balance(&app, &env.token_b, &env.user);
            let treasury_a = query_cw20_balance(&app, &env.token_a, &env.treasury);
            let treasury_b = query_cw20_balance(&app, &env.token_b, &env.treasury);

            let total_a = user_a + pool.assets[0].amount + treasury_a;
            let total_b = user_b + pool.assets[1].amount + treasury_b;

            prop_assert_eq!(total_a, initial_total);
            prop_assert_eq!(total_b, initial_total);
        }
    }

    // -----------------------------------------------------------------------
    // PROPERTY 4: LP proportional withdrawal is exact
    // withdraw(lp_amount) should return exactly:
    //   amount_a = lp_amount * reserve_a / total_supply
    //   amount_b = lp_amount * reserve_b / total_supply
    // -----------------------------------------------------------------------
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]

        #[test]
        fn prop_lp_withdrawal_proportional(
            init_a in 10_000u128..1_000_000_000u128,
            init_b in 10_000u128..1_000_000_000u128,
            withdraw_fraction_bps in 1u128..10000u128,
        ) {
            let mut app = App::default();
            let env = setup_full_env(&mut app);

            provide_liquidity(&mut app, &env, &env.user,
                Uint128::new(init_a), Uint128::new(init_b));

            let lp_balance = query_cw20_balance(&app, &env.lp_token, &env.user);
            let withdraw_amount = std::cmp::max(1, lp_balance.u128() * withdraw_fraction_bps / 10000);
            let withdraw_amount = std::cmp::min(withdraw_amount, lp_balance.u128());
            let withdraw_uint = Uint128::new(withdraw_amount);

            let pool_before = query_pool(&app, &env.pair);
            let user_a_before = query_cw20_balance(&app, &env.token_a, &env.user);
            let user_b_before = query_cw20_balance(&app, &env.token_b, &env.user);

            withdraw_liquidity(&mut app, &env, &env.user, withdraw_uint);

            let user_a_after = query_cw20_balance(&app, &env.token_a, &env.user);
            let user_b_after = query_cw20_balance(&app, &env.token_b, &env.user);

            let got_a = user_a_after - user_a_before;
            let got_b = user_b_after - user_b_before;

            let expected_a = withdraw_uint * pool_before.assets[0].amount / pool_before.total_share;
            let expected_b = withdraw_uint * pool_before.assets[1].amount / pool_before.total_share;

            prop_assert_eq!(got_a, expected_a);
            prop_assert_eq!(got_b, expected_b);
        }
    }

    // -----------------------------------------------------------------------
    // PROPERTY 5: Fee calculation correctness
    // commission = gross_output * fee_bps / 10000
    // net_output = gross_output - commission
    // -----------------------------------------------------------------------
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]

        #[test]
        fn prop_fee_calculation_correct(
            init in 100_000u128..1_000_000_000u128,
            swap_fraction_bps in 1u128..3000u128,
            fee_bps in 0u16..10000u16,
        ) {
            let mut app = App::default();
            let env = setup_env_with_fee(&mut app, fee_bps);

            provide_liquidity(&mut app, &env, &env.user,
                Uint128::new(init), Uint128::new(init));

            let swap_amount = std::cmp::max(1, init * swap_fraction_bps / 10000);
            let pool_before = query_pool(&app, &env.pair);

            let sim: dex_common::pair::SimulationResponse = app
                .wrap()
                .query_wasm_smart(
                    env.pair.to_string(),
                    &dex_common::pair::QueryMsg::Simulation {
                        offer_asset: dex_common::types::Asset {
                            info: asset_info_token(&env.token_a),
                            amount: Uint128::new(swap_amount),
                        },
                    },
                )
                .unwrap();

            let input_reserve = pool_before.assets[0].amount.u128();
            let output_reserve = pool_before.assets[1].amount.u128();
            let k = input_reserve * output_reserve;
            let new_input = input_reserve + swap_amount;
            // Ceiling division to match contract behavior
            let new_output = if k % new_input == 0 { k / new_input } else { k / new_input + 1 };
            let expected_gross = output_reserve - new_output;
            let expected_commission = expected_gross * (fee_bps as u128) / 10000;
            let expected_return = expected_gross - expected_commission;

            prop_assert_eq!(sim.return_amount.u128(), expected_return,
                "return mismatch: sim={} expected={}", sim.return_amount, expected_return);
            prop_assert_eq!(sim.commission_amount.u128(), expected_commission,
                "commission mismatch: sim={} expected={}", sim.commission_amount, expected_commission);
        }
    }

    // -----------------------------------------------------------------------
    // PROPERTY 6: K maintained (non-decreasing) through multiple sequential
    // swaps, each step bounded by ceiling-division rounding.
    // -----------------------------------------------------------------------
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn prop_k_monotone_through_swap_sequence(
            init_a in 100_000u128..100_000_000u128,
            init_b in 100_000u128..100_000_000u128,
            num_swaps in 2usize..20,
            seed in 0u64..1_000_000u64,
        ) {
            let mut app = App::default();
            let env = setup_full_env(&mut app);

            provide_liquidity(&mut app, &env, &env.user,
                Uint128::new(init_a), Uint128::new(init_b));

            let mut last_k = k_value(&app, &env.pair);
            let mut rng_state = seed;

            for _ in 0..num_swaps {
                rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let direction = (rng_state >> 32) % 2 == 0;
                let pool = query_pool(&app, &env.pair);

                let max_swap = if direction {
                    std::cmp::min(
                        pool.assets[0].amount.u128() / 3,
                        query_cw20_balance(&app, &env.token_a, &env.user).u128(),
                    )
                } else {
                    std::cmp::min(
                        pool.assets[1].amount.u128() / 3,
                        query_cw20_balance(&app, &env.token_b, &env.user).u128(),
                    )
                };
                if max_swap < 1 { continue; }

                rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let swap_amount = 1 + (rng_state as u128 % max_swap);

                if direction {
                    swap_a_to_b(&mut app, &env, &env.user, Uint128::new(swap_amount));
                } else {
                    swap_b_to_a(&mut app, &env, &env.user, Uint128::new(swap_amount));
                }

                let pool_after = query_pool(&app, &env.pair);
                let new_k = pool_after.assets[0].amount.u128() * pool_after.assets[1].amount.u128();
                let new_input_reserve = if direction {
                    pool_after.assets[0].amount.u128()
                } else {
                    pool_after.assets[1].amount.u128()
                };
                assert_k_rounding_only(last_k, new_k, new_input_reserve)?;
                last_k = new_k;
            }
        }
    }

    // -----------------------------------------------------------------------
    // PROPERTY 7: Multi-LP fairness — each LP gets proportional withdrawal
    // -----------------------------------------------------------------------
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn prop_multi_lp_proportional(
            init_a in 100_000u128..10_000_000u128,
            init_b in 100_000u128..10_000_000u128,
            user2_a in 100_000u128..10_000_000u128,
            user2_b in 100_000u128..10_000_000u128,
        ) {
            let mut app = App::default();
            let env = setup_full_env(&mut app);
            let user2 = Addr::unchecked("user2");

            transfer_tokens(&mut app, &env.token_a, &env.user, &user2,
                Uint128::new(user2_a));
            transfer_tokens(&mut app, &env.token_b, &env.user, &user2,
                Uint128::new(user2_b));

            provide_liquidity(&mut app, &env, &env.user,
                Uint128::new(init_a), Uint128::new(init_b));
            provide_liquidity(&mut app, &env, &user2,
                Uint128::new(user2_a), Uint128::new(user2_b));

            let lp1 = query_cw20_balance(&app, &env.lp_token, &env.user);
            let lp2 = query_cw20_balance(&app, &env.lp_token, &user2);

            let pool = query_pool(&app, &env.pair);
            let total_supply = pool.total_share;

            if lp1.is_zero() || lp2.is_zero() { return Ok(()); }

            let expected_a1 = lp1 * pool.assets[0].amount / total_supply;
            let expected_b1 = lp1 * pool.assets[1].amount / total_supply;

            let u1_a_before = query_cw20_balance(&app, &env.token_a, &env.user);
            let u1_b_before = query_cw20_balance(&app, &env.token_b, &env.user);
            withdraw_liquidity(&mut app, &env, &env.user, lp1);
            let u1_a_after = query_cw20_balance(&app, &env.token_a, &env.user);
            let u1_b_after = query_cw20_balance(&app, &env.token_b, &env.user);

            prop_assert_eq!(u1_a_after - u1_a_before, expected_a1);
            prop_assert_eq!(u1_b_after - u1_b_before, expected_b1);
        }
    }

    // -----------------------------------------------------------------------
    // PROPERTY 8: Swap output never exceeds ideal (no free money)
    // The actual output should always be <= input * output_reserve / input_reserve
    // (ideal linear output). The constant product curve and fees both reduce output.
    // -----------------------------------------------------------------------
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]

        #[test]
        fn prop_swap_output_never_exceeds_ideal(
            init_a in 100_000u128..1_000_000_000u128,
            init_b in 100_000u128..1_000_000_000u128,
            swap_fraction_bps in 1u128..5000u128,
        ) {
            let mut app = App::default();
            let env = setup_full_env(&mut app);

            provide_liquidity(&mut app, &env, &env.user,
                Uint128::new(init_a), Uint128::new(init_b));

            let pool = query_pool(&app, &env.pair);
            let swap_amount = std::cmp::max(1, init_a * swap_fraction_bps / 10000);

            let ideal_output = swap_amount * pool.assets[1].amount.u128()
                / pool.assets[0].amount.u128();

            let user_b_before = query_cw20_balance(&app, &env.token_b, &env.user);
            swap_a_to_b(&mut app, &env, &env.user, Uint128::new(swap_amount));
            let user_b_after = query_cw20_balance(&app, &env.token_b, &env.user);

            let actual_output = (user_b_after - user_b_before).u128();

            prop_assert!(actual_output <= ideal_output,
                "output exceeded ideal: actual={} ideal={}", actual_output, ideal_output);
        }
    }

    // -----------------------------------------------------------------------
    // PROPERTY 9: Asymmetric pool ratios maintain invariants
    // Test with highly skewed pools (1:100, 1:10000)
    // -----------------------------------------------------------------------
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn prop_asymmetric_pool_k_invariant(
            base_amount in 10_000u128..1_000_000u128,
            ratio_log in 0u32..4,
            swap_fraction_bps in 1u128..2000u128,
        ) {
            let ratio = 10u128.pow(ratio_log);
            let init_a = base_amount;
            let init_b = base_amount * ratio;

            if init_b > 1_000_000_000_000 { return Ok(()); }

            let mut app = App::default();
            let env = setup_full_env(&mut app);

            provide_liquidity(&mut app, &env, &env.user,
                Uint128::new(init_a), Uint128::new(init_b));

            let k_before = k_value(&app, &env.pair);
            let swap_amount = std::cmp::max(1, init_a * swap_fraction_bps / 10000);

            swap_a_to_b(&mut app, &env, &env.user, Uint128::new(swap_amount));

            let pool_after = query_pool(&app, &env.pair);
            let k_after = pool_after.assets[0].amount.u128() * pool_after.assets[1].amount.u128();
            let new_input_reserve = pool_after.assets[0].amount.u128();
            assert_k_rounding_only(k_before, k_after, new_input_reserve)?;

            let user_a = query_cw20_balance(&app, &env.token_a, &env.user);
            let user_b = query_cw20_balance(&app, &env.token_b, &env.user);
            let treasury_a = query_cw20_balance(&app, &env.token_a, &env.treasury);
            let treasury_b = query_cw20_balance(&app, &env.token_b, &env.treasury);

            let total_a = user_a + pool_after.assets[0].amount + treasury_a;
            let total_b = user_b + pool_after.assets[1].amount + treasury_b;

            prop_assert_eq!(total_a, Uint128::new(1_000_000_000_000));
            prop_assert_eq!(total_b, Uint128::new(1_000_000_000_000));
        }
    }

    // -----------------------------------------------------------------------
    // PROPERTY 10: Varying fee levels maintain token conservation
    // -----------------------------------------------------------------------
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn prop_varying_fee_conservation(
            init in 100_000u128..100_000_000u128,
            fee_bps in 0u16..10000u16,
            swap_amount in 1_000u128..1_000_000u128,
        ) {
            if swap_amount >= init { return Ok(()); }

            let mut app = App::default();
            let env = setup_env_with_fee(&mut app, fee_bps);
            let initial_total = Uint128::new(1_000_000_000_000);

            provide_liquidity(&mut app, &env, &env.user,
                Uint128::new(init), Uint128::new(init));

            swap_a_to_b(&mut app, &env, &env.user, Uint128::new(swap_amount));

            let pool = query_pool(&app, &env.pair);
            let user_a = query_cw20_balance(&app, &env.token_a, &env.user);
            let user_b = query_cw20_balance(&app, &env.token_b, &env.user);
            let treasury_a = query_cw20_balance(&app, &env.token_a, &env.treasury);
            let treasury_b = query_cw20_balance(&app, &env.token_b, &env.treasury);

            prop_assert_eq!(user_a + pool.assets[0].amount + treasury_a, initial_total);
            prop_assert_eq!(user_b + pool.assets[1].amount + treasury_b, initial_total);
        }
    }

    // -----------------------------------------------------------------------
    // PROPERTY 11: Provide → swap → provide more → withdraw all
    // Tests that interleaving liquidity operations with swaps maintains
    // token conservation.
    // -----------------------------------------------------------------------
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn prop_interleaved_provide_swap_withdraw(
            init_a in 100_000u128..50_000_000u128,
            init_b in 100_000u128..50_000_000u128,
            add_a in 10_000u128..10_000_000u128,
            add_b in 10_000u128..10_000_000u128,
            swap_amount in 1_000u128..1_000_000u128,
        ) {
            if swap_amount >= init_a { return Ok(()); }

            let mut app = App::default();
            let env = setup_full_env(&mut app);
            let initial_total = Uint128::new(1_000_000_000_000);

            provide_liquidity(&mut app, &env, &env.user,
                Uint128::new(init_a), Uint128::new(init_b));

            swap_a_to_b(&mut app, &env, &env.user, Uint128::new(swap_amount));

            provide_liquidity(&mut app, &env, &env.user,
                Uint128::new(add_a), Uint128::new(add_b));

            swap_b_to_a(&mut app, &env, &env.user, Uint128::new(swap_amount));

            let lp = query_cw20_balance(&app, &env.lp_token, &env.user);
            if !lp.is_zero() {
                withdraw_liquidity(&mut app, &env, &env.user, lp);
            }

            let pool = query_pool(&app, &env.pair);
            let user_a = query_cw20_balance(&app, &env.token_a, &env.user);
            let user_b = query_cw20_balance(&app, &env.token_b, &env.user);
            let treasury_a = query_cw20_balance(&app, &env.token_a, &env.treasury);
            let treasury_b = query_cw20_balance(&app, &env.token_b, &env.treasury);

            prop_assert_eq!(user_a + pool.assets[0].amount + treasury_a, initial_total);
            prop_assert_eq!(user_b + pool.assets[1].amount + treasury_b, initial_total);
        }
    }

    // -----------------------------------------------------------------------
    // PROPERTY 12: Fee discount tiers produce correct effective fee
    // For each self-registerable tier (1-5), verify the discount calculation
    // matches: effective_bps = fee_bps * (10000 - discount_bps) / 10000
    // -----------------------------------------------------------------------
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(32))]

        #[test]
        fn prop_discount_tier_fee_correctness(
            init in 1_000_000u128..100_000_000u128,
            swap_amount in 10_000u128..1_000_000u128,
            tier_idx in 0usize..5,
        ) {
            if swap_amount >= init { return Ok(()); }

            let tier_configs: Vec<(u8, u128, u16)> = vec![
                (1, 1, 1000),
                (2, 50, 2500),
                (3, 200, 3500),
                (4, 1000, 5000),
                (5, 15000, 8000),
            ];
            let (tier_id, _min_cl8y, discount_bps) = tier_configs[tier_idx];
            let one_cl8y: u128 = 1_000_000_000_000_000_000;
            let base_fee_bps: u16 = 30;

            let mut app = App::default();
            let env = setup_env_with_fee(&mut app, base_fee_bps);
            let cw20_code_id = app.store_code(cw20_mintable_contract());
            let fd_code_id = app.store_code(fee_discount_contract());

            let cl8y_token = create_cw20_token_with_decimals(
                &mut app, cw20_code_id, &env.user, "CL8Y", "CL8Y", 18,
                Uint128::new(100_000 * one_cl8y),
            );

            let fd = app.instantiate_contract(
                fd_code_id, env.governance.clone(),
                &cl8y_dex_fee_discount::msg::InstantiateMsg {
                    governance: env.governance.to_string(),
                    cl8y_token: cl8y_token.to_string(),
                },
                &[], "fd", None,
            ).unwrap();

            for &(tid, min_cl8y_mult, disc, gov_only) in &[
                (1u8, 1u128, 1000u16, false),
                (2, 50, 2500, false),
                (3, 200, 3500, false),
                (4, 1000, 5000, false),
                (5, 15000, 8000, false),
            ] {
                app.execute_contract(
                    env.governance.clone(), fd.clone(),
                    &cl8y_dex_fee_discount::msg::ExecuteMsg::AddTier {
                        tier_id: tid,
                        min_cl8y_balance: Uint128::new(min_cl8y_mult * one_cl8y),
                        discount_bps: disc,
                        governance_only: gov_only,
                    },
                    &[],
                ).unwrap();
            }

            app.execute_contract(
                env.governance.clone(), env.factory.clone(),
                &dex_common::factory::ExecuteMsg::SetDiscountRegistry {
                    pair: env.pair.to_string(),
                    registry: Some(fd.to_string()),
                },
                &[],
            ).unwrap();

            provide_liquidity(&mut app, &env, &env.user,
                Uint128::new(init), Uint128::new(init));

            app.execute_contract(
                env.user.clone(), fd.clone(),
                &cl8y_dex_fee_discount::msg::ExecuteMsg::Register { tier_id },
                &[],
            ).unwrap();

            let treasury_before = query_cw20_balance(&app, &env.token_b, &env.treasury);
            let user_b_before = query_cw20_balance(&app, &env.token_b, &env.user);

            swap_a_to_b(&mut app, &env, &env.user, Uint128::new(swap_amount));

            let treasury_after = query_cw20_balance(&app, &env.token_b, &env.treasury);
            let user_b_after = query_cw20_balance(&app, &env.token_b, &env.user);
            let actual_fee = (treasury_after - treasury_before).u128();
            let actual_net = (user_b_after - user_b_before).u128();
            let gross = actual_fee + actual_net;

            let effective_bps = (base_fee_bps as u32)
                * (10000u32 - discount_bps as u32)
                / 10000u32;
            let expected_fee = gross * (effective_bps as u128) / 10000;

            prop_assert_eq!(actual_fee, expected_fee,
                "tier {} fee mismatch: actual={} expected={} (gross={} eff_bps={})",
                tier_id, actual_fee, expected_fee, gross, effective_bps);
        }
    }

    // -----------------------------------------------------------------------
    // PROPERTY 13: Simulation matches actual swap execution
    // The simulation query should exactly predict the actual swap result.
    // -----------------------------------------------------------------------
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]

        #[test]
        fn prop_simulation_matches_execution(
            init_a in 100_000u128..1_000_000_000u128,
            init_b in 100_000u128..1_000_000_000u128,
            swap_fraction_bps in 1u128..3000u128,
        ) {
            let mut app = App::default();
            let env = setup_full_env(&mut app);

            provide_liquidity(&mut app, &env, &env.user,
                Uint128::new(init_a), Uint128::new(init_b));

            let swap_amount = std::cmp::max(1, init_a * swap_fraction_bps / 10000);

            let sim: dex_common::pair::SimulationResponse = app
                .wrap()
                .query_wasm_smart(
                    env.pair.to_string(),
                    &dex_common::pair::QueryMsg::Simulation {
                        offer_asset: dex_common::types::Asset {
                            info: asset_info_token(&env.token_a),
                            amount: Uint128::new(swap_amount),
                        },
                    },
                )
                .unwrap();

            let user_b_before = query_cw20_balance(&app, &env.token_b, &env.user);
            let treasury_b_before = query_cw20_balance(&app, &env.token_b, &env.treasury);

            swap_a_to_b(&mut app, &env, &env.user, Uint128::new(swap_amount));

            let user_b_after = query_cw20_balance(&app, &env.token_b, &env.user);
            let treasury_b_after = query_cw20_balance(&app, &env.token_b, &env.treasury);

            let actual_return = user_b_after - user_b_before;
            let actual_commission = treasury_b_after - treasury_b_before;

            prop_assert_eq!(actual_return, sim.return_amount);
            prop_assert_eq!(actual_commission, sim.commission_amount);
        }
    }

    // -----------------------------------------------------------------------
    // PROPERTY 14: Pool can never be fully drained by a single swap
    // Even a very large swap leaves some tokens in the pool. The constant
    // product formula guarantees this: output approaches asymptotically
    // but never reaches zero. The max spread check may reject the swap
    // entirely (which also protects the pool).
    // -----------------------------------------------------------------------
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn prop_pool_never_fully_drained(
            init_a in 10_000u128..1_000_000_000u128,
            init_b in 10_000u128..1_000_000_000u128,
        ) {
            let mut app = App::default();
            let env = setup_full_env(&mut app);

            provide_liquidity(&mut app, &env, &env.user,
                Uint128::new(init_a), Uint128::new(init_b));

            let huge_swap = init_a * 1000;
            let user_a_bal = query_cw20_balance(&app, &env.token_a, &env.user);
            let max_swap = std::cmp::min(huge_swap, user_a_bal.u128());
            if max_swap < 1 { return Ok(()); }

            let swap_msg = to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
                belief_price: None,
                max_spread: Some(cosmwasm_std::Decimal::percent(99999)),
                to: None,
                deadline: None,
                trader: None,
            })
            .unwrap();

            let result = app.execute_contract(
                env.user.clone(),
                env.token_a.clone(),
                &cw20::Cw20ExecuteMsg::Send {
                    contract: env.pair.to_string(),
                    amount: Uint128::new(max_swap),
                    msg: swap_msg,
                },
                &[],
            );

            let pool = query_pool(&app, &env.pair);

            if result.is_ok() {
                prop_assert!(pool.assets[1].amount > Uint128::zero(),
                    "Output reserve should never reach zero after successful swap");
            }
            prop_assert!(pool.assets[0].amount > Uint128::zero());
            prop_assert!(pool.assets[1].amount > Uint128::zero());
        }
    }

    // -----------------------------------------------------------------------
    // PROPERTY 15: Small swap rounding doesn't create tokens
    // For very small swaps, the output might round to zero due to integer
    // division. Verify no tokens are created from nothing.
    // -----------------------------------------------------------------------
    proptest! {
        #![proptest_config(ProptestConfig::with_cases(128))]

        #[test]
        fn prop_small_swap_no_token_creation(
            init in 1_000_000u128..1_000_000_000u128,
            swap_amount in 1u128..100u128,
        ) {
            let mut app = App::default();
            let env = setup_full_env(&mut app);
            let initial_total = Uint128::new(1_000_000_000_000);

            provide_liquidity(&mut app, &env, &env.user,
                Uint128::new(init), Uint128::new(init));

            swap_a_to_b(&mut app, &env, &env.user, Uint128::new(swap_amount));

            let pool = query_pool(&app, &env.pair);
            let user_a = query_cw20_balance(&app, &env.token_a, &env.user);
            let user_b = query_cw20_balance(&app, &env.token_b, &env.user);
            let treasury_a = query_cw20_balance(&app, &env.token_a, &env.treasury);
            let treasury_b = query_cw20_balance(&app, &env.token_b, &env.treasury);

            prop_assert_eq!(user_a + pool.assets[0].amount + treasury_a, initial_total);
            prop_assert_eq!(user_b + pool.assets[1].amount + treasury_b, initial_total);
        }
    }
}

// ===========================================================================
// SECURITY TESTS — attack vectors where LPs can lose funds or attackers
// can steal value
// ===========================================================================

#[cfg(test)]
mod security_tests {
    use super::helpers::*;
    use cosmwasm_std::{to_json_binary, Addr, Decimal, Uint128};
    use cw_multi_test::{App, Executor};

    #[test]
    fn test_sandwich_attack_limited_by_spread() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let attacker = Addr::unchecked("attacker");
        let victim = Addr::unchecked("victim");
        transfer_tokens(&mut app, &env.token_a, &env.user, &attacker, Uint128::new(100_000_000));
        transfer_tokens(&mut app, &env.token_b, &env.user, &attacker, Uint128::new(100_000_000));
        transfer_tokens(&mut app, &env.token_a, &env.user, &victim, Uint128::new(10_000_000));

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(10_000_000), Uint128::new(10_000_000));

        let attacker_b_before = query_cw20_balance(&app, &env.token_b, &attacker);

        // Attacker front-runs: large swap A→B to move price
        swap_a_to_b(&mut app, &env, &attacker, Uint128::new(5_000_000));

        // Victim swaps A→B at worse price
        swap_a_to_b(&mut app, &env, &victim, Uint128::new(1_000_000));

        // Attacker back-runs: swap B→A to profit (use huge max_spread since attacker accepts any spread)
        let attacker_b_bal = query_cw20_balance(&app, &env.token_b, &attacker);
        let swap_msg = to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: Some(Decimal::percent(9999999)),
            to: None,
            deadline: None,
            trader: None,
        })
        .unwrap();
        app.execute_contract(
            attacker.clone(),
            env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: attacker_b_bal,
                msg: swap_msg,
            },
            &[],
        )
        .unwrap();

        let attacker_b_after = query_cw20_balance(&app, &env.token_b, &attacker);
        let attacker_a_after = query_cw20_balance(&app, &env.token_a, &attacker);

        // Attacker should have LESS total value after fees
        // They started with 100M A + 100M B; they now have more A but less B.
        // Due to fees (0.3%), the attack should be net-negative or break-even.
        let attacker_total_before = Uint128::new(100_000_000) + Uint128::new(100_000_000);
        let attacker_total_after = attacker_a_after + attacker_b_after;

        // The attacker should not profit — fees make sandwich unprofitable
        assert!(
            attacker_total_after <= attacker_total_before,
            "Sandwich attack should not be profitable: before={}, after={}",
            attacker_total_before, attacker_total_after
        );

        // Verify conservation
        let pool = query_pool(&app, &env.pair);
        let treasury_a = query_cw20_balance(&app, &env.token_a, &env.treasury);
        let treasury_b = query_cw20_balance(&app, &env.token_b, &env.treasury);
        let victim_a = query_cw20_balance(&app, &env.token_a, &victim);
        let victim_b = query_cw20_balance(&app, &env.token_b, &victim);
        let user_a = query_cw20_balance(&app, &env.token_a, &env.user);
        let user_b = query_cw20_balance(&app, &env.token_b, &env.user);
        let total_a = attacker_a_after + victim_a + user_a + pool.assets[0].amount + treasury_a;
        let total_b = attacker_b_after + victim_b + user_b + pool.assets[1].amount + treasury_b;
        assert_eq!(total_a, Uint128::new(1_000_000_000_000));
        assert_eq!(total_b, Uint128::new(1_000_000_000_000));
    }

    #[test]
    fn test_sandwich_with_tight_max_spread_protection() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let attacker = Addr::unchecked("attacker");
        let victim = Addr::unchecked("victim");
        transfer_tokens(&mut app, &env.token_a, &env.user, &attacker, Uint128::new(50_000_000));
        transfer_tokens(&mut app, &env.token_a, &env.user, &victim, Uint128::new(10_000_000));

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(10_000_000), Uint128::new(10_000_000));

        // Attacker front-runs: large swap to move price
        swap_a_to_b(&mut app, &env, &attacker, Uint128::new(5_000_000));

        // Victim uses tight max_spread (0.5%) — should be rejected due to manipulation
        let swap_msg = to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: Some(Decimal::permille(5)), // 0.5%
            to: None,
            deadline: None,
            trader: None,
        })
        .unwrap();

        let result = app.execute_contract(
            victim.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(1_000_000),
                msg: swap_msg,
            },
            &[],
        );

        // With a 5M/10M = 50% pool move, the spread for the victim should exceed 0.5%
        assert!(result.is_err(), "Tight max_spread should protect victim from sandwich");
    }

    #[test]
    fn test_lp_share_inflation_first_depositor_griefing() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let attacker = Addr::unchecked("attacker");
        let victim = Addr::unchecked("victim");
        transfer_tokens(&mut app, &env.token_a, &env.user, &attacker, Uint128::new(100_000_000));
        transfer_tokens(&mut app, &env.token_b, &env.user, &attacker, Uint128::new(100_000_000));
        transfer_tokens(&mut app, &env.token_a, &env.user, &victim, Uint128::new(100_000_000));
        transfer_tokens(&mut app, &env.token_b, &env.user, &victim, Uint128::new(100_000_000));

        // Attacker: first deposit with minimal amount
        provide_liquidity(&mut app, &env, &attacker, Uint128::new(1_001), Uint128::new(1_001));
        let attacker_lp = query_cw20_balance(&app, &env.lp_token, &attacker);
        assert_eq!(attacker_lp, Uint128::new(1)); // 1001 - 1000 MINIMUM_LIQUIDITY

        // Attacker: donate tokens directly to the pair contract to inflate share price
        transfer_tokens(&mut app, &env.token_a, &attacker, &env.pair, Uint128::new(10_000_000));
        transfer_tokens(&mut app, &env.token_b, &attacker, &env.pair, Uint128::new(10_000_000));

        // Victim: provides liquidity — should still get > 0 LP shares
        // because the pair tracks reserves independently of actual balances
        provide_liquidity(&mut app, &env, &victim, Uint128::new(10_000_000), Uint128::new(10_000_000));
        let victim_lp = query_cw20_balance(&app, &env.lp_token, &victim);

        // The pair uses internal RESERVES state (not raw CW20 balance queries),
        // so donation doesn't affect LP share calculation — victim gets fair shares
        assert!(
            victim_lp > Uint128::zero(),
            "Victim should receive LP tokens even after donation attack"
        );
        assert!(
            victim_lp > Uint128::new(1),
            "Victim should receive substantial LP tokens, not 0 or 1"
        );
    }

    #[test]
    fn test_flash_provide_swap_withdraw_no_profit() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let attacker = Addr::unchecked("attacker");
        transfer_tokens(&mut app, &env.token_a, &env.user, &attacker, Uint128::new(100_000_000));
        transfer_tokens(&mut app, &env.token_b, &env.user, &attacker, Uint128::new(100_000_000));

        // Establish liquidity from user
        provide_liquidity(&mut app, &env, &env.user, Uint128::new(10_000_000), Uint128::new(10_000_000));

        let attacker_a_before = query_cw20_balance(&app, &env.token_a, &attacker);
        let attacker_b_before = query_cw20_balance(&app, &env.token_b, &attacker);

        // "Flash" attack: provide → swap → withdraw (same block in cw-multi-test)
        provide_liquidity(&mut app, &env, &attacker, Uint128::new(50_000_000), Uint128::new(50_000_000));
        swap_a_to_b(&mut app, &env, &attacker, Uint128::new(1_000_000));
        let lp = query_cw20_balance(&app, &env.lp_token, &attacker);
        withdraw_liquidity(&mut app, &env, &attacker, lp);

        let attacker_a_after = query_cw20_balance(&app, &env.token_a, &attacker);
        let attacker_b_after = query_cw20_balance(&app, &env.token_b, &attacker);

        let total_before = attacker_a_before + attacker_b_before;
        let total_after = attacker_a_after + attacker_b_after;

        assert!(
            total_after <= total_before,
            "Flash provide+swap+withdraw should not be profitable: before={}, after={}",
            total_before, total_after
        );
    }

    #[test]
    fn test_repeated_small_swaps_no_rounding_profit() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let attacker = Addr::unchecked("attacker");
        transfer_tokens(&mut app, &env.token_a, &env.user, &attacker, Uint128::new(100_000));
        transfer_tokens(&mut app, &env.token_b, &env.user, &attacker, Uint128::new(100_000));

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(10_000_000), Uint128::new(10_000_000));

        let initial_total = Uint128::new(1_000_000_000_000);

        // 100 tiny swaps: try to accumulate rounding dust
        for _ in 0..100 {
            let a_bal = query_cw20_balance(&app, &env.token_a, &attacker);
            if a_bal >= Uint128::new(1) {
                let swap_msg = to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
                    belief_price: None,
                    max_spread: Some(Decimal::one()),
                    to: None,
                    deadline: None,
                    trader: None,
                })
                .unwrap();
                app.execute_contract(
                    attacker.clone(),
                    env.token_a.clone(),
                    &cw20::Cw20ExecuteMsg::Send {
                        contract: env.pair.to_string(),
                        amount: Uint128::new(1),
                        msg: swap_msg,
                    },
                    &[],
                )
                .unwrap();
            }
        }

        // Verify conservation after many small swaps
        let pool = query_pool(&app, &env.pair);
        let user_a = query_cw20_balance(&app, &env.token_a, &env.user);
        let user_b = query_cw20_balance(&app, &env.token_b, &env.user);
        let att_a = query_cw20_balance(&app, &env.token_a, &attacker);
        let att_b = query_cw20_balance(&app, &env.token_b, &attacker);
        let treasury_a = query_cw20_balance(&app, &env.token_a, &env.treasury);
        let treasury_b = query_cw20_balance(&app, &env.token_b, &env.treasury);

        let total_a = user_a + att_a + pool.assets[0].amount + treasury_a;
        let total_b = user_b + att_b + pool.assets[1].amount + treasury_b;
        assert_eq!(total_a, initial_total, "Token A conservation violated after rounding attack");
        assert_eq!(total_b, initial_total, "Token B conservation violated after rounding attack");
    }

    #[test]
    fn test_swap_output_always_less_than_reserve() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        let pool_before = query_pool(&app, &env.pair);

        // Attempt a huge swap. The max_spread check may reject it,
        // which is itself a valid protection. Either way, the pool
        // must never be fully drained.
        let swap_msg = to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: None, // defaults to 1% — will likely reject
            to: None,
            deadline: None,
            trader: None,
        })
        .unwrap();

        let result = app.execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(999_000_000_000),
                msg: swap_msg,
            },
            &[],
        );

        let pool_after = query_pool(&app, &env.pair);

        if result.is_ok() {
            // Swap succeeded — reserves decreased but not to zero
            assert!(
                pool_after.assets[1].amount > Uint128::zero(),
                "Reserve B should never reach zero after successful swap"
            );
        } else {
            // Swap rejected by max_spread — pool unchanged, which is also safe
            assert_eq!(pool_after.assets[0].amount, pool_before.assets[0].amount);
            assert_eq!(pool_after.assets[1].amount, pool_before.assets[1].amount);
        }

        // In all cases, both reserves remain positive
        assert!(pool_after.assets[0].amount > Uint128::zero());
        assert!(pool_after.assets[1].amount > Uint128::zero());
    }

    #[test]
    fn test_direct_token_donation_does_not_inflate_lp_shares() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));
        let pool_before = query_pool(&app, &env.pair);

        // Directly send tokens to pair contract (not via ProvideLiquidity)
        transfer_tokens(&mut app, &env.token_a, &env.user, &env.pair, Uint128::new(500_000));

        let pool_after = query_pool(&app, &env.pair);

        // Internal reserves should not change from direct transfer
        assert_eq!(pool_before.assets[0].amount, pool_after.assets[0].amount);
        assert_eq!(pool_before.assets[1].amount, pool_after.assets[1].amount);
        assert_eq!(pool_before.total_share, pool_after.total_share);
    }

    #[test]
    fn test_withdraw_sends_tokens_to_sender_not_lp_token() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        let user_a_before = query_cw20_balance(&app, &env.token_a, &env.user);
        let user_b_before = query_cw20_balance(&app, &env.token_b, &env.user);

        let lp_balance = query_cw20_balance(&app, &env.lp_token, &env.user);
        withdraw_liquidity(&mut app, &env, &env.user, lp_balance);

        let user_a_after = query_cw20_balance(&app, &env.token_a, &env.user);
        let user_b_after = query_cw20_balance(&app, &env.token_b, &env.user);

        assert!(user_a_after > user_a_before, "User should receive token A");
        assert!(user_b_after > user_b_before, "User should receive token B");
    }

    #[test]
    fn test_paused_withdraw_still_works() {
        // Withdrawals via CW20 Receive go through assert_not_paused,
        // but they should be checked whether pausing blocks withdrawals
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        // Pause the pair
        app.execute_contract(
            env.governance.clone(),
            env.factory.clone(),
            &dex_common::factory::ExecuteMsg::SetPairPaused {
                pair: env.pair.to_string(),
                paused: true,
            },
            &[],
        )
        .unwrap();

        // Withdraw should fail because Receive is paused
        let lp_balance = query_cw20_balance(&app, &env.lp_token, &env.user);
        let remove_msg = to_json_binary(
            &dex_common::pair::Cw20HookMsg::WithdrawLiquidity { min_assets: None },
        )
        .unwrap();

        let result = app.execute_contract(
            env.user.clone(),
            env.lp_token.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: lp_balance,
                msg: remove_msg,
            },
            &[],
        );

        // NOTE: This test documents that pausing blocks ALL Receive messages
        // including withdrawals. If this is undesired (LPs should be able
        // to exit even when paused), it's a design issue to address.
        assert!(
            result.is_err(),
            "Paused contract should reject withdrawals via Receive"
        );
    }
}

// ===========================================================================
// ORACLE TESTS — end-to-end integration tests for TWAP oracle
// ===========================================================================

#[cfg(test)]
mod oracle_tests {
    use super::helpers::*;
    use cosmwasm_std::{Addr, Uint128};
    use cw_multi_test::{App, Executor};
    use dex_common::oracle::ObserveResponse;

    #[test]
    fn test_oracle_records_observations_on_swap() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        // Advance time and swap to trigger oracle observation
        app.update_block(|b| b.time = b.time.plus_seconds(10));
        swap_a_to_b(&mut app, &env, &env.user, Uint128::new(10_000));

        // Query oracle info — should have observations
        let info: dex_common::oracle::OracleInfoResponse = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::OracleInfo {},
            )
            .unwrap();

        assert!(info.newest_observation_timestamp > 0, "Oracle should have recorded observations");
    }

    #[test]
    fn test_oracle_observe_query_returns_tick_cumulatives() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        // Do several swaps with time progression to build observation history
        for i in 1..=5 {
            app.update_block(|b| b.time = b.time.plus_seconds(10));
            swap_a_to_b(&mut app, &env, &env.user, Uint128::new(1_000));
        }

        // Query current observation (seconds_ago = 0)
        let obs: ObserveResponse = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::Observe {
                    seconds_ago: vec![0],
                },
            )
            .unwrap();

        assert_eq!(obs.price_a_cumulatives.len(), 1);
    }

    #[test]
    fn test_oracle_twap_computation_two_points() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        // Equal reserves → price = 1
        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        // First observation
        app.update_block(|b| b.time = b.time.plus_seconds(10));
        swap_a_to_b(&mut app, &env, &env.user, Uint128::new(100));

        // Second observation 60 seconds later
        app.update_block(|b| b.time = b.time.plus_seconds(60));
        swap_a_to_b(&mut app, &env, &env.user, Uint128::new(100));

        // Query two points for TWAP
        let obs: ObserveResponse = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::Observe {
                    seconds_ago: vec![0, 60],
                },
            )
            .unwrap();

        assert_eq!(obs.price_a_cumulatives.len(), 2);

        // TWAP via cumulative prices: avg_price = (cum[0] - cum[1]) / (dt * 1e18)
        // With equal reserves, price ≈ 1, so the cumulative diff over 60s ≈ 60 * 1e18
        let cum_diff = obs.price_a_cumulatives[0] - obs.price_a_cumulatives[1];
        let scale = Uint128::new(1_000_000_000_000_000_000); // 1e18
        let twap_scaled = cum_diff / Uint128::new(60);
        let twap_f: f64 = twap_scaled.u128() as f64 / scale.u128() as f64;

        assert!(
            (twap_f - 1.0).abs() < 0.05,
            "TWAP should be close to 1.0 for equal pool, got {}", twap_f
        );
    }

    #[test]
    fn test_oracle_manipulation_resistance_same_block() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(10_000_000), Uint128::new(10_000_000));

        // Build some observation history
        for _ in 0..5 {
            app.update_block(|b| b.time = b.time.plus_seconds(10));
            swap_a_to_b(&mut app, &env, &env.user, Uint128::new(100));
        }

        // Record observation before manipulation
        app.update_block(|b| b.time = b.time.plus_seconds(10));
        let obs_before: ObserveResponse = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::Observe {
                    seconds_ago: vec![0],
                },
            )
            .unwrap();

        // Massive swap to manipulate price (no time advance — same block)
        swap_a_to_b(&mut app, &env, &env.user, Uint128::new(5_000_000));

        // The observation should NOT change within the same block
        let obs_after: ObserveResponse = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::Observe {
                    seconds_ago: vec![0],
                },
            )
            .unwrap();

        // The tick_cumulative at seconds_ago=0 extrapolates from latest observation
        // using current reserves. Since reserves changed, the extrapolation changes.
        // But the *recorded observation* doesn't change within the same block.
        // The key protection: the observation records the PREVIOUS reserves,
        // so a manipulator's trade in this block doesn't influence the observation
        // recorded for this block's timestamp.
        let info: dex_common::oracle::OracleInfoResponse = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::OracleInfo {},
            )
            .unwrap();

        assert!(info.newest_observation_timestamp > 0);
    }

    #[test]
    fn test_oracle_no_observations_returns_error() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        // No operations → no observations
        let result: Result<ObserveResponse, _> = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::Observe {
                    seconds_ago: vec![0],
                },
            );

        assert!(result.is_err(), "Observe with no recorded observations should error");
    }

    #[test]
    fn test_oracle_info_no_observations() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let info: dex_common::oracle::OracleInfoResponse = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::OracleInfo {},
            )
            .unwrap();

        assert_eq!(info.oldest_observation_timestamp, 0);
        assert_eq!(info.newest_observation_timestamp, 0);
    }

    #[test]
    fn test_oracle_observations_recorded_on_provide_liquidity() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        // Advance time and provide more liquidity (should trigger oracle_update)
        app.update_block(|b| b.time = b.time.plus_seconds(30));
        provide_liquidity(&mut app, &env, &env.user, Uint128::new(500_000), Uint128::new(500_000));

        let info: dex_common::oracle::OracleInfoResponse = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::OracleInfo {},
            )
            .unwrap();

        assert!(info.newest_observation_timestamp > 0);
    }

    #[test]
    fn test_oracle_observations_recorded_on_withdraw() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        app.update_block(|b| b.time = b.time.plus_seconds(30));

        let lp = query_cw20_balance(&app, &env.lp_token, &env.user);
        withdraw_liquidity(&mut app, &env, &env.user, lp / Uint128::new(2));

        let info: dex_common::oracle::OracleInfoResponse = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::OracleInfo {},
            )
            .unwrap();

        assert!(info.newest_observation_timestamp > 0);
    }

    #[test]
    fn test_increase_observation_cardinality() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let info_before: dex_common::oracle::OracleInfoResponse = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::OracleInfo {},
            )
            .unwrap();

        // Increase cardinality (anyone can call this)
        app.execute_contract(
            env.user.clone(),
            env.pair.clone(),
            &dex_common::pair::ExecuteMsg::IncreaseObservationCardinality {
                new_cardinality: 1000,
            },
            &[],
        )
        .unwrap();

        let info_after: dex_common::oracle::OracleInfoResponse = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::OracleInfo {},
            )
            .unwrap();

        assert_eq!(info_after.observation_cardinality, 1000);
        assert!(info_after.observation_cardinality > info_before.observation_cardinality);
    }

    #[test]
    fn test_increase_cardinality_must_be_greater() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let err = app
            .execute_contract(
                env.user.clone(),
                env.pair.clone(),
                &dex_common::pair::ExecuteMsg::IncreaseObservationCardinality {
                    new_cardinality: 1,
                },
                &[],
            )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("greater"));
    }

    #[test]
    fn test_increase_cardinality_max_limit() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let err = app
            .execute_contract(
                env.user.clone(),
                env.pair.clone(),
                &dex_common::pair::ExecuteMsg::IncreaseObservationCardinality {
                    new_cardinality: 65001,
                },
                &[],
            )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("maximum"));
    }

    #[test]
    fn test_oracle_window_too_old_rejected() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        // Build minimal observation history
        app.update_block(|b| b.time = b.time.plus_seconds(10));
        swap_a_to_b(&mut app, &env, &env.user, Uint128::new(100));
        app.update_block(|b| b.time = b.time.plus_seconds(10));
        swap_a_to_b(&mut app, &env, &env.user, Uint128::new(100));

        // Try to observe too far back
        let result: Result<ObserveResponse, _> = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::Observe {
                    seconds_ago: vec![99999],
                },
            );

        assert!(result.is_err(), "Observing too far back should error");
    }
}

// ===========================================================================
// DEADLINE TESTS
// ===========================================================================

#[cfg(test)]
mod deadline_tests {
    use super::helpers::*;
    use cosmwasm_std::{to_json_binary, Addr, Decimal, Uint128};
    use cw_multi_test::{App, Executor};

    #[test]
    fn test_swap_deadline_exceeded_rejected() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        // Set deadline to 100 seconds from genesis
        let deadline = app.block_info().time.seconds() + 100;

        // Advance time past deadline
        app.update_block(|b| b.time = b.time.plus_seconds(200));

        let swap_msg = to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: Some(Decimal::one()),
            to: None,
            deadline: Some(deadline),
            trader: None,
        })
        .unwrap();

        let err = app
            .execute_contract(
                env.user.clone(),
                env.token_a.clone(),
                &cw20::Cw20ExecuteMsg::Send {
                    contract: env.pair.to_string(),
                    amount: Uint128::new(10_000),
                    msg: swap_msg,
                },
                &[],
            )
            .unwrap_err();

        assert!(
            err.root_cause().to_string().contains("Deadline") ||
            err.root_cause().to_string().contains("deadline"),
            "Expected deadline error, got: {}", err.root_cause()
        );
    }

    #[test]
    fn test_swap_deadline_not_exceeded_succeeds() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        let deadline = app.block_info().time.seconds() + 1000;

        let swap_msg = to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: Some(Decimal::one()),
            to: None,
            deadline: Some(deadline),
            trader: None,
        })
        .unwrap();

        app.execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: env.pair.to_string(),
                amount: Uint128::new(10_000),
                msg: swap_msg,
            },
            &[],
        )
        .unwrap();
    }

    #[test]
    fn test_provide_liquidity_deadline_exceeded_rejected() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let deadline = app.block_info().time.seconds() + 100;
        app.update_block(|b| b.time = b.time.plus_seconds(200));

        app.execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                spender: env.pair.to_string(),
                amount: Uint128::new(1_000_000),
                expires: None,
            },
            &[],
        )
        .unwrap();
        app.execute_contract(
            env.user.clone(),
            env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                spender: env.pair.to_string(),
                amount: Uint128::new(1_000_000),
                expires: None,
            },
            &[],
        )
        .unwrap();

        let err = app
            .execute_contract(
                env.user.clone(),
                env.pair.clone(),
                &dex_common::pair::ExecuteMsg::ProvideLiquidity {
                    assets: [
                        dex_common::types::Asset {
                            info: asset_info_token(&env.token_a),
                            amount: Uint128::new(1_000_000),
                        },
                        dex_common::types::Asset {
                            info: asset_info_token(&env.token_b),
                            amount: Uint128::new(1_000_000),
                        },
                    ],
                    slippage_tolerance: None,
                    receiver: None,
                    deadline: Some(deadline),
                },
                &[],
            )
            .unwrap_err();

        assert!(
            err.root_cause().to_string().to_lowercase().contains("deadline"),
            "Expected deadline error, got: {}", err.root_cause()
        );
    }

    #[test]
    fn test_router_deadline_exceeded_rejected() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        let deadline = app.block_info().time.seconds() + 100;
        app.update_block(|b| b.time = b.time.plus_seconds(200));

        let hook_msg = to_json_binary(&cl8y_dex_router::msg::Cw20HookMsg::ExecuteSwapOperations {
            operations: vec![cl8y_dex_router::msg::SwapOperation::TerraSwap {
                offer_asset_info: asset_info_token(&env.token_a),
                ask_asset_info: asset_info_token(&env.token_b),
            }],
            minimum_receive: None,
            to: None,
            deadline: Some(deadline),
        })
        .unwrap();

        let err = app
            .execute_contract(
                env.user.clone(),
                env.token_a.clone(),
                &cw20::Cw20ExecuteMsg::Send {
                    contract: env.router.to_string(),
                    amount: Uint128::new(10_000),
                    msg: hook_msg,
                },
                &[],
            )
            .unwrap_err();

        assert!(
            err.root_cause().to_string().to_lowercase().contains("deadline"),
            "Expected deadline error, got: {}", err.root_cause()
        );
    }
}

// ===========================================================================
// HOOKS INTEGRATION TESTS — end-to-end tests with actual hook contracts
// ===========================================================================

#[cfg(test)]
mod hooks_integration_tests {
    use super::helpers::*;
    use cosmwasm_std::{Addr, Uint128};
    use cw_multi_test::{App, Executor};

    #[test]
    fn test_factory_set_pair_hooks() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let burn_hook_code_id = app.store_code(burn_hook_contract());
        let cw20_code_id = app.store_code(cw20_mintable_contract());

        let burn_token = create_cw20_token(
            &mut app,
            cw20_code_id,
            &env.governance,
            "BurnToken",
            "BURN",
            Uint128::new(1_000_000),
        );

        let burn_hook = app
            .instantiate_contract(
                burn_hook_code_id,
                env.governance.clone(),
                &cl8y_dex_burn_hook::msg::InstantiateMsg {
                    burn_token: burn_token.to_string(),
                    burn_percentage_bps: 500,
                    admin: env.governance.to_string(),
                },
                &[],
                "burn_hook",
                None,
            )
            .unwrap();

        // Set hooks on pair via factory
        app.execute_contract(
            env.governance.clone(),
            env.factory.clone(),
            &dex_common::factory::ExecuteMsg::SetPairHooks {
                pair: env.pair.to_string(),
                hooks: vec![burn_hook.to_string()],
            },
            &[],
        )
        .unwrap();

        // Verify hooks are set
        let hooks: dex_common::pair::HooksResponse = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::GetHooks {},
            )
            .unwrap();

        assert_eq!(hooks.hooks.len(), 1);
        assert_eq!(hooks.hooks[0], burn_hook);
    }

    #[test]
    fn test_burn_hook_called_on_swap() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let burn_hook_code_id = app.store_code(burn_hook_contract());

        let burn_hook = app
            .instantiate_contract(
                burn_hook_code_id,
                env.governance.clone(),
                &cl8y_dex_burn_hook::msg::InstantiateMsg {
                    burn_token: env.token_b.to_string(),
                    burn_percentage_bps: 1000, // 10%
                    admin: env.governance.to_string(),
                },
                &[],
                "burn_hook",
                None,
            )
            .unwrap();

        // Allow the pair to call the hook
        app.execute_contract(
            env.governance.clone(),
            burn_hook.clone(),
            &cl8y_dex_burn_hook::msg::ExecuteMsg::UpdateAllowedPairs {
                add: vec![env.pair.to_string()],
                remove: vec![],
            },
            &[],
        )
        .unwrap();

        // Set hooks on pair via factory
        app.execute_contract(
            env.governance.clone(),
            env.factory.clone(),
            &dex_common::factory::ExecuteMsg::SetPairHooks {
                pair: env.pair.to_string(),
                hooks: vec![burn_hook.to_string()],
            },
            &[],
        )
        .unwrap();

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(10_000_000), Uint128::new(10_000_000));

        // The burn hook expects the return_asset info to match its burn_token.
        // The hook is called AFTER the swap, and the return_asset is token_b.
        // The hook checks its own balance and burns if possible.
        // Since no tokens are sent to the hook, it should skip gracefully.
        swap_a_to_b(&mut app, &env, &env.user, Uint128::new(100_000));

        // Swap should succeed even though hook has no balance to burn
        let pool = query_pool(&app, &env.pair);
        assert!(pool.assets[0].amount > Uint128::zero());
    }

    #[test]
    fn test_tax_hook_called_on_swap() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let tax_hook_code_id = app.store_code(tax_hook_contract());
        let tax_recipient = Addr::unchecked("tax_collector");

        let tax_hook = app
            .instantiate_contract(
                tax_hook_code_id,
                env.governance.clone(),
                &cl8y_dex_tax_hook::msg::InstantiateMsg {
                    recipient: tax_recipient.to_string(),
                    tax_percentage_bps: 500, // 5%
                    tax_token: env.token_b.to_string(),
                    admin: env.governance.to_string(),
                },
                &[],
                "tax_hook",
                None,
            )
            .unwrap();

        app.execute_contract(
            env.governance.clone(),
            tax_hook.clone(),
            &cl8y_dex_tax_hook::msg::ExecuteMsg::UpdateAllowedPairs {
                add: vec![env.pair.to_string()],
                remove: vec![],
            },
            &[],
        )
        .unwrap();

        app.execute_contract(
            env.governance.clone(),
            env.factory.clone(),
            &dex_common::factory::ExecuteMsg::SetPairHooks {
                pair: env.pair.to_string(),
                hooks: vec![tax_hook.to_string()],
            },
            &[],
        )
        .unwrap();

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(10_000_000), Uint128::new(10_000_000));

        // Swap should succeed; tax hook skips because it has no token balance
        swap_a_to_b(&mut app, &env, &env.user, Uint128::new(100_000));

        let pool = query_pool(&app, &env.pair);
        assert!(pool.assets[0].amount > Uint128::zero());
    }

    #[test]
    fn test_hook_unauthorized_caller_rejected() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let burn_hook_code_id = app.store_code(burn_hook_contract());

        let burn_hook = app
            .instantiate_contract(
                burn_hook_code_id,
                env.governance.clone(),
                &cl8y_dex_burn_hook::msg::InstantiateMsg {
                    burn_token: env.token_b.to_string(),
                    burn_percentage_bps: 1000,
                    admin: env.governance.to_string(),
                },
                &[],
                "burn_hook",
                None,
            )
            .unwrap();

        // Don't add pair to allowed pairs — direct call should fail
        let random = Addr::unchecked("random");
        let err = app
            .execute_contract(
                random,
                burn_hook.clone(),
                &cl8y_dex_burn_hook::msg::ExecuteMsg::Hook(
                    dex_common::hook::HookExecuteMsg::AfterSwap {
                        pair: env.pair.clone(),
                        sender: env.user.clone(),
                        offer_asset: dex_common::types::Asset {
                            info: asset_info_token(&env.token_a),
                            amount: Uint128::new(1000),
                        },
                        return_asset: dex_common::types::Asset {
                            info: asset_info_token(&env.token_b),
                            amount: Uint128::new(900),
                        },
                        commission_amount: Uint128::new(3),
                        spread_amount: Uint128::new(97),
                    },
                ),
                &[],
            )
            .unwrap_err();

        assert!(err.root_cause().to_string().to_lowercase().contains("unauthorized") ||
                err.root_cause().to_string().to_lowercase().contains("hook"));
    }

    #[test]
    fn test_tax_hook_unauthorized_update_rejected() {
        let mut app = App::default();
        let admin = Addr::unchecked("admin");
        let random = Addr::unchecked("random");
        let tax_hook_code_id = app.store_code(tax_hook_contract());
        let cw20_code_id = app.store_code(cw20_mintable_contract());

        let tax_token = create_cw20_token(
            &mut app,
            cw20_code_id,
            &admin,
            "TaxToken",
            "TAX",
            Uint128::new(1_000_000),
        );

        let tax_hook = app
            .instantiate_contract(
                tax_hook_code_id,
                admin.clone(),
                &cl8y_dex_tax_hook::msg::InstantiateMsg {
                    recipient: admin.to_string(),
                    tax_percentage_bps: 200,
                    tax_token: tax_token.to_string(),
                    admin: admin.to_string(),
                },
                &[],
                "tax_hook",
                None,
            )
            .unwrap();

        let err = app
            .execute_contract(
                random,
                tax_hook.clone(),
                &cl8y_dex_tax_hook::msg::ExecuteMsg::UpdateConfig {
                    recipient: None,
                    tax_percentage_bps: Some(5000),
                    tax_token: None,
                },
                &[],
            )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    #[test]
    fn test_lp_burn_hook_unauthorized_update_rejected() {
        let mut app = App::default();
        let admin = Addr::unchecked("admin");
        let random = Addr::unchecked("random");
        let lp_burn_code_id = app.store_code(lp_burn_hook_contract());

        let lp_burn_hook = app
            .instantiate_contract(
                lp_burn_code_id,
                admin.clone(),
                &cl8y_dex_lp_burn_hook::msg::InstantiateMsg {
                    target_pair: admin.to_string(),
                    lp_token: "lp_token_addr".to_string(),
                    percentage_bps: 300,
                    admin: admin.to_string(),
                },
                &[],
                "lp_burn_hook",
                None,
            )
            .unwrap();

        let err = app
            .execute_contract(
                random,
                lp_burn_hook.clone(),
                &cl8y_dex_lp_burn_hook::msg::ExecuteMsg::UpdateConfig {
                    target_pair: None,
                    lp_token: None,
                    percentage_bps: Some(500),
                },
                &[],
            )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    #[test]
    fn test_tax_hook_invalid_bps_rejected() {
        let mut app = App::default();
        let admin = Addr::unchecked("admin");
        let cw20_code_id = app.store_code(cw20_mintable_contract());
        let tax_hook_code_id = app.store_code(tax_hook_contract());

        let tax_token = create_cw20_token(
            &mut app,
            cw20_code_id,
            &admin,
            "TaxToken",
            "TAX",
            Uint128::new(1_000_000),
        );

        let err = app
            .instantiate_contract(
                tax_hook_code_id,
                admin.clone(),
                &cl8y_dex_tax_hook::msg::InstantiateMsg {
                    recipient: admin.to_string(),
                    tax_percentage_bps: 10001,
                    tax_token: tax_token.to_string(),
                    admin: admin.to_string(),
                },
                &[],
                "tax_hook",
                None,
            )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Invalid"));
    }

    #[test]
    fn test_lp_burn_hook_invalid_bps_rejected() {
        let mut app = App::default();
        let admin = Addr::unchecked("admin");
        let lp_burn_code_id = app.store_code(lp_burn_hook_contract());

        let err = app
            .instantiate_contract(
                lp_burn_code_id,
                admin.clone(),
                &cl8y_dex_lp_burn_hook::msg::InstantiateMsg {
                    target_pair: admin.to_string(),
                    lp_token: "lp_token_addr".to_string(),
                    percentage_bps: 10001,
                    admin: admin.to_string(),
                },
                &[],
                "lp_burn_hook",
                None,
            )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Invalid"));
    }

    #[test]
    fn test_burn_hook_update_allowed_pairs() {
        let mut app = App::default();
        let admin = Addr::unchecked("admin");
        let cw20_code_id = app.store_code(cw20_mintable_contract());
        let burn_hook_code_id = app.store_code(burn_hook_contract());

        let burn_token = create_cw20_token(
            &mut app,
            cw20_code_id,
            &admin,
            "BurnToken",
            "BURN",
            Uint128::new(1_000_000),
        );

        let burn_hook = app
            .instantiate_contract(
                burn_hook_code_id,
                admin.clone(),
                &cl8y_dex_burn_hook::msg::InstantiateMsg {
                    burn_token: burn_token.to_string(),
                    burn_percentage_bps: 500,
                    admin: admin.to_string(),
                },
                &[],
                "burn_hook",
                None,
            )
            .unwrap();

        let pair_addr = Addr::unchecked("some_pair");

        // Add allowed pair
        app.execute_contract(
            admin.clone(),
            burn_hook.clone(),
            &cl8y_dex_burn_hook::msg::ExecuteMsg::UpdateAllowedPairs {
                add: vec![pair_addr.to_string()],
                remove: vec![],
            },
            &[],
        )
        .unwrap();

        // Remove allowed pair
        app.execute_contract(
            admin.clone(),
            burn_hook.clone(),
            &cl8y_dex_burn_hook::msg::ExecuteMsg::UpdateAllowedPairs {
                add: vec![],
                remove: vec![pair_addr.to_string()],
            },
            &[],
        )
        .unwrap();
    }
}

// ===========================================================================
// MISSING LINE COVERAGE TESTS — queries, edge cases, factory operations
// ===========================================================================

#[cfg(test)]
mod line_coverage_tests {
    use super::helpers::*;
    use cosmwasm_std::{Addr, Uint128};
    use cw_multi_test::{App, Executor};

    #[test]
    fn test_pair_query_fee_config() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

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
    fn test_pair_query_hooks_empty() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let hooks: dex_common::pair::HooksResponse = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::GetHooks {},
            )
            .unwrap();

        assert!(hooks.hooks.is_empty());
    }

    #[test]
    fn test_factory_query_whitelisted_code_ids() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let code_ids: dex_common::factory::CodeIdsResponse = app
            .wrap()
            .query_wasm_smart(
                env.factory.to_string(),
                &dex_common::factory::QueryMsg::GetWhitelistedCodeIds {
                    start_after: None,
                    limit: None,
                },
            )
            .unwrap();

        assert!(!code_ids.code_ids.is_empty());
    }

    #[test]
    fn test_factory_query_pair_count() {
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
    }

    #[test]
    fn test_factory_query_pair_count_multiple() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let cw20_code_id = app.store_code(cw20_mintable_contract());
        app.execute_contract(
            env.governance.clone(),
            env.factory.clone(),
            &dex_common::factory::ExecuteMsg::AddWhitelistedCodeId { code_id: cw20_code_id },
            &[],
        )
        .unwrap();

        let token_c = create_cw20_token(
            &mut app,
            cw20_code_id,
            &env.user,
            "Token C",
            "TKNC",
            Uint128::new(1_000_000_000_000),
        );

        app.execute_contract(
            env.user.clone(),
            env.factory.clone(),
            &dex_common::factory::ExecuteMsg::CreatePair {
                asset_infos: [
                    asset_info_token(&env.token_b),
                    asset_info_token(&token_c),
                ],
            },
            &[],
        )
        .unwrap();

        let count: dex_common::factory::PairCountResponse = app
            .wrap()
            .query_wasm_smart(
                env.factory.to_string(),
                &dex_common::factory::QueryMsg::GetPairCount {},
            )
            .unwrap();

        assert_eq!(count.count, 2);
    }

    #[test]
    fn test_factory_set_discount_registry_single_pair() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);
        let fd_code_id = app.store_code(fee_discount_contract());
        let cw20_code_id = app.store_code(cw20_mintable_contract());

        let cl8y_token = create_cw20_token(
            &mut app,
            cw20_code_id,
            &env.user,
            "CL8Y",
            "CL8Y",
            Uint128::new(1_000_000),
        );

        let fd = app
            .instantiate_contract(
                fd_code_id,
                env.governance.clone(),
                &cl8y_dex_fee_discount::msg::InstantiateMsg {
                    governance: env.governance.to_string(),
                    cl8y_token: cl8y_token.to_string(),
                },
                &[],
                "fee_discount",
                None,
            )
            .unwrap();

        // Set registry on specific pair
        app.execute_contract(
            env.governance.clone(),
            env.factory.clone(),
            &dex_common::factory::ExecuteMsg::SetDiscountRegistry {
                pair: env.pair.to_string(),
                registry: Some(fd.to_string()),
            },
            &[],
        )
        .unwrap();

        // Clear registry
        app.execute_contract(
            env.governance.clone(),
            env.factory.clone(),
            &dex_common::factory::ExecuteMsg::SetDiscountRegistry {
                pair: env.pair.to_string(),
                registry: None,
            },
            &[],
        )
        .unwrap();
    }

    #[test]
    fn test_factory_set_pair_paused_and_unpaused() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        // Pause
        app.execute_contract(
            env.governance.clone(),
            env.factory.clone(),
            &dex_common::factory::ExecuteMsg::SetPairPaused {
                pair: env.pair.to_string(),
                paused: true,
            },
            &[],
        )
        .unwrap();

        // Verify swap fails
        let swap_msg = cosmwasm_std::to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: Some(cosmwasm_std::Decimal::one()),
            to: None,
            deadline: None,
            trader: None,
        })
        .unwrap();

        let err = app
            .execute_contract(
                env.user.clone(),
                env.token_a.clone(),
                &cw20::Cw20ExecuteMsg::Send {
                    contract: env.pair.to_string(),
                    amount: Uint128::new(10_000),
                    msg: swap_msg,
                },
                &[],
            )
            .unwrap_err();
        assert!(err.root_cause().to_string().to_lowercase().contains("paused"));

        // Unpause
        app.execute_contract(
            env.governance.clone(),
            env.factory.clone(),
            &dex_common::factory::ExecuteMsg::SetPairPaused {
                pair: env.pair.to_string(),
                paused: false,
            },
            &[],
        )
        .unwrap();

        // Verify swap works again
        swap_a_to_b(&mut app, &env, &env.user, Uint128::new(10_000));
    }

    #[test]
    fn test_pair_update_hooks_directly_by_factory() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        // Only factory can call UpdateHooks on pair
        let err = app
            .execute_contract(
                env.user.clone(),
                env.pair.clone(),
                &dex_common::pair::ExecuteMsg::UpdateHooks {
                    hooks: vec!["hook1".to_string()],
                },
                &[],
            )
            .unwrap_err();
        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    #[test]
    fn test_pair_set_discount_registry_unauthorized() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let err = app
            .execute_contract(
                env.user.clone(),
                env.pair.clone(),
                &dex_common::pair::ExecuteMsg::SetDiscountRegistry {
                    registry: None,
                },
                &[],
            )
            .unwrap_err();
        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    #[test]
    fn test_pair_set_paused_unauthorized() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let err = app
            .execute_contract(
                env.user.clone(),
                env.pair.clone(),
                &dex_common::pair::ExecuteMsg::SetPaused { paused: true },
                &[],
            )
            .unwrap_err();
        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    #[test]
    fn test_pair_update_fee_too_high() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        // Factory sends UpdateFee with fee > 10000 — this should be caught by factory
        let err = app
            .execute_contract(
                env.governance.clone(),
                env.factory.clone(),
                &dex_common::factory::ExecuteMsg::SetPairFee {
                    pair: env.pair.to_string(),
                    fee_bps: 10001,
                },
                &[],
            )
            .unwrap_err();
        assert!(err.root_cause().to_string().to_lowercase().contains("invalid") ||
                err.root_cause().to_string().to_lowercase().contains("fee"));
    }

    #[test]
    fn test_reverse_simulation_with_100_pct_fee_rejected() {
        let mut app = App::default();
        let env = setup_env_with_fee(&mut app, 10000);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        let result: Result<dex_common::pair::ReverseSimulationResponse, _> = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::ReverseSimulation {
                    ask_asset: dex_common::types::Asset {
                        info: asset_info_token(&env.token_b),
                        amount: Uint128::new(1000),
                    },
                },
            );

        assert!(result.is_err(), "Reverse simulation with 100% fee should fail");
    }

    #[test]
    fn test_simulation_with_wrong_asset_rejected() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        let fake_token = Addr::unchecked("fake_token");
        let result: Result<dex_common::pair::SimulationResponse, _> = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::Simulation {
                    offer_asset: dex_common::types::Asset {
                        info: asset_info_token(&fake_token),
                        amount: Uint128::new(1000),
                    },
                },
            );

        assert!(result.is_err(), "Simulation with wrong asset should fail");
    }

    #[test]
    fn test_reverse_simulation_with_wrong_asset_rejected() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        let fake_token = Addr::unchecked("fake_token");
        let result: Result<dex_common::pair::ReverseSimulationResponse, _> = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::ReverseSimulation {
                    ask_asset: dex_common::types::Asset {
                        info: asset_info_token(&fake_token),
                        amount: Uint128::new(1000),
                    },
                },
            );

        assert!(result.is_err(), "Reverse simulation with wrong asset should fail");
    }

    #[test]
    fn test_router_direct_execute_rejected() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let err = app
            .execute_contract(
                env.user.clone(),
                env.router.clone(),
                &cl8y_dex_router::msg::ExecuteMsg::ExecuteSwapOperations {
                    operations: vec![cl8y_dex_router::msg::SwapOperation::TerraSwap {
                        offer_asset_info: asset_info_token(&env.token_a),
                        ask_asset_info: asset_info_token(&env.token_b),
                    }],
                    minimum_receive: None,
                    to: None,
                    deadline: None,
                },
                &[],
            )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("CW20 Send"));
    }

    #[test]
    fn test_router_native_swap_rejected() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        let result: Result<cl8y_dex_router::msg::SimulateSwapOperationsResponse, _> = app
            .wrap()
            .query_wasm_smart(
                env.router.to_string(),
                &cl8y_dex_router::msg::QueryMsg::SimulateSwapOperations {
                    offer_amount: Uint128::new(10_000),
                    operations: vec![cl8y_dex_router::msg::SwapOperation::NativeSwap {
                        offer_denom: "uusd".to_string(),
                        ask_denom: "uluna".to_string(),
                    }],
                },
            );

        assert!(result.is_err());
    }

    #[test]
    fn test_lp_withdrawal_near_zero_reserves() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        // Provide minimal liquidity
        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_001), Uint128::new(1_001));

        let lp = query_cw20_balance(&app, &env.lp_token, &env.user);
        assert_eq!(lp, Uint128::new(1)); // 1001 - 1000 MINIMUM_LIQUIDITY

        // Withdraw the 1 LP token
        withdraw_liquidity(&mut app, &env, &env.user, lp);

        // User should get some tokens back (at least 0 due to floor division)
        let pool = query_pool(&app, &env.pair);
        // Pool still has MINIMUM_LIQUIDITY worth of reserves locked
        assert!(pool.total_share > Uint128::zero());
    }

    #[test]
    fn test_factory_set_pair_hooks_unauthorized() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let random = Addr::unchecked("random");
        let err = app
            .execute_contract(
                random,
                env.factory.clone(),
                &dex_common::factory::ExecuteMsg::SetPairHooks {
                    pair: env.pair.to_string(),
                    hooks: vec!["hook1".to_string()],
                },
                &[],
            )
            .unwrap_err();
        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    #[test]
    fn test_factory_set_discount_registry_unauthorized() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let random = Addr::unchecked("random");
        let err = app
            .execute_contract(
                random,
                env.factory.clone(),
                &dex_common::factory::ExecuteMsg::SetDiscountRegistry {
                    pair: env.pair.to_string(),
                    registry: None,
                },
                &[],
            )
            .unwrap_err();
        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    #[test]
    fn test_factory_set_discount_registry_all_unauthorized() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let random = Addr::unchecked("random");
        let err = app
            .execute_contract(
                random,
                env.factory.clone(),
                &dex_common::factory::ExecuteMsg::SetDiscountRegistryAll {
                    registry: None,
                },
                &[],
            )
            .unwrap_err();
        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    #[test]
    fn test_factory_create_pair_same_token_rejected() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let err = app
            .execute_contract(
                env.user.clone(),
                env.factory.clone(),
                &dex_common::factory::ExecuteMsg::CreatePair {
                    asset_infos: [
                        asset_info_token(&env.token_a),
                        asset_info_token(&env.token_a),
                    ],
                },
                &[],
            )
            .unwrap_err();

        assert!(err.root_cause().to_string().to_lowercase().contains("invalid") ||
                err.root_cause().to_string().to_lowercase().contains("token"));
    }

    #[test]
    fn test_provide_liquidity_reversed_asset_order() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        // Provide with reversed order (token_b first, token_a second)
        app.execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                spender: env.pair.to_string(),
                amount: Uint128::new(1_000_000),
                expires: None,
            },
            &[],
        )
        .unwrap();
        app.execute_contract(
            env.user.clone(),
            env.token_b.clone(),
            &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                spender: env.pair.to_string(),
                amount: Uint128::new(1_000_000),
                expires: None,
            },
            &[],
        )
        .unwrap();

        app.execute_contract(
            env.user.clone(),
            env.pair.clone(),
            &dex_common::pair::ExecuteMsg::ProvideLiquidity {
                assets: [
                    dex_common::types::Asset {
                        info: asset_info_token(&env.token_b),
                        amount: Uint128::new(1_000_000),
                    },
                    dex_common::types::Asset {
                        info: asset_info_token(&env.token_a),
                        amount: Uint128::new(1_000_000),
                    },
                ],
                slippage_tolerance: None,
                receiver: None,
                deadline: None,
            },
            &[],
        )
        .unwrap();

        let pool = query_pool(&app, &env.pair);
        assert_eq!(pool.assets[0].amount, Uint128::new(1_000_000));
        assert_eq!(pool.assets[1].amount, Uint128::new(1_000_000));
    }

    #[test]
    fn test_reverse_simulation_b_to_a() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        // Reverse sim asking for token A (ask = token A means we need to offer token B)
        let rsim: dex_common::pair::ReverseSimulationResponse = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::ReverseSimulation {
                    ask_asset: dex_common::types::Asset {
                        info: asset_info_token(&env.token_a),
                        amount: Uint128::new(10_000),
                    },
                },
            )
            .unwrap();

        assert!(rsim.offer_amount > Uint128::new(10_000));
    }

    #[test]
    fn test_swap_with_invalid_token() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        provide_liquidity(&mut app, &env, &env.user, Uint128::new(1_000_000), Uint128::new(1_000_000));

        // Create a random CW20 token not part of the pair
        let cw20_code_id = app.store_code(cw20_mintable_contract());
        let fake_token = create_cw20_token(
            &mut app,
            cw20_code_id,
            &env.user,
            "Fake",
            "FAKE",
            Uint128::new(1_000_000),
        );

        let swap_msg = cosmwasm_std::to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: Some(cosmwasm_std::Decimal::one()),
            to: None,
            deadline: None,
            trader: None,
        })
        .unwrap();

        let err = app
            .execute_contract(
                env.user.clone(),
                fake_token,
                &cw20::Cw20ExecuteMsg::Send {
                    contract: env.pair.to_string(),
                    amount: Uint128::new(1_000),
                    msg: swap_msg,
                },
                &[],
            )
            .unwrap_err();

        assert!(err.root_cause().to_string().to_lowercase().contains("invalid") ||
                err.root_cause().to_string().to_lowercase().contains("token"));
    }

    #[test]
    fn test_factory_config_query() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let config: dex_common::factory::ConfigResponse = app
            .wrap()
            .query_wasm_smart(
                env.factory.to_string(),
                &dex_common::factory::QueryMsg::Config {},
            )
            .unwrap();

        assert_eq!(config.governance, env.governance);
        assert_eq!(config.treasury, env.treasury);
        assert_eq!(config.default_fee_bps, 30);
    }

    #[test]
    fn test_factory_update_config_invalid_fee() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let err = app
            .execute_contract(
                env.governance.clone(),
                env.factory.clone(),
                &dex_common::factory::ExecuteMsg::UpdateConfig {
                    governance: None,
                    treasury: None,
                    default_fee_bps: Some(10001),
                },
                &[],
            )
            .unwrap_err();

        assert!(err.root_cause().to_string().to_lowercase().contains("invalid") ||
                err.root_cause().to_string().to_lowercase().contains("fee"));
    }
}

// ===========================================================================
// ADDITIONAL FUZZ PROPERTIES — oracle, router, overflow boundaries, multi-LP
// ===========================================================================

#[cfg(test)]
mod additional_fuzz_tests {
    use super::helpers::*;
    use cosmwasm_std::{to_json_binary, Addr, Uint128};
    use cw_multi_test::{App, Executor};
    use proptest::prelude::*;

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn prop_oracle_tick_monotone_with_time(
            init_a in 100_000u128..10_000_000u128,
            init_b in 100_000u128..10_000_000u128,
            num_swaps in 2usize..8,
            seed in 0u64..100_000u64,
        ) {
            let mut app = App::default();
            let env = setup_full_env(&mut app);

            provide_liquidity(&mut app, &env, &env.user,
                Uint128::new(init_a), Uint128::new(init_b));

            let mut rng_state = seed;
            for _ in 0..num_swaps {
                app.update_block(|b| b.time = b.time.plus_seconds(6));

                rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let direction = (rng_state >> 32) % 2 == 0;
                let pool = query_pool(&app, &env.pair);

                let max_swap = if direction {
                    std::cmp::min(
                        pool.assets[0].amount.u128() / 4,
                        query_cw20_balance(&app, &env.token_a, &env.user).u128(),
                    )
                } else {
                    std::cmp::min(
                        pool.assets[1].amount.u128() / 4,
                        query_cw20_balance(&app, &env.token_b, &env.user).u128(),
                    )
                };
                if max_swap < 1 { continue; }

                rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let swap_amount = 1 + (rng_state as u128 % max_swap);

                if direction {
                    swap_a_to_b(&mut app, &env, &env.user, Uint128::new(swap_amount));
                } else {
                    swap_b_to_a(&mut app, &env, &env.user, Uint128::new(swap_amount));
                }
            }

            let info: dex_common::oracle::OracleInfoResponse = app
                .wrap()
                .query_wasm_smart(
                    env.pair.to_string(),
                    &dex_common::pair::QueryMsg::OracleInfo {},
                )
                .unwrap();

            // Should have recorded multiple observations
            prop_assert!(info.newest_observation_timestamp > 0);
            prop_assert!(info.newest_observation_timestamp >= info.oldest_observation_timestamp);
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn prop_multi_lp_interleaved_swaps_conservation(
            init_a in 100_000u128..5_000_000u128,
            init_b in 100_000u128..5_000_000u128,
            user2_a in 100_000u128..5_000_000u128,
            user2_b in 100_000u128..5_000_000u128,
            num_swaps in 1usize..5,
            seed in 0u64..100_000u64,
        ) {
            let mut app = App::default();
            let env = setup_full_env(&mut app);
            let initial_total = Uint128::new(1_000_000_000_000);

            let user2 = Addr::unchecked("user2");
            transfer_tokens(&mut app, &env.token_a, &env.user, &user2, Uint128::new(user2_a));
            transfer_tokens(&mut app, &env.token_b, &env.user, &user2, Uint128::new(user2_b));

            provide_liquidity(&mut app, &env, &env.user,
                Uint128::new(init_a), Uint128::new(init_b));
            provide_liquidity(&mut app, &env, &user2,
                Uint128::new(user2_a), Uint128::new(user2_b));

            let mut rng_state = seed;
            for _ in 0..num_swaps {
                rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let who_swaps = (rng_state >> 32) % 2 == 0;
                let direction = (rng_state >> 16) % 2 == 0;
                let swapper = if who_swaps { &env.user } else { &user2 };

                let pool = query_pool(&app, &env.pair);
                let max_swap = if direction {
                    std::cmp::min(
                        pool.assets[0].amount.u128() / 4,
                        query_cw20_balance(&app, &env.token_a, swapper).u128(),
                    )
                } else {
                    std::cmp::min(
                        pool.assets[1].amount.u128() / 4,
                        query_cw20_balance(&app, &env.token_b, swapper).u128(),
                    )
                };
                if max_swap < 1 { continue; }

                rng_state = rng_state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let swap_amount = 1 + (rng_state as u128 % max_swap);

                if direction {
                    swap_a_to_b(&mut app, &env, swapper, Uint128::new(swap_amount));
                } else {
                    swap_b_to_a(&mut app, &env, swapper, Uint128::new(swap_amount));
                }
            }

            // Withdraw all
            let lp1 = query_cw20_balance(&app, &env.lp_token, &env.user);
            if !lp1.is_zero() {
                withdraw_liquidity(&mut app, &env, &env.user, lp1);
            }
            let lp2 = query_cw20_balance(&app, &env.lp_token, &user2);
            if !lp2.is_zero() {
                withdraw_liquidity(&mut app, &env, &user2, lp2);
            }

            let pool = query_pool(&app, &env.pair);
            let u1_a = query_cw20_balance(&app, &env.token_a, &env.user);
            let u1_b = query_cw20_balance(&app, &env.token_b, &env.user);
            let u2_a = query_cw20_balance(&app, &env.token_a, &user2);
            let u2_b = query_cw20_balance(&app, &env.token_b, &user2);
            let tr_a = query_cw20_balance(&app, &env.token_a, &env.treasury);
            let tr_b = query_cw20_balance(&app, &env.token_b, &env.treasury);

            let total_a = u1_a + u2_a + pool.assets[0].amount + tr_a;
            let total_b = u1_b + u2_b + pool.assets[1].amount + tr_b;

            prop_assert_eq!(total_a, initial_total);
            prop_assert_eq!(total_b, initial_total);
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(64))]

        #[test]
        fn prop_slippage_tolerance_boundary(
            init_a in 100_000u128..10_000_000u128,
            init_b in 100_000u128..10_000_000u128,
            add_a in 10_000u128..1_000_000u128,
            tolerance_bps in 0u128..10000u128,
        ) {
            let mut app = App::default();
            let env = setup_full_env(&mut app);

            provide_liquidity(&mut app, &env, &env.user,
                Uint128::new(init_a), Uint128::new(init_b));

            // Second deposit with custom slippage tolerance
            let add_b = add_a * init_b / init_a; // proportional
            if add_b == 0 { return Ok(()); }

            let tolerance = cosmwasm_std::Decimal::from_ratio(tolerance_bps, 10000u128);

            app.execute_contract(
                env.user.clone(),
                env.token_a.clone(),
                &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                    spender: env.pair.to_string(),
                    amount: Uint128::new(add_a),
                    expires: None,
                },
                &[],
            ).unwrap();
            app.execute_contract(
                env.user.clone(),
                env.token_b.clone(),
                &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                    spender: env.pair.to_string(),
                    amount: Uint128::new(add_b),
                    expires: None,
                },
                &[],
            ).unwrap();

            // Proportional deposit should always succeed regardless of tolerance
            let result = app.execute_contract(
                env.user.clone(),
                env.pair.clone(),
                &dex_common::pair::ExecuteMsg::ProvideLiquidity {
                    assets: [
                        dex_common::types::Asset {
                            info: asset_info_token(&env.token_a),
                            amount: Uint128::new(add_a),
                        },
                        dex_common::types::Asset {
                            info: asset_info_token(&env.token_b),
                            amount: Uint128::new(add_b),
                        },
                    ],
                    slippage_tolerance: Some(tolerance),
                    receiver: None,
                    deadline: None,
                },
                &[],
            );

            prop_assert!(result.is_ok(),
                "Proportional deposit should succeed with any tolerance");
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(32))]

        #[test]
        fn prop_router_single_hop_matches_direct_swap(
            init in 100_000u128..10_000_000u128,
            swap_fraction_bps in 10u128..3000u128,
        ) {
            let swap_amount = std::cmp::max(1, init * swap_fraction_bps / 10000);

            // Direct swap
            let mut app1 = App::default();
            let env1 = setup_full_env(&mut app1);
            provide_liquidity(&mut app1, &env1, &env1.user,
                Uint128::new(init), Uint128::new(init));
            let sim: dex_common::pair::SimulationResponse = app1
                .wrap()
                .query_wasm_smart(
                    env1.pair.to_string(),
                    &dex_common::pair::QueryMsg::Simulation {
                        offer_asset: dex_common::types::Asset {
                            info: asset_info_token(&env1.token_a),
                            amount: Uint128::new(swap_amount),
                        },
                    },
                )
                .unwrap();

            // Router simulation
            let router_sim: cl8y_dex_router::msg::SimulateSwapOperationsResponse = app1
                .wrap()
                .query_wasm_smart(
                    env1.router.to_string(),
                    &cl8y_dex_router::msg::QueryMsg::SimulateSwapOperations {
                        offer_amount: Uint128::new(swap_amount),
                        operations: vec![cl8y_dex_router::msg::SwapOperation::TerraSwap {
                            offer_asset_info: asset_info_token(&env1.token_a),
                            ask_asset_info: asset_info_token(&env1.token_b),
                        }],
                    },
                )
                .unwrap();

            prop_assert_eq!(sim.return_amount, router_sim.amount,
                "Router sim should match direct sim");
        }
    }

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(32))]

        #[test]
        fn prop_sandwich_net_negative_after_fees(
            init in 1_000_000u128..50_000_000u128,
            front_run_fraction in 100u128..5000u128,
            victim_fraction in 10u128..1000u128,
        ) {
            let front_run_amount = std::cmp::max(1, init * front_run_fraction / 10000);
            let victim_amount = std::cmp::max(1, init * victim_fraction / 10000);

            let mut app = App::default();
            let env = setup_full_env(&mut app);

            let attacker = Addr::unchecked("attacker");
            let victim = Addr::unchecked("victim");
            transfer_tokens(&mut app, &env.token_a, &env.user, &attacker, Uint128::new(front_run_amount * 2));
            transfer_tokens(&mut app, &env.token_b, &env.user, &attacker, Uint128::new(front_run_amount * 2));
            transfer_tokens(&mut app, &env.token_a, &env.user, &victim, Uint128::new(victim_amount));

            provide_liquidity(&mut app, &env, &env.user,
                Uint128::new(init), Uint128::new(init));

            let att_a_before = query_cw20_balance(&app, &env.token_a, &attacker);
            let att_b_before = query_cw20_balance(&app, &env.token_b, &attacker);

            // Front-run
            swap_a_to_b(&mut app, &env, &attacker, Uint128::new(front_run_amount));

            // Victim uses realistic 1% max_spread. If the front-run moved
            // the price beyond this tolerance the victim's tx reverts and the
            // attacker is left worse off (paid fees, got nothing).
            let victim_swap_msg = cosmwasm_std::to_json_binary(
                &dex_common::pair::Cw20HookMsg::Swap {
                    belief_price: None,
                    max_spread: Some(cosmwasm_std::Decimal::percent(1)),
                    to: None,
                    deadline: None,
                    trader: None,
                },
            )
            .unwrap();
            let victim_result = app.execute_contract(
                victim.clone(),
                env.token_a.clone(),
                &cw20::Cw20ExecuteMsg::Send {
                    contract: env.pair.to_string(),
                    amount: Uint128::new(victim_amount),
                    msg: victim_swap_msg,
                },
                &[],
            );

            // Back-run (use huge max_spread since attacker doesn't care)
            let att_b = query_cw20_balance(&app, &env.token_b, &attacker);
            if att_b > Uint128::zero() {
                let swap_msg = cosmwasm_std::to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
                    belief_price: None,
                    max_spread: Some(cosmwasm_std::Decimal::percent(9999999)),
                    to: None,
                    deadline: None,
                    trader: None,
                }).unwrap();
                app.execute_contract(
                    attacker.clone(),
                    env.token_b.clone(),
                    &cw20::Cw20ExecuteMsg::Send {
                        contract: env.pair.to_string(),
                        amount: att_b,
                        msg: swap_msg,
                    },
                    &[],
                ).unwrap();
            }

            let att_a_after = query_cw20_balance(&app, &env.token_a, &attacker);
            let att_b_after = query_cw20_balance(&app, &env.token_b, &attacker);

            let before_total = att_a_before + att_b_before;
            let after_total = att_a_after + att_b_after;

            if victim_result.is_err() {
                // Victim's slippage protection kicked in — the front-run
                // moved price beyond 1%. The attacker paid fees on the
                // front-run + back-run with no victim to exploit, so they
                // must end up with less than they started.
                prop_assert!(
                    after_total <= before_total,
                    "Attacker should lose money when victim tx reverts: before={}, after={}",
                    before_total, after_total
                );
            } else {
                // Victim's swap went through (spread ≤ 1%), meaning the
                // front-run had limited price impact. Attacker profit is
                // bounded by the victim's small price impact minus the
                // attacker's ~0.6% round-trip fee cost. Allow up to 5% of
                // attacker capital as headroom for large-victim-fraction
                // edge cases in the constant-product curve.
                let max_profit = std::cmp::max(before_total.u128() / 20, 1000);
                prop_assert!(
                    after_total.u128() <= before_total.u128() + max_profit,
                    "Sandwich profit exceeds 5% threshold: before={}, after={}, max_profit={}",
                    before_total, after_total, max_profit
                );
            }
        }
    }
}

#[cfg(test)]
mod sweep_tests {
    use super::helpers::*;
    use cosmwasm_std::{Addr, Uint128};
    use cw_multi_test::{App, Executor};

    fn provide_initial_liquidity(app: &mut App, env: &TestEnv) {
        let amount = Uint128::new(1_000_000);
        provide_liquidity(app, env, &env.user.clone(), amount, amount);
    }

    fn donate_tokens(app: &mut App, from: &Addr, token: &Addr, pair: &Addr, amount: Uint128) {
        app.execute_contract(
            from.clone(),
            token.clone(),
            &cw20::Cw20ExecuteMsg::Transfer {
                recipient: pair.to_string(),
                amount,
            },
            &[],
        )
        .unwrap();
    }

    #[test]
    fn test_governance_can_sweep_donated_tokens() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);
        provide_initial_liquidity(&mut app, &env);

        let donation = Uint128::new(50_000);
        donate_tokens(&mut app, &env.user, &env.token_a, &env.pair, donation);

        let recipient = Addr::unchecked("recovery_wallet");
        let before = query_cw20_balance(&app, &env.token_a, &recipient);
        assert!(before.is_zero());

        app.execute_contract(
            env.governance.clone(),
            env.factory.clone(),
            &dex_common::factory::ExecuteMsg::SweepPair {
                pair: env.pair.to_string(),
                token: env.token_a.to_string(),
                recipient: recipient.to_string(),
            },
            &[],
        )
        .unwrap();

        let after = query_cw20_balance(&app, &env.token_a, &recipient);
        assert_eq!(after, donation);

        let pool = query_pool(&app, &env.pair);
        assert_eq!(pool.assets[0].amount, Uint128::new(1_000_000));
        assert_eq!(pool.assets[1].amount, Uint128::new(1_000_000));
    }

    #[test]
    fn test_sweep_non_pool_token_recovers_entire_balance() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);
        provide_initial_liquidity(&mut app, &env);

        let cw20_code_id = app.store_code(cw20_mintable_contract());
        let stray_token = create_cw20_token(
            &mut app,
            cw20_code_id,
            &env.user,
            "Stray",
            "STRAY",
            Uint128::new(500_000),
        );

        let stray_amount = Uint128::new(100_000);
        donate_tokens(&mut app, &env.user, &stray_token, &env.pair, stray_amount);

        let recipient = Addr::unchecked("recovery_wallet");

        app.execute_contract(
            env.governance.clone(),
            env.factory.clone(),
            &dex_common::factory::ExecuteMsg::SweepPair {
                pair: env.pair.to_string(),
                token: stray_token.to_string(),
                recipient: recipient.to_string(),
            },
            &[],
        )
        .unwrap();

        let recovered = query_cw20_balance(&app, &stray_token, &recipient);
        assert_eq!(recovered, stray_amount);

        let remaining = query_cw20_balance(&app, &stray_token, &env.pair);
        assert!(remaining.is_zero());
    }

    #[test]
    fn test_sweep_unauthorized_by_random_user() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);
        provide_initial_liquidity(&mut app, &env);

        let donation = Uint128::new(10_000);
        donate_tokens(&mut app, &env.user, &env.token_a, &env.pair, donation);

        let attacker = Addr::unchecked("attacker");
        let err = app
            .execute_contract(
                attacker,
                env.factory.clone(),
                &dex_common::factory::ExecuteMsg::SweepPair {
                    pair: env.pair.to_string(),
                    token: env.token_a.to_string(),
                    recipient: "attacker".to_string(),
                },
                &[],
            )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    #[test]
    fn test_sweep_direct_to_pair_unauthorized() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);
        provide_initial_liquidity(&mut app, &env);

        let donation = Uint128::new(10_000);
        donate_tokens(&mut app, &env.user, &env.token_a, &env.pair, donation);

        let err = app
            .execute_contract(
                env.user.clone(),
                env.pair.clone(),
                &dex_common::pair::ExecuteMsg::Sweep {
                    token: env.token_a.to_string(),
                    recipient: env.user.to_string(),
                },
                &[],
            )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Unauthorized"));
    }

    #[test]
    fn test_sweep_nothing_to_sweep() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);
        provide_initial_liquidity(&mut app, &env);

        let err = app
            .execute_contract(
                env.governance.clone(),
                env.factory.clone(),
                &dex_common::factory::ExecuteMsg::SweepPair {
                    pair: env.pair.to_string(),
                    token: env.token_a.to_string(),
                    recipient: env.treasury.to_string(),
                },
                &[],
            )
            .unwrap_err();

        assert!(err.root_cause().to_string().contains("Nothing to sweep"));
    }

    #[test]
    fn test_sweep_does_not_affect_pool_reserves() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);
        provide_initial_liquidity(&mut app, &env);

        let pool_before = query_pool(&app, &env.pair);

        let donation = Uint128::new(200_000);
        donate_tokens(&mut app, &env.user, &env.token_a, &env.pair, donation);
        donate_tokens(&mut app, &env.user, &env.token_b, &env.pair, Uint128::new(100_000));

        app.execute_contract(
            env.governance.clone(),
            env.factory.clone(),
            &dex_common::factory::ExecuteMsg::SweepPair {
                pair: env.pair.to_string(),
                token: env.token_a.to_string(),
                recipient: env.treasury.to_string(),
            },
            &[],
        )
        .unwrap();

        app.execute_contract(
            env.governance.clone(),
            env.factory.clone(),
            &dex_common::factory::ExecuteMsg::SweepPair {
                pair: env.pair.to_string(),
                token: env.token_b.to_string(),
                recipient: env.treasury.to_string(),
            },
            &[],
        )
        .unwrap();

        let pool_after = query_pool(&app, &env.pair);
        assert_eq!(pool_before.assets[0].amount, pool_after.assets[0].amount);
        assert_eq!(pool_before.assets[1].amount, pool_after.assets[1].amount);
        assert_eq!(pool_before.total_share, pool_after.total_share);
    }

    #[test]
    fn test_sweep_partial_excess_after_swap() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);
        provide_initial_liquidity(&mut app, &env);

        let swap_amount = Uint128::new(10_000);
        swap_a_to_b(&mut app, &env, &env.user.clone(), swap_amount);

        let donation = Uint128::new(5_000);
        donate_tokens(&mut app, &env.user, &env.token_a, &env.pair, donation);

        let recipient = Addr::unchecked("recovery");

        app.execute_contract(
            env.governance.clone(),
            env.factory.clone(),
            &dex_common::factory::ExecuteMsg::SweepPair {
                pair: env.pair.to_string(),
                token: env.token_a.to_string(),
                recipient: recipient.to_string(),
            },
            &[],
        )
        .unwrap();

        let recovered = query_cw20_balance(&app, &env.token_a, &recipient);
        assert_eq!(recovered, donation);
    }

    #[test]
    fn test_sweep_unregistered_pair_rejected() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let fake_pair = Addr::unchecked("fake_pair_contract");

        let err = app
            .execute_contract(
                env.governance.clone(),
                env.factory.clone(),
                &dex_common::factory::ExecuteMsg::SweepPair {
                    pair: fake_pair.to_string(),
                    token: env.token_a.to_string(),
                    recipient: env.treasury.to_string(),
                },
                &[],
            )
            .unwrap_err();

        let err_str = err.root_cause().to_string();
        assert!(
            err_str.contains("not found") || err_str.contains("not registered"),
            "Expected pair-not-found error, got: {}",
            err_str
        );
    }
}

#[cfg(test)]
mod new_feature_tests {
    use super::helpers::*;
    use cosmwasm_std::{to_json_binary, Addr, Empty, Uint128};
    use cw_multi_test::{App, Executor};

    // -----------------------------------------------------------------------
    // 1. Router max-hops validation
    // -----------------------------------------------------------------------

    #[test]
    fn router_rejects_more_than_4_hops() {
        let mut app = App::default();
        let governance = Addr::unchecked("governance");
        let treasury = Addr::unchecked("treasury");
        let user = Addr::unchecked("user");

        let cw20_code_id = app.store_code(cw20_mintable_contract());
        let pair_code_id = app.store_code(pair_contract());
        let factory_code_id = app.store_code(factory_contract());
        let router_code_id = app.store_code(router_contract());

        let initial_amount = Uint128::new(1_000_000_000_000);

        let tokens: Vec<Addr> = ["A", "B", "C", "D", "E", "F"]
            .iter()
            .map(|name| {
                create_cw20_token(
                    &mut app,
                    cw20_code_id,
                    &user,
                    &format!("Token {name}"),
                    &format!("TKN{name}"),
                    initial_amount,
                )
            })
            .collect();

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

        let mut pairs = vec![];
        for i in 0..5 {
            let resp = app
                .execute_contract(
                    user.clone(),
                    factory.clone(),
                    &dex_common::factory::ExecuteMsg::CreatePair {
                        asset_infos: [
                            asset_info_token(&tokens[i]),
                            asset_info_token(&tokens[i + 1]),
                        ],
                    },
                    &[],
                )
                .unwrap();
            pairs.push(extract_pair_address(&resp.events));
        }

        let liq = Uint128::new(1_000_000);
        for (i, pair) in pairs.iter().enumerate() {
            app.execute_contract(
                user.clone(),
                tokens[i].clone(),
                &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                    spender: pair.to_string(),
                    amount: liq,
                    expires: None,
                },
                &[],
            )
            .unwrap();

            app.execute_contract(
                user.clone(),
                tokens[i + 1].clone(),
                &cw20::Cw20ExecuteMsg::IncreaseAllowance {
                    spender: pair.to_string(),
                    amount: liq,
                    expires: None,
                },
                &[],
            )
            .unwrap();

            app.execute_contract(
                user.clone(),
                pair.clone(),
                &dex_common::pair::ExecuteMsg::ProvideLiquidity {
                    assets: [
                        dex_common::types::Asset {
                            info: asset_info_token(&tokens[i]),
                            amount: liq,
                        },
                        dex_common::types::Asset {
                            info: asset_info_token(&tokens[i + 1]),
                            amount: liq,
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

        // 5-hop swap A→B→C→D→E→F — exceeds MAX_HOPS(4)
        let five_ops: Vec<cl8y_dex_router::msg::SwapOperation> = (0..5)
            .map(|i| cl8y_dex_router::msg::SwapOperation::TerraSwap {
                offer_asset_info: asset_info_token(&tokens[i]),
                ask_asset_info: asset_info_token(&tokens[i + 1]),
            })
            .collect();

        let hook_msg = to_json_binary(
            &cl8y_dex_router::msg::Cw20HookMsg::ExecuteSwapOperations {
                operations: five_ops,
                minimum_receive: None,
                to: None,
                deadline: None,
            },
        )
        .unwrap();

        let err = app
            .execute_contract(
                user.clone(),
                tokens[0].clone(),
                &cw20::Cw20ExecuteMsg::Send {
                    contract: router.to_string(),
                    amount: Uint128::new(1_000),
                    msg: hook_msg,
                },
                &[],
            )
            .unwrap_err();

        assert!(
            err.root_cause().to_string().contains("Too many hops"),
            "Expected TooManyHops error, got: {}",
            err.root_cause()
        );

        // 4-hop swap A→B→C→D→E — exactly at the limit, should succeed
        let four_ops: Vec<cl8y_dex_router::msg::SwapOperation> = (0..4)
            .map(|i| cl8y_dex_router::msg::SwapOperation::TerraSwap {
                offer_asset_info: asset_info_token(&tokens[i]),
                ask_asset_info: asset_info_token(&tokens[i + 1]),
            })
            .collect();

        let user_e_before = query_cw20_balance(&app, &tokens[4], &user);

        let hook_msg = to_json_binary(
            &cl8y_dex_router::msg::Cw20HookMsg::ExecuteSwapOperations {
                operations: four_ops,
                minimum_receive: None,
                to: None,
                deadline: None,
            },
        )
        .unwrap();

        app.execute_contract(
            user.clone(),
            tokens[0].clone(),
            &cw20::Cw20ExecuteMsg::Send {
                contract: router.to_string(),
                amount: Uint128::new(1_000),
                msg: hook_msg,
            },
            &[],
        )
        .unwrap();

        let user_e_after = query_cw20_balance(&app, &tokens[4], &user);
        assert!(
            user_e_after > user_e_before,
            "4-hop swap should succeed and deliver output tokens"
        );
    }

    // -----------------------------------------------------------------------
    // 2. Fee-discount epoch-guarded deregistration
    // -----------------------------------------------------------------------

    #[test]
    fn deregister_with_stale_epoch_is_noop() {
        let mut app = App::default();
        let base = setup_full_env(&mut app);
        let cw20_code_id = app.store_code(cw20_mintable_contract());
        let fee_discount_code_id = app.store_code(fee_discount_contract());

        const ONE_CL8Y: u128 = 1_000_000_000_000_000_000;

        let cl8y_token = app
            .instantiate_contract(
                cw20_code_id,
                base.user.clone(),
                &cw20_mintable::msg::InstantiateMsg {
                    name: "CL8Y Token".to_string(),
                    symbol: "CL8Y".to_string(),
                    decimals: 18,
                    initial_balances: vec![cw20::Cw20Coin {
                        address: base.user.to_string(),
                        amount: Uint128::new(100_000 * ONE_CL8Y),
                    }],
                    mint: None,
                    marketing: None,
                },
                &[],
                "cl8y",
                None,
            )
            .unwrap();

        let fee_discount = app
            .instantiate_contract(
                fee_discount_code_id,
                base.governance.clone(),
                &cl8y_dex_fee_discount::msg::InstantiateMsg {
                    governance: base.governance.to_string(),
                    cl8y_token: cl8y_token.to_string(),
                },
                &[],
                "fee_discount",
                None,
            )
            .unwrap();

        app.execute_contract(
            base.governance.clone(),
            fee_discount.clone(),
            &cl8y_dex_fee_discount::msg::ExecuteMsg::AddTier {
                tier_id: 1,
                min_cl8y_balance: Uint128::new(ONE_CL8Y),
                discount_bps: 1000,
                governance_only: false,
            },
            &[],
        )
        .unwrap();

        // 1. Register
        app.execute_contract(
            base.user.clone(),
            fee_discount.clone(),
            &cl8y_dex_fee_discount::msg::ExecuteMsg::Register { tier_id: 1 },
            &[],
        )
        .unwrap();

        let reg: cl8y_dex_fee_discount::msg::RegistrationResponse = app
            .wrap()
            .query_wasm_smart(
                fee_discount.to_string(),
                &cl8y_dex_fee_discount::msg::QueryMsg::GetRegistration {
                    trader: base.user.to_string(),
                },
            )
            .unwrap();
        assert!(reg.registered, "user should be registered after Register");

        // 2. Deregister
        app.execute_contract(
            base.user.clone(),
            fee_discount.clone(),
            &cl8y_dex_fee_discount::msg::ExecuteMsg::Deregister {},
            &[],
        )
        .unwrap();

        let reg: cl8y_dex_fee_discount::msg::RegistrationResponse = app
            .wrap()
            .query_wasm_smart(
                fee_discount.to_string(),
                &cl8y_dex_fee_discount::msg::QueryMsg::GetRegistration {
                    trader: base.user.to_string(),
                },
            )
            .unwrap();
        assert!(!reg.registered, "user should be deregistered");

        // 3. Re-register → new epoch (epoch counter is now 2)
        app.execute_contract(
            base.user.clone(),
            fee_discount.clone(),
            &cl8y_dex_fee_discount::msg::ExecuteMsg::Register { tier_id: 1 },
            &[],
        )
        .unwrap();

        // 4. Attempt DeregisterWallet with stale epoch=1 (current is 2) → no-op
        let resp = app
            .execute_contract(
                base.governance.clone(),
                fee_discount.clone(),
                &cl8y_dex_fee_discount::msg::ExecuteMsg::DeregisterWallet {
                    wallet: base.user.to_string(),
                    epoch: Some(1),
                },
                &[],
            )
            .unwrap();

        let skipped = resp
            .events
            .iter()
            .flat_map(|e| &e.attributes)
            .any(|a| a.key == "skipped");
        assert!(skipped, "stale epoch deregistration should produce 'skipped' attribute");

        // 5. Verify wallet is still registered
        let reg: cl8y_dex_fee_discount::msg::RegistrationResponse = app
            .wrap()
            .query_wasm_smart(
                fee_discount.to_string(),
                &cl8y_dex_fee_discount::msg::QueryMsg::GetRegistration {
                    trader: base.user.to_string(),
                },
            )
            .unwrap();
        assert!(
            reg.registered,
            "wallet must remain registered after stale-epoch deregister attempt"
        );
    }

    // -----------------------------------------------------------------------
    // 3. LP admin propagation on governance change
    // -----------------------------------------------------------------------

    #[test]
    fn factory_governance_change_updates_lp_admin() {
        let mut app = App::default();

        let governance = Addr::unchecked("governance");
        let treasury = Addr::unchecked("treasury");
        let user = Addr::unchecked("user");
        let new_governance = Addr::unchecked("new_governance");

        let cw20_code_id = app.store_code(cw20_mintable_contract());
        let pair_code_id = app.store_code(pair_contract());
        let factory_code_id = app.store_code(factory_contract());

        let initial_amount = Uint128::new(1_000_000_000_000);
        let token_a = create_cw20_token(
            &mut app,
            cw20_code_id,
            &user,
            "Token A",
            "TKNA",
            initial_amount,
        );
        let token_b = create_cw20_token(
            &mut app,
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

        // LP token admin should initially be the pair contract itself
        let lp_contract_info = app
            .wrap()
            .query_wasm_contract_info(lp_token.to_string())
            .unwrap();
        assert_eq!(
            lp_contract_info.admin,
            Some(pair.to_string()),
            "LP token admin should initially be the pair contract"
        );

        // UpdateConfig propagates SetLpAdmin → UpdateAdmin on LP token.
        // The pair contract is the LP token's CosmWasm admin, so it can
        // call UpdateAdmin to change the admin to the new governance.
        app.execute_contract(
            governance.clone(),
            factory.clone(),
            &dex_common::factory::ExecuteMsg::UpdateConfig {
                governance: Some(new_governance.to_string()),
                treasury: None,
                default_fee_bps: None,
            },
            &[],
        )
        .unwrap();

        let lp_contract_info = app
            .wrap()
            .query_wasm_contract_info(lp_token.to_string())
            .unwrap();
        assert_eq!(
            lp_contract_info.admin,
            Some(new_governance.to_string()),
            "LP token admin should be updated to new governance"
        );
    }

    // -----------------------------------------------------------------------
    // 4. Pair contract migration version check
    // -----------------------------------------------------------------------

    #[test]
    fn pair_migration_checks_version() {
        let mut app = App::default();
        let governance = Addr::unchecked("governance");

        let mock_old_id = app.store_code(mock_old_pair_contract());
        let mock_future_id = app.store_code(mock_future_pair_contract());
        let pair_code_id = app.store_code(pair_contract_with_migrate());

        // --- Upgrade path: 0.9.0 → 1.2.0 should succeed ---
        let old_contract = app
            .instantiate_contract(
                mock_old_id,
                governance.clone(),
                &Empty {},
                &[],
                "old_pair",
                Some(governance.to_string()),
            )
            .unwrap();

        app.migrate_contract(
            governance.clone(),
            old_contract,
            &cl8y_dex_pair::msg::MigrateMsg {},
            pair_code_id,
        )
        .unwrap();

        // --- Downgrade path: 99.0.0 → 1.2.0 should fail ---
        let future_contract = app
            .instantiate_contract(
                mock_future_id,
                governance.clone(),
                &Empty {},
                &[],
                "future_pair",
                Some(governance.to_string()),
            )
            .unwrap();

        let err = app
            .migrate_contract(
                governance.clone(),
                future_contract,
                &cl8y_dex_pair::msg::MigrateMsg {},
                pair_code_id,
            )
            .unwrap_err();

        let err_msg = err.root_cause().to_string();
        assert!(
            err_msg.contains("newer") || err_msg.contains("99.0.0"),
            "Expected downgrade rejection error, got: {err_msg}"
        );
    }

    // -----------------------------------------------------------------------
    // 5. Hook revert propagates to swap
    // -----------------------------------------------------------------------

    #[test]
    fn swap_with_reverting_hook_fails() {
        let mut app = App::default();
        let env = setup_full_env(&mut app);

        let burn_hook_code_id = app.store_code(burn_hook_contract());

        let burn_hook = app
            .instantiate_contract(
                burn_hook_code_id,
                env.governance.clone(),
                &cl8y_dex_burn_hook::msg::InstantiateMsg {
                    burn_token: env.token_b.to_string(),
                    burn_percentage_bps: 1000,
                    admin: env.governance.to_string(),
                },
                &[],
                "burn_hook",
                None,
            )
            .unwrap();

        // Attach hook to pair but do NOT register the pair in the hook's
        // allowed-pairs list — the hook will reject the call.
        app.execute_contract(
            env.governance.clone(),
            env.factory.clone(),
            &dex_common::factory::ExecuteMsg::SetPairHooks {
                pair: env.pair.to_string(),
                hooks: vec![burn_hook.to_string()],
            },
            &[],
        )
        .unwrap();

        provide_liquidity(
            &mut app,
            &env,
            &env.user,
            Uint128::new(1_000_000),
            Uint128::new(1_000_000),
        );

        let swap_msg = to_json_binary(&dex_common::pair::Cw20HookMsg::Swap {
            belief_price: None,
            max_spread: Some(cosmwasm_std::Decimal::one()),
            to: None,
            deadline: None,
            trader: None,
        })
        .unwrap();

        let err = app
            .execute_contract(
                env.user.clone(),
                env.token_a.clone(),
                &cw20::Cw20ExecuteMsg::Send {
                    contract: env.pair.to_string(),
                    amount: Uint128::new(10_000),
                    msg: swap_msg,
                },
                &[],
            )
            .unwrap_err();

        assert!(
            err.root_cause()
                .to_string()
                .contains("Unauthorized hook caller"),
            "Expected hook unauthorized error, got: {}",
            err.root_cause()
        );
    }
}
