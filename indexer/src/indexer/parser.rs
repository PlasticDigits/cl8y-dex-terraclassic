use std::collections::HashMap;

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::config::Config;
use crate::db::queries::{assets, liquidity, pairs, swap_events};
use crate::lcd::{LcdClient, TxResponse};

use super::{asset_resolver, candle_builder, oracle, pair_discovery, position_tracker, trader_tracker};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

#[derive(Debug, Clone)]
struct ParsedSwap {
    pair_address: String,
    sender: String,
    receiver: Option<String>,
    offer_asset: String,
    ask_asset: String,
    offer_amount: BigDecimal,
    return_amount: BigDecimal,
    spread_amount: Option<BigDecimal>,
    commission_amount: Option<BigDecimal>,
}

pub async fn process_block_txs(
    pool: &PgPool,
    lcd: &LcdClient,
    config: &Config,
    txs: &[TxResponse],
    height: i64,
    block_time: DateTime<Utc>,
    ustc_price: &oracle::SharedPrice,
) -> Result<(), BoxError> {
    for tx in txs {
        let swaps = parse_swaps(tx);
        for swap in &swaps {
            if let Err(e) = process_swap(pool, lcd, config, swap, height, block_time, &tx.txhash, ustc_price).await {
                tracing::warn!(
                    "Failed to process swap in tx {} for pair {}: {}",
                    tx.txhash,
                    swap.pair_address,
                    e
                );
            }
        }

        let liq_events = parse_liquidity_events(tx);
        for liq in &liq_events {
            if let Err(e) = process_liquidity_event(pool, lcd, liq, height, block_time, &tx.txhash).await {
                tracing::warn!(
                    "Failed to process liquidity event in tx {} for pair {}: {}",
                    tx.txhash,
                    liq.pair_address,
                    e
                );
            }
        }

        let hook_events = parse_hook_events(tx);
        for hook in &hook_events {
            if crate::db::queries::hook_events::hook_event_exists(pool, &tx.txhash, &hook.hook_address, &hook.action).await.unwrap_or(false) {
                continue;
            }
            if let Err(e) = crate::db::queries::hook_events::insert_hook_event(
                pool,
                &tx.txhash,
                &hook.hook_address,
                &hook.action,
                hook.amount.as_ref(),
                hook.token.as_deref(),
                hook.skipped.as_deref(),
                hook.warning.as_deref(),
                height,
                block_time,
            )
            .await
            {
                tracing::warn!(
                    "Failed to save hook event in tx {}: {}",
                    tx.txhash,
                    e
                );
            }
        }
    }
    Ok(())
}

async fn process_swap(
    pool: &PgPool,
    lcd: &LcdClient,
    config: &Config,
    swap: &ParsedSwap,
    height: i64,
    block_time: DateTime<Utc>,
    tx_hash: &str,
    ustc_price: &oracle::SharedPrice,
) -> Result<(), BoxError> {
    let pair = match pairs::get_pair_by_address(pool, &swap.pair_address).await? {
        Some(p) => p,
        None => {
            match pair_discovery::discover_new_pair(pool, lcd, &swap.pair_address).await {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!("Could not discover pair {}: {}", swap.pair_address, e);
                    return Ok(());
                }
            }
        }
    };

    if swap_events::trade_exists(pool, tx_hash, pair.id).await? {
        return Ok(());
    }

    let offer_asset_id = asset_resolver::resolve_asset_str(pool, lcd, &swap.offer_asset).await?;
    let ask_asset_id = asset_resolver::resolve_asset_str(pool, lcd, &swap.ask_asset).await?;

    let price = if swap.offer_amount > BigDecimal::from(0) {
        &swap.return_amount / &swap.offer_amount
    } else {
        BigDecimal::from(0)
    };

    let volume_usd = compute_volume_usd(
        pool,
        config,
        ustc_price,
        offer_asset_id,
        ask_asset_id,
        &swap.offer_amount,
        &swap.return_amount,
    )
    .await;

    swap_events::insert_swap(
        pool,
        pair.id,
        height,
        block_time,
        tx_hash,
        &swap.sender,
        swap.receiver.as_deref(),
        offer_asset_id,
        ask_asset_id,
        &swap.offer_amount,
        &swap.return_amount,
        swap.spread_amount.as_ref(),
        swap.commission_amount.as_ref(),
        None,
        &price,
        volume_usd.as_ref(),
    )
    .await?;

    candle_builder::update_candles_for_swap(
        pool,
        pair.id,
        block_time,
        &price,
        &swap.offer_amount,
        &swap.return_amount,
    )
    .await?;

    trader_tracker::update_trader_on_swap(pool, &swap.sender, &swap.offer_amount).await?;

    position_tracker::update_position_on_swap(
        pool,
        pair.id,
        pair.asset_0_id,
        &swap.sender,
        offer_asset_id,
        &swap.offer_amount,
        &swap.return_amount,
        swap.spread_amount.as_ref(),
        swap.commission_amount.as_ref(),
    )
    .await?;

    Ok(())
}

fn is_ustc_asset(asset: &assets::AssetRow, ustc_denom: Option<&str>) -> bool {
    if let Some(denom) = &asset.denom {
        if denom == "uusd" {
            return true;
        }
    }
    if let Some(configured) = ustc_denom {
        if let Some(addr) = &asset.contract_address {
            if addr == configured {
                return true;
            }
        }
        if let Some(denom) = &asset.denom {
            if denom == configured {
                return true;
            }
        }
    }
    false
}

async fn compute_volume_usd(
    pool: &PgPool,
    config: &Config,
    ustc_price: &oracle::SharedPrice,
    offer_asset_id: i32,
    ask_asset_id: i32,
    offer_amount: &BigDecimal,
    return_amount: &BigDecimal,
) -> Option<BigDecimal> {
    let price_usd = ustc_price.read().await.clone()?;

    let offer_asset = assets::get_asset_by_id(pool, offer_asset_id).await.ok()??;
    let ask_asset = assets::get_asset_by_id(pool, ask_asset_id).await.ok()??;

    let ustc_denom = config.ustc_denom.as_deref();
    let decimals_factor = BigDecimal::from(1_000_000i64);

    if is_ustc_asset(&offer_asset, ustc_denom) {
        let human_amount = offer_amount / &decimals_factor;
        return Some(human_amount * &price_usd);
    }

    if is_ustc_asset(&ask_asset, ustc_denom) {
        let human_amount = return_amount / &decimals_factor;
        return Some(human_amount * &price_usd);
    }

    None
}

fn parse_swaps(tx: &TxResponse) -> Vec<ParsedSwap> {
    let mut swaps = Vec::new();

    let events: Vec<&crate::lcd::Event> = if let Some(logs) = &tx.logs {
        logs.iter().flat_map(|l| l.events.iter()).collect()
    } else if let Some(evts) = &tx.events {
        evts.iter().collect()
    } else {
        Vec::new()
    };

    for event in &events {
        if event.event_type != "wasm" {
            continue;
        }

        let attrs: HashMap<&str, &str> = event
            .attributes
            .iter()
            .map(|a| (a.key.as_str(), a.value.as_str()))
            .collect();

        if attrs.get("action").copied() != Some("swap") {
            continue;
        }

        let contract = attrs
            .get("_contract_address")
            .or(attrs.get("contract_address"));
        let sender = attrs.get("sender");
        let offer_amount = attrs.get("offer_amount");
        let return_amount = attrs.get("return_amount");

        if let (Some(contract), Some(sender), Some(offer), Some(ret)) =
            (contract, sender, offer_amount, return_amount)
        {
            swaps.push(ParsedSwap {
                pair_address: contract.to_string(),
                sender: sender.to_string(),
                receiver: attrs.get("receiver").map(|s| s.to_string()),
                offer_asset: attrs.get("offer_asset").unwrap_or(&"").to_string(),
                ask_asset: attrs.get("ask_asset").unwrap_or(&"").to_string(),
                offer_amount: offer.parse().unwrap_or_default(),
                return_amount: ret.parse().unwrap_or_default(),
                spread_amount: attrs.get("spread_amount").and_then(|s| s.parse().ok()),
                commission_amount: attrs.get("commission_amount").and_then(|s| s.parse().ok()),
            });
        }
    }

    swaps
}

#[derive(Debug, Clone)]
struct ParsedLiquidityEvent {
    pair_address: String,
    provider: String,
    event_type: String,
    asset_0_amount: BigDecimal,
    asset_1_amount: BigDecimal,
    lp_amount: BigDecimal,
}

fn parse_liquidity_events(tx: &TxResponse) -> Vec<ParsedLiquidityEvent> {
    let mut out = Vec::new();

    let events: Vec<&crate::lcd::Event> = if let Some(logs) = &tx.logs {
        logs.iter().flat_map(|l| l.events.iter()).collect()
    } else if let Some(evts) = &tx.events {
        evts.iter().collect()
    } else {
        Vec::new()
    };

    for event in &events {
        if event.event_type != "wasm" {
            continue;
        }

        let attrs: HashMap<&str, &str> = event
            .attributes
            .iter()
            .map(|a| (a.key.as_str(), a.value.as_str()))
            .collect();

        let (event_type, assets_key, lp_key) = match attrs.get("action").copied() {
            Some("provide_liquidity") => ("add", "assets", "share"),
            Some("withdraw_liquidity") => ("remove", "refund_assets", "withdrawn_share"),
            _ => continue,
        };

        let contract = attrs
            .get("_contract_address")
            .or(attrs.get("contract_address"));
        let provider = attrs.get("sender");
        let (a0, a1) = parse_asset_amounts(attrs.get(assets_key).copied().unwrap_or(""));
        let lp: BigDecimal = attrs
            .get(lp_key)
            .and_then(|s| s.parse().ok())
            .unwrap_or_default();

        if let (Some(contract), Some(provider)) = (contract, provider) {
            out.push(ParsedLiquidityEvent {
                pair_address: contract.to_string(),
                provider: provider.to_string(),
                event_type: event_type.to_string(),
                asset_0_amount: a0,
                asset_1_amount: a1,
                lp_amount: lp,
            });
        }
    }

    out
}

/// Parse the stringified assets attribute emitted by pair contracts.
/// Format: `"<addr_or_denom> <amount>, <addr_or_denom> <amount>"`
fn parse_asset_amounts(assets_str: &str) -> (BigDecimal, BigDecimal) {
    let parts: Vec<&str> = assets_str.split(", ").collect();
    let amount_0 = parts
        .first()
        .and_then(|p| p.rsplit_once(' '))
        .and_then(|(_, amt)| amt.parse().ok())
        .unwrap_or_default();
    let amount_1 = parts
        .get(1)
        .and_then(|p| p.rsplit_once(' '))
        .and_then(|(_, amt)| amt.parse().ok())
        .unwrap_or_default();
    (amount_0, amount_1)
}

async fn process_liquidity_event(
    pool: &PgPool,
    lcd: &LcdClient,
    event: &ParsedLiquidityEvent,
    height: i64,
    block_time: DateTime<Utc>,
    tx_hash: &str,
) -> Result<(), BoxError> {
    let pair = match pairs::get_pair_by_address(pool, &event.pair_address).await? {
        Some(p) => p,
        None => {
            match pair_discovery::discover_new_pair(pool, lcd, &event.pair_address).await {
                Ok(p) => p,
                Err(e) => {
                    tracing::warn!("Could not discover pair {}: {}", event.pair_address, e);
                    return Ok(());
                }
            }
        }
    };

    if liquidity::liquidity_event_exists(pool, tx_hash, pair.id, &event.event_type).await? {
        return Ok(());
    }

    liquidity::insert_liquidity_event(
        pool,
        pair.id,
        height,
        block_time,
        tx_hash,
        &event.provider,
        &event.event_type,
        &event.asset_0_amount,
        &event.asset_1_amount,
        &event.lp_amount,
    )
    .await?;

    Ok(())
}

#[derive(Debug, Clone)]
struct ParsedHookEvent {
    hook_address: String,
    action: String,
    amount: Option<BigDecimal>,
    token: Option<String>,
    skipped: Option<String>,
    warning: Option<String>,
}

fn parse_hook_events(tx: &TxResponse) -> Vec<ParsedHookEvent> {
    let mut hooks = Vec::new();

    let events: Vec<&crate::lcd::Event> = if let Some(logs) = &tx.logs {
        logs.iter().flat_map(|l| l.events.iter()).collect()
    } else if let Some(evts) = &tx.events {
        evts.iter().collect()
    } else {
        Vec::new()
    };

    for event in &events {
        if event.event_type != "wasm" {
            continue;
        }

        let attrs: HashMap<&str, &str> = event
            .attributes
            .iter()
            .map(|a| (a.key.as_str(), a.value.as_str()))
            .collect();

        let action = match attrs.get("action").copied() {
            Some(a) if a.starts_with("after_swap_") => a,
            _ => continue,
        };

        let contract = attrs
            .get("_contract_address")
            .or(attrs.get("contract_address"));

        if let Some(contract) = contract {
            hooks.push(ParsedHookEvent {
                hook_address: contract.to_string(),
                action: action.to_string(),
                amount: attrs
                    .get("burn_amount")
                    .or(attrs.get("tax_amount"))
                    .and_then(|s| s.parse().ok()),
                token: attrs
                    .get("burn_token")
                    .or(attrs.get("tax_token"))
                    .or(attrs.get("lp_token"))
                    .map(|s| s.to_string()),
                skipped: attrs.get("skipped").map(|s| s.to_string()),
                warning: attrs.get("warning").map(|s| s.to_string()),
            });
        }
    }

    hooks
}
