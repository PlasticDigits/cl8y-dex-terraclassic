use super::*;
use crate::msg::{IssuanceLimits, LegacyMarketPin};
use crate::state::MarketPins;
use cosmwasm_std::{Addr, Coin};
use cw20::Cw20QueryMsg;
use cw20_base::msg::{ExecuteMsg as Cw20ExecuteMsgBase, InstantiateMsg as Cw20InstantiateMsg};
use cw_multi_test::{App, ContractWrapper, Executor};

const OWNER: &str = "terra1owner000000000000000000000000000000000";
const CMM: &str = "terra1cmm00000000000000000000000000000000000";
const LP: &str = "terra1user00000000000000000000000000000000";
const OTHER: &str = "terra1dest000000000000000000000000000000000";

fn units(whole: u128) -> Uint128 {
    Uint128::new(whole) * Uint128::new(1_000_000_000_000_000_000)
}

fn issuer_code(app: &mut App) -> u64 {
    app.store_code(Box::new(
        ContractWrapper::new(execute, instantiate, query).with_migrate(migrate),
    ))
}

fn debt_code(app: &mut App) -> u64 {
    app.store_code(Box::new(
        ContractWrapper::new(
            bridge_debt_token::contract::execute,
            bridge_debt_token::contract::instantiate,
            bridge_debt_token::contract::query,
        )
        .with_migrate(bridge_debt_token::contract::migrate),
    ))
}

fn cw20_code(app: &mut App) -> u64 {
    app.store_code(Box::new(
        ContractWrapper::new(
            cw20_base::contract::execute,
            cw20_base::contract::instantiate,
            cw20_base::contract::query,
        )
        .with_migrate(preserve_cw20),
    ))
}

fn preserve_cw20(_deps: DepsMut, _env: Env, _msg: cosmwasm_std::Empty) -> StdResult<Response> {
    Ok(Response::new())
}

fn owner_execute(
    app: &mut App,
    issuer: &Addr,
    msg: ExecuteMsg,
) -> cw_multi_test::error::AnyResult<cw_multi_test::AppResponse> {
    app.execute_contract(Addr::unchecked(OWNER), issuer.clone(), &msg, &[])
}

fn approve_underlying(app: &mut App, issuer: &Addr, code_id: u64, multi_minter: bool) {
    owner_execute(
        app,
        issuer,
        ExecuteMsg::ApproveUnderlyingCode {
            code_id,
            multi_minter,
        },
    )
    .unwrap();
}

fn generous_limits() -> IssuanceLimits {
    IssuanceLimits {
        max_per_issue: Uint128::MAX,
        max_outstanding: Uint128::MAX,
        epoch_seconds: 86_400,
        max_per_epoch: Uint128::MAX,
    }
}

fn configure(
    app: &mut App,
    issuer: &Addr,
    underlying: &Addr,
    limits: IssuanceLimits,
    paused: bool,
) {
    owner_execute(
        app,
        issuer,
        ExecuteMsg::ConfigureMarket {
            underlying: underlying.to_string(),
            limits,
            paused,
        },
    )
    .unwrap();
}

fn market(app: &App, issuer: &Addr, underlying: &Addr) -> MarketResponse {
    app.wrap()
        .query_wasm_smart(
            issuer,
            &QueryMsg::Market {
                underlying: underlying.to_string(),
            },
        )
        .unwrap()
}

fn try_issue(
    world: &mut World,
    amount: Uint128,
) -> cw_multi_test::error::AnyResult<cw_multi_test::AppResponse> {
    owner_execute(
        &mut world.app,
        &world.issuer,
        ExecuteMsg::Issue {
            underlying: world.usdt.to_string(),
            recipient: LP.to_string(),
            amount,
        },
    )
}

fn repay(world: &mut World, amount: Uint128) {
    world
        .app
        .execute_contract(
            Addr::unchecked(LP),
            world.usdt.clone(),
            &Cw20ExecuteMsgBase::Send {
                contract: world.issuer.to_string(),
                amount,
                msg: to_json_binary(&Cw20HookMsg::Repay {}).unwrap(),
            },
            &[],
        )
        .unwrap();
}

struct World {
    app: App,
    issuer: Addr,
    usdt: Addr,
    debt: Addr,
    debt_code_id: u64,
    cw20_code_id: u64,
}

fn instantiate_cw20(
    app: &mut App,
    code_id: u64,
    symbol: &str,
    decimals: u8,
    minter: &Addr,
) -> Addr {
    instantiate_cw20_with(app, code_id, symbol, decimals, minter, vec![])
}

fn instantiate_cw20_with(
    app: &mut App,
    code_id: u64,
    symbol: &str,
    decimals: u8,
    minter: &Addr,
    initial_balances: Vec<cw20::Cw20Coin>,
) -> Addr {
    app.instantiate_contract(
        code_id,
        Addr::unchecked(OWNER),
        &Cw20InstantiateMsg {
            name: symbol.to_string(),
            symbol: symbol.to_string(),
            decimals,
            initial_balances,
            mint: Some(cw20::MinterResponse {
                minter: minter.to_string(),
                cap: None,
            }),
            marketing: None,
        },
        &[],
        symbol,
        Some(OWNER.to_string()),
    )
    .unwrap()
}

fn shows(err: &cw_multi_test::error::AnyError, text: &str) -> bool {
    format!("{err:?}").contains(text)
}

fn instantiate_debt(
    app: &mut App,
    code_id: u64,
    symbol: &str,
    decimals: u8,
    minter: &Addr,
    debtor: &str,
) -> Addr {
    app.instantiate_contract(
        code_id,
        Addr::unchecked(OWNER),
        &bridge_debt_token::msg::InstantiateMsg {
            name: symbol.to_string(),
            symbol: symbol.to_string(),
            decimals,
            minter: minter.to_string(),
            debtor: debtor.to_string(),
            cap: None,
        },
        &[],
        symbol,
        Some(OWNER.to_string()),
    )
    .unwrap()
}

fn world() -> World {
    let mut app = App::default();
    let debt_code_id = debt_code(&mut app);
    let cw20_code_id = cw20_code(&mut app);
    let issuer_id = issuer_code(&mut app);
    let issuer = app
        .instantiate_contract(
            issuer_id,
            Addr::unchecked(OWNER),
            &InstantiateMsg {
                owner: OWNER.to_string(),
            },
            &[],
            "bridge-debt",
            Some(OWNER.to_string()),
        )
        .unwrap();
    let usdt = instantiate_cw20(&mut app, cw20_code_id, "USDT", 18, &issuer);
    let debt = instantiate_debt(&mut app, debt_code_id, "USDT-Bridge-Debt", 18, &issuer, CMM);
    app.execute_contract(
        Addr::unchecked(OWNER),
        issuer.clone(),
        &ExecuteMsg::SetDebtCodeId {
            code_id: debt_code_id,
        },
        &[],
    )
    .unwrap();
    approve_underlying(&mut app, &issuer, cw20_code_id, false);
    app.execute_contract(
        Addr::unchecked(OWNER),
        issuer.clone(),
        &ExecuteMsg::RegisterMarket {
            underlying: usdt.to_string(),
            debt_token: debt.to_string(),
        },
        &[],
    )
    .unwrap();
    configure(&mut app, &issuer, &usdt, generous_limits(), false);

    World {
        app,
        issuer,
        usdt,
        debt,
        debt_code_id,
        cw20_code_id,
    }
}

fn balance(app: &App, token: &Addr, account: &str) -> Uint128 {
    let response: cw20::BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            token,
            &Cw20QueryMsg::Balance {
                address: account.to_string(),
            },
        )
        .unwrap();
    response.balance
}

fn supply(app: &App, token: &Addr) -> Uint128 {
    let info: TokenInfoResponse = app
        .wrap()
        .query_wasm_smart(token, &Cw20QueryMsg::TokenInfo {})
        .unwrap();
    info.total_supply
}

fn outstanding(app: &App, issuer: &Addr, underlying: &Addr) -> Uint128 {
    let market: MarketResponse = app
        .wrap()
        .query_wasm_smart(
            issuer,
            &QueryMsg::Market {
                underlying: underlying.to_string(),
            },
        )
        .unwrap();
    market.outstanding
}

fn issue(world: &mut World, recipient: &str, amount: Uint128) {
    world
        .app
        .execute_contract(
            Addr::unchecked(OWNER),
            world.issuer.clone(),
            &ExecuteMsg::Issue {
                underlying: world.usdt.to_string(),
                recipient: recipient.to_string(),
                amount,
            },
            &[],
        )
        .unwrap();
}

#[test]
fn ten_thousand_usdt_for_cmm_lp_mints_matching_debt() {
    let mut world = world();
    let amount = units(10_000);
    issue(&mut world, LP, amount);

    assert_eq!(balance(&world.app, &world.usdt, LP), amount);
    assert_eq!(balance(&world.app, &world.debt, CMM), amount);
    assert_eq!(supply(&world.app, &world.debt), amount);
    assert_eq!(outstanding(&world.app, &world.issuer, &world.usdt), amount);
    assert_eq!(balance(&world.app, &world.debt, LP), Uint128::zero());

    let info: TokenInfoResponse = world
        .app
        .wrap()
        .query_wasm_smart(&world.debt, &Cw20QueryMsg::TokenInfo {})
        .unwrap();
    assert_eq!(info.symbol, "USDT-Bridge-Debt");
    assert_eq!(info.decimals, 18);
}

#[test]
fn debt_receipt_cannot_leave_the_cmm() {
    let mut world = world();
    issue(&mut world, LP, units(1));

    let err = world
        .app
        .execute_contract(
            Addr::unchecked(CMM),
            world.debt.clone(),
            &bridge_debt_token::msg::ExecuteMsg::Transfer {
                recipient: OTHER.to_string(),
                amount: units(1),
            },
            &[],
        )
        .unwrap_err();
    assert!(shows(&err, "Debt tokens stay with the debtor"), "{err:?}");
    assert_eq!(balance(&world.app, &world.debt, CMM), units(1));
}

#[test]
fn repay_burns_usdt_and_cmm_debt_together() {
    let mut world = world();
    issue(&mut world, LP, units(10_000));

    world
        .app
        .execute_contract(
            Addr::unchecked(LP),
            world.usdt.clone(),
            &Cw20ExecuteMsgBase::Send {
                contract: world.issuer.to_string(),
                amount: units(1_000),
                msg: to_json_binary(&Cw20HookMsg::Repay {}).unwrap(),
            },
            &[],
        )
        .unwrap();

    assert_eq!(balance(&world.app, &world.usdt, LP), units(9_000));
    assert_eq!(balance(&world.app, &world.debt, CMM), units(9_000));
    assert_eq!(
        outstanding(&world.app, &world.issuer, &world.usdt),
        units(9_000)
    );
    assert_eq!(supply(&world.app, &world.usdt), units(9_000));

    world
        .app
        .execute_contract(
            Addr::unchecked(LP),
            world.usdt.clone(),
            &Cw20ExecuteMsgBase::Send {
                contract: world.issuer.to_string(),
                amount: units(9_000),
                msg: to_json_binary(&Cw20HookMsg::Repay {}).unwrap(),
            },
            &[],
        )
        .unwrap();
    assert_eq!(balance(&world.app, &world.usdt, LP), Uint128::zero());
    assert_eq!(balance(&world.app, &world.debt, CMM), Uint128::zero());
    assert_eq!(
        outstanding(&world.app, &world.issuer, &world.usdt),
        Uint128::zero()
    );
}

#[test]
fn repay_above_outstanding_is_rejected() {
    let mut app = App::default();
    let debt_code_id = debt_code(&mut app);
    let cw20_code_id = cw20_code(&mut app);
    let issuer_id = issuer_code(&mut app);
    let issuer = app
        .instantiate_contract(
            issuer_id,
            Addr::unchecked(OWNER),
            &InstantiateMsg {
                owner: OWNER.to_string(),
            },
            &[],
            "bridge-debt",
            Some(OWNER.to_string()),
        )
        .unwrap();
    // Pre-existing bridged USDT (a normal bridge mint) is not debt.
    let usdt = instantiate_cw20_with(
        &mut app,
        cw20_code_id,
        "USDT",
        18,
        &issuer,
        vec![cw20::Cw20Coin {
            address: LP.to_string(),
            amount: units(5),
        }],
    );
    let debt = instantiate_debt(&mut app, debt_code_id, "USDT-Bridge-Debt", 18, &issuer, CMM);
    app.execute_contract(
        Addr::unchecked(OWNER),
        issuer.clone(),
        &ExecuteMsg::SetDebtCodeId {
            code_id: debt_code_id,
        },
        &[],
    )
    .unwrap();
    approve_underlying(&mut app, &issuer, cw20_code_id, false);
    app.execute_contract(
        Addr::unchecked(OWNER),
        issuer.clone(),
        &ExecuteMsg::RegisterMarket {
            underlying: usdt.to_string(),
            debt_token: debt.to_string(),
        },
        &[],
    )
    .unwrap();
    configure(&mut app, &issuer, &usdt, generous_limits(), false);
    app.execute_contract(
        Addr::unchecked(OWNER),
        issuer.clone(),
        &ExecuteMsg::Issue {
            underlying: usdt.to_string(),
            recipient: LP.to_string(),
            amount: units(1),
        },
        &[],
    )
    .unwrap();

    let err = app
        .execute_contract(
            Addr::unchecked(LP),
            usdt.clone(),
            &Cw20ExecuteMsgBase::Send {
                contract: issuer.to_string(),
                amount: units(2),
                msg: to_json_binary(&Cw20HookMsg::Repay {}).unwrap(),
            },
            &[],
        )
        .unwrap_err();
    assert!(shows(&err, "exceeds outstanding"), "{err:?}");
    assert_eq!(balance(&app, &debt, CMM), units(1));
    assert_eq!(balance(&app, &usdt, LP), units(6));
}

#[test]
fn plain_transfer_does_not_reduce_debt_and_can_be_recovered() {
    let mut world = world();
    issue(&mut world, LP, units(5));
    world
        .app
        .execute_contract(
            Addr::unchecked(LP),
            world.usdt.clone(),
            &Cw20ExecuteMsgBase::Transfer {
                recipient: world.issuer.to_string(),
                amount: units(1),
            },
            &[],
        )
        .unwrap();
    assert_eq!(
        outstanding(&world.app, &world.issuer, &world.usdt),
        units(5)
    );

    world
        .app
        .execute_contract(
            Addr::unchecked(OWNER),
            world.issuer.clone(),
            &ExecuteMsg::Recover {
                underlying: world.usdt.to_string(),
                recipient: OTHER.to_string(),
                amount: units(1),
            },
            &[],
        )
        .unwrap();
    assert_eq!(balance(&world.app, &world.usdt, OTHER), units(1));
    assert_eq!(balance(&world.app, &world.debt, CMM), units(5));
}

#[test]
fn second_asset_uses_its_own_bridge_debt_token() {
    let mut world = world();
    issue(&mut world, LP, units(10_000));

    let xxx = instantiate_cw20(&mut world.app, world.cw20_code_id, "XXX", 18, &world.issuer);
    let xxx_debt = instantiate_debt(
        &mut world.app,
        world.debt_code_id,
        "XXX-Bridge-Debt",
        18,
        &world.issuer,
        CMM,
    );
    world
        .app
        .execute_contract(
            Addr::unchecked(OWNER),
            world.issuer.clone(),
            &ExecuteMsg::RegisterMarket {
                underlying: xxx.to_string(),
                debt_token: xxx_debt.to_string(),
            },
            &[],
        )
        .unwrap();
    configure(
        &mut world.app,
        &world.issuer,
        &xxx,
        generous_limits(),
        false,
    );
    world
        .app
        .execute_contract(
            Addr::unchecked(OWNER),
            world.issuer.clone(),
            &ExecuteMsg::Issue {
                underlying: xxx.to_string(),
                recipient: LP.to_string(),
                amount: units(2),
            },
            &[],
        )
        .unwrap();

    assert_eq!(balance(&world.app, &xxx, LP), units(2));
    assert_eq!(balance(&world.app, &xxx_debt, CMM), units(2));
    assert_eq!(balance(&world.app, &world.debt, CMM), units(10_000));
    assert_eq!(outstanding(&world.app, &world.issuer, &xxx), units(2));

    let listed: MarketsResponse = world
        .app
        .wrap()
        .query_wasm_smart(
            &world.issuer,
            &QueryMsg::Markets {
                start_after: None,
                limit: None,
            },
        )
        .unwrap();
    assert_eq!(listed.markets.len(), 2);
}

#[test]
fn register_rejects_plain_cw20_and_wrong_code() {
    let mut app = App::default();
    let debt_code_id = debt_code(&mut app);
    let cw20_code_id = cw20_code(&mut app);
    let issuer_id = issuer_code(&mut app);
    let issuer = app
        .instantiate_contract(
            issuer_id,
            Addr::unchecked(OWNER),
            &InstantiateMsg {
                owner: OWNER.to_string(),
            },
            &[],
            "bridge-debt",
            Some(OWNER.to_string()),
        )
        .unwrap();
    let usdt = instantiate_cw20(&mut app, cw20_code_id, "USDT", 18, &issuer);

    let err = app
        .execute_contract(
            Addr::unchecked(OWNER),
            issuer.clone(),
            &ExecuteMsg::RegisterMarket {
                underlying: usdt.to_string(),
                debt_token: usdt.to_string(),
            },
            &[],
        )
        .unwrap_err();
    assert!(
        shows(&err, "code id is not set") || shows(&err, "different"),
        "{err:?}"
    );

    app.execute_contract(
        Addr::unchecked(OWNER),
        issuer.clone(),
        &ExecuteMsg::SetDebtCodeId {
            code_id: cw20_code_id,
        },
        &[],
    )
    .unwrap();
    approve_underlying(&mut app, &issuer, cw20_code_id, false);
    let fake_debt = instantiate_cw20(&mut app, cw20_code_id, "FAKE", 18, &issuer);
    let err = app
        .execute_contract(
            Addr::unchecked(OWNER),
            issuer.clone(),
            &ExecuteMsg::RegisterMarket {
                underlying: usdt.to_string(),
                debt_token: fake_debt.to_string(),
            },
            &[],
        )
        .unwrap_err();
    assert!(shows(&err, "debtor"), "{err:?}");

    app.execute_contract(
        Addr::unchecked(OWNER),
        issuer.clone(),
        &ExecuteMsg::SetDebtCodeId {
            code_id: debt_code_id,
        },
        &[],
    )
    .unwrap();
    approve_underlying(&mut app, &issuer, cw20_code_id, false);
    let six = instantiate_cw20(&mut app, cw20_code_id, "SIX", 6, &issuer);
    let debt = instantiate_debt(&mut app, debt_code_id, "USDT-Bridge-Debt", 18, &issuer, CMM);
    let err = app
        .execute_contract(
            Addr::unchecked(OWNER),
            issuer.clone(),
            &ExecuteMsg::RegisterMarket {
                underlying: six.to_string(),
                debt_token: debt.to_string(),
            },
            &[],
        )
        .unwrap_err();
    assert!(shows(&err, "decimals"), "{err:?}");
}

#[test]
fn only_owner_can_issue() {
    let mut world = world();
    let err = world
        .app
        .execute_contract(
            Addr::unchecked(LP),
            world.issuer.clone(),
            &ExecuteMsg::Issue {
                underlying: world.usdt.to_string(),
                recipient: LP.to_string(),
                amount: units(1),
            },
            &[],
        )
        .unwrap_err();
    assert!(shows(&err, "Unauthorized"), "{err:?}");

    let err = world
        .app
        .execute_contract(
            Addr::unchecked(OWNER),
            world.issuer.clone(),
            &ExecuteMsg::Issue {
                underlying: world.usdt.to_string(),
                recipient: LP.to_string(),
                amount: Uint128::zero(),
            },
            &[],
        )
        .unwrap_err();
    assert!(shows(&err, "greater than zero"), "{err:?}");
}

#[test]
fn rejects_native_funds() {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};

    let mut deps = mock_dependencies();
    instantiate(
        deps.as_mut(),
        mock_env(),
        mock_info("creator", &[]),
        InstantiateMsg {
            owner: OWNER.to_string(),
        },
    )
    .unwrap();

    let err = execute(
        deps.as_mut(),
        mock_env(),
        mock_info(OWNER, &[Coin::new(1u128, "uluna")]),
        ExecuteMsg::Issue {
            underlying: "terra1usdt00000000000000000000000000000000".to_string(),
            recipient: LP.to_string(),
            amount: units(1),
        },
    )
    .unwrap_err();
    assert_eq!(err, ContractError::UnexpectedFunds {});
}

#[test]
fn register_fails_when_this_contract_cannot_mint_underlying() {
    let mut world = world();
    let token = instantiate_cw20(
        &mut world.app,
        world.cw20_code_id,
        "BAD",
        18,
        &Addr::unchecked(OWNER),
    );
    let debt = instantiate_debt(
        &mut world.app,
        world.debt_code_id,
        "BAD-Bridge-Debt",
        18,
        &world.issuer,
        CMM,
    );
    let err = owner_execute(
        &mut world.app,
        &world.issuer,
        ExecuteMsg::RegisterMarket {
            underlying: token.to_string(),
            debt_token: debt.to_string(),
        },
    )
    .unwrap_err();
    assert!(shows(&err, "issuer is not authorized"), "{err:?}");
    assert_eq!(supply(&world.app, &token), Uint128::zero());
    assert_eq!(supply(&world.app, &debt), Uint128::zero());
}
#[test]
fn registration_requires_underlying_approval_and_explicit_limits() {
    let mut world = world();
    let code_id = cw20_code(&mut world.app);
    let token = instantiate_cw20(&mut world.app, code_id, "NEW", 18, &world.issuer);
    let debt = instantiate_debt(
        &mut world.app,
        world.debt_code_id,
        "NEW-Bridge-Debt",
        18,
        &world.issuer,
        CMM,
    );
    let registration = ExecuteMsg::RegisterMarket {
        underlying: token.to_string(),
        debt_token: debt.to_string(),
    };
    let err = owner_execute(&mut world.app, &world.issuer, registration.clone()).unwrap_err();
    assert!(shows(&err, "not approved or pinned"), "{err:?}");
    approve_underlying(&mut world.app, &world.issuer, code_id, false);
    owner_execute(&mut world.app, &world.issuer, registration).unwrap();
    let state = market(&world.app, &world.issuer, &token);
    assert_eq!(state.debt_code_id, Some(world.debt_code_id));
    assert_eq!(state.underlying_code_id, Some(code_id));
    assert!(state.limits.is_none());
    let err = owner_execute(
        &mut world.app,
        &world.issuer,
        ExecuteMsg::Issue {
            underlying: token.to_string(),
            recipient: LP.to_string(),
            amount: units(1),
        },
    )
    .unwrap_err();
    assert!(shows(&err, "disabled or paused"), "{err:?}");
    assert_eq!(supply(&world.app, &token), Uint128::zero());
    assert_eq!(supply(&world.app, &debt), Uint128::zero());
}

#[test]
fn limits_are_atomic_epoch_reconfiguration_cannot_reset_and_pause_allows_repay_recover() {
    let mut world = world();
    let limits = IssuanceLimits {
        max_per_issue: units(5),
        max_outstanding: units(8),
        epoch_seconds: 86_400,
        max_per_epoch: units(10),
    };
    configure(
        &mut world.app,
        &world.issuer,
        &world.usdt,
        limits.clone(),
        false,
    );
    assert!(shows(
        &try_issue(&mut world, units(6)).unwrap_err(),
        "limit exceeded"
    ));
    assert_eq!(
        market(&world.app, &world.issuer, &world.usdt).epoch_issued,
        Uint128::zero()
    );
    issue(&mut world, LP, units(5));
    assert!(shows(
        &try_issue(&mut world, units(4)).unwrap_err(),
        "limit exceeded"
    ));
    repay(&mut world, units(3));
    issue(&mut world, LP, units(5));
    assert!(shows(
        &try_issue(&mut world, units(1)).unwrap_err(),
        "limit exceeded"
    ));
    let before = market(&world.app, &world.issuer, &world.usdt);
    assert_eq!(before.outstanding, units(7));
    assert_eq!(before.epoch_issued, units(10));
    let mut reset = limits.clone();
    reset.epoch_seconds = 120;
    let err = owner_execute(
        &mut world.app,
        &world.issuer,
        ExecuteMsg::ConfigureMarket {
            underlying: world.usdt.to_string(),
            limits: reset,
            paused: false,
        },
    )
    .unwrap_err();
    assert!(shows(&err, "epoch reset"), "{err:?}");
    assert_eq!(market(&world.app, &world.issuer, &world.usdt), before);
    configure(
        &mut world.app,
        &world.issuer,
        &world.usdt,
        limits.clone(),
        true,
    );
    assert!(shows(
        &try_issue(&mut world, units(1)).unwrap_err(),
        "paused"
    ));
    repay(&mut world, units(2));
    world
        .app
        .execute_contract(
            Addr::unchecked(LP),
            world.usdt.clone(),
            &Cw20ExecuteMsgBase::Transfer {
                recipient: world.issuer.to_string(),
                amount: units(1),
            },
            &[],
        )
        .unwrap();
    owner_execute(
        &mut world.app,
        &world.issuer,
        ExecuteMsg::Recover {
            underlying: world.usdt.to_string(),
            recipient: OTHER.to_string(),
            amount: units(1),
        },
    )
    .unwrap();
    assert_eq!(balance(&world.app, &world.usdt, OTHER), units(1));
    assert_eq!(
        outstanding(&world.app, &world.issuer, &world.usdt),
        units(5)
    );
    assert_eq!(
        market(&world.app, &world.issuer, &world.usdt).epoch_issued,
        units(10)
    );
    configure(&mut world.app, &world.issuer, &world.usdt, limits, false);
    world.app.update_block(|block| {
        block.height += 1;
        block.time = block.time.plus_seconds(86_401);
    });
    issue(&mut world, LP, units(1));
    assert_eq!(
        market(&world.app, &world.issuer, &world.usdt).epoch_issued,
        units(1)
    );
    assert_eq!(
        outstanding(&world.app, &world.issuer, &world.usdt),
        units(6)
    );
}

#[test]
fn owner_transfer_requires_acceptance_and_can_be_cancelled() {
    let mut world = world();
    owner_execute(
        &mut world.app,
        &world.issuer,
        ExecuteMsg::UpdateOwner {
            owner: OTHER.to_string(),
        },
    )
    .unwrap();
    let config: ConfigResponse = world
        .app
        .wrap()
        .query_wasm_smart(&world.issuer, &QueryMsg::Config {})
        .unwrap();
    assert_eq!(config.owner, Addr::unchecked(OWNER));
    assert_eq!(config.pending_owner, Some(Addr::unchecked(OTHER)));
    issue(&mut world, LP, units(1));
    let err = world
        .app
        .execute_contract(
            Addr::unchecked(LP),
            world.issuer.clone(),
            &ExecuteMsg::AcceptOwner {},
            &[],
        )
        .unwrap_err();
    assert!(shows(&err, "Unauthorized"));
    owner_execute(
        &mut world.app,
        &world.issuer,
        ExecuteMsg::CancelOwnerTransfer {},
    )
    .unwrap();
    assert!(world
        .app
        .execute_contract(
            Addr::unchecked(OTHER),
            world.issuer.clone(),
            &ExecuteMsg::AcceptOwner {},
            &[]
        )
        .is_err());
    owner_execute(
        &mut world.app,
        &world.issuer,
        ExecuteMsg::UpdateOwner {
            owner: OTHER.to_string(),
        },
    )
    .unwrap();
    world
        .app
        .execute_contract(
            Addr::unchecked(OTHER),
            world.issuer.clone(),
            &ExecuteMsg::AcceptOwner {},
            &[],
        )
        .unwrap();
    assert!(shows(
        &try_issue(&mut world, units(1)).unwrap_err(),
        "Unauthorized"
    ));
    world
        .app
        .execute_contract(
            Addr::unchecked(OTHER),
            world.issuer.clone(),
            &ExecuteMsg::Issue {
                underlying: world.usdt.to_string(),
                recipient: LP.to_string(),
                amount: units(1),
            },
            &[],
        )
        .unwrap();
    let config: ConfigResponse = world
        .app
        .wrap()
        .query_wasm_smart(&world.issuer, &QueryMsg::Config {})
        .unwrap();
    assert_eq!(config.owner, Addr::unchecked(OTHER));
    assert_eq!(config.pending_owner, None);
}

#[test]
fn receipt_default_rotation_does_not_repin_existing_markets_and_upgrade_is_local() {
    let mut world = world();
    issue(&mut world, LP, units(5));
    let second_debt_code = debt_code(&mut world.app);
    assert_ne!(world.debt_code_id, second_debt_code);
    owner_execute(
        &mut world.app,
        &world.issuer,
        ExecuteMsg::SetDebtCodeId {
            code_id: second_debt_code,
        },
    )
    .unwrap();
    issue(&mut world, LP, units(1));
    assert_eq!(
        market(&world.app, &world.issuer, &world.usdt).debt_code_id,
        Some(world.debt_code_id)
    );
    let token = instantiate_cw20(&mut world.app, world.cw20_code_id, "XXX", 18, &world.issuer);
    let debt = instantiate_debt(
        &mut world.app,
        second_debt_code,
        "XXX-Bridge-Debt",
        18,
        &world.issuer,
        CMM,
    );
    owner_execute(
        &mut world.app,
        &world.issuer,
        ExecuteMsg::RegisterMarket {
            underlying: token.to_string(),
            debt_token: debt.to_string(),
        },
    )
    .unwrap();
    configure(
        &mut world.app,
        &world.issuer,
        &token,
        generous_limits(),
        false,
    );
    owner_execute(
        &mut world.app,
        &world.issuer,
        ExecuteMsg::Issue {
            underlying: token.to_string(),
            recipient: LP.to_string(),
            amount: units(2),
        },
    )
    .unwrap();
    let second_before = market(&world.app, &world.issuer, &token);
    owner_execute(
        &mut world.app,
        &world.issuer,
        ExecuteMsg::ProposeMarketUpgrade {
            underlying: world.usdt.to_string(),
            debt_code_id: second_debt_code,
            underlying_code_id: world.cw20_code_id,
        },
    )
    .unwrap();
    assert!(owner_execute(
        &mut world.app,
        &world.issuer,
        ExecuteMsg::AcceptMarketUpgrade {
            underlying: world.usdt.to_string()
        }
    )
    .is_err());
    world
        .app
        .migrate_contract(
            Addr::unchecked(OWNER),
            world.debt.clone(),
            &bridge_debt_token::msg::MigrateMsg {},
            second_debt_code,
        )
        .unwrap();
    assert!(try_issue(&mut world, units(1)).is_err());
    assert_eq!(market(&world.app, &world.issuer, &token), second_before);
    owner_execute(
        &mut world.app,
        &world.issuer,
        ExecuteMsg::AcceptMarketUpgrade {
            underlying: world.usdt.to_string(),
        },
    )
    .unwrap();
    issue(&mut world, LP, units(1));
    repay(&mut world, units(1));
    assert_eq!(
        market(&world.app, &world.issuer, &world.usdt).debt_code_id,
        Some(second_debt_code)
    );
    assert_eq!(market(&world.app, &world.issuer, &token), second_before);
    assert_eq!(balance(&world.app, &debt, CMM), units(2));
}

#[test]
fn downstream_receipt_mint_failure_rolls_back_underlying_and_accounting() {
    let mut world = world();
    let token = instantiate_cw20(&mut world.app, world.cw20_code_id, "CAP", 18, &world.issuer);
    let debt = world
        .app
        .instantiate_contract(
            world.debt_code_id,
            Addr::unchecked(OWNER),
            &bridge_debt_token::msg::InstantiateMsg {
                name: "CAP Bridge Debt".to_string(),
                symbol: "CAP-Bridge-Debt".to_string(),
                decimals: 18,
                minter: world.issuer.to_string(),
                debtor: CMM.to_string(),
                cap: Some(units(1)),
            },
            &[],
            "capped debt",
            Some(OWNER.to_string()),
        )
        .unwrap();
    owner_execute(
        &mut world.app,
        &world.issuer,
        ExecuteMsg::RegisterMarket {
            underlying: token.to_string(),
            debt_token: debt.to_string(),
        },
    )
    .unwrap();
    configure(
        &mut world.app,
        &world.issuer,
        &token,
        generous_limits(),
        false,
    );
    let before = market(&world.app, &world.issuer, &token);
    let err = owner_execute(
        &mut world.app,
        &world.issuer,
        ExecuteMsg::Issue {
            underlying: token.to_string(),
            recipient: LP.to_string(),
            amount: units(2),
        },
    )
    .unwrap_err();
    assert!(shows(&err, "cap"), "{err:?}");
    assert_eq!(market(&world.app, &world.issuer, &token), before);
    assert_eq!(balance(&world.app, &token, LP), Uint128::zero());
    assert_eq!(supply(&world.app, &token), Uint128::zero());
    assert_eq!(supply(&world.app, &debt), Uint128::zero());
    owner_execute(
        &mut world.app,
        &world.issuer,
        ExecuteMsg::Issue {
            underlying: token.to_string(),
            recipient: LP.to_string(),
            amount: units(1),
        },
    )
    .unwrap();
    assert_eq!(balance(&world.app, &debt, CMM), units(1));
}
#[cosmwasm_schema::cw_serde]
struct UnderlyingMutation {
    minter: Option<String>,
    decimals: Option<u8>,
    supply: Option<Uint128>,
}

fn mutate_underlying(deps: DepsMut, _env: Env, msg: UnderlyingMutation) -> StdResult<Response> {
    cw20_base::state::TOKEN_INFO.update(deps.storage, |mut token| -> StdResult<_> {
        if let Some(minter) = msg.minter {
            token.mint.as_mut().unwrap().minter = Addr::unchecked(minter);
        }
        if let Some(decimals) = msg.decimals {
            token.decimals = decimals;
        }
        if let Some(supply) = msg.supply {
            token.total_supply = supply;
        }
        Ok(token)
    })?;
    Ok(Response::new())
}

#[test]
fn approved_underlying_upgrade_rejects_minter_decimals_and_supply_mutations_atomically() {
    for mutation in [
        UnderlyingMutation {
            minter: Some(OTHER.to_string()),
            decimals: None,
            supply: None,
        },
        UnderlyingMutation {
            minter: None,
            decimals: Some(6),
            supply: None,
        },
        UnderlyingMutation {
            minter: None,
            decimals: None,
            supply: Some(units(99)),
        },
    ] {
        let mut world = world();
        issue(&mut world, LP, units(3));
        let target = world.app.store_code(Box::new(
            ContractWrapper::new(
                cw20_base::contract::execute,
                cw20_base::contract::instantiate,
                cw20_base::contract::query,
            )
            .with_migrate(mutate_underlying),
        ));
        approve_underlying(&mut world.app, &world.issuer, target, false);
        owner_execute(
            &mut world.app,
            &world.issuer,
            ExecuteMsg::ProposeMarketUpgrade {
                underlying: world.usdt.to_string(),
                debt_code_id: world.debt_code_id,
                underlying_code_id: target,
            },
        )
        .unwrap();
        let before = market(&world.app, &world.issuer, &world.usdt);
        world
            .app
            .migrate_contract(
                Addr::unchecked(OWNER),
                world.usdt.clone(),
                &mutation,
                target,
            )
            .unwrap();
        assert!(owner_execute(
            &mut world.app,
            &world.issuer,
            ExecuteMsg::AcceptMarketUpgrade {
                underlying: world.usdt.to_string(),
            }
        )
        .is_err());
        assert_eq!(market(&world.app, &world.issuer, &world.usdt), before);
        assert!(try_issue(&mut world, units(1)).is_err());
        assert_eq!(balance(&world.app, &world.debt, CMM), units(3));
        assert_eq!(balance(&world.app, &world.usdt, LP), units(3));
        assert_eq!(
            outstanding(&world.app, &world.issuer, &world.usdt),
            units(3)
        );
    }
}

#[test]
fn coordinated_underlying_upgrade_allows_repayment_and_can_be_cancelled() {
    let mut world = world();
    issue(&mut world, LP, units(3));
    let target = cw20_code(&mut world.app);
    let proposal = ExecuteMsg::ProposeMarketUpgrade {
        underlying: world.usdt.to_string(),
        debt_code_id: world.debt_code_id,
        underlying_code_id: target,
    };
    assert!(owner_execute(&mut world.app, &world.issuer, proposal.clone()).is_err());
    approve_underlying(&mut world.app, &world.issuer, target, false);
    owner_execute(&mut world.app, &world.issuer, proposal.clone()).unwrap();
    assert!(try_issue(&mut world, units(1)).is_err());
    owner_execute(
        &mut world.app,
        &world.issuer,
        ExecuteMsg::CancelMarketUpgrade {
            underlying: world.usdt.to_string(),
        },
    )
    .unwrap();
    issue(&mut world, LP, units(1));
    owner_execute(&mut world.app, &world.issuer, proposal).unwrap();
    repay(&mut world, units(1));
    world
        .app
        .migrate_contract(
            Addr::unchecked(OWNER),
            world.usdt.clone(),
            &cosmwasm_std::Empty {},
            target,
        )
        .unwrap();
    owner_execute(
        &mut world.app,
        &world.issuer,
        ExecuteMsg::AcceptMarketUpgrade {
            underlying: world.usdt.to_string(),
        },
    )
    .unwrap();
    assert_eq!(
        market(&world.app, &world.issuer, &world.usdt).underlying_code_id,
        Some(target)
    );
    issue(&mut world, LP, units(1));
    assert_eq!(
        outstanding(&world.app, &world.issuer, &world.usdt),
        units(4)
    );
}

#[test]
fn issuer_migration_rejects_foreign_invalid_and_unsupported_versions_without_writes() {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    for (name, version) in [
        ("other-contract", "1.0.0"),
        (CONTRACT_NAME, "bad"),
        (CONTRACT_NAME, "2.0.0"),
    ] {
        let mut deps = mock_dependencies();
        instantiate(
            deps.as_mut(),
            mock_env(),
            mock_info(OWNER, &[]),
            InstantiateMsg {
                owner: OWNER.to_string(),
            },
        )
        .unwrap();
        cw2::set_contract_version(deps.as_mut().storage, name, version).unwrap();
        let err = migrate(
            deps.as_mut(),
            mock_env(),
            MigrateMsg {
                legacy_pins: vec![],
            },
        )
        .unwrap_err();
        assert_eq!(err, ContractError::InvalidMigration {});
        let identity = cw2::get_contract_version(deps.as_ref().storage).unwrap();
        assert_eq!(identity.contract, name);
        assert_eq!(identity.version, version);
    }
}
#[cosmwasm_schema::cw_serde]
struct LegacyMarket {
    underlying: Addr,
    debt_token: Addr,
    debtor: Addr,
    decimals: u8,
    outstanding: Uint128,
}

#[cosmwasm_schema::cw_serde]
struct LegacyConfig {
    owner: Addr,
    debt_code_id: Option<u64>,
}

// Reproduce the persisted 1.0.0 layout, including missing policy fields.
fn restore_legacy_layout(
    deps: DepsMut,
    _env: Env,
    _msg: cosmwasm_std::Empty,
) -> StdResult<Response> {
    let config = CONFIG.load(deps.storage)?;
    cw_storage_plus::Item::<LegacyConfig>::new("config").save(
        deps.storage,
        &LegacyConfig {
            owner: config.owner,
            debt_code_id: config.debt_code_id,
        },
    )?;
    let markets = MARKETS
        .range(deps.storage, None, None, Order::Ascending)
        .collect::<StdResult<Vec<_>>>()?;
    for (addr, market) in markets {
        cw_storage_plus::Map::<&Addr, LegacyMarket>::new("markets").save(
            deps.storage,
            &addr,
            &LegacyMarket {
                underlying: market.underlying,
                debt_token: market.debt_token,
                debtor: market.debtor,
                decimals: market.decimals,
                outstanding: market.outstanding,
            },
        )?;
    }
    cw2::set_contract_version(deps.storage, CONTRACT_NAME, "1.0.0")?;
    Ok(Response::new())
}

#[test]
fn legacy_market_migration_requires_explicit_matching_pins_and_preserves_outstanding() {
    let mut world = world();
    issue(&mut world, LP, units(3));
    let pins = market(&world.app, &world.issuer, &world.usdt).pins.unwrap();
    let legacy_code = world.app.store_code(Box::new(
        ContractWrapper::new(execute, instantiate, query).with_migrate(restore_legacy_layout),
    ));
    world
        .app
        .migrate_contract(
            Addr::unchecked(OWNER),
            world.issuer.clone(),
            &cosmwasm_std::Empty {},
            legacy_code,
        )
        .unwrap();
    assert!(market(&world.app, &world.issuer, &world.usdt)
        .pins
        .is_none());
    let target = issuer_code(&mut world.app);
    assert!(world
        .app
        .migrate_contract(
            Addr::unchecked(OWNER),
            world.issuer.clone(),
            &MigrateMsg {
                legacy_pins: vec![]
            },
            target
        )
        .is_err());
    assert!(market(&world.app, &world.issuer, &world.usdt)
        .pins
        .is_none());
    let mut wrong = pins.clone();
    wrong.underlying_code_id = 999;
    assert!(world
        .app
        .migrate_contract(
            Addr::unchecked(OWNER),
            world.issuer.clone(),
            &MigrateMsg {
                legacy_pins: vec![LegacyMarketPin {
                    underlying: world.usdt.to_string(),
                    pins: wrong
                }],
            },
            target
        )
        .is_err());
    assert!(market(&world.app, &world.issuer, &world.usdt)
        .pins
        .is_none());
    world
        .app
        .migrate_contract(
            Addr::unchecked(OWNER),
            world.issuer.clone(),
            &MigrateMsg {
                legacy_pins: vec![LegacyMarketPin {
                    underlying: world.usdt.to_string(),
                    pins: pins.clone(),
                }],
            },
            target,
        )
        .unwrap();
    let state = market(&world.app, &world.issuer, &world.usdt);
    assert_eq!(state.pins, Some(pins));
    assert_eq!(state.outstanding, units(3));
    assert_eq!(balance(&world.app, &world.debt, CMM), units(3));
    assert_eq!(balance(&world.app, &world.usdt, LP), units(3));
    assert_eq!(state.limits, None);
    assert!(try_issue(&mut world, units(1)).is_err());
    repay(&mut world, units(1));
    configure(
        &mut world.app,
        &world.issuer,
        &world.usdt,
        generous_limits(),
        false,
    );
    issue(&mut world, LP, units(1));
    assert_eq!(
        outstanding(&world.app, &world.issuer, &world.usdt),
        units(3)
    );
}

#[cosmwasm_schema::cw_serde]
enum UnderlyingTestQuery {
    TokenInfo {},
    Minter {},
    Minters {
        start_after: Option<String>,
        limit: Option<u32>,
    },
}

#[test]
fn multi_minter_inventory_is_complete_bounded_and_pins_include_cap_and_sorted_minters() {
    use cosmwasm_std::testing::{mock_dependencies, mock_env};
    use cosmwasm_std::{ContractResult, SystemResult};
    for count in [2usize, 30, 31] {
        let mut deps = mock_dependencies();
        let env = mock_env();
        let issuer = env.contract.address.clone();
        let mut minters: Vec<String> = (0..count).map(|i| format!("terra1minter{i:03}")).collect();
        minters[0] = issuer.to_string();
        minters.sort();
        let expected = minters.clone();
        deps.querier.update_wasm(move |request| {
            let data = match request {
                WasmQuery::ContractInfo { .. } => {
                    let mut info = ContractInfoResponse::default();
                    info.code_id = 7;
                    to_json_binary(&info).unwrap()
                }
                WasmQuery::Smart { msg, .. } => {
                    match from_json::<UnderlyingTestQuery>(msg).unwrap() {
                        UnderlyingTestQuery::TokenInfo {} => to_json_binary(&TokenInfoResponse {
                            name: "USDT".to_string(),
                            symbol: "USDT".to_string(),
                            decimals: 18,
                            total_supply: Uint128::zero(),
                        })
                        .unwrap(),
                        UnderlyingTestQuery::Minter {} => to_json_binary(&MinterResponse {
                            minter: OWNER.to_string(),
                            cap: Some(units(100)),
                        })
                        .unwrap(),
                        UnderlyingTestQuery::Minters { start_after, limit } => {
                            let values: Vec<String> = minters
                                .iter()
                                .filter(|a| start_after.as_ref().is_none_or(|start| *a > start))
                                .take(limit.unwrap_or(10).min(30) as usize)
                                .cloned()
                                .collect();
                            to_json_binary(&MintersProbe { minters: values }).unwrap()
                        }
                    }
                }
                _ => panic!("unexpected request"),
            };
            SystemResult::Ok(ContractResult::Ok(data))
        });
        let result = inspect_underlying(
            deps.as_ref(),
            &env,
            &Addr::unchecked("terra1underlying"),
            5,
            7,
            true,
        );
        if count > 30 {
            assert_eq!(
                result.unwrap_err(),
                ContractError::UnderlyingMinterMismatch {}
            );
        } else {
            let pins: MarketPins = result.unwrap();
            assert_eq!(pins.minter, Addr::unchecked(OWNER));
            assert_eq!(pins.minter_cap, Some(units(100)));
            assert_eq!(
                pins.additional_minters,
                expected
                    .into_iter()
                    .map(Addr::unchecked)
                    .collect::<Vec<_>>()
            );
            assert!(pins.additional_minters.contains(&issuer));
        }
    }
}

fn reject_receipt_burn(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: bridge_debt_token::msg::ExecuteMsg,
) -> Result<Response, bridge_debt_token::error::ContractError> {
    if matches!(msg, bridge_debt_token::msg::ExecuteMsg::BurnFrom { .. }) {
        return Err(cosmwasm_std::StdError::generic_err("test receipt burn failure").into());
    }
    bridge_debt_token::contract::execute(deps, env, info, msg)
}

fn repayment_snapshot(world: &World) -> (MarketResponse, Uint128, Uint128, [Uint128; 6]) {
    (
        market(&world.app, &world.issuer, &world.usdt),
        supply(&world.app, &world.usdt),
        supply(&world.app, &world.debt),
        [
            balance(&world.app, &world.usdt, LP),
            balance(&world.app, &world.usdt, world.issuer.as_str()),
            balance(&world.app, &world.usdt, CMM),
            balance(&world.app, &world.debt, LP),
            balance(&world.app, &world.debt, world.issuer.as_str()),
            balance(&world.app, &world.debt, CMM),
        ],
    )
}

#[test]
fn second_leg_receipt_burn_failure_rolls_back_repayment_and_pending_upgrade_accounting() {
    let mut world = world();
    issue(&mut world, LP, units(5));
    // A pre-existing unsolicited balance must also survive the failed repayment.
    world
        .app
        .execute_contract(
            Addr::unchecked(LP),
            world.usdt.clone(),
            &Cw20ExecuteMsgBase::Transfer {
                recipient: world.issuer.to_string(),
                amount: units(1),
            },
            &[],
        )
        .unwrap();
    let rejecting_code = world.app.store_code(Box::new(
        ContractWrapper::new(
            reject_receipt_burn,
            bridge_debt_token::contract::instantiate,
            bridge_debt_token::contract::query,
        )
        .with_migrate(bridge_debt_token::contract::migrate),
    ));
    let proposal = ExecuteMsg::ProposeMarketUpgrade {
        underlying: world.usdt.to_string(),
        debt_code_id: rejecting_code,
        underlying_code_id: world.cw20_code_id,
    };
    owner_execute(&mut world.app, &world.issuer, proposal.clone()).unwrap();
    world
        .app
        .migrate_contract(
            Addr::unchecked(OWNER),
            world.debt.clone(),
            &bridge_debt_token::msg::MigrateMsg {},
            rejecting_code,
        )
        .unwrap();
    owner_execute(
        &mut world.app,
        &world.issuer,
        ExecuteMsg::AcceptMarketUpgrade {
            underlying: world.usdt.to_string(),
        },
    )
    .unwrap();
    issue(&mut world, LP, units(1));
    // Exercise restoration of the pending proposal's supply snapshot too.
    owner_execute(&mut world.app, &world.issuer, proposal).unwrap();
    let before = repayment_snapshot(&world);
    assert!(before.0.pending_upgrade.is_some());
    let err = world
        .app
        .execute_contract(
            Addr::unchecked(LP),
            world.usdt.clone(),
            &Cw20ExecuteMsgBase::Send {
                contract: world.issuer.to_string(),
                amount: units(2),
                msg: to_json_binary(&Cw20HookMsg::Repay {}).unwrap(),
            },
            &[],
        )
        .unwrap_err();
    assert!(shows(&err, "test receipt burn failure"), "{err:?}");
    assert_eq!(repayment_snapshot(&world), before);
    assert_eq!(before.0.outstanding, units(6));
    assert_eq!(before.0.epoch_issued, units(6));
    assert_eq!(before.3[0], units(5));
    assert_eq!(before.3[1], units(1));
    assert_eq!(before.3[5], units(6));
}

#[test]
fn zero_and_overflow_issuance_preserve_token_balances_and_quota() {
    let mut world = world();
    let empty = repayment_snapshot(&world);
    let err = try_issue(&mut world, Uint128::zero()).unwrap_err();
    assert!(shows(&err, "greater than zero"), "{err:?}");
    assert_eq!(repayment_snapshot(&world), empty);
    issue(&mut world, LP, Uint128::MAX);
    let before = repayment_snapshot(&world);
    assert_eq!(before.0.outstanding, Uint128::MAX);
    assert_eq!(before.0.epoch_issued, Uint128::MAX);
    let err = try_issue(&mut world, Uint128::new(1)).unwrap_err();
    assert!(shows(&err, "Amount overflow"), "{err:?}");
    assert_eq!(repayment_snapshot(&world), before);
}

#[test]
fn issuer_migration_rejects_malformed_config_limits_aliases_and_receipt_index_without_writes() {
    use cosmwasm_std::testing::{mock_dependencies, mock_env, mock_info};
    use cosmwasm_std::Storage;

    for case in 0..10 {
        let mut deps = mock_dependencies();
        let env = mock_env();
        instantiate(
            deps.as_mut(),
            env.clone(),
            mock_info(OWNER, &[]),
            InstantiateMsg {
                owner: OWNER.to_string(),
            },
        )
        .unwrap();
        cw2::set_contract_version(deps.as_mut().storage, CONTRACT_NAME, "1.0.0").unwrap();
        if case < 3 {
            let mut config = CONFIG.load(deps.as_ref().storage).unwrap();
            match case {
                0 => config.owner = Addr::unchecked("x"),
                1 => config.pending_owner = Some(Addr::unchecked("x")),
                2 => config.debt_code_id = Some(0),
                _ => unreachable!(),
            }
            CONFIG.save(deps.as_mut().storage, &config).unwrap();
        } else if case == 3 {
            DEBT_TOKENS
                .save(
                    deps.as_mut().storage,
                    &Addr::unchecked("terra1orphanreceipt"),
                    &cosmwasm_std::Empty {},
                )
                .unwrap();
        } else {
            let mut stored = Market {
                underlying: Addr::unchecked("terra1underlying"),
                debt_token: Addr::unchecked("terra1receipt"),
                debtor: Addr::unchecked(CMM),
                decimals: 18,
                outstanding: Uint128::zero(),
                pins: Some(MarketPins {
                    debt_code_id: 5,
                    underlying_code_id: 7,
                    multi_minter: false,
                    minter: env.contract.address.clone(),
                    minter_cap: None,
                    additional_minters: vec![],
                }),
                limits: Some(generous_limits()),
                paused: false,
                epoch: env.block.time.seconds() / 86_400,
                epoch_issued: Uint128::zero(),
                pending_upgrade: None,
            };
            match case {
                4 => stored.debt_token = stored.underlying.clone(),
                5 => stored.underlying = env.contract.address.clone(),
                6 => stored.debt_token = env.contract.address.clone(),
                7 => stored.limits.as_mut().unwrap().epoch_seconds = 0,
                8 => stored.limits.as_mut().unwrap().max_per_issue = Uint128::zero(),
                9 => stored.epoch += 1,
                _ => unreachable!(),
            }
            MARKETS
                .save(deps.as_mut().storage, &stored.underlying, &stored)
                .unwrap();
            DEBT_TOKENS
                .save(
                    deps.as_mut().storage,
                    &stored.debt_token,
                    &cosmwasm_std::Empty {},
                )
                .unwrap();
        }
        let before: Vec<_> = deps.storage.range(None, None, Order::Ascending).collect();
        let err = migrate(
            deps.as_mut(),
            env,
            MigrateMsg {
                legacy_pins: vec![],
            },
        )
        .unwrap_err();
        if (2..=6).contains(&case) {
            assert_eq!(err, ContractError::InvalidMigration {}, "case {case}");
        } else if case >= 7 {
            assert_eq!(err, ContractError::InvalidLimits {}, "case {case}");
        }
        let after: Vec<_> = deps.storage.range(None, None, Order::Ascending).collect();
        assert_eq!(after, before, "case {case}");
    }
}
