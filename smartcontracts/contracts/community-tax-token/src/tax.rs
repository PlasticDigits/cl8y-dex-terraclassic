use cosmwasm_std::{
    Addr, Binary, CosmosMsg, Deps, DepsMut, Env, Response, StdResult, Storage, Uint128,
};
use cw20::Cw20ReceiveMsg;
use cw20_base::state::{TokenInfo, BALANCES, TOKEN_INFO};
use dex_common::pair::Cw20HookMsg;

use crate::error::ContractError;
use crate::msg::{SinkKind, TaxKind, TaxPreviewResponse, BPS_DENOM};
use crate::state::{
    Config, Features, LaunchGuards, StoredSink, CONFIG, FEATURES, LAST_TRADE_BLOCK, LAUNCH_GUARDS,
    LISTED_PAIRS, MANAGER_EXEMPT, PROTOCOL_EXEMPT, SINKS,
};

pub fn tax_amount(amount: Uint128, bps: u16) -> Uint128 {
    amount.multiply_ratio(u128::from(bps), u128::from(BPS_DENOM))
}

pub fn is_listed_pair(storage: &dyn Storage, addr: &Addr) -> bool {
    LISTED_PAIRS
        .may_load(storage, addr)
        .ok()
        .flatten()
        .unwrap_or(false)
}

pub fn is_protocol_exempt(storage: &dyn Storage, self_addr: &Addr, addr: &Addr) -> bool {
    if addr == self_addr {
        return true;
    }
    if is_listed_pair(storage, addr) {
        return true;
    }
    PROTOCOL_EXEMPT
        .may_load(storage, addr)
        .ok()
        .flatten()
        .unwrap_or(false)
}

pub fn is_manager_exempt(storage: &dyn Storage, addr: &Addr) -> bool {
    MANAGER_EXEMPT
        .may_load(storage, addr)
        .ok()
        .flatten()
        .unwrap_or(false)
}

pub fn is_transfer_exempt(storage: &dyn Storage, self_addr: &Addr, addr: &Addr) -> bool {
    is_protocol_exempt(storage, self_addr, addr) || is_manager_exempt(storage, addr)
}

/// Cooldown subjects are user wallets only (H608-1 / #608).
/// Do not check or record listed pairs, router, factory, this token, or AutoLP —
/// those are protocol-exempt. Manager-exempt wallets stay subjects (anti-snipe).
pub fn is_cooldown_subject(storage: &dyn Storage, self_addr: &Addr, addr: &Addr) -> bool {
    !is_protocol_exempt(storage, self_addr, addr)
}

pub fn is_swap_send_hook(msg: &Binary) -> bool {
    matches!(
        cosmwasm_std::from_json::<Cw20HookMsg>(msg),
        Ok(Cw20HookMsg::Swap { .. })
    )
}

/// **T592-7** classification (Option A, no pair wasm change):
///
/// - **Sell** — `Send` to a registered listed pair whose hook is `Cw20HookMsg::Swap`.
///   Extra-debit: pair is credited exactly `amount` (inbound 1:1 / **P2**).
/// - **Buy** — `Transfer`/`Send` **from** a registered listed pair to a non-protocol-exempt
///   recipient. Pair is debited exactly `amount`. Pair→EOA `Transfer` is also how withdraw
///   and limit refunds are paid; those paths therefore take buy tax (same primitive).
///   Provide (`TransferFrom`) and limit `PlaceLimitOrder*` `Send` stay 1:1.
/// - **Transfer** — TransferTax SKU, neither side protocol-exempt.
/// - **Honest** — everything else (protocol inbound, wallet 1:1, TransferFrom to pair).
pub fn classify(
    storage: &dyn Storage,
    self_addr: &Addr,
    from: &Addr,
    to: &Addr,
    send_msg: Option<&Binary>,
    features: &Features,
    config: &Config,
) -> (TaxKind, u16) {
    let from_pair = is_listed_pair(storage, from);
    let to_pair = is_listed_pair(storage, to);

    if to_pair {
        if let Some(msg) = send_msg {
            if is_swap_send_hook(msg) && !is_protocol_exempt(storage, self_addr, from) {
                return (TaxKind::Sell, config.sell_bps);
            }
        }
        return (TaxKind::Honest, 0);
    }

    if from_pair && !is_protocol_exempt(storage, self_addr, to) {
        return (TaxKind::Buy, config.buy_bps);
    }

    if features.transfer_tax
        && config.transfer_bps > 0
        && !is_transfer_exempt(storage, self_addr, from)
        && !is_transfer_exempt(storage, self_addr, to)
    {
        return (TaxKind::Transfer, config.transfer_bps);
    }

    (TaxKind::Honest, 0)
}

pub fn preview(
    deps: Deps,
    self_addr: &Addr,
    from: &Addr,
    to: &Addr,
    amount: Uint128,
    send_msg: Option<&Binary>,
) -> StdResult<TaxPreviewResponse> {
    let config = CONFIG.load(deps.storage)?;
    let features = FEATURES.load(deps.storage)?;
    let (kind, bps) = classify(
        deps.storage,
        self_addr,
        from,
        to,
        send_msg,
        &features,
        &config,
    );
    let tax = tax_amount(amount, bps);
    let (debit, credit) = match kind {
        TaxKind::Sell => (amount.checked_add(tax)?, amount),
        TaxKind::Buy | TaxKind::Transfer => (amount, amount.checked_sub(tax)?),
        TaxKind::Honest => (amount, amount),
    };
    Ok(TaxPreviewResponse {
        kind,
        declared: amount,
        debit,
        credit,
        tax,
    })
}

#[allow(clippy::too_many_arguments)]
fn apply_launch_guards(
    storage: &dyn Storage,
    env: &Env,
    features: &Features,
    self_addr: &Addr,
    from: &Addr,
    to: &Addr,
    kind: &TaxKind,
    credit_to: Uint128,
    to_new_balance: Uint128,
) -> Result<(), ContractError> {
    if !features.launch_guards {
        return Ok(());
    }
    let Some(guards) = LAUNCH_GUARDS.may_load(storage)? else {
        return Ok(());
    };
    match kind {
        TaxKind::Buy | TaxKind::Sell => {
            if !guards.trading_enabled {
                return Err(ContractError::TradingDisabled {});
            }
            // H-5 residual: pause still blocks both sides (T592-11). Do not carve
            // withdraw/cancel/claim here (#608 out of scope).
            check_cooldown(storage, &guards, env, self_addr, from)?;
            check_cooldown(storage, &guards, env, self_addr, to)?;
        }
        _ => {}
    }
    // H608-4 / T592-11: skip max_wallet when `to` is a listed pair or other
    // protocol-exempt address (provide, sell-to-pair, router, AutoLP, self).
    // User wallets on Buy / Transfer stay capped (H608-5).
    if let Some(max) = guards.max_wallet {
        if !is_protocol_exempt(storage, self_addr, to)
            && !to_new_balance.is_zero()
            && to_new_balance > max
            && credit_to > Uint128::zero()
        {
            return Err(ContractError::MaxWallet {});
        }
    }
    Ok(())
}

fn check_cooldown(
    storage: &dyn Storage,
    guards: &LaunchGuards,
    env: &Env,
    self_addr: &Addr,
    wallet: &Addr,
) -> Result<(), ContractError> {
    if guards.cooldown_blocks == 0 {
        return Ok(());
    }
    if !is_cooldown_subject(storage, self_addr, wallet) {
        return Ok(());
    }
    if let Some(last) = LAST_TRADE_BLOCK.may_load(storage, wallet)? {
        if env.block.height < last.saturating_add(guards.cooldown_blocks) {
            return Err(ContractError::Cooldown {});
        }
    }
    Ok(())
}

fn record_trade_blocks(
    storage: &mut dyn Storage,
    features: &Features,
    env: &Env,
    kind: &TaxKind,
    self_addr: &Addr,
    from: &Addr,
    to: &Addr,
) -> Result<(), ContractError> {
    if !features.launch_guards {
        return Ok(());
    }
    if !matches!(kind, TaxKind::Buy | TaxKind::Sell) {
        return Ok(());
    }
    if is_cooldown_subject(storage, self_addr, from) {
        LAST_TRADE_BLOCK.save(storage, from, &env.block.height)?;
    }
    if is_cooldown_subject(storage, self_addr, to) {
        LAST_TRADE_BLOCK.save(storage, to, &env.block.height)?;
    }
    Ok(())
}

fn sink_credits(
    storage: &dyn Storage,
    features: &Features,
    config: &Config,
    tax: Uint128,
) -> Result<Vec<(Addr, Uint128, bool)>, ContractError> {
    if tax.is_zero() {
        return Ok(vec![]);
    }
    if features.split_router {
        if let Some(sinks) = SINKS.may_load(storage)? {
            if !sinks.is_empty() {
                return split_tax(&sinks, config, tax);
            }
        }
    }
    Ok(vec![(config.treasury.clone(), tax, false)])
}

fn split_tax(
    sinks: &[StoredSink],
    config: &Config,
    tax: Uint128,
) -> Result<Vec<(Addr, Uint128, bool)>, ContractError> {
    let mut out = Vec::with_capacity(sinks.len());
    let mut allocated = Uint128::zero();
    for (i, sink) in sinks.iter().enumerate() {
        let share = if i + 1 == sinks.len() {
            tax.checked_sub(allocated)?
        } else {
            tax_amount(tax, sink.bps)
        };
        allocated = allocated.checked_add(share)?;
        if share.is_zero() {
            continue;
        }
        let (addr, burn) = match sink.kind {
            SinkKind::Treasury => (config.treasury.clone(), false),
            SinkKind::Burn => (Addr::unchecked(""), true),
            SinkKind::AutoLp => {
                let addr = config.autolp.clone().ok_or_else(|| {
                    ContractError::Std(cosmwasm_std::StdError::generic_err(
                        "AutoLP sink but autolp unset",
                    ))
                })?;
                (addr, false)
            }
            SinkKind::Wallet => {
                let addr = sink.addr.clone().ok_or(ContractError::WalletSinkAddr {})?;
                (addr, false)
            }
        };
        out.push((addr, share, burn));
    }
    Ok(out)
}

fn add_balance(
    storage: &mut dyn Storage,
    addr: &Addr,
    amount: Uint128,
) -> Result<(), ContractError> {
    BALANCES.update(storage, addr, |bal| -> Result<_, ContractError> {
        Ok(bal.unwrap_or_default().checked_add(amount)?)
    })?;
    Ok(())
}

fn sub_balance(
    storage: &mut dyn Storage,
    addr: &Addr,
    amount: Uint128,
) -> Result<(), ContractError> {
    BALANCES.update(storage, addr, |bal| -> Result<_, ContractError> {
        let bal = bal.unwrap_or_default();
        bal.checked_sub(amount)
            .map_err(|_| ContractError::InsufficientForSellTax {})
    })?;
    Ok(())
}

/// Apply a classified transfer. Returns (credit_to, tax) and updates balances.
pub fn apply_transfer(
    deps: DepsMut,
    env: &Env,
    self_addr: &Addr,
    from: &Addr,
    to: &Addr,
    amount: Uint128,
    send_msg: Option<&Binary>,
) -> Result<(Uint128, Uint128, TaxKind, Response), ContractError> {
    let config = CONFIG.load(deps.storage)?;
    let features = FEATURES.load(deps.storage)?;
    let (kind, bps) = classify(
        deps.storage,
        self_addr,
        from,
        to,
        send_msg,
        &features,
        &config,
    );
    let tax = tax_amount(amount, bps);
    let (debit, credit) = match kind {
        TaxKind::Sell => {
            let debit = amount.checked_add(tax)?;
            (debit, amount)
        }
        TaxKind::Buy | TaxKind::Transfer => (amount, amount.checked_sub(tax)?),
        TaxKind::Honest => (amount, amount),
    };

    let from_bal_after = BALANCES
        .may_load(deps.storage, from)?
        .unwrap_or_default()
        .checked_sub(debit)
        .map_err(|_| ContractError::InsufficientForSellTax {})?;
    let to_bal_after = BALANCES
        .may_load(deps.storage, to)?
        .unwrap_or_default()
        .checked_add(credit)?;

    apply_launch_guards(
        deps.storage,
        env,
        &features,
        self_addr,
        from,
        to,
        &kind,
        credit,
        to_bal_after,
    )?;

    BALANCES.save(deps.storage, from, &from_bal_after)?;
    add_balance(deps.storage, to, credit)?;

    let sinks = sink_credits(deps.storage, &features, &config, tax)?;
    let mut info = TOKEN_INFO.load(deps.storage)?;
    let mut burned = Uint128::zero();
    for (addr, share, is_burn) in &sinks {
        if *is_burn {
            burned = burned.checked_add(*share)?;
        } else {
            add_balance(deps.storage, addr, *share)?;
        }
    }
    if !burned.is_zero() {
        info.total_supply = info.total_supply.checked_sub(burned)?;
        TOKEN_INFO.save(deps.storage, &info)?;
    }

    record_trade_blocks(deps.storage, &features, env, &kind, self_addr, from, to)?;

    let mut resp = Response::new()
        .add_attribute("action", "transfer")
        .add_attribute("from", from)
        .add_attribute("to", to)
        .add_attribute("amount", credit)
        .add_attribute("declared", amount)
        .add_attribute("debit", debit)
        .add_attribute("tax", tax)
        .add_attribute("tax_kind", format!("{kind:?}").to_lowercase());
    for (addr, share, is_burn) in &sinks {
        if *is_burn {
            resp = resp
                .add_attribute("sink_burn", share.to_string())
                .add_attribute("action", "burn");
        } else {
            resp = resp
                .add_attribute("sink", addr)
                .add_attribute("sink_amount", share.to_string());
        }
    }
    let _ = info;
    Ok((credit, tax, kind, resp))
}

pub fn execute_send_hook(
    contract: &Addr,
    sender: &Addr,
    amount: Uint128,
    msg: Binary,
) -> StdResult<CosmosMsg> {
    let receive = Cw20ReceiveMsg {
        sender: sender.to_string(),
        amount,
        msg,
    };
    receive.into_cosmos_msg(contract)
}

/// Burn helper used by `Burn` / `BurnFrom` (honest, no tax).
pub fn burn_tokens(
    storage: &mut dyn Storage,
    owner: &Addr,
    amount: Uint128,
) -> Result<TokenInfo, ContractError> {
    sub_balance(storage, owner, amount)?;
    TOKEN_INFO.update(storage, |mut info| -> Result<_, ContractError> {
        info.total_supply = info.total_supply.checked_sub(amount)?;
        Ok(info)
    })
}
