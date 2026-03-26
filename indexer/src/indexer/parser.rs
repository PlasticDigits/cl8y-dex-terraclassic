//! Tx log parsing for swaps, liquidity, and hooks.
//!
//! **Invariants:** duplicate wasm attribute keys use the last value (`wasm_attr_last`).
//! Parsing must not panic on adversarial attribute lists (see stress tests in `#[cfg(test)]`).
//! Full matrix: `docs/indexer-invariants.md`.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::config::Config;
use crate::db::queries::{
    assets, limit_order_fills, limit_order_lifecycle, liquidity, pairs, swap_events,
};
use crate::lcd::{Attribute, LcdClient, TxResponse};

use super::{
    asset_resolver, candle_builder, oracle, pair_discovery, position_tracker, trader_tracker,
};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Last value wins for duplicate keys (CosmWasm can emit repeated attribute keys).
fn wasm_attr_last<'a>(attributes: &'a [Attribute], key: &str) -> Option<&'a str> {
    attributes
        .iter()
        .rev()
        .find(|a| a.key == key)
        .map(|a| a.value.as_str())
}

fn wasm_contract_addr(attributes: &[Attribute]) -> Option<&str> {
    wasm_attr_last(attributes, "_contract_address")
        .or_else(|| wasm_attr_last(attributes, "contract_address"))
}

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
    effective_fee_bps: Option<i16>,
    pool_return_amount: Option<BigDecimal>,
    book_return_amount: Option<BigDecimal>,
    limit_book_offer_consumed: Option<BigDecimal>,
}

#[derive(Debug, Clone)]
struct ParsedLimitOrderFill {
    pair_address: String,
    order_id: i64,
    side: String,
    maker: String,
    price: BigDecimal,
    token0_amount: BigDecimal,
    token1_amount: BigDecimal,
    commission_amount: BigDecimal,
}

#[derive(Debug, Clone)]
struct ParsedLimitOrderPlacement {
    pair_address: String,
    order_id: i64,
}

#[derive(Debug, Clone)]
struct ParsedLimitOrderCancellation {
    pair_address: String,
    order_id: i64,
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
            if let Err(e) = process_swap(
                pool, lcd, config, swap, height, block_time, &tx.txhash, ustc_price,
            )
            .await
            {
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
            if let Err(e) =
                process_liquidity_event(pool, lcd, liq, height, block_time, &tx.txhash).await
            {
                tracing::warn!(
                    "Failed to process liquidity event in tx {} for pair {}: {}",
                    tx.txhash,
                    liq.pair_address,
                    e
                );
            }
        }

        let lo_fills = parse_limit_order_fills(tx);
        for fill in &lo_fills {
            if let Err(e) =
                process_limit_order_fill(pool, lcd, fill, height, block_time, &tx.txhash).await
            {
                tracing::warn!(
                    "Failed to process limit order fill in tx {} for pair {}: {}",
                    tx.txhash,
                    fill.pair_address,
                    e
                );
            }
        }

        let placements = parse_limit_order_placements(tx);
        for p in &placements {
            if let Err(e) =
                process_limit_order_placement(pool, lcd, p, height, block_time, &tx.txhash).await
            {
                tracing::warn!(
                    "Failed to process limit order placement in tx {} for pair {}: {}",
                    tx.txhash,
                    p.pair_address,
                    e
                );
            }
        }

        let cancellations = parse_limit_order_cancellations(tx);
        for c in &cancellations {
            if let Err(e) =
                process_limit_order_cancellation(pool, lcd, c, height, block_time, &tx.txhash).await
            {
                tracing::warn!(
                    "Failed to process limit order cancel in tx {} for pair {}: {}",
                    tx.txhash,
                    c.pair_address,
                    e
                );
            }
        }

        let hook_events = parse_hook_events(tx);
        for hook in &hook_events {
            if crate::db::queries::hook_events::hook_event_exists(
                pool,
                &tx.txhash,
                &hook.hook_address,
                &hook.action,
            )
            .await
            .unwrap_or(false)
            {
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
                tracing::warn!("Failed to save hook event in tx {}: {}", tx.txhash, e);
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
        None => match pair_discovery::discover_new_pair(pool, lcd, &swap.pair_address).await {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Could not discover pair {}: {}", swap.pair_address, e);
                return Ok(());
            }
        },
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

    let inserted = swap_events::insert_swap(
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
        swap.effective_fee_bps,
        &price,
        volume_usd.as_ref(),
        swap.pool_return_amount.as_ref(),
        swap.book_return_amount.as_ref(),
        swap.limit_book_offer_consumed.as_ref(),
    )
    .await?;
    if inserted.is_none() {
        return Ok(());
    }

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

        let attrs = &event.attributes;

        if wasm_attr_last(attrs, "action") != Some("swap") {
            continue;
        }

        let contract = wasm_contract_addr(attrs);
        let sender = wasm_attr_last(attrs, "sender");
        let offer_amount = wasm_attr_last(attrs, "offer_amount");
        let return_amount = wasm_attr_last(attrs, "return_amount");

        if let (Some(contract), Some(sender), Some(offer), Some(ret)) =
            (contract, sender, offer_amount, return_amount)
        {
            swaps.push(ParsedSwap {
                pair_address: contract.to_string(),
                sender: sender.to_string(),
                receiver: wasm_attr_last(attrs, "receiver").map(|s| s.to_string()),
                offer_asset: wasm_attr_last(attrs, "offer_asset")
                    .unwrap_or("")
                    .to_string(),
                ask_asset: wasm_attr_last(attrs, "ask_asset").unwrap_or("").to_string(),
                offer_amount: offer.parse().unwrap_or_default(),
                return_amount: ret.parse().unwrap_or_default(),
                spread_amount: wasm_attr_last(attrs, "spread_amount").and_then(|s| s.parse().ok()),
                commission_amount: wasm_attr_last(attrs, "commission_amount")
                    .and_then(|s| s.parse().ok()),
                effective_fee_bps: wasm_attr_last(attrs, "effective_fee_bps")
                    .and_then(|s| s.parse().ok()),
                pool_return_amount: wasm_attr_last(attrs, "pool_return_amount")
                    .and_then(|s| s.parse().ok()),
                book_return_amount: wasm_attr_last(attrs, "book_return_amount")
                    .and_then(|s| s.parse().ok()),
                limit_book_offer_consumed: wasm_attr_last(attrs, "limit_book_offer_consumed")
                    .and_then(|s| s.parse().ok()),
            });
        }
    }

    swaps
}

fn parse_limit_order_fills(tx: &TxResponse) -> Vec<ParsedLimitOrderFill> {
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
        let attrs = &event.attributes;
        if wasm_attr_last(attrs, "action") != Some("limit_order_fill") {
            continue;
        }

        let contract = wasm_contract_addr(attrs);
        let order_id_s = wasm_attr_last(attrs, "order_id");
        let side = wasm_attr_last(attrs, "side");
        let maker = wasm_attr_last(attrs, "maker");
        let price_s = wasm_attr_last(attrs, "price");
        let t0 = wasm_attr_last(attrs, "token0_amount");
        let t1 = wasm_attr_last(attrs, "token1_amount");
        let comm = wasm_attr_last(attrs, "commission_amount");

        let Some(contract) = contract else { continue };
        let Some(side) = side else { continue };
        if side != "bid" && side != "ask" {
            continue;
        }
        let Some(maker) = maker else { continue };

        let Some(oid) = order_id_s.and_then(|s| s.parse::<i64>().ok()) else {
            continue;
        };
        let Some(price) = price_s.and_then(|s| s.parse::<BigDecimal>().ok()) else {
            continue;
        };
        let Some(token0_amount) = t0.and_then(|s| s.parse::<BigDecimal>().ok()) else {
            continue;
        };
        let Some(token1_amount) = t1.and_then(|s| s.parse::<BigDecimal>().ok()) else {
            continue;
        };
        let Some(commission_amount) = comm.and_then(|s| s.parse::<BigDecimal>().ok()) else {
            continue;
        };

        out.push(ParsedLimitOrderFill {
            pair_address: contract.to_string(),
            order_id: oid,
            side: side.to_string(),
            maker: maker.to_string(),
            price,
            token0_amount,
            token1_amount,
            commission_amount,
        });
    }

    out
}

async fn process_limit_order_fill(
    pool: &PgPool,
    lcd: &LcdClient,
    fill: &ParsedLimitOrderFill,
    height: i64,
    block_time: DateTime<Utc>,
    tx_hash: &str,
) -> Result<(), BoxError> {
    let pair = match pairs::get_pair_by_address(pool, &fill.pair_address).await? {
        Some(p) => p,
        None => match pair_discovery::discover_new_pair(pool, lcd, &fill.pair_address).await {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Could not discover pair {}: {}", fill.pair_address, e);
                return Ok(());
            }
        },
    };

    if limit_order_fills::fill_exists(pool, tx_hash, pair.id, fill.order_id).await? {
        return Ok(());
    }

    let swap_event_id = limit_order_fills::swap_id_for_tx_pair(pool, tx_hash, pair.id).await?;

    limit_order_fills::insert_fill(
        pool,
        pair.id,
        swap_event_id,
        height,
        block_time,
        tx_hash,
        fill.order_id,
        &fill.side,
        &fill.maker,
        &fill.price,
        &fill.token0_amount,
        &fill.token1_amount,
        &fill.commission_amount,
    )
    .await?;

    Ok(())
}

fn parse_limit_order_placements(tx: &TxResponse) -> Vec<ParsedLimitOrderPlacement> {
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
        let attrs = &event.attributes;
        if wasm_attr_last(attrs, "action") != Some("place_limit_order") {
            continue;
        }
        let Some(contract) = wasm_contract_addr(attrs) else {
            continue;
        };
        let oid = wasm_attr_last(attrs, "order_id")
            .and_then(|s| s.parse::<i64>().ok())
            .or_else(|| {
                wasm_attr_last(attrs, "limit_order_placed").and_then(|s| s.parse::<i64>().ok())
            });
        let Some(order_id) = oid else {
            continue;
        };
        out.push(ParsedLimitOrderPlacement {
            pair_address: contract.to_string(),
            order_id,
        });
    }
    out
}

fn parse_limit_order_cancellations(tx: &TxResponse) -> Vec<ParsedLimitOrderCancellation> {
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
        let attrs = &event.attributes;
        if wasm_attr_last(attrs, "action") != Some("cancel_limit_order") {
            continue;
        }
        let Some(contract) = wasm_contract_addr(attrs) else {
            continue;
        };
        let oid = wasm_attr_last(attrs, "limit_order_cancelled").and_then(|s| s.parse::<i64>().ok());
        let Some(order_id) = oid else {
            continue;
        };
        out.push(ParsedLimitOrderCancellation {
            pair_address: contract.to_string(),
            order_id,
        });
    }
    out
}

async fn process_limit_order_placement(
    pool: &PgPool,
    lcd: &LcdClient,
    p: &ParsedLimitOrderPlacement,
    height: i64,
    block_time: DateTime<Utc>,
    tx_hash: &str,
) -> Result<(), BoxError> {
    let pair = match pairs::get_pair_by_address(pool, &p.pair_address).await? {
        Some(pair) => pair,
        None => match pair_discovery::discover_new_pair(pool, lcd, &p.pair_address).await {
            Ok(pair) => pair,
            Err(e) => {
                tracing::warn!("Could not discover pair {}: {}", p.pair_address, e);
                return Ok(());
            }
        },
    };

    limit_order_lifecycle::insert_placement(
        pool,
        pair.id,
        height,
        block_time,
        tx_hash,
        p.order_id,
        None,
        None,
        None,
        None,
    )
    .await?;
    Ok(())
}

async fn process_limit_order_cancellation(
    pool: &PgPool,
    lcd: &LcdClient,
    c: &ParsedLimitOrderCancellation,
    height: i64,
    block_time: DateTime<Utc>,
    tx_hash: &str,
) -> Result<(), BoxError> {
    let pair = match pairs::get_pair_by_address(pool, &c.pair_address).await? {
        Some(pair) => pair,
        None => match pair_discovery::discover_new_pair(pool, lcd, &c.pair_address).await {
            Ok(pair) => pair,
            Err(e) => {
                tracing::warn!("Could not discover pair {}: {}", c.pair_address, e);
                return Ok(());
            }
        },
    };

    limit_order_lifecycle::insert_cancellation(
        pool,
        pair.id,
        height,
        block_time,
        tx_hash,
        c.order_id,
        None,
    )
    .await?;
    Ok(())
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

        let attrs = &event.attributes;

        let (event_type, assets_key, lp_key) = match wasm_attr_last(attrs, "action") {
            Some("provide_liquidity") => ("add", "assets", "share"),
            Some("withdraw_liquidity") => ("remove", "refund_assets", "withdrawn_share"),
            _ => continue,
        };

        let contract = wasm_contract_addr(attrs);
        let provider = wasm_attr_last(attrs, "sender");
        let (a0, a1) = parse_asset_amounts(wasm_attr_last(attrs, assets_key).unwrap_or(""));
        let lp: BigDecimal = wasm_attr_last(attrs, lp_key)
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
        None => match pair_discovery::discover_new_pair(pool, lcd, &event.pair_address).await {
            Ok(p) => p,
            Err(e) => {
                tracing::warn!("Could not discover pair {}: {}", event.pair_address, e);
                return Ok(());
            }
        },
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

        let attrs = &event.attributes;

        let action = match wasm_attr_last(attrs, "action") {
            Some(a) if a.starts_with("after_swap_") => a,
            _ => continue,
        };

        let contract = wasm_contract_addr(attrs);

        if let Some(contract) = contract {
            hooks.push(ParsedHookEvent {
                hook_address: contract.to_string(),
                action: action.to_string(),
                amount: wasm_attr_last(attrs, "burn_amount")
                    .or_else(|| wasm_attr_last(attrs, "tax_amount"))
                    .and_then(|s| s.parse().ok()),
                token: wasm_attr_last(attrs, "burn_token")
                    .or_else(|| wasm_attr_last(attrs, "tax_token"))
                    .or_else(|| wasm_attr_last(attrs, "lp_token"))
                    .map(|s| s.to_string()),
                skipped: wasm_attr_last(attrs, "skipped").map(|s| s.to_string()),
                warning: wasm_attr_last(attrs, "warning").map(|s| s.to_string()),
            });
        }
    }

    hooks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lcd::{Attribute, Event, TxLog, TxResponse};

    fn wasm_tx(attrs: Vec<(&str, &str)>) -> TxResponse {
        let attributes: Vec<Attribute> = attrs
            .into_iter()
            .map(|(k, v)| Attribute {
                key: k.to_string(),
                value: v.to_string(),
            })
            .collect();
        TxResponse {
            height: "1".into(),
            txhash: "ABCDHASH".into(),
            logs: Some(vec![TxLog {
                events: vec![Event {
                    event_type: "wasm".into(),
                    attributes,
                }],
            }]),
            timestamp: None,
            events: None,
        }
    }

    fn wasm_tx_multi(events: Vec<Vec<(&str, &str)>>) -> TxResponse {
        let evs: Vec<Event> = events
            .into_iter()
            .map(|attrs| Event {
                event_type: "wasm".into(),
                attributes: attrs
                    .into_iter()
                    .map(|(k, v)| Attribute {
                        key: k.to_string(),
                        value: v.to_string(),
                    })
                    .collect(),
            })
            .collect();
        TxResponse {
            height: "1".into(),
            txhash: "ABCDHASH".into(),
            logs: Some(vec![TxLog { events: evs }]),
            timestamp: None,
            events: None,
        }
    }

    #[test]
    fn parse_swaps_extracts_swap_event() {
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1pair"),
            ("action", "swap"),
            ("sender", "terra1user"),
            ("offer_amount", "100"),
            ("return_amount", "95"),
            ("offer_asset", "uluna"),
            ("ask_asset", "uusd"),
        ]);
        let swaps = parse_swaps(&tx);
        assert_eq!(swaps.len(), 1);
        assert_eq!(swaps[0].pair_address, "terra1pair");
        assert_eq!(swaps[0].sender, "terra1user");
        assert_eq!(swaps[0].offer_amount.to_string(), "100");
        assert_eq!(swaps[0].return_amount.to_string(), "95");
    }

    #[test]
    fn parse_swaps_hybrid_and_fee_attrs() {
        let tx = wasm_tx(vec![
            ("contract_address", "terra1pair"),
            ("action", "swap"),
            ("sender", "terra1user"),
            ("offer_amount", "100"),
            ("return_amount", "95"),
            ("offer_asset", "uluna"),
            ("ask_asset", "uusd"),
            ("pool_return_amount", "40"),
            ("book_return_amount", "55"),
            ("limit_book_offer_consumed", "60"),
            ("effective_fee_bps", "30"),
        ]);
        let swaps = parse_swaps(&tx);
        assert_eq!(swaps.len(), 1);
        assert_eq!(
            swaps[0].pool_return_amount.as_ref().unwrap().to_string(),
            "40"
        );
        assert_eq!(
            swaps[0].book_return_amount.as_ref().unwrap().to_string(),
            "55"
        );
        assert_eq!(
            swaps[0]
                .limit_book_offer_consumed
                .as_ref()
                .unwrap()
                .to_string(),
            "60"
        );
        assert_eq!(swaps[0].effective_fee_bps, Some(30));
    }

    #[test]
    fn parse_limit_order_fills_extracts_events() {
        let tx = wasm_tx_multi(vec![
            vec![
                ("contract_address", "terra1pair"),
                ("action", "limit_order_fill"),
                ("order_id", "7"),
                ("side", "bid"),
                ("maker", "terra1maker"),
                ("price", "1.5"),
                ("token0_amount", "100"),
                ("token1_amount", "150"),
                ("commission_amount", "1"),
            ],
            vec![
                ("contract_address", "terra1pair"),
                ("action", "limit_order_fill"),
                ("order_id", "8"),
                ("side", "ask"),
                ("maker", "terra1mk2"),
                ("price", "2"),
                ("token0_amount", "10"),
                ("token1_amount", "20"),
                ("commission_amount", "0"),
            ],
        ]);
        let fills = parse_limit_order_fills(&tx);
        assert_eq!(fills.len(), 2);
        assert_eq!(fills[0].order_id, 7);
        assert_eq!(fills[0].side, "bid");
        assert_eq!(fills[1].order_id, 8);
        assert_eq!(fills[1].side, "ask");
    }

    #[test]
    fn parse_limit_order_placements_and_cancellations() {
        let tx = wasm_tx_multi(vec![
            vec![
                ("_contract_address", "terra1pair"),
                ("action", "place_limit_order"),
                ("order_id", "99"),
                ("limit_order_placed", "99"),
            ],
            vec![
                ("_contract_address", "terra1pair"),
                ("action", "cancel_limit_order"),
                ("limit_order_cancelled", "99"),
            ],
        ]);
        let p = parse_limit_order_placements(&tx);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].order_id, 99);
        let c = parse_limit_order_cancellations(&tx);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].order_id, 99);
    }

    #[test]
    fn parse_swaps_duplicate_attr_keys_last_wins() {
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1pair"),
            ("action", "swap"),
            ("sender", "terra1user"),
            ("offer_amount", "1"),
            ("offer_amount", "200"),
            ("return_amount", "95"),
        ]);
        let swaps = parse_swaps(&tx);
        assert_eq!(swaps.len(), 1);
        assert_eq!(swaps[0].offer_amount.to_string(), "200");
    }

    #[test]
    fn parse_swaps_ignores_non_swap_action() {
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1pair"),
            ("action", "provide_liquidity"),
        ]);
        assert!(parse_swaps(&tx).is_empty());
    }

    #[test]
    fn parse_asset_amounts_parses_pair() {
        let (a0, a1) = parse_asset_amounts("uluna 1000, uusd 950");
        assert_eq!(a0, BigDecimal::from(1000));
        assert_eq!(a1, BigDecimal::from(950));
    }

    #[test]
    fn parse_liquidity_add() {
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1pair"),
            ("action", "provide_liquidity"),
            ("sender", "terra1lp"),
            ("assets", "uluna 100, uusd 200"),
            ("share", "50"),
        ]);
        let evs = parse_liquidity_events(&tx);
        assert_eq!(evs.len(), 1);
        assert_eq!(evs[0].event_type, "add");
        assert_eq!(evs[0].asset_0_amount, BigDecimal::from(100));
        assert_eq!(evs[0].asset_1_amount, BigDecimal::from(200));
    }

    #[test]
    fn parse_hook_after_swap() {
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1hook"),
            ("action", "after_swap_burn"),
            ("burn_amount", "10"),
        ]);
        let hooks = parse_hook_events(&tx);
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].hook_address, "terra1hook");
        assert_eq!(hooks[0].action, "after_swap_burn");
    }

    #[test]
    fn wasm_attr_last_finds_last_duplicate() {
        let attrs = vec![
            Attribute {
                key: "k".into(),
                value: "first".into(),
            },
            Attribute {
                key: "k".into(),
                value: "second".into(),
            },
        ];
        assert_eq!(wasm_attr_last(&attrs, "k"), Some("second"));
    }

    /// Deterministic pseudo-fuzz: must never panic (denial-of-service via indexer parse path).
    #[test]
    fn parse_asset_amounts_ascii_stress_no_panic() {
        use rand::rngs::StdRng;
        use rand::{Rng, SeedableRng};
        let mut rng = StdRng::seed_from_u64(0xC0FFEE);
        for _ in 0..12_000 {
            let len = rng.gen_range(0..512usize);
            let s: String = (0..len)
                .map(|_| rng.gen_range(b'!'..=b'~') as char)
                .collect();
            let _ = parse_asset_amounts(&s);
        }
    }

    #[test]
    fn parse_wasm_event_pipelines_stress_no_panic() {
        use rand::rngs::StdRng;
        use rand::{Rng, SeedableRng};
        let mut rng = StdRng::seed_from_u64(0xDECAF);
        for seed_i in 0..800u32 {
            let n = rng.gen_range(0..80usize);
            let mut attrs = Vec::with_capacity(n);
            for _ in 0..n {
                let kl = rng.gen_range(0..32usize);
                let vl = rng.gen_range(0..64usize);
                let key: String = (0..kl)
                    .map(|_| (b'a' + rng.gen_range(0..26)) as char)
                    .collect();
                let val: String = (0..vl)
                    .map(|_| rng.gen_range(b'0'..=b'z') as char)
                    .collect();
                attrs.push(Attribute { key, value: val });
            }
            let tx = TxResponse {
                height: "1".into(),
                txhash: format!("hash{}", seed_i),
                logs: Some(vec![TxLog {
                    events: vec![Event {
                        event_type: "wasm".into(),
                        attributes: attrs,
                    }],
                }]),
                timestamp: None,
                events: None,
            };
            let _ = parse_swaps(&tx);
            let _ = parse_liquidity_events(&tx);
            let _ = parse_hook_events(&tx);
            let _ = parse_limit_order_fills(&tx);
        }
    }
}
