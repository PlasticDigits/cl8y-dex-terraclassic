//! SEC-C14 / GitLab #405: contract migration state-preservation integration tests.
//!
//! Prior deployed wasm binaries are not checked into this repo. Tests simulate an upgrade by
//! populating live contract state, downgrading the cw2 version in `cw-multi-test` storage, then
//! migrating to the current code id. Rollback operator guidance: [`docs/runbooks/wasm-admin-migration.md`](../../docs/runbooks/wasm-admin-migration.md#rollback-and-limitations-sec-h05).

use cosmwasm_std::{to_json_binary, Addr, Decimal, Empty, Uint128};
use cw2::{get_contract_version, set_contract_version};
use cw_multi_test::{App, Executor};

use dex_common::blacklist::BlacklistCheck;
use dex_common::factory::ExecuteMsg as FactoryExecuteMsg;
use dex_common::limit_placement::LimitOrderPlacementItem;
use dex_common::pair::{
    pool_only_hybrid_params, Cw20HookMsg, FeeConfigResponse, HybridSimulationResponse,
    LimitOrderConfigResponse, LimitOrderResponse, LimitOrderSide, OrderStatus, OrderStatusResponse,
    PoolResponse, QueryMsg,
};
use dex_common::types::Asset;

use crate::helpers::*;
use crate::tier_fixtures::STANDARD_PRODUCTION_TIERS;

const FACTORY_NAME: &str = "cl8y-dex-factory";
const FACTORY_VERSION: &str = "1.6.0";
const FACTORY_PRIOR_VERSION: &str = "1.5.0";

const PAIR_NAME: &str = "cl8y-dex-pair";
const PAIR_VERSION: &str = "1.12.0";
const PAIR_PRIOR_VERSION: &str = "1.7.0";

const FEE_DISCOUNT_NAME: &str = "crates.io:cl8y-dex-fee-discount";
const FEE_DISCOUNT_VERSION: &str = "1.1.0";
const FEE_DISCOUNT_PRIOR_VERSION: &str = "0.9.0";

const ONE_CL8Y: u128 = 1_000_000_000_000_000_000;

struct MigrationEnv {
    factory: Addr,
    pair: Addr,
    lp_token: Addr,
    fee_discount: Addr,
    token_a: Addr,
    router: Addr,
    governance: Addr,
    treasury: Addr,
    user: Addr,
    blacklisted_wallet: Addr,
    factory_code_id: u64,
    pair_code_id: u64,
    fee_discount_code_id: u64,
    limit_order_id: u64,
}

fn downgrade_cw2_version(app: &mut App, contract: &Addr, name: &str, version: &str) {
    let mut storage = app.contract_storage_mut(contract);
    set_contract_version(&mut *storage, name, version).unwrap();
}

fn batch_place_bid_msg(price: Decimal, amount: Uint128) -> cosmwasm_std::Binary {
    to_json_binary(&Cw20HookMsg::PlaceLimitOrderBatch {
        side: LimitOrderSide::Bid,
        orders: vec![LimitOrderPlacementItem {
            price,
            amount,
            max_adjust_steps: 32,
            expires_at: None,
            hint_after_order_id: None,
        }],
    })
    .unwrap()
}

fn setup_migration_env(app: &mut App) -> MigrationEnv {
    let governance = Addr::unchecked("governance");
    let treasury = Addr::unchecked("treasury");
    let user = Addr::unchecked("user");
    let blacklisted_wallet = Addr::unchecked("blocked_wallet");

    let cw20_code_id = app.store_code(cw20_mintable_contract());
    let pair_code_id = app.store_code(pair_contract_with_migrate());
    let factory_code_id = app.store_code(factory_contract_with_migrate());
    let router_code_id = app.store_code(router_contract());
    let fee_discount_code_id = app.store_code(fee_discount_contract_with_migrate());

    let initial_amount = Uint128::new(1_000_000_000_000);
    let token_a = create_cw20_token(app, cw20_code_id, &user, "Token A", "TKNA", initial_amount);
    let token_b = create_cw20_token(app, cw20_code_id, &user, "Token B", "TKNB", initial_amount);

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
                default_limit_batch_max_rungs:
                    dex_common::pair::SUGGESTED_FACTORY_DEFAULT_LIMIT_BATCH_MAX_RUNGS,
                pair_creation_fee_uluna: Uint128::zero(),
            },
            &[],
            "factory",
            Some(governance.to_string()),
        )
        .unwrap();

    let resp = app
        .execute_contract(
            user.clone(),
            factory.clone(),
            &FactoryExecuteMsg::CreatePair {
                asset_infos: [asset_info_token(&token_a), asset_info_token(&token_b)],
            },
            &[],
        )
        .unwrap();
    let pair = extract_pair_address(&resp.events);

    let pair_info: dex_common::types::PairInfo = app
        .wrap()
        .query_wasm_smart(pair.to_string(), &QueryMsg::Pair {})
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

    let cl8y_token = app
        .instantiate_contract(
            cw20_code_id,
            user.clone(),
            &cw20_mintable::msg::InstantiateMsg {
                name: "CL8Y Token".to_string(),
                symbol: "CL8Y".to_string(),
                decimals: 18,
                initial_balances: vec![cw20::Cw20Coin {
                    address: user.to_string(),
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
            governance.clone(),
            &cl8y_dex_fee_discount::msg::InstantiateMsg {
                governance: governance.to_string(),
                cl8y_token: cl8y_token.to_string(),
            },
            &[],
            "fee_discount",
            Some(governance.to_string()),
        )
        .unwrap();

    for &(tier_id, min_cl8y, discount_bps, governance_only) in STANDARD_PRODUCTION_TIERS {
        app.execute_contract(
            governance.clone(),
            fee_discount.clone(),
            &cl8y_dex_fee_discount::msg::ExecuteMsg::AddTier {
                tier_id,
                min_cl8y_balance: Uint128::new(min_cl8y),
                discount_bps,
                limit_discount_bps: Some(
                    dex_common::fee_discount::standard_shifted_limit_discount_bps(discount_bps),
                ),
                governance_only,
            },
            &[],
        )
        .unwrap();
    }

    app.execute_contract(
        governance.clone(),
        fee_discount.clone(),
        &cl8y_dex_fee_discount::msg::ExecuteMsg::AddTrustedRouter {
            router: router.to_string(),
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        governance.clone(),
        factory.clone(),
        &FactoryExecuteMsg::SetDiscountRegistry {
            pair: pair.to_string(),
            registry: Some(fee_discount.to_string()),
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        user.clone(),
        fee_discount.clone(),
        &cl8y_dex_fee_discount::msg::ExecuteMsg::Register { tier_id: 1 },
        &[],
    )
    .unwrap();

    app.execute_contract(
        governance.clone(),
        factory.clone(),
        &FactoryExecuteMsg::SetLpAdminAll {
            admin: governance.to_string(),
        },
        &[],
    )
    .unwrap();

    let test_env = TestEnv {
        factory: factory.clone(),
        token_a: token_a.clone(),
        token_b: token_b.clone(),
        pair: pair.clone(),
        lp_token: lp_token.clone(),
        router: router.clone(),
        governance: governance.clone(),
        treasury: treasury.clone(),
        user: user.clone(),
    };
    provide_liquidity(
        app,
        &test_env,
        &user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let place_amount = Uint128::new(10_000);
    app.execute_contract(
        user.clone(),
        token_b.clone(),
        &cw20::Cw20ExecuteMsg::Send {
            contract: pair.to_string(),
            amount: place_amount,
            msg: batch_place_bid_msg(Decimal::one(), place_amount),
        },
        &[],
    )
    .unwrap();
    let limit_order_id = 1u64;

    app.execute_contract(
        governance.clone(),
        factory.clone(),
        &FactoryExecuteMsg::BlacklistWallet {
            address: blacklisted_wallet.to_string(),
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        governance.clone(),
        factory.clone(),
        &FactoryExecuteMsg::BlacklistToken {
            token: token_a.to_string(),
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        governance.clone(),
        factory.clone(),
        &FactoryExecuteMsg::BlacklistPair {
            pair: pair.to_string(),
        },
        &[],
    )
    .unwrap();

    MigrationEnv {
        factory,
        pair,
        lp_token,
        fee_discount,
        token_a: token_a.clone(),
        router,
        governance,
        treasury,
        user,
        blacklisted_wallet,
        factory_code_id,
        pair_code_id,
        fee_discount_code_id,
        limit_order_id,
    }
}

#[derive(Debug, PartialEq)]
struct FactorySnapshot {
    config: dex_common::factory::ConfigResponse,
    pair_count: u64,
    primary_pair: dex_common::types::PairInfo,
    wallet_blacklisted: bool,
    token_blacklisted: bool,
    pair_blacklisted: bool,
    lp_admin: String,
}

fn snapshot_factory(app: &App, env: &MigrationEnv) -> FactorySnapshot {
    let config: dex_common::factory::ConfigResponse = app
        .wrap()
        .query_wasm_smart(
            env.factory.to_string(),
            &dex_common::factory::QueryMsg::Config {},
        )
        .unwrap();
    let pair_count: dex_common::factory::PairCountResponse = app
        .wrap()
        .query_wasm_smart(
            env.factory.to_string(),
            &dex_common::factory::QueryMsg::GetPairCount {},
        )
        .unwrap();
    let primary_pair: dex_common::types::PairInfo = app
        .wrap()
        .query_wasm_smart(env.pair.to_string(), &QueryMsg::Pair {})
        .unwrap();

    let wallet_check: dex_common::blacklist::BlacklistCheckResponse = app
        .wrap()
        .query_wasm_smart(
            env.factory.to_string(),
            &dex_common::factory::QueryMsg::BlacklistCheck(BlacklistCheck {
                wallet: Some(env.blacklisted_wallet.to_string()),
                tokens: vec![],
                pair: None,
                pairs: vec![],
            }),
        )
        .unwrap();
    let token_check: dex_common::blacklist::BlacklistCheckResponse = app
        .wrap()
        .query_wasm_smart(
            env.factory.to_string(),
            &dex_common::factory::QueryMsg::BlacklistCheck(BlacklistCheck {
                wallet: None,
                tokens: vec![env.token_a.to_string()],
                pair: None,
                pairs: vec![],
            }),
        )
        .unwrap();
    let pair_check: dex_common::blacklist::BlacklistCheckResponse = app
        .wrap()
        .query_wasm_smart(
            env.factory.to_string(),
            &dex_common::factory::QueryMsg::BlacklistCheck(BlacklistCheck {
                wallet: None,
                tokens: vec![],
                pair: Some(env.pair.to_string()),
                pairs: vec![],
            }),
        )
        .unwrap();

    let lp_admin = app
        .wrap()
        .query_wasm_contract_info(env.lp_token.to_string())
        .unwrap()
        .admin
        .expect("LP token admin");

    FactorySnapshot {
        config,
        pair_count: pair_count.count,
        primary_pair,
        wallet_blacklisted: wallet_check.wallet_blacklisted,
        token_blacklisted: !token_check.blacklisted_tokens.is_empty(),
        pair_blacklisted: pair_check.pair_blacklisted,
        lp_admin,
    }
}

#[derive(Debug, PartialEq)]
struct PairSnapshot {
    pair: dex_common::types::PairInfo,
    fee_config: FeeConfigResponse,
    pool: PoolResponse,
    limit_order: LimitOrderResponse,
    bid_head: Option<u64>,
    ask_head: Option<u64>,
    limit_order_config: LimitOrderConfigResponse,
    lp_admin: String,
    discounted_commission: Uint128,
}

fn snapshot_pair(app: &App, env: &MigrationEnv) -> PairSnapshot {
    let pair: dex_common::types::PairInfo = app
        .wrap()
        .query_wasm_smart(env.pair.to_string(), &QueryMsg::Pair {})
        .unwrap();
    let fee_config: FeeConfigResponse = app
        .wrap()
        .query_wasm_smart(env.pair.to_string(), &QueryMsg::GetFeeConfig {})
        .unwrap();
    let pool: PoolResponse = app
        .wrap()
        .query_wasm_smart(env.pair.to_string(), &QueryMsg::Pool {})
        .unwrap();
    let limit_order: LimitOrderResponse = app
        .wrap()
        .query_wasm_smart(
            env.pair.to_string(),
            &QueryMsg::LimitOrder {
                order_id: env.limit_order_id,
            },
        )
        .unwrap();
    let bid_head: Option<u64> = app
        .wrap()
        .query_wasm_smart(
            env.pair.to_string(),
            &QueryMsg::OrderBookHead {
                side: LimitOrderSide::Bid,
            },
        )
        .unwrap();
    let ask_head: Option<u64> = app
        .wrap()
        .query_wasm_smart(
            env.pair.to_string(),
            &QueryMsg::OrderBookHead {
                side: LimitOrderSide::Ask,
            },
        )
        .unwrap();
    let limit_order_config: LimitOrderConfigResponse = app
        .wrap()
        .query_wasm_smart(env.pair.to_string(), &QueryMsg::LimitOrderConfig {})
        .unwrap();
    let lp_admin = app
        .wrap()
        .query_wasm_contract_info(env.lp_token.to_string())
        .unwrap()
        .admin
        .expect("LP token admin");

    let offer = Asset {
        info: asset_info_token(&env.token_a),
        amount: Uint128::new(10_000),
    };
    let sim: HybridSimulationResponse = app
        .wrap()
        .query_wasm_smart(
            env.pair.to_string(),
            &QueryMsg::HybridSimulation {
                offer_asset: offer,
                hybrid: pool_only_hybrid_params(Uint128::new(10_000)),
                trader: Some(env.user.to_string()),
                sender: None,
                belief_price: None,
            },
        )
        .unwrap();

    PairSnapshot {
        pair,
        fee_config,
        pool,
        limit_order,
        bid_head,
        ask_head,
        limit_order_config,
        lp_admin,
        discounted_commission: sim.commission_amount,
    }
}

#[derive(Debug, PartialEq)]
struct FeeDiscountSnapshot {
    config: cl8y_dex_fee_discount::msg::ConfigResponse,
    tier1: cl8y_dex_fee_discount::msg::TierResponse,
    registration: cl8y_dex_fee_discount::msg::RegistrationResponse,
    trusted_router: bool,
    discount_bps: u16,
}

fn snapshot_fee_discount(app: &App, env: &MigrationEnv) -> FeeDiscountSnapshot {
    let config: cl8y_dex_fee_discount::msg::ConfigResponse = app
        .wrap()
        .query_wasm_smart(
            env.fee_discount.to_string(),
            &cl8y_dex_fee_discount::msg::QueryMsg::Config {},
        )
        .unwrap();
    let tier1: cl8y_dex_fee_discount::msg::TierResponse = app
        .wrap()
        .query_wasm_smart(
            env.fee_discount.to_string(),
            &cl8y_dex_fee_discount::msg::QueryMsg::GetTier { tier_id: 1 },
        )
        .unwrap();
    let registration: cl8y_dex_fee_discount::msg::RegistrationResponse = app
        .wrap()
        .query_wasm_smart(
            env.fee_discount.to_string(),
            &cl8y_dex_fee_discount::msg::QueryMsg::GetRegistration {
                trader: env.user.to_string(),
            },
        )
        .unwrap();
    let trusted: cl8y_dex_fee_discount::msg::IsTrustedRouterResponse = app
        .wrap()
        .query_wasm_smart(
            env.fee_discount.to_string(),
            &cl8y_dex_fee_discount::msg::QueryMsg::IsTrustedRouter {
                addr: env.router.to_string(),
            },
        )
        .unwrap();
    let discount: cl8y_dex_fee_discount::msg::DiscountResponse = app
        .wrap()
        .query_wasm_smart(
            env.fee_discount.to_string(),
            &cl8y_dex_fee_discount::msg::QueryMsg::GetDiscount {
                trader: env.user.to_string(),
                sender: env.user.to_string(),
            },
        )
        .unwrap();

    FeeDiscountSnapshot {
        config,
        tier1,
        registration,
        trusted_router: trusted.is_trusted,
        discount_bps: discount.discount_bps,
    }
}

#[test]
fn factory_migration_preserves_governance_treasury_registry_blacklists_and_lp_admin() {
    let mut app = App::default();
    let env = setup_migration_env(&mut app);
    let before = snapshot_factory(&app, &env);

    assert_eq!(before.config.governance, env.governance);
    assert_eq!(before.config.treasury, env.treasury);
    assert_eq!(before.pair_count, 1);
    assert!(before.wallet_blacklisted);
    assert!(before.token_blacklisted);
    assert!(before.pair_blacklisted);
    assert_eq!(before.lp_admin, env.governance.to_string());

    downgrade_cw2_version(&mut app, &env.factory, FACTORY_NAME, FACTORY_PRIOR_VERSION);
    app.migrate_contract(
        env.governance.clone(),
        env.factory.clone(),
        &cl8y_dex_factory::msg::MigrateMsg {},
        env.factory_code_id,
    )
    .unwrap();

    let ver = get_contract_version(&*app.contract_storage(&env.factory)).unwrap();
    assert_eq!(ver.contract, FACTORY_NAME);
    assert_eq!(ver.version, FACTORY_VERSION);

    let after = snapshot_factory(&app, &env);
    assert_eq!(after, before);
}

#[test]
fn pair_migration_preserves_fee_registry_lp_admin_and_limit_book() {
    let mut app = App::default();
    let env = setup_migration_env(&mut app);
    let before = snapshot_pair(&app, &env);

    assert_eq!(before.bid_head, Some(env.limit_order_id));
    assert!(before.limit_order.remaining > Uint128::zero());
    assert_eq!(before.lp_admin, env.governance.to_string());
    assert!(before.discounted_commission < Uint128::new(10_000));

    downgrade_cw2_version(&mut app, &env.pair, PAIR_NAME, PAIR_PRIOR_VERSION);
    app.migrate_contract(
        env.governance.clone(),
        env.pair.clone(),
        &cl8y_dex_pair::msg::MigrateMsg {},
        env.pair_code_id,
    )
    .unwrap();

    let ver = get_contract_version(&*app.contract_storage(&env.pair)).unwrap();
    assert_eq!(ver.contract, PAIR_NAME);
    assert_eq!(ver.version, PAIR_VERSION);

    let after = snapshot_pair(&app, &env);
    assert_eq!(after, before);

    // T13 / GitLab #505 — additive OrderStatus works immediately after migrate (no backfill).
    let status: OrderStatusResponse = app
        .wrap()
        .query_wasm_smart(
            env.pair.to_string(),
            &QueryMsg::OrderStatus {
                order_id: env.limit_order_id,
            },
        )
        .unwrap();
    assert_eq!(status.status, OrderStatus::Active);
    assert_eq!(status.remaining, Some(after.limit_order.remaining));
}

#[test]
fn fee_discount_migration_preserves_tiers_registrations_and_trusted_routers() {
    let mut app = App::default();
    let env = setup_migration_env(&mut app);
    let before = snapshot_fee_discount(&app, &env);

    assert!(before.registration.registered);
    assert_eq!(before.registration.tier_id, Some(1));
    assert!(before.trusted_router);
    assert_eq!(before.discount_bps, 250);

    downgrade_cw2_version(
        &mut app,
        &env.fee_discount,
        FEE_DISCOUNT_NAME,
        FEE_DISCOUNT_PRIOR_VERSION,
    );
    app.migrate_contract(
        env.governance.clone(),
        env.fee_discount.clone(),
        &cl8y_dex_fee_discount::msg::MigrateMsg {},
        env.fee_discount_code_id,
    )
    .unwrap();

    let ver = get_contract_version(&*app.contract_storage(&env.fee_discount)).unwrap();
    assert_eq!(ver.contract, FEE_DISCOUNT_NAME);
    assert_eq!(ver.version, FEE_DISCOUNT_VERSION);

    let after = snapshot_fee_discount(&app, &env);
    assert_eq!(after, before);
}

#[test]
fn pair_migration_checks_version() {
    let mut app = App::default();
    let governance = Addr::unchecked("governance");

    let mock_old_id = app.store_code(mock_old_pair_contract());
    let mock_future_id = app.store_code(mock_future_pair_contract());
    let pair_code_id = app.store_code(pair_contract_with_migrate());

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
