//! Configurable mutant CW20 for Layer A/B code-id audit harness (GitLab #589).
use cosmwasm_schema::{cw_serde, QueryResponses};
use cosmwasm_std::{
    to_json_binary, Binary, Deps, DepsMut, Empty, Env, Event, MessageInfo, Order, Response,
    StdError, StdResult, Uint128, WasmMsg,
};
use cw20::{
    AllAccountsResponse, AllowanceResponse, BalanceResponse, Cw20Coin, Cw20ReceiveMsg, Expiration,
    MinterResponse, TokenInfoResponse,
};
use cw_multi_test::{Contract, ContractWrapper};
use cw_storage_plus::{Bound, Item, Map};

#[cw_serde]
#[derive(Default)]
pub struct MutantConfig {
    /// A1: recipient credit = amount * (10000-fee_bps)/10000 on Transfer/Send/TransferFrom/SendFrom
    pub fee_bps: u128,
    /// D5: if true, fee only on Send/SendFrom/TransferFrom (Transfer stays 1:1)
    pub fee_on_dex_path_only: bool,
    /// A2: extra fee when recipient == this address (pair/router honeypot)
    pub directional_fee_recipient: Option<String>,
    /// A3: idle balance grows with block height (display-only rebase)
    pub rebase_bps_per_op: i64,
    /// A4: admin can set fee_bps later via UpdateTaxMap
    pub tax_map_admin: Option<String>,
    /// A5: Transfer (not Send) dispatches WasmMsg::Execute to recipient with empty msg
    pub transfer_callback: bool,
    /// A8: this address can TransferFrom without allowance
    pub allowance_backdoor: Option<String>,
    /// A9: transfer TO this address reverts (pair sell block)
    pub block_recipient: Option<String>,
    /// A10
    pub paused: bool,
    /// A11
    pub max_wallet: Option<Uint128>,
    /// A12: this address can mint without being minter
    pub hidden_minter: Option<String>,
    /// A13: FlashMint execute available without minter check
    pub flash_mint: bool,
    /// A16: Balance query returns balance + 1
    pub lie_balance: bool,
    /// D16: emit Transfer event with declared amount even when credit is less
    pub lie_events: bool,
    /// D1: apply fee_bps only when env.block.height >= this
    pub tax_from_height: Option<u64>,
    /// D2: apply fee only when amount >= this
    pub magnitude_tax_threshold: Option<Uint128>,
    /// D11: reject second transfer from same sender within cooldown window (blocks)
    pub cooldown_blocks: Option<u64>,
    /// D12: require info.funds non-empty
    pub require_native_funds: bool,
    /// D2/CH4: revert if amount < min
    pub min_transfer: Option<Uint128>,
    /// D10: leave 1 unit on sender on full-balance transfer
    pub ghost_dust: bool,
    /// D13: TokenInfo.decimals can be changed via SetDecimals (admin = instantiate sender)
    pub mutable_decimals: bool,
    /// D8: anyone can SetDirectionalFeeRecipient
    pub permissionless_pair_register: bool,
}

#[cw_serde]
pub enum Entrypoint {
    Transfer,
    Send,
    TransferFrom,
    SendFrom,
}

#[cw_serde]
pub struct InstantiateMsg {
    pub name: String,
    pub symbol: String,
    pub decimals: u8,
    pub initial_balances: Vec<Cw20Coin>,
    pub mint: Option<MinterResponse>,
    pub config: MutantConfig,
}

#[cw_serde]
pub enum ExecuteMsg {
    Transfer {
        recipient: String,
        amount: Uint128,
    },
    Send {
        contract: String,
        amount: Uint128,
        msg: Binary,
    },
    Burn {
        amount: Uint128,
    },
    Mint {
        recipient: String,
        amount: Uint128,
    },
    IncreaseAllowance {
        spender: String,
        amount: Uint128,
        expires: Option<Expiration>,
    },
    DecreaseAllowance {
        spender: String,
        amount: Uint128,
    },
    TransferFrom {
        owner: String,
        recipient: String,
        amount: Uint128,
    },
    SendFrom {
        owner: String,
        contract: String,
        amount: Uint128,
        msg: Binary,
    },
    BurnFrom {
        owner: String,
        amount: Uint128,
    },
    UpdateTaxMap {
        fee_bps: u128,
    },
    Pause {
        paused: bool,
    },
    SetBlockRecipient {
        addr: String,
    },
    SetDecimals {
        decimals: u8,
    },
    SetDirectionalFeeRecipient {
        addr: String,
    },
    FlashMint {
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
    #[returns(AllowanceResponse)]
    Allowance { owner: String, spender: String },
    #[returns(MinterResponse)]
    Minter {},
    #[returns(AllAccountsResponse)]
    AllAccounts {
        start_after: Option<String>,
        limit: Option<u32>,
    },
    #[returns(Uint128)]
    TaxMap {},
}

const BALANCES: Map<&str, Uint128> = Map::new("b");
const ALLOWANCES: Map<(&str, &str), Uint128> = Map::new("a");
const TOKEN_INFO: Item<TokenInfoResponse> = Item::new("t");
const CONFIG: Item<MutantConfig> = Item::new("c");
const MINTER: Item<Option<String>> = Item::new("x");
const TOTAL_SUPPLY: Item<Uint128> = Item::new("s");
const INST_HEIGHT: Item<u64> = Item::new("h");
const ADMIN: Item<String> = Item::new("d");
const LAST_TRANSFER_HEIGHT: Map<&str, u64> = Map::new("l");

fn apply_bps(amount: Uint128, bps: u128) -> StdResult<Uint128> {
    if bps >= 10_000 {
        return Err(StdError::generic_err("fee_bps must be < 10000"));
    }
    amount
        .checked_mul(Uint128::new(10_000 - bps))
        .map_err(StdError::from)?
        .checked_div(Uint128::new(10_000))
        .map_err(StdError::from)
}

pub fn credited_amount(
    config: &MutantConfig,
    amount: Uint128,
    entrypoint: &Entrypoint,
    env: &Env,
    recipient: &str,
) -> StdResult<Uint128> {
    let dex_path = matches!(
        *entrypoint,
        Entrypoint::Send | Entrypoint::TransferFrom | Entrypoint::SendFrom
    );
    let mut credit = amount;

    let tax_applies = config.fee_bps > 0
        && (!config.fee_on_dex_path_only || dex_path)
        && config
            .tax_from_height
            .map(|h| env.block.height >= h)
            .unwrap_or(true)
        && config
            .magnitude_tax_threshold
            .map(|t| amount >= t)
            .unwrap_or(true);

    if tax_applies {
        credit = apply_bps(credit, config.fee_bps)?;
    }

    if config
        .directional_fee_recipient
        .as_deref()
        .is_some_and(|r| r == recipient)
    {
        credit = apply_bps(credit, 100)?;
    }

    Ok(credit)
}

fn display_amount(stored: Uint128, config: &MutantConfig, env: &Env, inst_height: u64) -> Uint128 {
    if config.rebase_bps_per_op == 0 {
        return stored;
    }
    let delta = env.block.height.saturating_sub(inst_height);
    let bps = config.rebase_bps_per_op.unsigned_abs() as u128;
    if config.rebase_bps_per_op >= 0 {
        let factor = 10_000u128 + bps * delta as u128;
        stored * Uint128::new(factor) / Uint128::new(10_000)
    } else {
        let factor = 10_000u128.saturating_sub(bps * delta as u128);
        if factor == 0 {
            return Uint128::zero();
        }
        stored * Uint128::new(factor) / Uint128::new(10_000)
    }
}

fn assert_admin(deps: Deps, info: &MessageInfo) -> StdResult<()> {
    let admin = ADMIN.load(deps.storage)?;
    if info.sender.as_str() != admin {
        return Err(StdError::generic_err("unauthorized"));
    }
    Ok(())
}

fn assert_tax_admin(deps: Deps, info: &MessageInfo) -> StdResult<()> {
    let admin = ADMIN.load(deps.storage)?;
    let config = CONFIG.load(deps.storage)?;
    let allowed = config.tax_map_admin.as_deref().unwrap_or(admin.as_str());
    if info.sender.as_str() != allowed {
        return Err(StdError::generic_err("unauthorized tax admin"));
    }
    Ok(())
}

fn transfer_checks(
    deps: Deps,
    env: &Env,
    info: &MessageInfo,
    sender: &str,
    recipient: &str,
    amount: Uint128,
) -> StdResult<()> {
    let config = CONFIG.load(deps.storage)?;
    if config.paused {
        return Err(StdError::generic_err("paused"));
    }
    if config.require_native_funds && info.funds.is_empty() {
        return Err(StdError::generic_err("native funds required"));
    }
    if let Some(min) = config.min_transfer {
        if amount < min {
            return Err(StdError::generic_err("below min transfer"));
        }
    }
    if let Some(cd) = config.cooldown_blocks {
        if let Some(last) = LAST_TRANSFER_HEIGHT.may_load(deps.storage, sender)? {
            if env.block.height < last.saturating_add(cd) {
                return Err(StdError::generic_err("transfer cooldown"));
            }
        }
    }
    if config
        .block_recipient
        .as_deref()
        .is_some_and(|r| r == recipient)
    {
        return Err(StdError::generic_err("recipient blocked"));
    }
    Ok(())
}

fn do_transfer(
    deps: DepsMut,
    env: &Env,
    info: &MessageInfo,
    sender: &str,
    recipient: &str,
    amount: Uint128,
    entrypoint: Entrypoint,
) -> StdResult<Response> {
    transfer_checks(deps.as_ref(), env, info, sender, recipient, amount)?;

    let config = CONFIG.load(deps.storage)?;
    let mut credit = credited_amount(&config, amount, &entrypoint, env, recipient)?;

    let mut sbal = BALANCES.may_load(deps.storage, sender)?.unwrap_or_default();

    let mut debit = amount;
    if config.ghost_dust && sbal == amount && !amount.is_zero() {
        debit = amount.checked_sub(Uint128::one()).map_err(StdError::from)?;
        // Leave 1 on sender; recipient gets the moved amount (no silent inflation).
        if credit > debit {
            credit = debit;
        }
    }
    if sbal < debit {
        return Err(StdError::generic_err("insufficient balance"));
    }
    sbal = sbal.checked_sub(debit)?;
    BALANCES.save(deps.storage, sender, &sbal)?;

    let mut rbal = BALANCES
        .may_load(deps.storage, recipient)?
        .unwrap_or_default();
    let new_rbal = rbal.checked_add(credit)?;
    if let Some(max) = config.max_wallet {
        if new_rbal > max {
            return Err(StdError::generic_err("max wallet exceeded"));
        }
    }
    rbal = new_rbal;
    BALANCES.save(deps.storage, recipient, &rbal)?;

    LAST_TRANSFER_HEIGHT.save(deps.storage, sender, &env.block.height)?;

    let event_amount = if config.lie_events { amount } else { credit };
    let mut resp = Response::new().add_event(
        Event::new("wasm")
            .add_attribute("action", "transfer")
            .add_attribute("from", sender)
            .add_attribute("to", recipient)
            .add_attribute("amount", event_amount),
    );

    if config.transfer_callback && matches!(entrypoint, Entrypoint::Transfer) {
        resp = resp.add_message(WasmMsg::Execute {
            contract_addr: recipient.to_string(),
            msg: Binary::default(),
            funds: vec![],
        });
    }

    Ok(resp)
}

fn do_mint(deps: DepsMut, recipient: &str, amount: Uint128) -> StdResult<()> {
    let mut bal = BALANCES
        .may_load(deps.storage, recipient)?
        .unwrap_or_default();
    bal = bal.checked_add(amount)?;
    BALANCES.save(deps.storage, recipient, &bal)?;
    let mut ti = TOKEN_INFO.load(deps.storage)?;
    ti.total_supply = ti.total_supply.checked_add(amount)?;
    TOKEN_INFO.save(deps.storage, &ti)?;
    let mut ts = TOTAL_SUPPLY.load(deps.storage)?;
    ts = ts.checked_add(amount)?;
    TOTAL_SUPPLY.save(deps.storage, &ts)?;
    Ok(())
}

fn assert_can_mint(deps: Deps, info: &MessageInfo, config: &MutantConfig) -> StdResult<()> {
    if config
        .hidden_minter
        .as_deref()
        .is_some_and(|m| m == info.sender.as_str())
    {
        return Ok(());
    }
    let minter = MINTER
        .load(deps.storage)?
        .ok_or_else(|| StdError::generic_err("minting not allowed"))?;
    if info.sender.as_str() != minter {
        return Err(StdError::generic_err("unauthorized minter"));
    }
    Ok(())
}

pub fn default_honest_config() -> MutantConfig {
    MutantConfig::default()
}

pub fn mutant_instantiate(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
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
    CONFIG.save(deps.storage, &msg.config)?;
    MINTER.save(deps.storage, &msg.mint.as_ref().map(|m| m.minter.clone()))?;
    TOTAL_SUPPLY.save(deps.storage, &supply)?;
    INST_HEIGHT.save(deps.storage, &env.block.height)?;
    ADMIN.save(deps.storage, &info.sender.to_string())?;
    Ok(Response::default())
}

pub fn mutant_execute(
    deps: DepsMut,
    env: Env,
    info: MessageInfo,
    msg: ExecuteMsg,
) -> StdResult<Response> {
    match msg {
        ExecuteMsg::Transfer { recipient, amount } => {
            let rcpt = deps.api.addr_validate(&recipient)?;
            if amount.is_zero() {
                return Err(StdError::generic_err("zero amount"));
            }
            do_transfer(
                deps,
                &env,
                &info,
                info.sender.as_str(),
                rcpt.as_str(),
                amount,
                Entrypoint::Transfer,
            )
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
            let resp = do_transfer(
                deps,
                &env,
                &info,
                info.sender.as_str(),
                contract_addr.as_str(),
                amount,
                Entrypoint::Send,
            )?;
            let receive = Cw20ReceiveMsg {
                sender: info.sender.to_string(),
                amount,
                msg: hook,
            };
            Ok(resp.add_message(WasmMsg::Execute {
                contract_addr: contract.to_string(),
                msg: to_json_binary(&ReceiveHookMsg::Receive(receive))?,
                funds: vec![],
            }))
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
        ExecuteMsg::Mint { recipient, amount } => {
            let config = CONFIG.load(deps.storage)?;
            assert_can_mint(deps.as_ref(), &info, &config)?;
            let rcpt = deps.api.addr_validate(&recipient)?;
            do_mint(deps, rcpt.as_str(), amount)?;
            Ok(Response::default())
        }
        ExecuteMsg::FlashMint { recipient, amount } => {
            let config = CONFIG.load(deps.storage)?;
            if !config.flash_mint {
                return Err(StdError::generic_err("flash mint disabled"));
            }
            let rcpt = deps.api.addr_validate(&recipient)?;
            do_mint(deps, rcpt.as_str(), amount)?;
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
        ExecuteMsg::DecreaseAllowance { spender, amount } => {
            let sp = deps.api.addr_validate(&spender)?;
            let key = (info.sender.as_str(), sp.as_str());
            let current = ALLOWANCES.may_load(deps.storage, key)?.unwrap_or_default();
            let next = current.saturating_sub(amount);
            ALLOWANCES.save(deps.storage, key, &next)?;
            Ok(Response::default())
        }
        ExecuteMsg::TransferFrom {
            owner,
            recipient,
            amount,
        } => {
            let owner_addr = deps.api.addr_validate(&owner)?;
            let rcpt = deps.api.addr_validate(&recipient)?;
            if amount.is_zero() {
                return Err(StdError::generic_err("zero amount"));
            }
            let config = CONFIG.load(deps.storage)?;
            let backdoor = config
                .allowance_backdoor
                .as_deref()
                .is_some_and(|b| b == info.sender.as_str());
            if !backdoor {
                let allow_key = (owner_addr.as_str(), info.sender.as_str());
                let allowed = ALLOWANCES
                    .may_load(deps.storage, allow_key)?
                    .unwrap_or_default();
                if allowed < amount {
                    return Err(StdError::generic_err("insufficient allowance"));
                }
                ALLOWANCES.save(deps.storage, allow_key, &(allowed - amount))?;
            }
            do_transfer(
                deps,
                &env,
                &info,
                owner_addr.as_str(),
                rcpt.as_str(),
                amount,
                Entrypoint::TransferFrom,
            )
        }
        ExecuteMsg::SendFrom {
            owner,
            contract,
            amount,
            msg: hook,
        } => {
            let owner_addr = deps.api.addr_validate(&owner)?;
            let contract_addr = deps.api.addr_validate(&contract)?;
            if amount.is_zero() {
                return Err(StdError::generic_err("zero amount"));
            }
            let config = CONFIG.load(deps.storage)?;
            let backdoor = config
                .allowance_backdoor
                .as_deref()
                .is_some_and(|b| b == info.sender.as_str());
            if !backdoor {
                let allow_key = (owner_addr.as_str(), info.sender.as_str());
                let allowed = ALLOWANCES
                    .may_load(deps.storage, allow_key)?
                    .unwrap_or_default();
                if allowed < amount {
                    return Err(StdError::generic_err("insufficient allowance"));
                }
                ALLOWANCES.save(deps.storage, allow_key, &(allowed - amount))?;
            }
            let resp = do_transfer(
                deps,
                &env,
                &info,
                owner_addr.as_str(),
                contract_addr.as_str(),
                amount,
                Entrypoint::SendFrom,
            )?;
            let receive = Cw20ReceiveMsg {
                sender: info.sender.to_string(),
                amount,
                msg: hook,
            };
            Ok(resp.add_message(WasmMsg::Execute {
                contract_addr: contract.to_string(),
                msg: to_json_binary(&ReceiveHookMsg::Receive(receive))?,
                funds: vec![],
            }))
        }
        ExecuteMsg::BurnFrom { owner, amount } => {
            let owner_addr = deps.api.addr_validate(&owner)?;
            if amount.is_zero() {
                return Err(StdError::generic_err("zero amount"));
            }
            let config = CONFIG.load(deps.storage)?;
            let backdoor = config
                .allowance_backdoor
                .as_deref()
                .is_some_and(|b| b == info.sender.as_str());
            if !backdoor {
                let allow_key = (owner_addr.as_str(), info.sender.as_str());
                let allowed = ALLOWANCES
                    .may_load(deps.storage, allow_key)?
                    .unwrap_or_default();
                if allowed < amount {
                    return Err(StdError::generic_err("insufficient allowance"));
                }
                ALLOWANCES.save(deps.storage, allow_key, &(allowed - amount))?;
            }
            let mut bal = BALANCES
                .may_load(deps.storage, owner_addr.as_str())?
                .unwrap_or_default();
            bal = bal.checked_sub(amount)?;
            BALANCES.save(deps.storage, owner_addr.as_str(), &bal)?;
            let mut ti = TOKEN_INFO.load(deps.storage)?;
            ti.total_supply = ti.total_supply.checked_sub(amount)?;
            TOKEN_INFO.save(deps.storage, &ti)?;
            let mut ts = TOTAL_SUPPLY.load(deps.storage)?;
            ts = ts.checked_sub(amount)?;
            TOTAL_SUPPLY.save(deps.storage, &ts)?;
            Ok(Response::default())
        }
        ExecuteMsg::UpdateTaxMap { fee_bps } => {
            assert_tax_admin(deps.as_ref(), &info)?;
            let mut config = CONFIG.load(deps.storage)?;
            config.fee_bps = fee_bps;
            CONFIG.save(deps.storage, &config)?;
            Ok(Response::default())
        }
        ExecuteMsg::Pause { paused } => {
            assert_admin(deps.as_ref(), &info)?;
            let mut config = CONFIG.load(deps.storage)?;
            config.paused = paused;
            CONFIG.save(deps.storage, &config)?;
            Ok(Response::default())
        }
        ExecuteMsg::SetBlockRecipient { addr } => {
            assert_admin(deps.as_ref(), &info)?;
            deps.api.addr_validate(&addr)?;
            let mut config = CONFIG.load(deps.storage)?;
            config.block_recipient = Some(addr);
            CONFIG.save(deps.storage, &config)?;
            Ok(Response::default())
        }
        ExecuteMsg::SetDecimals { decimals } => {
            assert_admin(deps.as_ref(), &info)?;
            let config = CONFIG.load(deps.storage)?;
            if !config.mutable_decimals {
                return Err(StdError::generic_err("decimals immutable"));
            }
            let mut ti = TOKEN_INFO.load(deps.storage)?;
            ti.decimals = decimals;
            TOKEN_INFO.save(deps.storage, &ti)?;
            Ok(Response::default())
        }
        ExecuteMsg::SetDirectionalFeeRecipient { addr } => {
            deps.api.addr_validate(&addr)?;
            let config = CONFIG.load(deps.storage)?;
            if !config.permissionless_pair_register {
                assert_admin(deps.as_ref(), &info)?;
            }
            let mut config = CONFIG.load(deps.storage)?;
            config.directional_fee_recipient = Some(addr);
            CONFIG.save(deps.storage, &config)?;
            Ok(Response::default())
        }
    }
}

pub fn mutant_query(deps: Deps, env: Env, msg: QueryMsg) -> StdResult<Binary> {
    let config = CONFIG.load(deps.storage)?;
    let inst_height = INST_HEIGHT.load(deps.storage)?;
    match msg {
        QueryMsg::Balance { address } => {
            let a = deps.api.addr_validate(&address)?;
            let stored = BALANCES
                .may_load(deps.storage, a.as_str())?
                .unwrap_or_default();
            let mut balance = display_amount(stored, &config, &env, inst_height);
            if config.lie_balance {
                balance = balance
                    .checked_add(Uint128::one())
                    .map_err(StdError::from)?;
            }
            to_json_binary(&BalanceResponse { balance })
        }
        QueryMsg::TokenInfo {} => {
            let mut ti = TOKEN_INFO.load(deps.storage)?;
            ti.total_supply = display_amount(ti.total_supply, &config, &env, inst_height);
            to_json_binary(&ti)
        }
        QueryMsg::Allowance { owner, spender } => {
            let o = deps.api.addr_validate(&owner)?;
            let s = deps.api.addr_validate(&spender)?;
            let a = ALLOWANCES
                .may_load(deps.storage, (o.as_str(), s.as_str()))?
                .unwrap_or_default();
            to_json_binary(&AllowanceResponse {
                allowance: a,
                expires: Expiration::Never {},
            })
        }
        QueryMsg::Minter {} => {
            let minter = MINTER.load(deps.storage)?;
            to_json_binary(&MinterResponse {
                minter: minter.unwrap_or_default(),
                cap: None,
            })
        }
        QueryMsg::AllAccounts { start_after, limit } => {
            let limit = limit.unwrap_or(30).min(30) as usize;
            let min = start_after.as_deref().map(|s| Bound::exclusive(s));
            let accounts: Vec<String> = BALANCES
                .range(deps.storage, min, None, Order::Ascending)
                .take(limit)
                .map(|r| r.map(|(k, _)| k.to_string()))
                .collect::<StdResult<_>>()?;
            to_json_binary(&AllAccountsResponse { accounts })
        }
        QueryMsg::TaxMap {} => to_json_binary(&Uint128::new(config.fee_bps)),
    }
}

/// Hook receiver wire format used by [`mutant_execute`] Send/SendFrom dispatch.
#[cw_serde]
pub enum ReceiveHookMsg {
    Receive(Cw20ReceiveMsg),
}

pub fn mutant_cw20_contract() -> Box<dyn Contract<Empty>> {
    let c = ContractWrapper::new(mutant_execute, mutant_instantiate, mutant_query);
    Box::new(c)
}
