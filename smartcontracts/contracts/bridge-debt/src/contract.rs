//! Issues bridged-token borrows for the CMM LP and a 1:1 debt receipt.
//!
//! `Issue` mints the bridged CW20 (USDT, later other assets) to the liquidity
//! recipient and mints the same amount of `XXX-Bridge-Debt` to the debtor, the
//! CMM. Repaying the bridged token through this contract burns both. User
//! bridge withdrawals stay on the bridge: they are backed by a remote lock and
//! do not mint a Bridge-Debt token.

#[cfg(not(feature = "library"))]
use cosmwasm_std::entry_point;
use cosmwasm_std::{
    from_json, to_json_binary, Addr, Binary, ContractInfoResponse, Deps, DepsMut, Env, MessageInfo,
    Order, QueryRequest, Response, StdResult, Uint128, WasmMsg, WasmQuery,
};
use cw2::set_contract_version;
use cw20::{Cw20ExecuteMsg, Cw20QueryMsg, Cw20ReceiveMsg, MinterResponse, TokenInfoResponse};
use cw_storage_plus::Bound;

use crate::error::ContractError;
use crate::msg::{
    ConfigResponse, Cw20HookMsg, ExecuteMsg, InstantiateMsg, MarketResponse, MarketsResponse,
    MigrateMsg, QueryMsg,
};
use crate::state::{Config, Market, CONFIG, CONTRACT_NAME, CONTRACT_VERSION, DEBT_TOKENS, MARKETS};

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn instantiate(
    deps: DepsMut,
    _env: Env,
    info: MessageInfo,
    msg: InstantiateMsg,
) -> Result<Response, ContractError> {
    reject_funds(&info)?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    let owner = deps.api.addr_validate(&msg.owner)?;
    CONFIG.save(
        deps.storage,
        &Config {
            owner,
            debt_code_id: None,
        },
    )?;

    Ok(Response::new()
        .add_attribute("action", "instantiate")
        .add_attribute("owner", msg.owner))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> Result<Response, ContractError> {
    reject_funds(&info)?;
    match msg {
        ExecuteMsg::SetDebtCodeId { code_id } => execute_set_debt_code_id(deps, info, code_id),
        ExecuteMsg::RegisterMarket {
            underlying,
            debt_token,
        } => execute_register_market(deps, env, info, underlying, debt_token),
        ExecuteMsg::Issue {
            underlying,
            recipient,
            amount,
        } => execute_issue(deps, env, info, underlying, recipient, amount),
        ExecuteMsg::Receive(msg) => execute_receive(deps, env, info, msg),
        ExecuteMsg::Recover {
            underlying,
            recipient,
            amount,
        } => execute_recover(deps, info, underlying, recipient, amount),
        ExecuteMsg::UpdateOwner { owner } => execute_update_owner(deps, info, owner),
    }
}

fn execute_set_debt_code_id(
    deps: DepsMut,
    info: MessageInfo,
    code_id: u64,
) -> Result<Response, ContractError> {
    if code_id == 0 {
        return Err(ContractError::UnexpectedDebtCode {});
    }
    let mut config = load_config(deps.as_ref())?;
    must_owner(&config, &info)?;
    config.debt_code_id = Some(code_id);
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "set_debt_code_id")
        .add_attribute("code_id", code_id.to_string()))
}

fn execute_register_market(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    underlying: String,
    debt_token: String,
) -> Result<Response, ContractError> {
    let config = load_config(deps.as_ref())?;
    must_owner(&config, &info)?;

    let underlying = deps.api.addr_validate(&underlying)?;
    let debt_token = deps.api.addr_validate(&debt_token)?;
    if underlying == debt_token
        || underlying == env.contract.address
        || debt_token == env.contract.address
    {
        return Err(ContractError::InvalidMarket {});
    }
    if MARKETS.may_load(deps.storage, &underlying)?.is_some() {
        return Err(ContractError::MarketExists {});
    }
    if DEBT_TOKENS.has(deps.storage, &debt_token) {
        return Err(ContractError::DebtTokenInUse {});
    }

    let debt = inspect_debt_token(deps.as_ref(), &env, &debt_token)?;
    let decimals = underlying_decimals(deps.as_ref(), &underlying)?;
    if debt.decimals != decimals {
        return Err(ContractError::DecimalsMismatch {});
    }
    if !debt.total_supply.is_zero() {
        return Err(ContractError::DebtSupplyNotZero {});
    }
    if debt.debtor == env.contract.address || debt.debtor == debt_token {
        return Err(ContractError::InvalidDebtor {});
    }

    let market = Market {
        underlying: underlying.clone(),
        debt_token: debt_token.clone(),
        debtor: debt.debtor,
        decimals,
        outstanding: Uint128::zero(),
    };
    MARKETS.save(deps.storage, &underlying, &market)?;
    DEBT_TOKENS.save(deps.storage, &debt_token, &cosmwasm_std::Empty {})?;

    Ok(Response::new()
        .add_attribute("action", "register_bridge_debt_market")
        .add_attribute("underlying", market.underlying)
        .add_attribute("debt_token", market.debt_token)
        .add_attribute("debtor", market.debtor)
        .add_attribute("decimals", market.decimals.to_string()))
}

fn execute_issue(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    underlying: String,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = load_config(deps.as_ref())?;
    must_owner(&config, &info)?;
    if amount.is_zero() {
        return Err(ContractError::ZeroAmount {});
    }

    let underlying = deps.api.addr_validate(&underlying)?;
    let recipient = deps.api.addr_validate(&recipient)?;
    let mut market = load_market(deps.as_ref(), &underlying)?;
    assert_market_consistent(deps.as_ref(), &env, &market)?;

    market.outstanding = market
        .outstanding
        .checked_add(amount)
        .map_err(|_| ContractError::Overflow {})?;
    MARKETS.save(deps.storage, &underlying, &market)?;

    let mint_underlying = WasmMsg::Execute {
        contract_addr: market.underlying.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Mint {
            recipient: recipient.to_string(),
            amount,
        })?,
        funds: vec![],
    };
    let mint_debt = WasmMsg::Execute {
        contract_addr: market.debt_token.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Mint {
            recipient: market.debtor.to_string(),
            amount,
        })?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(mint_underlying)
        .add_message(mint_debt)
        .add_attribute("action", "issue_bridge_debt")
        .add_attribute("underlying", market.underlying)
        .add_attribute("debt_token", market.debt_token)
        .add_attribute("recipient", recipient)
        .add_attribute("debtor", market.debtor)
        .add_attribute("amount", amount))
}

fn execute_receive(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: Cw20ReceiveMsg,
) -> Result<Response, ContractError> {
    let hook: Cw20HookMsg = from_json(&msg.msg).map_err(|_| ContractError::InvalidHook {})?;
    let Cw20HookMsg::Repay {} = hook;
    if msg.amount.is_zero() {
        return Err(ContractError::ZeroAmount {});
    }

    let mut market = load_market(deps.as_ref(), &info.sender)?;
    assert_market_consistent(deps.as_ref(), &env, &market)?;
    if msg.amount > market.outstanding {
        return Err(ContractError::RepayExceedsDebt {});
    }

    market.outstanding -= msg.amount;
    MARKETS.save(deps.storage, &info.sender, &market)?;

    let burn_underlying = WasmMsg::Execute {
        contract_addr: market.underlying.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Burn { amount: msg.amount })?,
        funds: vec![],
    };
    let burn_debt = WasmMsg::Execute {
        contract_addr: market.debt_token.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::BurnFrom {
            owner: market.debtor.to_string(),
            amount: msg.amount,
        })?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(burn_underlying)
        .add_message(burn_debt)
        .add_attribute("action", "repay_bridge_debt")
        .add_attribute("underlying", market.underlying)
        .add_attribute("payer", msg.sender)
        .add_attribute("debtor", market.debtor)
        .add_attribute("amount", msg.amount)
        .add_attribute("outstanding", market.outstanding))
}

fn execute_recover(
    deps: DepsMut,
    info: MessageInfo,
    underlying: String,
    recipient: String,
    amount: Uint128,
) -> Result<Response, ContractError> {
    let config = load_config(deps.as_ref())?;
    must_owner(&config, &info)?;
    if amount.is_zero() {
        return Err(ContractError::ZeroAmount {});
    }
    let underlying = deps.api.addr_validate(&underlying)?;
    let recipient = deps.api.addr_validate(&recipient)?;
    let market = load_market(deps.as_ref(), &underlying)?;

    let transfer = WasmMsg::Execute {
        contract_addr: market.underlying.to_string(),
        msg: to_json_binary(&Cw20ExecuteMsg::Transfer {
            recipient: recipient.to_string(),
            amount,
        })?,
        funds: vec![],
    };

    Ok(Response::new()
        .add_message(transfer)
        .add_attribute("action", "recover_bridged_token")
        .add_attribute("underlying", market.underlying)
        .add_attribute("recipient", recipient)
        .add_attribute("amount", amount))
}

fn execute_update_owner(
    deps: DepsMut,
    info: MessageInfo,
    owner: String,
) -> Result<Response, ContractError> {
    let mut config = load_config(deps.as_ref())?;
    must_owner(&config, &info)?;
    let owner = deps.api.addr_validate(&owner)?;
    config.owner = owner.clone();
    CONFIG.save(deps.storage, &config)?;
    Ok(Response::new()
        .add_attribute("action", "update_owner")
        .add_attribute("owner", owner))
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn query(deps: Deps, _env: Env, msg: QueryMsg) -> StdResult<Binary> {
    match msg {
        QueryMsg::Config {} => to_json_binary(&query_config(deps)?),
        QueryMsg::Market { underlying } => to_json_binary(&query_market(deps, underlying)?),
        QueryMsg::Markets { start_after, limit } => {
            to_json_binary(&query_markets(deps, start_after, limit)?)
        }
    }
}

fn query_config(deps: Deps) -> StdResult<ConfigResponse> {
    let config = CONFIG.load(deps.storage)?;
    Ok(ConfigResponse {
        owner: config.owner,
        debt_code_id: config.debt_code_id,
    })
}

fn query_market(deps: Deps, underlying: String) -> StdResult<MarketResponse> {
    let underlying = deps.api.addr_validate(&underlying)?;
    let market = MARKETS.load(deps.storage, &underlying)?;
    Ok(market_response(market))
}

fn query_markets(
    deps: Deps,
    start_after: Option<String>,
    limit: Option<u32>,
) -> StdResult<MarketsResponse> {
    let limit = match limit {
        Some(n) if n > 0 => n.min(30),
        _ => 10,
    } as usize;
    let start = start_after
        .map(|addr| deps.api.addr_validate(&addr))
        .transpose()?;
    let min = start.as_ref().map(Bound::exclusive);

    let markets = MARKETS
        .range(deps.storage, min, None, Order::Ascending)
        .take(limit)
        .map(|item| {
            let (_, market) = item?;
            Ok(market_response(market))
        })
        .collect::<StdResult<Vec<_>>>()?;
    Ok(MarketsResponse { markets })
}

#[cfg_attr(not(feature = "library"), entry_point)]
pub fn migrate(deps: DepsMut, _env: Env, _msg: MigrateMsg) -> Result<Response, ContractError> {
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;
    Ok(Response::new().add_attribute("action", "migrate"))
}

struct DebtView {
    debtor: Addr,
    decimals: u8,
    total_supply: Uint128,
}

fn inspect_debt_token(deps: Deps, env: &Env, debt_token: &Addr) -> Result<DebtView, ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let allowed = config.debt_code_id.ok_or(ContractError::DebtCodeUnset {})?;
    let code: ContractInfoResponse =
        deps.querier
            .query(&QueryRequest::Wasm(WasmQuery::ContractInfo {
                contract_addr: debt_token.to_string(),
            }))?;
    if code.code_id != allowed {
        return Err(ContractError::UnexpectedDebtCode {});
    }

    let info: TokenInfoResponse = deps
        .querier
        .query_wasm_smart(debt_token, &Cw20QueryMsg::TokenInfo {})
        .map_err(|_| ContractError::NotDebtToken {})?;
    let minter: MinterResponse = deps
        .querier
        .query_wasm_smart(debt_token, &Cw20QueryMsg::Minter {})
        .map_err(|_| ContractError::DebtMinterMismatch {})?;
    let minter = deps.api.addr_validate(&minter.minter)?;
    if minter != env.contract.address {
        return Err(ContractError::DebtMinterMismatch {});
    }

    let debtor: DebtorProbe = deps
        .querier
        .query_wasm_smart(debt_token, &DebtProbe::Debtor {})
        .map_err(|_| ContractError::NotDebtToken {})?;
    let debtor = deps.api.addr_validate(&debtor.debtor)?;

    let listed: MintersProbe = deps
        .querier
        .query_wasm_smart(debt_token, &DebtProbe::Minters {})
        .map_err(|_| ContractError::NotDebtToken {})?;
    if !listed.minters.is_empty() {
        return Err(ContractError::AdditionalMinters {});
    }

    Ok(DebtView {
        debtor,
        decimals: info.decimals,
        total_supply: info.total_supply,
    })
}

fn assert_market_consistent(deps: Deps, env: &Env, market: &Market) -> Result<(), ContractError> {
    let debt = inspect_debt_token(deps, env, &market.debt_token)?;
    if debt.debtor != market.debtor {
        return Err(ContractError::DebtorMismatch {});
    }
    if debt.decimals != market.decimals {
        return Err(ContractError::DecimalsMismatch {});
    }
    if debt.total_supply != market.outstanding {
        return Err(ContractError::DebtSupplyDiverged {});
    }
    let decimals = underlying_decimals(deps, &market.underlying)?;
    if decimals != market.decimals {
        return Err(ContractError::DecimalsMismatch {});
    }
    Ok(())
}

fn underlying_decimals(deps: Deps, underlying: &Addr) -> Result<u8, ContractError> {
    let info: TokenInfoResponse = deps
        .querier
        .query_wasm_smart(underlying, &Cw20QueryMsg::TokenInfo {})
        .map_err(|_| ContractError::InvalidUnderlying {})?;
    Ok(info.decimals)
}

fn load_config(deps: Deps) -> Result<Config, ContractError> {
    Ok(CONFIG.load(deps.storage)?)
}

fn load_market(deps: Deps, underlying: &Addr) -> Result<Market, ContractError> {
    MARKETS
        .load(deps.storage, underlying)
        .map_err(|_| ContractError::UnknownMarket {})
}

fn must_owner(config: &Config, info: &MessageInfo) -> Result<(), ContractError> {
    if info.sender == config.owner {
        Ok(())
    } else {
        Err(ContractError::Unauthorized {})
    }
}

fn reject_funds(info: &MessageInfo) -> Result<(), ContractError> {
    if info.funds.is_empty() {
        Ok(())
    } else {
        Err(ContractError::UnexpectedFunds {})
    }
}

fn market_response(market: Market) -> MarketResponse {
    MarketResponse {
        underlying: market.underlying,
        debt_token: market.debt_token,
        debtor: market.debtor,
        decimals: market.decimals,
        outstanding: market.outstanding,
    }
}

#[cosmwasm_schema::cw_serde]
enum DebtProbe {
    Debtor {},
    Minters {},
}

#[cosmwasm_schema::cw_serde]
struct DebtorProbe {
    debtor: String,
}

#[cosmwasm_schema::cw_serde]
struct MintersProbe {
    minters: Vec<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
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
        app.store_code(Box::new(ContractWrapper::new(execute, instantiate, query)))
    }

    fn debt_code(app: &mut App) -> u64 {
        app.store_code(Box::new(ContractWrapper::new(
            bridge_debt_token::contract::execute,
            bridge_debt_token::contract::instantiate,
            bridge_debt_token::contract::query,
        )))
    }

    fn cw20_code(app: &mut App) -> u64 {
        app.store_code(Box::new(ContractWrapper::new(
            cw20_base::contract::execute,
            cw20_base::contract::instantiate,
            cw20_base::contract::query,
        )))
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
            None,
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
            None,
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
                None,
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
                None,
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
                None,
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
    fn issue_fails_when_this_contract_cannot_mint_underlying() {
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
                None,
            )
            .unwrap();
        let usdt = instantiate_cw20(&mut app, cw20_code_id, "USDT", 18, &Addr::unchecked(OWNER));
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
        let err = app
            .execute_contract(
                Addr::unchecked(OWNER),
                issuer,
                &ExecuteMsg::Issue {
                    underlying: usdt.to_string(),
                    recipient: LP.to_string(),
                    amount: units(1),
                },
                &[],
            )
            .unwrap_err();
        assert!(
            shows(&err, "Unauthorized") || shows(&err, "mint"),
            "{err:?}"
        );
    }
}
