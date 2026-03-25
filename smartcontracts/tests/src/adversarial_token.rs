//! Minimal CW20 (adversarial modes) + hook-spoofer for security tests.
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Empty, Env, MessageInfo, Response, StdError, StdResult,
    Uint128, WasmMsg,
};
use cw20::{BalanceResponse, Cw20Coin, Cw20ReceiveMsg, MinterResponse, TokenInfoResponse};
use cw_storage_plus::{Item, Map};
use dex_common::hook::HookExecuteMsg;
use dex_common::pair::ExecuteMsg as PairExecuteMsg;
use dex_common::types::{Asset, AssetInfo};

#[cw_serde]
pub enum AdversarialMode {
    Honest,
    /// Sender debits `amount`; recipient credits `amount * (10000 - fee_bps) / 10000`.
    FeeOnTransfer {
        fee_bps: u128,
    },
}

#[cw_serde]
pub struct InstantiateMsg {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub initial_balances: Vec<Cw20Coin>,
    pub mint: Option<MinterResponse>,
    pub mode: AdversarialMode,
}

#[cw_serde]
pub enum ExecuteMsg {
    Transfer {
        recipient: String,
        amount: Uint128,
    },
    Burn {
        amount: Uint128,
    },
    Send {
        contract: String,
        amount: Uint128,
        msg: Binary,
    },
    Mint {
        recipient: String,
        amount: Uint128,
    },
    IncreaseAllowance {
        spender: String,
        amount: Uint128,
        expires: Option<cw20::Expiration>,
    },
    TransferFrom {
        owner: String,
        recipient: String,
        amount: Uint128,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum QueryMsg {
    #[returns(BalanceResponse)]
    Balance { address: String },
    #[returns(TokenInfoResponse)]
    TokenInfo {},
    #[returns(cw20::AllowanceResponse)]
    Allowance { owner: String, spender: String },
}

const BALANCES: Map<&str, Uint128> = Map::new("b");
const ALLOWANCES: Map<(&str, &str), Uint128> = Map::new("a");
const TOKEN_INFO: Item<TokenInfoResponse> = Item::new("t");
const MODE: Item<AdversarialMode> = Item::new("m");
const MINTER: Item<Option<String>> = Item::new("x");
const TOTAL_SUPPLY: Item<Uint128> = Item::new("s");

fn credited_amount(mode: &AdversarialMode, amount: Uint128) -> StdResult<Uint128> {
    match mode {
        AdversarialMode::Honest => Ok(amount),
        AdversarialMode::FeeOnTransfer { fee_bps } => {
            if *fee_bps >= 10_000 {
                return Err(StdError::generic_err("fee_bps must be < 10000"));
            }
            amount
                .checked_mul(Uint128::new(10_000 - fee_bps))
                .map_err(StdError::from)?
                .checked_div(Uint128::new(10_000))
                .map_err(StdError::from)
        }
    }
}

pub fn adversarial_instantiate(
    deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    msg: InstantiateMsg,
) -> StdResult<Response> {
    let mut supply = Uint128::zero();
    for coin in &msg.initial_balances {
        let addr = deps.api.addr_validate(&coin.address)?;
        supply = supply.checked_add(coin.amount)?;
        BALANCES.save(deps.storage, addr.as_str(), &coin.amount)?;
    }
    TOKEN_INFO.save(
        deps.storage,
        &TokenInfoResponse {
            name: msg.name,
            symbol: msg.symbol,
            decimals: msg.decimals,
            total_supply: supply,
        },
    )?;
    MODE.save(deps.storage, &msg.mode)?;
    MINTER.save(deps.storage, &msg.mint.as_ref().map(|m| m.minter.clone()))?;
    TOTAL_SUPPLY.save(deps.storage, &supply)?;
    Ok(Response::default())
}

pub fn adversarial_execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    let mode = MODE.load(deps.storage)?;
    match msg {
        ExecuteMsg::Transfer { recipient, amount } => {
            let rcpt = deps.api.addr_validate(&recipient)?;
            if amount.is_zero() {
                return Err(StdError::generic_err("zero amount"));
            }
            let mut bal = BALANCES
                .may_load(deps.storage, info.sender.as_str())?
                .unwrap_or_default();
            bal = bal.checked_sub(amount)?;
            BALANCES.save(deps.storage, info.sender.as_str(), &bal)?;

            let credit = credited_amount(&mode, amount)?;
            let mut rbal = BALANCES
                .may_load(deps.storage, rcpt.as_str())?
                .unwrap_or_default();
            rbal = rbal.checked_add(credit)?;
            BALANCES.save(deps.storage, rcpt.as_str(), &rbal)?;
            Ok(Response::default())
        }
        ExecuteMsg::Burn { amount } => {
            if amount.is_zero() {
                return Err(StdError::generic_err("zero amount"));
            }
            let mut bal = BALANCES
                .may_load(deps.storage, info.sender.as_str())?
                .unwrap_or_default();
            bal = bal.checked_sub(amount)?;
            BALANCES.save(deps.storage, info.sender.as_str(), &bal)?;
            let mut ti = TOKEN_INFO.load(deps.storage)?;
            ti.total_supply = ti.total_supply.checked_sub(amount)?;
            TOKEN_INFO.save(deps.storage, &ti)?;
            let mut ts = TOTAL_SUPPLY.load(deps.storage)?;
            ts = ts.checked_sub(amount)?;
            TOTAL_SUPPLY.save(deps.storage, &ts)?;
            Ok(Response::default())
        }
        ExecuteMsg::Send {
            contract,
            amount,
            msg: hook,
        } => {
            let contract_addr = deps.api.addr_validate(&contract)?;
            if amount.is_zero() {
                return Err(StdError::generic_err("zero amount"));
            }
            let mut bal = BALANCES
                .may_load(deps.storage, info.sender.as_str())?
                .unwrap_or_default();
            bal = bal.checked_sub(amount)?;
            BALANCES.save(deps.storage, info.sender.as_str(), &bal)?;

            let credit = credited_amount(&mode, amount)?;
            let mut cbal = BALANCES
                .may_load(deps.storage, contract_addr.as_str())?
                .unwrap_or_default();
            cbal = cbal.checked_add(credit)?;
            BALANCES.save(deps.storage, contract_addr.as_str(), &cbal)?;

            let receive = Cw20ReceiveMsg {
                sender: info.sender.to_string(),
                amount,
                msg: hook,
            };
            Ok(Response::new().add_message(WasmMsg::Execute {
                contract_addr: contract_addr.to_string(),
                msg: to_json_binary(&PairExecuteMsg::Receive(receive))?,
                funds: vec![],
            }))
        }
        ExecuteMsg::Mint { recipient, amount } => {
            let minter = MINTER
                .load(deps.storage)?
                .ok_or_else(|| StdError::generic_err("minting not allowed"))?;
            if info.sender.as_str() != minter {
                return Err(StdError::generic_err("unauthorized minter"));
            }
            let rcpt = deps.api.addr_validate(&recipient)?;
            let mut bal = BALANCES
                .may_load(deps.storage, rcpt.as_str())?
                .unwrap_or_default();
            bal = bal.checked_add(amount)?;
            BALANCES.save(deps.storage, rcpt.as_str(), &bal)?;
            let mut ti = TOKEN_INFO.load(deps.storage)?;
            ti.total_supply = ti.total_supply.checked_add(amount)?;
            TOKEN_INFO.save(deps.storage, &ti)?;
            let mut ts = TOTAL_SUPPLY.load(deps.storage)?;
            ts = ts.checked_add(amount)?;
            TOTAL_SUPPLY.save(deps.storage, &ts)?;
            Ok(Response::default())
        }
        ExecuteMsg::IncreaseAllowance {
            spender,
            amount,
            expires: _,
        } => {
            let sp = deps.api.addr_validate(&spender)?;
            let key = (info.sender.as_str(), sp.as_str());
            let current = ALLOWANCES.may_load(deps.storage, key)?.unwrap_or_default();
            ALLOWANCES.save(deps.storage, key, &(current + amount))?;
            Ok(Response::default())
        }
        ExecuteMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => {
            let owner_addr = deps.api.addr_validate(&owner)?;
            let rcpt = deps.api.addr_validate(&recipient)?;
            let spender = deps.api.addr_validate(info.sender.as_str())?;
            if amount.is_zero() {
                return Err(StdError::generic_err("zero amount"));
            }
            let allow_key = (owner_addr.as_str(), spender.as_str());
            let allowed = ALLOWANCES
                .may_load(deps.storage, allow_key)?
                .unwrap_or_default();
            if allowed < amount {
                return Err(StdError::generic_err("insufficient allowance"));
            }
            ALLOWANCES.save(deps.storage, allow_key, &(allowed - amount))?;

            let mut obal = BALANCES
                .may_load(deps.storage, owner_addr.as_str())?
                .unwrap_or_default();
            obal = obal.checked_sub(amount)?;
            BALANCES.save(deps.storage, owner_addr.as_str(), &obal)?;

            let credit = credited_amount(&mode, amount)?;
            let mut rbal = BALANCES
                .may_load(deps.storage, rcpt.as_str())?
                .unwrap_or_default();
            rbal = rbal.checked_add(credit)?;
            BALANCES.save(deps.storage, rcpt.as_str(), &rbal)?;
            Ok(Response::default())
        }
    }
}

pub fn adversarial_query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Balance { address } => {
            let a = deps.api.addr_validate(&address)?;
            let b = BALANCES
                .may_load(deps.storage, a.as_str())?
                .unwrap_or_default();
            cosmwasm_std::to_json_binary(&BalanceResponse { balance: b })
        }
        QueryMsg::TokenInfo {} => cosmwasm_std::to_json_binary(&TOKEN_INFO.load(deps.storage)?),
        QueryMsg::Allowance { owner, spender } => {
            let o = deps.api.addr_validate(&owner)?;
            let s = deps.api.addr_validate(&spender)?;
            let a = ALLOWANCES
                .may_load(deps.storage, (o.as_str(), s.as_str()))?
                .unwrap_or_default();
            cosmwasm_std::to_json_binary(&cw20::AllowanceResponse {
                allowance: a,
                expires: cw20::Expiration::Never {},
            })
        }
    }
}

pub fn adversarial_cw20_contract() -> Box<dyn cw_multi_test::Contract<Empty>> {
    let c = cw_multi_test::ContractWrapper::new(
        adversarial_execute,
        adversarial_instantiate,
        adversarial_query,
    );
    Box::new(c)
}

// ---------------------------------------------------------------------------
// Hook spoofer (calls lp-burn-hook with arbitrary `pair` payload)
// ---------------------------------------------------------------------------

#[cw_serde]
pub enum SpooferExecuteMsg {
    SpoofLpBurnHook {
        hook: String,
        claimed_pair: String,
        return_token: String,
        output_amount: Uint128,
    },
}

#[cw_serde]
#[derive(QueryResponses)]
pub enum SpooferQueryMsg {
    #[returns(String)]
    Ping {},
}

pub fn spoofer_instantiate(
    _deps: DepsMut,
    _env: Env,
    _info: MessageInfo,
    _msg: Empty,
) -> StdResult<Response> {
    Ok(Response::default())
}

pub fn spoofer_execute(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: SpooferExecuteMsg,
) -> StdResult<Response> {
    match msg {
        SpooferExecuteMsg::SpoofLpBurnHook {
            hook,
            claimed_pair,
            return_token,
            output_amount,
        } => {
            let hook_addr = deps.api.addr_validate(&hook)?;
            let pair_addr = deps.api.addr_validate(&claimed_pair)?;
            let token_addr = deps.api.addr_validate(&return_token)?;
            let inner = HookExecuteMsg::AfterSwap {
                pair: pair_addr,
                sender: info.sender.clone(),
                offer_asset: Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addr.to_string(),
                    },
                    amount: Uint128::zero(),
                },
                return_asset: Asset {
                    info: AssetInfo::Token {
                        contract_addr: token_addr.to_string(),
                    },
                    amount: output_amount,
                },
                commission_amount: Uint128::zero(),
                spread_amount: Uint128::zero(),
            };
            Ok(Response::new().add_message(WasmMsg::Execute {
                contract_addr: hook_addr.to_string(),
                msg: to_json_binary(&cl8y_dex_lp_burn_hook::msg::ExecuteMsg::Hook(inner))?,
                funds: vec![],
            }))
        }
    }
}

pub fn spoofer_query(_deps: Deps, _env: Env, msg: SpooferQueryMsg) -> StdResult<Binary> {
    match msg {
        SpooferQueryMsg::Ping {} => cosmwasm_std::to_json_binary(&"pong".to_string()),
    }
}

pub fn hook_spoofer_contract() -> Box<dyn cw_multi_test::Contract<Empty>> {
    let c =
        cw_multi_test::ContractWrapper::new(spoofer_execute, spoofer_instantiate, spoofer_query);
    Box::new(c)
}

#[cfg(test)]
mod adversarial_tests {
    use super::*;
    use crate::helpers::{
        asset_info_token, create_cw20_token, cw20_mintable_contract, extract_pair_address,
        factory_contract, lp_burn_hook_contract, pair_contract, provide_liquidity,
        query_cw20_balance, query_pool, router_contract, swap_a_to_b, transfer_tokens, TestEnv,
    };
    use cosmwasm_std::{to_json_binary, Addr, Decimal, Empty, Uint128};
    use cw20::Cw20ExecuteMsg;
    use cw_multi_test::{App, Executor};

    fn setup_env_fee_on_transfer_pair(app: &mut App) -> TestEnv {
        let governance = Addr::unchecked("governance");
        let treasury = Addr::unchecked("treasury");
        let user = Addr::unchecked("user");

        let honest_cw20 = app.store_code(cw20_mintable_contract());
        let adv_code = app.store_code(adversarial_cw20_contract());
        let pair_code = app.store_code(pair_contract());
        let factory_code = app.store_code(factory_contract());
        let router_code = app.store_code(router_contract());

        let initial = Uint128::new(1_000_000_000_000);
        let token_b = create_cw20_token(app, honest_cw20, &user, "Token B", "TKNB", initial);

        let token_a = app
            .instantiate_contract(
                adv_code,
                user.clone(),
                &InstantiateMsg {
                    name: "Bad A".to_string(),
                    symbol: "BAD".to_string(),
                    decimals: 6,
                    initial_balances: vec![cw20::Cw20Coin {
                        address: user.to_string(),
                        amount: initial,
                    }],
                    mint: None,
                    mode: AdversarialMode::FeeOnTransfer { fee_bps: 100 },
                },
                &[],
                "bad-a",
                None,
            )
            .unwrap();

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
                    whitelisted_code_ids: vec![honest_cw20, adv_code],
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

    #[test]
    fn fee_on_transfer_creates_reserve_imbalance() {
        let mut app = App::default();
        let env = setup_env_fee_on_transfer_pair(&mut app);
        let liq = Uint128::new(1_000_000);
        provide_liquidity(&mut app, &env, &env.user, liq, liq);

        let pool = query_pool(&app, &env.pair);
        let on_chain_a = query_cw20_balance(&app, &env.token_a, &env.pair);
        assert_eq!(pool.assets[0].amount, liq);
        assert!(
            on_chain_a < pool.assets[0].amount,
            "fee-on-transfer: internal reserves exceed actual token_a balance"
        );
    }

    #[test]
    fn withdraw_min_assets_reverts_on_sandwich() {
        let mut app = App::default();
        let env = setup_env_fee_on_transfer_pair(&mut app);
        let victim = Addr::unchecked("victim");
        let attacker = Addr::unchecked("attacker");

        transfer_tokens(
            &mut app,
            &env.token_a,
            &env.user,
            &victim,
            Uint128::new(50_000_000),
        );
        transfer_tokens(
            &mut app,
            &env.token_b,
            &env.user,
            &victim,
            Uint128::new(50_000_000),
        );
        transfer_tokens(
            &mut app,
            &env.token_a,
            &env.user,
            &attacker,
            Uint128::new(200_000_000),
        );
        transfer_tokens(
            &mut app,
            &env.token_b,
            &env.user,
            &attacker,
            Uint128::new(200_000_000),
        );

        provide_liquidity(
            &mut app,
            &env,
            &env.user,
            Uint128::new(10_000_000),
            Uint128::new(10_000_000),
        );

        provide_liquidity(
            &mut app,
            &env,
            &victim,
            Uint128::new(5_000_000),
            Uint128::new(5_000_000),
        );

        let lp = query_cw20_balance(&app, &env.lp_token, &victim);

        let pool_before = query_pool(&app, &env.pair);
        let min_a = lp * pool_before.assets[0].amount / pool_before.total_share;
        let min_b = lp * pool_before.assets[1].amount / pool_before.total_share;

        swap_a_to_b(&mut app, &env, &attacker, Uint128::new(8_000_000));

        let msg = to_json_binary(&dex_common::pair::Cw20HookMsg::WithdrawLiquidity {
            min_assets: Some([min_a, min_b]),
        })
        .unwrap();

        let err = app
            .execute_contract(
                victim.clone(),
                env.lp_token.clone(),
                &Cw20ExecuteMsg::Send {
                    contract: env.pair.to_string(),
                    amount: lp,
                    msg,
                },
                &[],
            )
            .unwrap_err();
        let s = err.root_cause().to_string();
        assert!(
            s.contains("Withdraw slippage") || s.contains("slippage"),
            "expected withdraw min_assets failure, got: {}",
            s
        );
    }

    #[test]
    fn lp_burn_hook_accepts_spoofed_pair_when_spoofer_allowlisted() {
        let mut app = App::default();
        let env = crate::helpers::setup_full_env(&mut app);
        let hook_code = app.store_code(lp_burn_hook_contract());
        let spoofer_code = app.store_code(hook_spoofer_contract());

        provide_liquidity(
            &mut app,
            &env,
            &env.user,
            Uint128::new(10_000_000),
            Uint128::new(10_000_000),
        );

        let hook = app
            .instantiate_contract(
                hook_code,
                env.governance.clone(),
                &cl8y_dex_lp_burn_hook::msg::InstantiateMsg {
                    target_pair: env.pair.to_string(),
                    lp_token: env.lp_token.to_string(),
                    percentage_bps: 100,
                    admin: env.governance.to_string(),
                },
                &[],
                "lpburn",
                None,
            )
            .unwrap();

        app.execute_contract(
            env.governance.clone(),
            hook.clone(),
            &cl8y_dex_lp_burn_hook::msg::ExecuteMsg::UpdateAllowedPairs {
                add: vec![env.pair.to_string()],
                remove: vec![],
            },
            &[],
        )
        .unwrap();

        let spoofer = app
            .instantiate_contract(
                spoofer_code,
                env.user.clone(),
                &Empty {},
                &[],
                "spoofer",
                None,
            )
            .unwrap();

        app.execute_contract(
            env.governance.clone(),
            hook.clone(),
            &cl8y_dex_lp_burn_hook::msg::ExecuteMsg::UpdateAllowedPairs {
                add: vec![spoofer.to_string()],
                remove: vec![],
            },
            &[],
        )
        .unwrap();

        app.execute_contract(
            env.user.clone(),
            env.lp_token.clone(),
            &Cw20ExecuteMsg::Transfer {
                recipient: hook.to_string(),
                amount: Uint128::new(500_000),
            },
            &[],
        )
        .unwrap();

        let lp_before = query_cw20_balance(&app, &env.lp_token, &hook);
        app.execute_contract(
            spoofer.clone(),
            spoofer.clone(),
            &SpooferExecuteMsg::SpoofLpBurnHook {
                hook: hook.to_string(),
                claimed_pair: env.pair.to_string(),
                return_token: env.token_b.to_string(),
                output_amount: Uint128::new(1_000_000),
            },
            &[],
        )
        .unwrap();
        let lp_after = query_cw20_balance(&app, &env.lp_token, &hook);
        assert!(
            lp_after < lp_before,
            "allowlisted non-pair can drive LP burns by spoofing `pair` in AfterSwap"
        );
    }

    #[test]
    fn router_absorbs_pre_existing_dust_on_output_token() {
        let mut app = App::default();
        let env = crate::helpers::setup_full_env(&mut app);
        provide_liquidity(
            &mut app,
            &env,
            &env.user,
            Uint128::new(1_000_000),
            Uint128::new(1_000_000),
        );

        let dust = Uint128::new(123_456);
        app.execute_contract(
            env.user.clone(),
            env.token_b.clone(),
            &Cw20ExecuteMsg::Transfer {
                recipient: env.router.to_string(),
                amount: dust,
            },
            &[],
        )
        .unwrap();

        let hook_msg = to_json_binary(&cl8y_dex_router::msg::Cw20HookMsg::ExecuteSwapOperations {
            operations: vec![cl8y_dex_router::msg::SwapOperation::TerraSwap {
                offer_asset_info: asset_info_token(&env.token_a),
                ask_asset_info: asset_info_token(&env.token_b),
                hybrid: None,
            }],
            max_spread: Decimal::one(),
            minimum_receive: None,
            to: None,
            deadline: None,
            unwrap_output: None,
        })
        .unwrap();

        let swap_in = Uint128::new(10_000);
        app.execute_contract(
            env.user.clone(),
            env.token_a.clone(),
            &Cw20ExecuteMsg::Send {
                contract: env.router.to_string(),
                amount: swap_in,
                msg: hook_msg,
            },
            &[],
        )
        .unwrap();

        let user_b = query_cw20_balance(&app, &env.token_b, &env.user);
        let sim: dex_common::pair::SimulationResponse = app
            .wrap()
            .query_wasm_smart(
                env.pair.to_string(),
                &dex_common::pair::QueryMsg::Simulation {
                    offer_asset: dex_common::types::Asset {
                        info: asset_info_token(&env.token_a),
                        amount: swap_in,
                    },
                },
            )
            .unwrap();
        assert!(
            user_b >= sim.return_amount + dust,
            "user receives swap output plus prior router dust; got {} want >= {} + {}",
            user_b,
            sim.return_amount,
            dust
        );
    }

    #[test]
    fn router_two_sequential_swaps_both_succeed_state_cleared() {
        let mut app = App::default();
        let env = crate::helpers::setup_full_env(&mut app);
        provide_liquidity(
            &mut app,
            &env,
            &env.user,
            Uint128::new(1_000_000),
            Uint128::new(1_000_000),
        );

        for _ in 0..2 {
            let hook_msg =
                to_json_binary(&cl8y_dex_router::msg::Cw20HookMsg::ExecuteSwapOperations {
                    operations: vec![cl8y_dex_router::msg::SwapOperation::TerraSwap {
                        offer_asset_info: asset_info_token(&env.token_a),
                        ask_asset_info: asset_info_token(&env.token_b),
                        hybrid: None,
                    }],
                    max_spread: Decimal::one(),
                    minimum_receive: None,
                    to: None,
                    deadline: None,
                    unwrap_output: None,
                })
                .unwrap();
            app.execute_contract(
                env.user.clone(),
                env.token_a.clone(),
                &Cw20ExecuteMsg::Send {
                    contract: env.router.to_string(),
                    amount: Uint128::new(1_000),
                    msg: hook_msg,
                },
                &[],
            )
            .unwrap();
        }
    }
}
