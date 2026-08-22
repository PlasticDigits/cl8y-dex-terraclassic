//! Layer A (token-only) + Layer B (DEX invariant) CW20 code-id audit harness (GitLab #589).
use cosmwasm_schema::cw_serde;
use cosmwasm_std::{
    to_json_binary, Addr, Binary, Coin, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError,
    StdResult, Uint128,
};
use cw20::{Cw20ExecuteMsg, Cw20ReceiveMsg, MinterResponse, TokenInfoResponse};
use cw_multi_test::{App, Contract, ContractWrapper, Executor};

use crate::helpers::{
    asset_info_token, create_cw20_token, cw20_mintable_contract, extract_pair_address,
    factory_contract, pair_contract, provide_liquidity, query_cw20_balance, query_pool,
    router_contract, setup_full_env, swap_a_to_b, swap_b_to_a, transfer_tokens, withdraw_liquidity,
    TestEnv,
};

use super::cw20_mutants::{
    default_honest_config, mutant_cw20_contract, ExecuteMsg as MutantExecuteMsg,
    InstantiateMsg as MutantInstantiateMsg, MutantConfig,
};

// ---------------------------------------------------------------------------
// Inline hook receivers for Send atomicity tests
// ---------------------------------------------------------------------------

#[cw_serde]
enum HookOkExec {
    Receive(Cw20ReceiveMsg),
}

fn hook_ok_instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> StdResult<Response> {
    Ok(Response::new())
}

fn hook_ok_execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: HookOkExec,
) -> StdResult<Response> {
    match msg {
        HookOkExec::Receive(receive) => {
            Ok(Response::new().add_attribute("received", receive.amount))
        }
    }
}

fn hook_ok_query(_deps: Deps, _env: Env, _msg: Empty) -> StdResult<Binary> {
    Ok(Binary::default())
}

fn hook_ok_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        hook_ok_execute,
        hook_ok_instantiate,
        hook_ok_query,
    ))
}

#[cw_serde]
enum HookFailExec {
    Receive(Cw20ReceiveMsg),
}

fn hook_fail_execute(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: HookFailExec,
) -> StdResult<Response> {
    match msg {
        HookFailExec::Receive(_) => Err(StdError::generic_err("hook always fails")),
    }
}

fn hook_fail_contract() -> Box<dyn Contract<Empty>> {
    Box::new(ContractWrapper::new(
        hook_fail_execute,
        hook_ok_instantiate,
        hook_ok_query,
    ))
}

fn instantiate_mutant(
    app: &mut App,
    code: u64,
    owner: &Addr,
    config: MutantConfig,
    initial: Uint128,
) -> Addr {
    app.instantiate_contract(
        code,
        owner.clone(),
        &MutantInstantiateMsg {
            name: "Mutant".to_string(),
            symbol: "MUT".to_string(),
            decimals: 6,
            initial_balances: vec![cw20::Cw20Coin {
                address: owner.to_string(),
                amount: initial,
            }],
            mint: None,
            config,
        },
        &[],
        "mutant",
        None,
    )
    .unwrap()
}

fn setup_env_with_mutant(app: &mut App, config: MutantConfig) -> TestEnv {
    let governance = Addr::unchecked("governance");
    let treasury = Addr::unchecked("treasury");
    let user = Addr::unchecked("user");

    let honest_cw20 = app.store_code(cw20_mintable_contract());
    let mutant_code = app.store_code(mutant_cw20_contract());
    let pair_code = app.store_code(pair_contract());
    let factory_code = app.store_code(factory_contract());
    let router_code = app.store_code(router_contract());

    let initial = Uint128::new(1_000_000_000_000);
    let token_b = create_cw20_token(app, honest_cw20, &user, "Token B", "TKNB", initial);
    let token_a = instantiate_mutant(app, mutant_code, &user, config, initial);

    let factory = app
        .instantiate_contract(
            factory_code,
            governance.clone(),
            &dex_common::factory::InstantiateMsg {
                governance: governance.to_string(),
                treasury: treasury.to_string(),
                default_fee_bps: 30,
                pair_code_id: pair_code,
                lp_token_code_id: honest_cw20,
                whitelisted_code_ids: vec![honest_cw20, mutant_code],
                default_limit_batch_max_rungs:
                    dex_common::pair::SUGGESTED_FACTORY_DEFAULT_LIMIT_BATCH_MAX_RUNGS,
                pair_creation_fee_uluna: Uint128::zero(),
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
                asset_infos: [asset_info_token(&token_a), asset_info_token(&token_b)],
            },
            &[],
        )
        .unwrap();
    let pair = extract_pair_address(&resp.events);

    let pair_info: dex_common::types::PairInfo = app
        .wrap()
        .query_wasm_smart(pair.to_string(), &dex_common::pair::QueryMsg::Pair {})
        .unwrap();
    let lp_token = pair_info.liquidity_token;

    let router = app
        .instantiate_contract(
            router_code,
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

fn mintable_with_minter(app: &mut App, owner: &Addr, initial: Uint128) -> Addr {
    let code = app.store_code(cw20_mintable_contract());
    app.instantiate_contract(
        code,
        owner.clone(),
        &cw20_mintable::msg::InstantiateMsg {
            name: "Mintable".to_string(),
            symbol: "MNT".to_string(),
            decimals: 6,
            initial_balances: vec![cw20::Cw20Coin {
                address: owner.to_string(),
                amount: initial,
            }],
            mint: Some(MinterResponse {
                minter: owner.to_string(),
                cap: None,
            }),
            marketing: None,
        },
        &[],
        "mintable-minter",
        None,
    )
    .unwrap()
}

fn assert_reserves_match_balances(app: &App, env: &TestEnv) {
    let pool = query_pool(app, &env.pair);
    let bal_a = query_cw20_balance(app, &env.token_a, &env.pair);
    let bal_b = query_cw20_balance(app, &env.token_b, &env.pair);
    assert_eq!(pool.assets[0].amount, bal_a, "token_a reserve vs balance");
    assert_eq!(pool.assets[1].amount, bal_b, "token_b reserve vs balance");
}

// --- Layer A honest ---

#[test]
fn layer_a_mintable_transfer_is_one_to_one() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let recipient = Addr::unchecked("recipient");
    let code = app.store_code(cw20_mintable_contract());
    let initial = Uint128::new(1_000_000);
    let token = create_cw20_token(&mut app, code, &owner, "Honest", "HON", initial);
    let amount = Uint128::new(50_000);

    let sender_before = query_cw20_balance(&app, &token, &owner);
    let rcpt_before = query_cw20_balance(&app, &token, &recipient);

    transfer_tokens(&mut app, &token, &owner, &recipient, amount);

    let sender_after = query_cw20_balance(&app, &token, &owner);
    let rcpt_after = query_cw20_balance(&app, &token, &recipient);

    assert_eq!(sender_before - sender_after, amount);
    assert_eq!(rcpt_after - rcpt_before, amount);
}

#[test]
fn layer_a_mintable_send_is_one_to_one_and_receiver_gets_amount() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let code = app.store_code(cw20_mintable_contract());
    let hook_code = app.store_code(hook_ok_contract());
    let receiver = app
        .instantiate_contract(hook_code, owner.clone(), &Empty {}, &[], "hook-ok", None)
        .unwrap();

    let initial = Uint128::new(1_000_000);
    let token = create_cw20_token(&mut app, code, &owner, "Honest", "HON", initial);
    let amount = Uint128::new(25_000);

    let sender_before = query_cw20_balance(&app, &token, &owner);
    let rcpt_before = query_cw20_balance(&app, &token, &receiver);

    app.execute_contract(
        owner.clone(),
        token.clone(),
        &Cw20ExecuteMsg::Send {
            contract: receiver.to_string(),
            amount,
            msg: Binary::default(),
        },
        &[],
    )
    .unwrap();

    assert_eq!(
        query_cw20_balance(&app, &token, &owner),
        sender_before - amount
    );
    assert_eq!(
        query_cw20_balance(&app, &token, &receiver),
        rcpt_before + amount
    );
}

#[test]
fn layer_a_mintable_receiver_revert_rolls_back() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let code = app.store_code(cw20_mintable_contract());
    let hook_code = app.store_code(hook_fail_contract());
    let receiver = app
        .instantiate_contract(hook_code, owner.clone(), &Empty {}, &[], "hook-fail", None)
        .unwrap();

    let initial = Uint128::new(1_000_000);
    let token = create_cw20_token(&mut app, code, &owner, "Honest", "HON", initial);
    let amount = Uint128::new(10_000);

    let sender_before = query_cw20_balance(&app, &token, &owner);
    let rcpt_before = query_cw20_balance(&app, &token, &receiver);

    let err = app
        .execute_contract(
            owner.clone(),
            token.clone(),
            &Cw20ExecuteMsg::Send {
                contract: receiver.to_string(),
                amount,
                msg: Binary::default(),
            },
            &[],
        )
        .unwrap_err();

    assert!(err.root_cause().to_string().contains("hook always fails"));
    assert_eq!(query_cw20_balance(&app, &token, &owner), sender_before);
    assert_eq!(query_cw20_balance(&app, &token, &receiver), rcpt_before);
}

#[test]
fn layer_a_mintable_zero_amount_deterministic() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let recipient = Addr::unchecked("recipient");
    let code = app.store_code(cw20_mintable_contract());
    let token = create_cw20_token(&mut app, code, &owner, "Honest", "HON", Uint128::new(1_000));

    let before_owner = query_cw20_balance(&app, &token, &owner);
    let before_rcpt = query_cw20_balance(&app, &token, &recipient);

    let result = app.execute_contract(
        owner.clone(),
        token.clone(),
        &Cw20ExecuteMsg::Transfer {
            recipient: recipient.to_string(),
            amount: Uint128::zero(),
        },
        &[],
    );

    match result {
        Ok(_) => {
            assert_eq!(query_cw20_balance(&app, &token, &owner), before_owner);
            assert_eq!(query_cw20_balance(&app, &token, &recipient), before_rcpt);
        }
        Err(_) => {
            assert_eq!(query_cw20_balance(&app, &token, &owner), before_owner);
            assert_eq!(query_cw20_balance(&app, &token, &recipient), before_rcpt);
        }
    }
}

#[test]
fn layer_a_mintable_self_transfer_net_zero() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let code = app.store_code(cw20_mintable_contract());
    let initial = Uint128::new(500_000);
    let token = create_cw20_token(&mut app, code, &owner, "Honest", "HON", initial);
    let amount = Uint128::new(100_000);

    let before = query_cw20_balance(&app, &token, &owner);
    transfer_tokens(&mut app, &token, &owner, &owner, amount);
    assert_eq!(query_cw20_balance(&app, &token, &owner), before);
}

#[test]
fn layer_a_mintable_oversize_rejected() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let recipient = Addr::unchecked("recipient");
    let code = app.store_code(cw20_mintable_contract());
    let initial = Uint128::new(1_000);
    let token = create_cw20_token(&mut app, code, &owner, "Honest", "HON", initial);

    let err = app
        .execute_contract(
            owner.clone(),
            token.clone(),
            &Cw20ExecuteMsg::Transfer {
                recipient: recipient.to_string(),
                amount: initial + Uint128::one(),
            },
            &[],
        )
        .unwrap_err();

    assert!(!err.root_cause().to_string().is_empty());
    assert_eq!(query_cw20_balance(&app, &token, &owner), initial);
}

#[test]
fn layer_a_mintable_unauthorized_mint_rejected() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let attacker = Addr::unchecked("attacker");
    let token = mintable_with_minter(&mut app, &owner, Uint128::new(1_000_000));

    let err = app
        .execute_contract(
            attacker,
            token.clone(),
            &Cw20ExecuteMsg::Mint {
                recipient: owner.to_string(),
                amount: Uint128::new(1),
            },
            &[],
        )
        .unwrap_err();

    assert!(!err.root_cause().to_string().is_empty());
}

#[test]
fn layer_a_mintable_insufficient_allowance_rejected() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let spender = Addr::unchecked("spender");
    let recipient = Addr::unchecked("recipient");
    let code = app.store_code(cw20_mintable_contract());
    let token = create_cw20_token(
        &mut app,
        code,
        &owner,
        "Honest",
        "HON",
        Uint128::new(1_000_000),
    );

    let err = app
        .execute_contract(
            spender.clone(),
            token.clone(),
            &Cw20ExecuteMsg::TransferFrom {
                owner: owner.to_string(),
                recipient: recipient.to_string(),
                amount: Uint128::new(100),
            },
            &[],
        )
        .unwrap_err();

    assert!(!err.root_cause().to_string().is_empty());
}

#[test]
fn layer_a_mintable_idle_balance_stable() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let code = app.store_code(cw20_mintable_contract());
    let initial = Uint128::new(999_999);
    let token = create_cw20_token(&mut app, code, &owner, "Honest", "HON", initial);

    let ti_before: TokenInfoResponse = app
        .wrap()
        .query_wasm_smart(token.to_string(), &cw20::Cw20QueryMsg::TokenInfo {})
        .unwrap();

    app.update_block(|b| b.height += 1000);

    assert_eq!(query_cw20_balance(&app, &token, &owner), initial);
    let ti_after: TokenInfoResponse = app
        .wrap()
        .query_wasm_smart(token.to_string(), &cw20::Cw20QueryMsg::TokenInfo {})
        .unwrap();
    assert_eq!(ti_before.total_supply, ti_after.total_supply);
}

#[test]
fn layer_a_mintable_decimals_le_18() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let code = app.store_code(cw20_mintable_contract());
    let token = create_cw20_token(&mut app, code, &owner, "Honest", "HON", Uint128::new(1));

    let ti: TokenInfoResponse = app
        .wrap()
        .query_wasm_smart(token.to_string(), &cw20::Cw20QueryMsg::TokenInfo {})
        .unwrap();
    assert!(ti.decimals <= 18);
}

#[test]
fn layer_a_mintable_supply_conserved_after_ops() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let recipient = Addr::unchecked("recipient");
    let token = mintable_with_minter(&mut app, &owner, Uint128::new(1_000_000));

    transfer_tokens(&mut app, &token, &owner, &recipient, Uint128::new(100_000));

    app.execute_contract(
        recipient.clone(),
        token.clone(),
        &Cw20ExecuteMsg::Burn {
            amount: Uint128::new(10_000),
        },
        &[],
    )
    .unwrap();

    app.execute_contract(
        owner.clone(),
        token.clone(),
        &Cw20ExecuteMsg::Mint {
            recipient: owner.to_string(),
            amount: Uint128::new(5_000),
        },
        &[],
    )
    .unwrap();

    let owner_bal = query_cw20_balance(&app, &token, &owner);
    let rcpt_bal = query_cw20_balance(&app, &token, &recipient);
    let ti: TokenInfoResponse = app
        .wrap()
        .query_wasm_smart(token.to_string(), &cw20::Cw20QueryMsg::TokenInfo {})
        .unwrap();

    assert_eq!(owner_bal + rcpt_bal, ti.total_supply);
}

// --- Layer A mutant detectors (known-bad MUST fail 1:1) ---

#[test]
fn mutant_a1_fot_breaks_one_to_one() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let recipient = Addr::unchecked("recipient");
    let code = app.store_code(mutant_cw20_contract());
    let mut config = default_honest_config();
    config.fee_bps = 100;
    let token = instantiate_mutant(&mut app, code, &owner, config, Uint128::new(1_000_000));
    let amount = Uint128::new(10_000);

    transfer_tokens(&mut app, &token, &owner, &recipient, amount);
    let received = query_cw20_balance(&app, &token, &recipient);
    assert!(
        received < amount,
        "FoT must credit less than declared amount"
    );
}

#[test]
fn mutant_d5_send_taxed_transfer_honest() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let recipient = Addr::unchecked("recipient");
    let code = app.store_code(mutant_cw20_contract());
    let hook_code = app.store_code(hook_ok_contract());
    let receiver = app
        .instantiate_contract(hook_code, owner.clone(), &Empty {}, &[], "hook-ok", None)
        .unwrap();

    let mut config = default_honest_config();
    config.fee_bps = 100;
    config.fee_on_dex_path_only = true;
    let token = instantiate_mutant(&mut app, code, &owner, config, Uint128::new(1_000_000));
    let amount = Uint128::new(10_000);

    transfer_tokens(&mut app, &token, &owner, &recipient, amount);
    assert_eq!(query_cw20_balance(&app, &token, &recipient), amount);

    app.execute_contract(
        owner.clone(),
        token.clone(),
        &Cw20ExecuteMsg::Send {
            contract: receiver.to_string(),
            amount,
            msg: Binary::default(),
        },
        &[],
    )
    .unwrap();
    let hook_bal = query_cw20_balance(&app, &token, &receiver);
    assert!(hook_bal < amount, "Send on dex path must be taxed");
}

#[test]
fn mutant_a3_rebase_idle_changes_balance() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let code = app.store_code(mutant_cw20_contract());
    let mut config = default_honest_config();
    config.rebase_bps_per_op = 10;
    let token = instantiate_mutant(&mut app, code, &owner, config, Uint128::new(1_000_000));

    let before = query_cw20_balance(&app, &token, &owner);
    app.update_block(|b| b.height += 50);
    let after = query_cw20_balance(&app, &token, &owner);
    assert_ne!(before, after, "idle rebase must change displayed balance");
}

#[test]
fn mutant_a8_backdoor_skips_allowance() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let backdoor = Addr::unchecked("backdoor");
    let recipient = Addr::unchecked("recipient");
    let code = app.store_code(mutant_cw20_contract());
    let mut config = default_honest_config();
    config.allowance_backdoor = Some(backdoor.to_string());
    let token = instantiate_mutant(&mut app, code, &owner, config, Uint128::new(1_000_000));

    app.execute_contract(
        backdoor.clone(),
        token.clone(),
        &MutantExecuteMsg::TransferFrom {
            owner: owner.to_string(),
            recipient: recipient.to_string(),
            amount: Uint128::new(1_000),
        },
        &[],
    )
    .unwrap();

    assert_eq!(
        query_cw20_balance(&app, &token, &recipient),
        Uint128::new(1_000)
    );
}

#[test]
fn mutant_a9_block_recipient_reverts() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let blocked = Addr::unchecked("blocked");
    let code = app.store_code(mutant_cw20_contract());
    let mut config = default_honest_config();
    config.block_recipient = Some(blocked.to_string());
    let token = instantiate_mutant(&mut app, code, &owner, config, Uint128::new(1_000_000));

    let err = app
        .execute_contract(
            owner.clone(),
            token.clone(),
            &MutantExecuteMsg::Transfer {
                recipient: blocked.to_string(),
                amount: Uint128::new(100),
            },
            &[],
        )
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("recipient blocked"));
}

#[test]
fn mutant_a10_pause_stops_transfer() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let recipient = Addr::unchecked("recipient");
    let code = app.store_code(mutant_cw20_contract());
    let mut config = default_honest_config();
    config.paused = true;
    let token = instantiate_mutant(&mut app, code, &owner, config, Uint128::new(1_000_000));

    let err = app
        .execute_contract(
            owner.clone(),
            token.clone(),
            &MutantExecuteMsg::Transfer {
                recipient: recipient.to_string(),
                amount: Uint128::new(100),
            },
            &[],
        )
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("paused"));
}

#[test]
fn mutant_a12_hidden_mint() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let hidden = Addr::unchecked("hidden");
    let code = app.store_code(mutant_cw20_contract());
    let mut config = default_honest_config();
    config.hidden_minter = Some(hidden.to_string());
    let token = instantiate_mutant(&mut app, code, &owner, config, Uint128::new(1_000));

    app.execute_contract(
        hidden.clone(),
        token.clone(),
        &MutantExecuteMsg::Mint {
            recipient: hidden.to_string(),
            amount: Uint128::new(500),
        },
        &[],
    )
    .unwrap();

    assert_eq!(query_cw20_balance(&app, &token, &hidden), Uint128::new(500));
}

#[test]
fn mutant_a16_lie_balance_disagrees_with_transfer() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let recipient = Addr::unchecked("recipient");
    let code = app.store_code(mutant_cw20_contract());
    let mut config = default_honest_config();
    config.lie_balance = true;
    let token = instantiate_mutant(&mut app, code, &owner, config, Uint128::new(1_000_000));
    let amount = Uint128::new(5_000);

    transfer_tokens(&mut app, &token, &owner, &recipient, amount);
    let queried = query_cw20_balance(&app, &token, &recipient);
    assert_eq!(queried, amount + Uint128::one());
}

#[test]
fn mutant_d1_tax_activates_after_height() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let recipient = Addr::unchecked("recipient");
    let code = app.store_code(mutant_cw20_contract());
    let mut config = default_honest_config();
    config.fee_bps = 100;
    config.tax_from_height = Some(100);
    let token = instantiate_mutant(&mut app, code, &owner, config, Uint128::new(1_000_000));
    app.update_block(|b| b.height = 1);
    let amount = Uint128::new(10_000);

    transfer_tokens(&mut app, &token, &owner, &recipient, amount);
    assert_eq!(query_cw20_balance(&app, &token, &recipient), amount);

    app.update_block(|b| b.height = 100);
    let owner2 = Addr::unchecked("owner2");
    transfer_tokens(&mut app, &token, &owner, &owner2, amount);
    let got = query_cw20_balance(&app, &token, &owner2);
    assert!(got < amount, "tax must apply at/after activation height");
}

#[test]
fn mutant_d2_magnitude_tax() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let dust_rcpt = Addr::unchecked("dust");
    let large_rcpt = Addr::unchecked("large");
    let code = app.store_code(mutant_cw20_contract());
    let mut config = default_honest_config();
    config.fee_bps = 100;
    config.magnitude_tax_threshold = Some(Uint128::new(1_000));
    let token = instantiate_mutant(&mut app, code, &owner, config, Uint128::new(1_000_000));

    transfer_tokens(&mut app, &token, &owner, &dust_rcpt, Uint128::new(100));
    assert_eq!(
        query_cw20_balance(&app, &token, &dust_rcpt),
        Uint128::new(100)
    );

    transfer_tokens(&mut app, &token, &owner, &large_rcpt, Uint128::new(10_000));
    assert!(
        query_cw20_balance(&app, &token, &large_rcpt) < Uint128::new(10_000),
        "large transfer must be taxed"
    );
}

#[test]
fn mutant_d11_cooldown_rejects_second_transfer_same_block() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let rcpt1 = Addr::unchecked("rcpt1");
    let rcpt2 = Addr::unchecked("rcpt2");
    let code = app.store_code(mutant_cw20_contract());
    let mut config = default_honest_config();
    config.cooldown_blocks = Some(1);
    let token = instantiate_mutant(&mut app, code, &owner, config, Uint128::new(1_000_000));

    transfer_tokens(&mut app, &token, &owner, &rcpt1, Uint128::new(100));
    let err = app
        .execute_contract(
            owner.clone(),
            token.clone(),
            &MutantExecuteMsg::Transfer {
                recipient: rcpt2.to_string(),
                amount: Uint128::new(100),
            },
            &[],
        )
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("transfer cooldown"));
}

#[test]
fn mutant_d12_payable_requires_funds() {
    let owner = Addr::unchecked("owner");
    let recipient = Addr::unchecked("recipient");
    let mut app = App::new(|router, _api, storage| {
        router
            .bank
            .init_balance(
                storage,
                &owner,
                vec![cosmwasm_std::Coin::new(1_000_000u128, "uluna")],
            )
            .unwrap();
    });
    let code = app.store_code(mutant_cw20_contract());
    let mut config = default_honest_config();
    config.require_native_funds = true;
    let token = instantiate_mutant(&mut app, code, &owner, config, Uint128::new(1_000_000));

    let err = app
        .execute_contract(
            owner.clone(),
            token.clone(),
            &MutantExecuteMsg::Transfer {
                recipient: recipient.to_string(),
                amount: Uint128::new(100),
            },
            &[],
        )
        .unwrap_err();
    assert!(err
        .root_cause()
        .to_string()
        .contains("native funds required"));

    app.execute_contract(
        owner.clone(),
        token.clone(),
        &MutantExecuteMsg::Transfer {
            recipient: recipient.to_string(),
            amount: Uint128::new(100),
        },
        &[Coin {
            denom: "uluna".to_string(),
            amount: Uint128::one(),
        }],
    )
    .unwrap();
}

#[test]
fn mutant_d10_ghost_dust_leaves_one() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let recipient = Addr::unchecked("recipient");
    let code = app.store_code(mutant_cw20_contract());
    let mut config = default_honest_config();
    config.ghost_dust = true;
    let initial = Uint128::new(1_000);
    let token = instantiate_mutant(&mut app, code, &owner, config, initial);

    app.execute_contract(
        owner.clone(),
        token.clone(),
        &MutantExecuteMsg::Transfer {
            recipient: recipient.to_string(),
            amount: initial,
        },
        &[],
    )
    .unwrap();

    assert_eq!(query_cw20_balance(&app, &token, &owner), Uint128::one());
    assert_eq!(
        query_cw20_balance(&app, &token, &recipient),
        initial - Uint128::one()
    );
}

#[test]
fn mutant_d13_set_decimals_mutates_tokeninfo() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let code = app.store_code(mutant_cw20_contract());
    let mut config = default_honest_config();
    config.mutable_decimals = true;
    let token = instantiate_mutant(&mut app, code, &owner, config, Uint128::new(1_000));

    app.execute_contract(
        owner.clone(),
        token.clone(),
        &MutantExecuteMsg::SetDecimals { decimals: 18 },
        &[],
    )
    .unwrap();

    let ti: TokenInfoResponse = app
        .wrap()
        .query_wasm_smart(token.to_string(), &cw20::Cw20QueryMsg::TokenInfo {})
        .unwrap();
    assert_eq!(ti.decimals, 18);
}

#[test]
fn mutant_d8_permissionless_pair_register_enables_directional_tax() {
    let mut app = App::default();
    let owner = Addr::unchecked("owner");
    let pair_addr = Addr::unchecked("fake-pair");
    let code = app.store_code(mutant_cw20_contract());
    let mut config = default_honest_config();
    config.fee_bps = 0;
    config.permissionless_pair_register = true;
    let token = instantiate_mutant(&mut app, code, &owner, config, Uint128::new(1_000_000));
    let amount = Uint128::new(10_000);

    transfer_tokens(&mut app, &token, &owner, &pair_addr, amount);
    assert_eq!(query_cw20_balance(&app, &token, &pair_addr), amount);

    app.execute_contract(
        owner.clone(),
        token.clone(),
        &MutantExecuteMsg::SetDirectionalFeeRecipient {
            addr: pair_addr.to_string(),
        },
        &[],
    )
    .unwrap();

    transfer_tokens(&mut app, &token, &owner, &pair_addr, amount);
    let pair_bal = query_cw20_balance(&app, &token, &pair_addr);
    assert!(
        pair_bal < amount * Uint128::new(2u128),
        "directional tax must reduce second transfer credit"
    );
}

// --- Layer B honest DEX invariants ---

#[test]
fn layer_b_p1_k_non_decreasing_honest() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );
    let before = query_pool(&app, &env.pair);
    let k_before = before.assets[0].amount.u128() * before.assets[1].amount.u128();
    swap_a_to_b(&mut app, &env, &env.user, Uint128::new(50_000));
    let after = query_pool(&app, &env.pair);
    let k_after = after.assets[0].amount.u128() * after.assets[1].amount.u128();
    assert!(k_after >= k_before, "k must not decrease on honest swap");
}

#[test]
fn layer_b_p2_reserves_match_cw20_balances_honest() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );
    assert_reserves_match_balances(&app, &env);

    swap_a_to_b(&mut app, &env, &env.user, Uint128::new(25_000));
    assert_reserves_match_balances(&app, &env);

    let lp = query_cw20_balance(&app, &env.lp_token, &env.user);
    withdraw_liquidity(&mut app, &env, &env.user, lp);
    assert_reserves_match_balances(&app, &env);
}

#[test]
fn layer_b_p3_donation_does_not_inflate_lp_shares() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let user2 = Addr::unchecked("user2");
    transfer_tokens(
        &mut app,
        &env.token_a,
        &env.user,
        &user2,
        Uint128::new(50_000_000),
    );
    transfer_tokens(
        &mut app,
        &env.token_b,
        &env.user,
        &user2,
        Uint128::new(50_000_000),
    );

    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );
    let pool_before = query_pool(&app, &env.pair);

    transfer_tokens(
        &mut app,
        &env.token_a,
        &env.user,
        &env.pair,
        Uint128::new(500_000),
    );
    let pool_donated = query_pool(&app, &env.pair);
    assert_eq!(pool_before.assets[0].amount, pool_donated.assets[0].amount);
    assert_eq!(pool_before.total_share, pool_donated.total_share);

    provide_liquidity(
        &mut app,
        &env,
        &user2,
        Uint128::new(500_000),
        Uint128::new(500_000),
    );
    let user2_lp = query_cw20_balance(&app, &env.lp_token, &user2);
    // Reserves (not donated balance) mint shares. Donation inflation would dilute this toward ~333k.
    assert!(
        user2_lp >= Uint128::new(400_000),
        "user2 LP shares must reflect reserves not donated balance, got {user2_lp}"
    );
}

#[test]
fn layer_b_b3_flash_provide_swap_withdraw_no_profit_honest() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    let attacker = Addr::unchecked("attacker");
    transfer_tokens(
        &mut app,
        &env.token_a,
        &env.user,
        &attacker,
        Uint128::new(100_000_000),
    );
    transfer_tokens(
        &mut app,
        &env.token_b,
        &env.user,
        &attacker,
        Uint128::new(100_000_000),
    );

    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(10_000_000),
        Uint128::new(10_000_000),
    );

    let a_before = query_cw20_balance(&app, &env.token_a, &attacker);
    let b_before = query_cw20_balance(&app, &env.token_b, &attacker);

    provide_liquidity(
        &mut app,
        &env,
        &attacker,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );
    swap_a_to_b(&mut app, &env, &attacker, Uint128::new(1_000));
    let lp = query_cw20_balance(&app, &env.lp_token, &attacker);
    withdraw_liquidity(&mut app, &env, &attacker, lp);

    let a_after = query_cw20_balance(&app, &env.token_a, &attacker);
    let b_after = query_cw20_balance(&app, &env.token_b, &attacker);
    assert!(a_after + b_after <= a_before + b_before);
}

#[test]
fn layer_b_b7_round_trip_swap_succeeds_honest() {
    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );
    swap_a_to_b(&mut app, &env, &env.user, Uint128::new(10_000));
    swap_b_to_a(&mut app, &env, &env.user, Uint128::new(5_000));
}

#[test]
fn layer_b_l1_limit_place_escrow_one_to_one() {
    use cosmwasm_std::Decimal;
    use dex_common::limit_placement::LimitOrderPlacementItem;
    use dex_common::pair::{Cw20HookMsg, LimitOrderSide};

    let mut app = App::default();
    let env = setup_full_env(&mut app);
    provide_liquidity(
        &mut app,
        &env,
        &env.user,
        Uint128::new(1_000_000),
        Uint128::new(1_000_000),
    );

    let amount = Uint128::new(5_000);
    let pair_b_before = query_cw20_balance(&app, &env.token_b, &env.pair);
    let user_before = query_cw20_balance(&app, &env.token_b, &env.user);
    let treasury_before = query_cw20_balance(&app, &env.token_b, &env.treasury);

    let msg = to_json_binary(&Cw20HookMsg::PlaceLimitOrderBatch {
        side: LimitOrderSide::Bid,
        orders: vec![LimitOrderPlacementItem {
            price: Decimal::one(),
            amount,
            max_adjust_steps: 32,
            expires_at: None,
            hint_after_order_id: None,
        }],
    })
    .unwrap();

    app.execute_contract(
        env.user.clone(),
        env.token_b.clone(),
        &Cw20ExecuteMsg::Send {
            contract: env.pair.to_string(),
            amount,
            msg,
        },
        &[],
    )
    .unwrap();

    let pair_b_after = query_cw20_balance(&app, &env.token_b, &env.pair);
    let user_after = query_cw20_balance(&app, &env.token_b, &env.user);
    let treasury_after = query_cw20_balance(&app, &env.token_b, &env.treasury);
    // Honest CW20: user debit == declared Send. Maker placement fee may leave the pair
    // to treasury; pair + treasury must still sum to the declared amount (L1 / no FoT).
    assert_eq!(user_before - user_after, amount);
    assert_eq!(
        (pair_b_after - pair_b_before) + (treasury_after - treasury_before),
        amount
    );
}

#[test]
fn layer_b_e1_create_pair_rejects_identical_assets() {
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

    assert!(!err.root_cause().to_string().is_empty());
}

// --- Layer B known-bad FoT ---

#[test]
fn layer_b_b1_fot_desyncs_reserves() {
    let mut app = App::default();
    let mut config = default_honest_config();
    config.fee_bps = 100;
    let env = setup_env_with_mutant(&mut app, config);
    let liq = Uint128::new(1_000_000);
    provide_liquidity(&mut app, &env, &env.user, liq, liq);

    let pool = query_pool(&app, &env.pair);
    let on_chain_a = query_cw20_balance(&app, &env.token_a, &env.pair);
    assert_eq!(pool.assets[0].amount, liq);
    assert!(
        on_chain_a < pool.assets[0].amount,
        "FoT token_a: reserves exceed actual pair balance"
    );
}
