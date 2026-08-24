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

/// **#609 / T592-7:** manager-directory wallets skip Transfer, Buy, and Sell tax.
/// Either side exempt is enough (same as the existing transfer rule).
pub fn is_manager_directory_tax_skip(storage: &dyn Storage, from: &Addr, to: &Addr) -> bool {
    is_manager_exempt(storage, from) || is_manager_exempt(storage, to)
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

pub fn is_official_router(config: &Config, addr: &Addr) -> bool {
    config.router.as_ref() == Some(addr)
}

/// `trader` on `Cw20HookMsg::Swap` — the official router already sets this to the
/// user who invoked `execute_swap_operations` (fee-discount path). Not trusted
/// until `from` is [`is_official_router`].
pub fn swap_hook_trader(msg: &Binary) -> Option<String> {
    match cosmwasm_std::from_json::<Cw20HookMsg>(msg) {
        Ok(Cw20HookMsg::Swap { trader, .. }) => trader,
        _ => None,
    }
}

/// Official-router `Send+Swap` to a listed pair (**T592-13** / #607 improved option 2).
pub fn is_router_sell_hop(
    storage: &dyn Storage,
    config: &Config,
    from: &Addr,
    to: &Addr,
    send_msg: Option<&Binary>,
) -> bool {
    is_listed_pair(storage, to)
        && send_msg.is_some_and(is_swap_send_hook)
        && is_official_router(config, from)
}

/// Authenticated hop trader: honor `Swap.trader` only when `from` is the stamped
/// official router. Pair-direct ignores the field (extra-debit `from`). Protocol-exempt
/// or missing trader is `None` here; execute fail-closes with [`ContractError::RouterTraderRequired`].
pub fn hop_trader_addr(
    storage: &dyn Storage,
    self_addr: &Addr,
    config: &Config,
    from: &Addr,
    to: &Addr,
    send_msg: Option<&Binary>,
) -> Option<Addr> {
    if !is_router_sell_hop(storage, config, from, to, send_msg) {
        return None;
    }
    let raw = send_msg.and_then(swap_hook_trader)?;
    let trader = Addr::unchecked(raw);
    if trader == *from || is_protocol_exempt(storage, self_addr, &trader) {
        return None;
    }
    Some(trader)
}

fn require_hop_trader(
    storage: &dyn Storage,
    self_addr: &Addr,
    config: &Config,
    from: &Addr,
    to: &Addr,
    send_msg: Option<&Binary>,
) -> Result<Addr, ContractError> {
    hop_trader_addr(storage, self_addr, config, from, to, send_msg)
        .ok_or(ContractError::RouterTraderRequired {})
}

/// Economic kind **before** the manager-directory tax skip (**T592-7**, **#609**) + **T592-13**.
///
/// Launch guards (`trading_enabled`, cooldown, `max_wallet`) use this so exemption
/// is tax-only and does not disable **T592-11**.
///
/// - **Sell** — `Send` to a registered listed pair whose hook is `Cw20HookMsg::Swap`
///   **and** (`from` is not protocol-exempt **or** `from` is the official router).
///   Pair-direct extra-debits `from`. Router hop extra-debits the authenticated
///   `Swap.trader` (official router already passes the user; **H-01** / no FoT).
///   Pair is credited exactly `amount` (inbound 1:1 / **T592-1**).
/// - **Buy** — `Transfer`/`Send` **from** a registered listed pair **or** the official
///   router to a non-protocol-exempt recipient. Debit `amount`; trader + sinks = `amount`.
///   Pair→router stays 1:1. Pair→EOA `Transfer` is also withdraw / limit refund (same
///   primitive). Provide (`TransferFrom`) and limit `PlaceLimitOrder*` `Send` stay 1:1.
/// - **Transfer** — TransferTax SKU, neither side protocol- or manager-exempt.
/// - **Honest** — everything else. Do not un-exempt the router (option 3 bricks hops).
pub fn classify_trade(
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
            if is_swap_send_hook(msg)
                && (!is_protocol_exempt(storage, self_addr, from)
                    || is_official_router(config, from))
            {
                return (TaxKind::Sell, config.sell_bps);
            }
        }
        return (TaxKind::Honest, 0);
    }

    if from_pair && !is_protocol_exempt(storage, self_addr, to) {
        return (TaxKind::Buy, config.buy_bps);
    }

    // Router → user (swap output / leftover return). Pair→router stayed 1:1 (**T592-1**).
    if is_official_router(config, from) && !is_protocol_exempt(storage, self_addr, to) {
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

/// Tax kind after **#609** manager-directory skip (Buy / Sell / Transfer → Honest).
/// Router-hop sell also skips when the authenticated `trader` is directory-exempt.
pub fn classify(
    storage: &dyn Storage,
    self_addr: &Addr,
    from: &Addr,
    to: &Addr,
    send_msg: Option<&Binary>,
    features: &Features,
    config: &Config,
) -> (TaxKind, u16) {
    let (kind, bps) = classify_trade(storage, self_addr, from, to, send_msg, features, config);
    let hop = hop_trader_addr(storage, self_addr, config, from, to, send_msg);
    apply_manager_directory_tax_skip(storage, from, to, hop.as_ref(), kind, bps)
}

fn apply_manager_directory_tax_skip(
    storage: &dyn Storage,
    from: &Addr,
    to: &Addr,
    hop_trader: Option<&Addr>,
    kind: TaxKind,
    bps: u16,
) -> (TaxKind, u16) {
    if matches!(kind, TaxKind::Sell | TaxKind::Buy | TaxKind::Transfer)
        && (is_manager_directory_tax_skip(storage, from, to)
            || hop_trader.is_some_and(|t| is_manager_exempt(storage, t)))
    {
        return (TaxKind::Honest, 0);
    }
    (kind, bps)
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
    let hop = hop_trader_addr(deps.storage, self_addr, &config, from, to, send_msg);
    let tax = tax_amount(amount, bps);
    let router_sell = matches!(kind, TaxKind::Sell) && is_official_router(&config, from);
    let (debit, credit) = match kind {
        TaxKind::Sell if router_sell => (amount, amount),
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
        hop_trader: hop.clone(),
        hop_trader_debit: if router_sell && !tax.is_zero() {
            Some(tax)
        } else {
            None
        },
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
    hop_trader: Option<&Addr>,
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
            // Router sell: from=router and to=pair are not cooldown subjects — check trader.
            if let Some(trader) = hop_trader {
                check_cooldown(storage, &guards, env, self_addr, trader)?;
            }
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

#[allow(clippy::too_many_arguments)]
fn record_trade_blocks(
    storage: &mut dyn Storage,
    features: &Features,
    env: &Env,
    kind: &TaxKind,
    self_addr: &Addr,
    from: &Addr,
    to: &Addr,
    hop_trader: Option<&Addr>,
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
    if let Some(trader) = hop_trader {
        if is_cooldown_subject(storage, self_addr, trader) {
            LAST_TRADE_BLOCK.save(storage, trader, &env.block.height)?;
        }
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
    // Guards use economic kind so #609 tax skip cannot disable T592-11.
    let (trade_kind, trade_bps) = classify_trade(
        deps.storage,
        self_addr,
        from,
        to,
        send_msg,
        &features,
        &config,
    );
    let hop = if is_router_sell_hop(deps.storage, &config, from, to, send_msg) {
        Some(require_hop_trader(
            deps.storage,
            self_addr,
            &config,
            from,
            to,
            send_msg,
        )?)
    } else {
        None
    };
    let (kind, bps) = apply_manager_directory_tax_skip(
        deps.storage,
        from,
        to,
        hop.as_ref(),
        trade_kind.clone(),
        trade_bps,
    );
    let tax = tax_amount(amount, bps);
    let router_sell = matches!(kind, TaxKind::Sell) && hop.is_some();
    let (debit, credit) = match kind {
        TaxKind::Sell if router_sell => (amount, amount),
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
        hop.as_ref(),
        &trade_kind,
        credit,
        to_bal_after,
    )?;

    BALANCES.save(deps.storage, from, &from_bal_after)?;
    add_balance(deps.storage, to, credit)?;
    if router_sell {
        if let Some(trader) = &hop {
            sub_balance(deps.storage, trader, tax)?;
        }
    }

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

    record_trade_blocks(
        deps.storage,
        &features,
        env,
        &trade_kind,
        self_addr,
        from,
        to,
        hop.as_ref(),
    )?;

    let mut resp = Response::new()
        .add_attribute("action", "transfer")
        .add_attribute("from", from)
        .add_attribute("to", to)
        .add_attribute("amount", credit)
        .add_attribute("declared", amount)
        .add_attribute("debit", debit)
        .add_attribute("tax", tax)
        .add_attribute("tax_kind", format!("{kind:?}").to_lowercase());
    if let Some(trader) = &hop {
        resp = resp.add_attribute("hop_trader", trader);
        if router_sell && !tax.is_zero() {
            resp = resp.add_attribute("hop_trader_debit", tax);
        }
    }
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
