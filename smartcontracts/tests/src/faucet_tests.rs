//! Soft-launch faucet contract tests (GitLab #473).
//!
//! Covers allowlist, fixed drip, global cooldown, pause, and admin auth.
//! Also verifies the faucet must be an authorized CW20 minter (F5).

use cosmwasm_std::{Addr, Uint128};
use cw20::{BalanceResponse, Cw20Coin, Cw20QueryMsg, MinterResponse};
use cw_multi_test::{App, Executor};

use crate::helpers::{cw20_mintable_contract, faucet_contract};

use cl8y_dex_faucet::msg::{
    ConfigResponse, CooldownResponse, ExecuteMsg, InstantiateMsg, QueryMsg,
};

const DRIP: u128 = 100_000_000;
const COOLDOWN: u64 = 300;

fn create_mintable_token(
    app: &mut App,
    code_id: u64,
    minter: &Addr,
    name: &str,
    symbol: &str,
) -> Addr {
    app.instantiate_contract(
        code_id,
        minter.clone(),
        &cw20_mintable::msg::InstantiateMsg {
            name: name.to_string(),
            symbol: symbol.to_string(),
            decimals: 6,
            initial_balances: vec![Cw20Coin {
                address: minter.to_string(),
                amount: Uint128::zero(),
            }],
            mint: Some(MinterResponse {
                minter: minter.to_string(),
                cap: None,
            }),
            marketing: None,
        },
        &[],
        name,
        None,
    )
    .unwrap()
}

fn setup_faucet_env(app: &mut App) -> (Addr, Addr, Addr, Addr, Addr) {
    let admin = Addr::unchecked("admin");
    let user = Addr::unchecked("user");
    let other = Addr::unchecked("other");

    let cw20_code = app.store_code(cw20_mintable_contract());
    let faucet_code = app.store_code(faucet_contract());

    let ember = create_mintable_token(app, cw20_code, &admin, "Ember", "EMBER");
    let coral = create_mintable_token(app, cw20_code, &admin, "Coral", "CORAL");

    let faucet = app
        .instantiate_contract(
            faucet_code,
            admin.clone(),
            &InstantiateMsg {
                admin: admin.to_string(),
                allowed_tokens: vec![ember.to_string(), coral.to_string()],
                drip_amount: Uint128::new(DRIP),
                cooldown_seconds: COOLDOWN,
            },
            &[],
            "faucet",
            None,
        )
        .unwrap();

    // F5: primary minter (admin) grants faucet AddMinter on allowlisted tokens.
    for token in [&ember, &coral] {
        app.execute_contract(
            admin.clone(),
            token.clone(),
            &cw20_mintable::msg::ExecuteMsg::AddMinter {
                minter: faucet.to_string(),
            },
            &[],
        )
        .unwrap();
    }

    (admin, user, other, faucet, ember)
}

fn balance(app: &App, token: &Addr, owner: &Addr) -> Uint128 {
    let resp: BalanceResponse = app
        .wrap()
        .query_wasm_smart(
            token,
            &Cw20QueryMsg::Balance {
                address: owner.to_string(),
            },
        )
        .unwrap();
    resp.balance
}

#[test]
fn drip_happy_path_mints_fixed_amount() {
    let mut app = App::default();
    let (_admin, user, _other, faucet, ember) = setup_faucet_env(&mut app);

    app.execute_contract(
        user.clone(),
        faucet.clone(),
        &ExecuteMsg::Drip {
            token: ember.to_string(),
        },
        &[],
    )
    .unwrap();

    assert_eq!(balance(&app, &ember, &user), Uint128::new(DRIP));

    let cooldown: CooldownResponse = app
        .wrap()
        .query_wasm_smart(
            faucet,
            &QueryMsg::Cooldown {
                address: user.to_string(),
            },
        )
        .unwrap();
    assert!(!cooldown.can_claim);
    assert!(cooldown.seconds_remaining > 0);
    assert!(cooldown.last_claim_at.is_some());
}

#[test]
fn cooldown_blocks_second_drip_until_time_advances() {
    let mut app = App::default();
    let (_admin, user, _other, faucet, ember) = setup_faucet_env(&mut app);

    app.execute_contract(
        user.clone(),
        faucet.clone(),
        &ExecuteMsg::Drip {
            token: ember.to_string(),
        },
        &[],
    )
    .unwrap();

    let err = app
        .execute_contract(
            user.clone(),
            faucet.clone(),
            &ExecuteMsg::Drip {
                token: ember.to_string(),
            },
            &[],
        )
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("Cooldown active"));

    // Global cooldown: switching token does not bypass.
    let config: ConfigResponse = app
        .wrap()
        .query_wasm_smart(&faucet, &QueryMsg::Config {})
        .unwrap();
    let coral = &config.allowed_tokens[1];
    let err = app
        .execute_contract(
            user.clone(),
            faucet.clone(),
            &ExecuteMsg::Drip {
                token: coral.to_string(),
            },
            &[],
        )
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("Cooldown active"));

    app.update_block(|b| {
        b.time = b.time.plus_seconds(COOLDOWN);
        b.height += 1;
    });

    app.execute_contract(
        user.clone(),
        faucet,
        &ExecuteMsg::Drip {
            token: ember.to_string(),
        },
        &[],
    )
    .unwrap();
    assert_eq!(balance(&app, &ember, &user), Uint128::new(DRIP * 2));
}

#[test]
fn non_allowlisted_token_rejected() {
    let mut app = App::default();
    let (admin, user, _other, faucet, _ember) = setup_faucet_env(&mut app);
    let cw20_code = app.store_code(cw20_mintable_contract());
    let quartz = create_mintable_token(&mut app, cw20_code, &admin, "Quartz", "QUARTZ");

    let err = app
        .execute_contract(
            user,
            faucet,
            &ExecuteMsg::Drip {
                token: quartz.to_string(),
            },
            &[],
        )
        .unwrap_err();
    assert!(err
        .root_cause()
        .to_string()
        .contains("not on the faucet allowlist"));
}

#[test]
fn pause_blocks_drip_unpause_restores() {
    let mut app = App::default();
    let (admin, user, _other, faucet, ember) = setup_faucet_env(&mut app);

    app.execute_contract(admin.clone(), faucet.clone(), &ExecuteMsg::Pause {}, &[])
        .unwrap();

    let err = app
        .execute_contract(
            user.clone(),
            faucet.clone(),
            &ExecuteMsg::Drip {
                token: ember.to_string(),
            },
            &[],
        )
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("paused"));

    let cooldown: CooldownResponse = app
        .wrap()
        .query_wasm_smart(
            &faucet,
            &QueryMsg::Cooldown {
                address: user.to_string(),
            },
        )
        .unwrap();
    assert!(!cooldown.can_claim);
    assert!(cooldown.paused);

    app.execute_contract(admin, faucet.clone(), &ExecuteMsg::Unpause {}, &[])
        .unwrap();

    app.execute_contract(
        user.clone(),
        faucet,
        &ExecuteMsg::Drip {
            token: ember.to_string(),
        },
        &[],
    )
    .unwrap();
    assert_eq!(balance(&app, &ember, &user), Uint128::new(DRIP));
}

#[test]
fn unauthorized_admin_msgs_rejected() {
    let mut app = App::default();
    let (_admin, user, _other, faucet, _ember) = setup_faucet_env(&mut app);

    let err = app
        .execute_contract(user.clone(), faucet.clone(), &ExecuteMsg::Pause {}, &[])
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("Unauthorized"));

    let err = app
        .execute_contract(
            user,
            faucet,
            &ExecuteMsg::UpdateConfig {
                drip_amount: Some(Uint128::new(1)),
                cooldown_seconds: None,
                admin: None,
            },
            &[],
        )
        .unwrap_err();
    assert!(err.root_cause().to_string().contains("Unauthorized"));
}

#[test]
fn drip_without_minter_grant_fails() {
    let mut app = App::default();
    let admin = Addr::unchecked("admin");
    let user = Addr::unchecked("user");
    let cw20_code = app.store_code(cw20_mintable_contract());
    let faucet_code = app.store_code(faucet_contract());
    let ember = create_mintable_token(&mut app, cw20_code, &admin, "Ember", "EMBER");

    let faucet = app
        .instantiate_contract(
            faucet_code,
            admin.clone(),
            &InstantiateMsg {
                admin: admin.to_string(),
                allowed_tokens: vec![ember.to_string()],
                drip_amount: Uint128::new(DRIP),
                cooldown_seconds: COOLDOWN,
            },
            &[],
            "faucet-no-minter",
            None,
        )
        .unwrap();

    // No AddMinter — Mint from faucet must fail.
    let err = app
        .execute_contract(
            user,
            faucet,
            &ExecuteMsg::Drip {
                token: ember.to_string(),
            },
            &[],
        )
        .unwrap_err();
    let msg = err.root_cause().to_string().to_lowercase();
    assert!(
        msg.contains("minter") || msg.contains("unauthorized") || msg.contains("cannot"),
        "expected minter-related error, got: {msg}"
    );
}

#[test]
fn config_query_lists_allowlist() {
    let mut app = App::default();
    let (admin, _user, _other, faucet, ember) = setup_faucet_env(&mut app);

    let config: ConfigResponse = app
        .wrap()
        .query_wasm_smart(&faucet, &QueryMsg::Config {})
        .unwrap();
    assert_eq!(config.admin, admin);
    assert_eq!(config.drip_amount, Uint128::new(DRIP));
    assert_eq!(config.cooldown_seconds, COOLDOWN);
    assert!(!config.paused);
    assert_eq!(config.allowed_tokens.len(), 2);
    assert!(config.allowed_tokens.contains(&ember));

    // Primary minter remains admin; faucet is additional minter.
    let minter: MinterResponse = app
        .wrap()
        .query_wasm_smart(&ember, &Cw20QueryMsg::Minter {})
        .unwrap();
    assert_eq!(minter.minter, admin.to_string());
}

#[test]
fn drip_always_mints_to_sender_not_arbitrary_recipient() {
    let mut app = App::default();
    let (_admin, user, other, faucet, ember) = setup_faucet_env(&mut app);

    app.execute_contract(
        user.clone(),
        faucet,
        &ExecuteMsg::Drip {
            token: ember.to_string(),
        },
        &[],
    )
    .unwrap();

    assert_eq!(balance(&app, &ember, &user), Uint128::new(DRIP));
    assert_eq!(balance(&app, &ember, &other), Uint128::zero());
}
