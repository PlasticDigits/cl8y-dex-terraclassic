//! Tx log parsing for swaps, liquidity, and hooks.
//!
//! **Invariants:** duplicate wasm attribute keys use the last value (`wasm_attr_last`) where a
//! single logical value is intended. **Lifecycle wasm** (`limit_order_expired_parked`,
//! `claim_expired_limit_order`, `limit_order_fill`) may share one `wasm` event with other actions
//! when LCD output flattens attribute streams — those parsers scan **every** `action` occurrence
//! (GitLab #141, #269).
//! LocalTerra also emits standalone lifecycle rows as **`wasm-wasm`** events (same attribute keys).
//! Parsing must not panic on adversarial attribute lists (see stress tests in `#[cfg(test)]`).
//! Full matrix: `docs/indexer-invariants.md`.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::PgPool;
use std::str::FromStr;

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
    // GitLab #285: only the runtime-reserved `_contract_address` (stamped by wasmd, which
    // contracts cannot set) scopes a lifecycle event. The no-underscore `contract_address`
    // is an ordinary attribute any contract can forge, so it must never be trusted as the
    // emitter — treat it as data only.
    wasm_attr_last(attributes, "_contract_address")
}

fn is_wasm_contract_addr_key(key: &str) -> bool {
    // GitLab #285: scope events by the runtime-reserved key only (see `wasm_contract_addr`).
    key == "_contract_address"
}

/// CosmWasm lifecycle rows on LocalTerra LCD: merged into `wasm` or emitted as `wasm-wasm`.
fn is_wasm_lifecycle_event_type(event_type: &str) -> bool {
    event_type == "wasm" || event_type == "wasm-wasm"
}

/// Last `_contract_address` / `contract_address` **before** `idx` (exclusive), document order.
///
/// Some LCD/REST paths flatten multiple logical CosmWasm wasm emissions into **one** `wasm`
/// event attribute stream. Per-logical-event contract scoping is then recovered by scanning
/// backward from each `action` occurrence (GitLab #141).
fn wasm_contract_addr_before(attrs: &[Attribute], idx: usize) -> Option<&str> {
    attrs[..idx]
        .iter()
        .rev()
        .find(|a| is_wasm_contract_addr_key(a.key.as_str()))
        .map(|a| a.value.as_str())
}

/// Key/value pairs after `action_pos` until the next logical boundary (`action` or `_contract_address`).
fn wasm_kv_map_after_action(
    attrs: &[Attribute],
    action_pos: usize,
) -> std::collections::HashMap<&str, &str> {
    let mut m = std::collections::HashMap::new();
    let mut i = action_pos.saturating_add(1);
    while i < attrs.len() {
        let k = attrs[i].key.as_str();
        if k == "action" || k == "_contract_address" {
            break;
        }
        m.insert(k, attrs[i].value.as_str());
        i += 1;
    }
    m
}

fn parse_limit_order_expired_parked_from_wasm_attrs(
    attrs: &[Attribute],
) -> Vec<ParsedLimitOrderExpiredParked> {
    let mut out = Vec::new();
    for (i, a) in attrs.iter().enumerate() {
        if a.key != "action" || a.value != "limit_order_expired_parked" {
            continue;
        }
        let Some(contract) = wasm_contract_addr_before(attrs, i) else {
            continue;
        };
        let seg = wasm_kv_map_after_action(attrs, i);
        let Some(order_id_s) = seg.get("order_id").copied() else {
            continue;
        };
        let Ok(order_id) = order_id_s.parse::<i64>() else {
            continue;
        };
        let Some(rem_s) = seg.get("remaining").copied() else {
            continue;
        };
        let Some(remaining) = BigDecimal::from_str(rem_s).ok() else {
            continue;
        };
        out.push(ParsedLimitOrderExpiredParked {
            pair_address: contract.to_string(),
            order_id,
            remaining,
        });
    }
    out
}

fn parse_claim_expired_limit_orders_columnar(
    attrs: &[Attribute],
) -> Option<Vec<ParsedClaimExpiredLimitOrder>> {
    let has_batch = attrs
        .iter()
        .any(|a| a.key == "action" && a.value == "claim_expired_limit_orders_batch");
    let claim_count = attrs
        .iter()
        .filter(|a| a.key == "action" && a.value == "claim_expired_limit_order")
        .count();
    if !has_batch && claim_count <= 1 {
        return None;
    }
    let contract = attrs
        .iter()
        .find(|a| is_wasm_contract_addr_key(a.key.as_str()))
        .map(|a| a.value.as_str())?;

    let order_ids: Vec<i64> = wasm_attr_values(attrs, "order_id")
        .iter()
        .filter_map(|s| s.parse().ok())
        .collect();
    if order_ids.is_empty() {
        return None;
    }

    let n = order_ids.len();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        out.push(ParsedClaimExpiredLimitOrder {
            pair_address: contract.to_string(),
            order_id: order_ids[i],
        });
    }
    Some(out)
}

fn parse_claim_expired_limit_orders_from_wasm_attrs(
    attrs: &[Attribute],
) -> Vec<ParsedClaimExpiredLimitOrder> {
    if let Some(columnar) = parse_claim_expired_limit_orders_columnar(attrs) {
        return columnar;
    }

    let mut out = Vec::new();
    for (i, a) in attrs.iter().enumerate() {
        if a.key != "action" || a.value != "claim_expired_limit_order" {
            continue;
        }
        let Some(contract) = wasm_contract_addr_before(attrs, i) else {
            continue;
        };
        let seg = wasm_kv_map_after_action(attrs, i);
        let Some(order_id_s) = seg.get("order_id").copied() else {
            continue;
        };
        let Ok(order_id) = order_id_s.parse::<i64>() else {
            continue;
        };
        out.push(ParsedClaimExpiredLimitOrder {
            pair_address: contract.to_string(),
            order_id,
        });
    }
    out
}

/// Parsed wasm `swap` event; same shape as persisted via [`crate::db::queries::swap_events::insert_swap`].
#[derive(Debug, Clone)]
pub struct ParsedSwap {
    pub pair_address: String,
    /// GitLab #287: 0-based ordinal of this swap among swaps on the SAME pair within the tx,
    /// taken from parser walk order. Makes `(tx_hash, pair_id, swap_index)` unique so multiple
    /// swaps on one pair in a single tx are stored instead of collapsed.
    pub swap_index: i32,
    pub sender: String,
    pub receiver: Option<String>,
    pub offer_asset: String,
    pub ask_asset: String,
    pub offer_amount: BigDecimal,
    pub return_amount: BigDecimal,
    pub spread_amount: Option<BigDecimal>,
    pub commission_amount: Option<BigDecimal>,
    pub effective_fee_bps: Option<i16>,
    pub pool_return_amount: Option<BigDecimal>,
    pub book_return_amount: Option<BigDecimal>,
    pub limit_book_offer_consumed: Option<BigDecimal>,
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
    /// GitLab #316: 0-based ordinal of the swap (on this pair, within the tx) that produced this
    /// fill, derived from parser walk order — maker fills are emitted before their `swap` action
    /// in the same execute. Links the fill to the correct `swap_events` row via
    /// `(tx_hash, pair_id, swap_index)` when a tx has multiple swaps on the same pair. Assigned in
    /// [`parse_limit_order_fills`]; the segment/columnar builders leave it 0.
    swap_index: i32,
}

#[derive(Debug, Clone)]
struct ParsedLimitOrderPlacement {
    pair_address: String,
    order_id: i64,
    owner: Option<String>,
    side: Option<String>,
    price: Option<BigDecimal>,
    expires_at: Option<i64>,
}

#[derive(Debug, Clone)]
struct ParsedLimitOrderCancellation {
    pair_address: String,
    order_id: i64,
    owner: Option<String>,
}

#[derive(Debug, Clone)]
struct ParsedLimitOrderExpiredParked {
    pair_address: String,
    order_id: i64,
    remaining: BigDecimal,
}

#[derive(Debug, Clone)]
struct ParsedClaimExpiredLimitOrder {
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
            process_swap(
                pool, lcd, config, swap, height, block_time, &tx.txhash, ustc_price,
            )
            .await?;
        }

        let liq_events = parse_liquidity_events(tx);
        for liq in &liq_events {
            process_liquidity_event(pool, lcd, liq, height, block_time, &tx.txhash).await?;
        }

        let lo_fills = parse_limit_order_fills(tx);
        for fill in &lo_fills {
            process_limit_order_fill(pool, lcd, fill, height, block_time, &tx.txhash).await?;
        }

        let placements = parse_limit_order_placements(tx);
        for p in &placements {
            process_limit_order_placement(pool, lcd, p, height, block_time, &tx.txhash).await?;
        }

        let cancellations = parse_limit_order_cancellations(tx);
        for c in &cancellations {
            process_limit_order_cancellation(pool, lcd, c, height, block_time, &tx.txhash).await?;
        }

        let parked = parse_limit_order_expired_parked(tx);
        for p in &parked {
            process_limit_order_expired_parked(pool, lcd, p, height, block_time, &tx.txhash)
                .await?;
        }

        let claims = parse_claim_expired_limit_orders(tx);
        for c in &claims {
            process_claim_expired_limit_order(pool, lcd, c, height, block_time, &tx.txhash)
                .await?;
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
            crate::db::queries::hook_events::insert_hook_event(
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
            .await?;
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

    if swap_events::trade_exists(pool, tx_hash, pair.id, swap.swap_index).await? {
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
        swap.swap_index,
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

/// Extract swap events from LCD tx logs (`wasm` events with `action=swap`).
/// Hybrid fields (`pool_return_amount`, `book_return_amount`, `limit_book_offer_consumed`, `effective_fee_bps`) are optional.
pub fn parse_swaps(tx: &TxResponse) -> Vec<ParsedSwap> {
    let mut swaps = Vec::new();
    // GitLab #287: assign each swap a per-pair ordinal within the tx (walk order).
    let mut per_pair_swap_index: std::collections::HashMap<String, i32> =
        std::collections::HashMap::new();

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
            let swap_index = {
                let c = per_pair_swap_index
                    .entry(contract.to_string())
                    .or_insert(0);
                let i = *c;
                *c += 1;
                i
            };
            swaps.push(ParsedSwap {
                pair_address: contract.to_string(),
                swap_index,
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

fn parse_limit_order_fill_segment(
    contract: &str,
    seg: &std::collections::HashMap<&str, &str>,
) -> Option<ParsedLimitOrderFill> {
    let side = seg.get("side").copied()?;
    if side != "bid" && side != "ask" {
        return None;
    }
    let maker = seg.get("maker").copied()?;
    let order_id = seg.get("order_id")?.parse::<i64>().ok()?;
    let price = seg.get("price")?.parse::<BigDecimal>().ok()?;
    let token0_amount = seg.get("token0_amount")?.parse::<BigDecimal>().ok()?;
    let token1_amount = seg.get("token1_amount")?.parse::<BigDecimal>().ok()?;
    let commission_amount = seg.get("commission_amount")?.parse::<BigDecimal>().ok()?;
    Some(ParsedLimitOrderFill {
        pair_address: contract.to_string(),
        order_id,
        side: side.to_string(),
        maker: maker.to_string(),
        price,
        token0_amount,
        token1_amount,
        commission_amount,
        swap_index: 0, // assigned by parse_limit_order_fills (GitLab #316)
    })
}

/// Merged LCD streams may list multiple `limit_order_fill` actions with columnar field keys.
fn parse_limit_order_fills_columnar(attrs: &[Attribute]) -> Option<Vec<ParsedLimitOrderFill>> {
    let fill_count = attrs
        .iter()
        .filter(|a| a.key == "action" && a.value == "limit_order_fill")
        .count();
    if fill_count <= 1 {
        return None;
    }
    let contract = attrs
        .iter()
        .find(|a| is_wasm_contract_addr_key(a.key.as_str()))
        .map(|a| a.value.as_str())?;

    let order_ids: Vec<i64> = wasm_attr_values(attrs, "order_id")
        .iter()
        .filter_map(|s| s.parse().ok())
        .collect();
    if order_ids.len() != fill_count {
        return None;
    }

    let sides: Vec<&str> = wasm_attr_values(attrs, "side");
    let makers: Vec<&str> = wasm_attr_values(attrs, "maker");
    let prices: Vec<BigDecimal> = wasm_attr_values(attrs, "price")
        .iter()
        .filter_map(|s| BigDecimal::from_str(s).ok())
        .collect();
    let token0_amounts: Vec<BigDecimal> = wasm_attr_values(attrs, "token0_amount")
        .iter()
        .filter_map(|s| BigDecimal::from_str(s).ok())
        .collect();
    let token1_amounts: Vec<BigDecimal> = wasm_attr_values(attrs, "token1_amount")
        .iter()
        .filter_map(|s| BigDecimal::from_str(s).ok())
        .collect();
    let commission_amounts: Vec<BigDecimal> = wasm_attr_values(attrs, "commission_amount")
        .iter()
        .filter_map(|s| BigDecimal::from_str(s).ok())
        .collect();

    let n = fill_count;
    if prices.len() != n
        || token0_amounts.len() != n
        || token1_amounts.len() != n
        || commission_amounts.len() != n
    {
        return None;
    }

    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        let side = sides.get(i).copied().or_else(|| sides.first().copied())?;
        if side != "bid" && side != "ask" {
            return None;
        }
        let maker = makers.get(i).copied().or_else(|| makers.first().copied())?;
        out.push(ParsedLimitOrderFill {
            pair_address: contract.to_string(),
            order_id: order_ids[i],
            side: side.to_string(),
            maker: maker.to_string(),
            price: prices[i].clone(),
            token0_amount: token0_amounts[i].clone(),
            token1_amount: token1_amounts[i].clone(),
            commission_amount: commission_amounts[i].clone(),
            swap_index: 0, // assigned by parse_limit_order_fills (GitLab #316)
        });
    }
    Some(out)
}

fn parse_limit_order_fills_from_wasm_attrs(attrs: &[Attribute]) -> Vec<ParsedLimitOrderFill> {
    if let Some(columnar) = parse_limit_order_fills_columnar(attrs) {
        return columnar;
    }

    let mut out = Vec::new();
    for (i, a) in attrs.iter().enumerate() {
        if a.key != "action" || a.value != "limit_order_fill" {
            continue;
        }
        let Some(contract) = wasm_contract_addr_before(attrs, i) else {
            continue;
        };
        let seg = wasm_kv_map_after_action(attrs, i);
        if let Some(fill) = parse_limit_order_fill_segment(contract, &seg) {
            out.push(fill);
        }
    }
    out
}

fn parse_limit_order_fills(tx: &TxResponse) -> Vec<ParsedLimitOrderFill> {
    let events: Vec<&crate::lcd::Event> = if let Some(logs) = &tx.logs {
        logs.iter().flat_map(|l| l.events.iter()).collect()
    } else if let Some(evts) = &tx.events {
        evts.iter().collect()
    } else {
        Vec::new()
    };

    // GitLab #316: tag each fill with the per-pair swap ordinal of the swap that produced it so it
    // links to the correct swap_events row when a tx has multiple swaps on one pair. Maker fills
    // are emitted before their `swap` action in the same execute (the pair adds book_fill_events,
    // then the swap attribute), so a fill belongs to the *upcoming* swap on its pair — the current
    // per-pair count, before this event's swap is counted. The swap count mirrors parse_swaps
    // exactly (one swap per wasm event, the event's last `action`), so the ordinals line up with
    // the persisted swap_events.swap_index. (Two swaps on one pair merged into a single wasm event
    // is not separable here — but parse_swaps only keeps the event's last swap in that case too,
    // so neither side splits it; realistic multi-swap-same-pair txs arrive as separate events.)
    let mut per_pair_swap_seen: std::collections::HashMap<String, i32> =
        std::collections::HashMap::new();
    let mut out = Vec::new();
    for event in &events {
        if !is_wasm_lifecycle_event_type(&event.event_type) {
            continue;
        }
        let attrs = &event.attributes;
        let mut fills = parse_limit_order_fills_from_wasm_attrs(attrs);
        for fill in &mut fills {
            fill.swap_index = per_pair_swap_seen
                .get(&fill.pair_address)
                .copied()
                .unwrap_or(0);
        }
        out.append(&mut fills);

        // Advance the per-pair ordinal for this event's swap, matching parse_swaps' detection.
        if event.event_type == "wasm" && wasm_attr_last(attrs, "action") == Some("swap") {
            if let (Some(contract), Some(_), Some(_), Some(_)) = (
                wasm_contract_addr(attrs),
                wasm_attr_last(attrs, "sender"),
                wasm_attr_last(attrs, "offer_amount"),
                wasm_attr_last(attrs, "return_amount"),
            ) {
                *per_pair_swap_seen.entry(contract.to_string()).or_insert(0) += 1;
            }
        }
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

    let swap_event_id =
        limit_order_fills::swap_id_for_tx_pair_index(pool, tx_hash, pair.id, fill.swap_index)
            .await?;

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

fn wasm_attr_values<'a>(attrs: &'a [Attribute], key: &str) -> Vec<&'a str> {
    attrs
        .iter()
        .filter(|a| a.key == key)
        .map(|a| a.value.as_str())
        .collect()
}

/// CosmWasm batch/ladder placement emits **columnar** wasm attrs (all `action`s, then parallel
/// `order_id` / `price` / … columns). Interleaved per-rung attrs are still supported (unit tests).
fn parse_limit_order_placements_columnar(
    attrs: &[Attribute],
) -> Option<Vec<ParsedLimitOrderPlacement>> {
    let has_batch = attrs
        .iter()
        .any(|a| a.key == "action" && a.value == "place_limit_order_batch");
    let place_count = attrs
        .iter()
        .filter(|a| a.key == "action" && a.value == "place_limit_order")
        .count();
    if !has_batch && place_count <= 1 {
        return None;
    }
    let contract = attrs
        .iter()
        .find(|a| is_wasm_contract_addr_key(a.key.as_str()))
        .map(|a| a.value.as_str())?;

    let mut order_ids: Vec<i64> = wasm_attr_values(attrs, "order_id")
        .iter()
        .filter_map(|s| s.parse().ok())
        .collect();
    if order_ids.is_empty() {
        order_ids = wasm_attr_values(attrs, "limit_order_placed")
            .iter()
            .filter_map(|s| s.parse().ok())
            .collect();
    }
    if order_ids.is_empty() {
        return None;
    }

    let sides: Vec<String> = wasm_attr_values(attrs, "side")
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    let owners: Vec<String> = wasm_attr_values(attrs, "owner")
        .into_iter()
        .map(|s| s.to_string())
        .collect();
    let prices: Vec<BigDecimal> = wasm_attr_values(attrs, "price")
        .iter()
        .filter_map(|s| BigDecimal::from_str(s).ok())
        .collect();
    let expires_at_list: Vec<i64> = wasm_attr_values(attrs, "expires_at")
        .iter()
        .filter_map(|s| s.parse().ok())
        .collect();

    let n = order_ids.len();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        out.push(ParsedLimitOrderPlacement {
            pair_address: contract.to_string(),
            order_id: order_ids[i],
            side: sides
                .get(i)
                .cloned()
                .or_else(|| sides.first().cloned()),
            owner: owners
                .get(i)
                .cloned()
                .or_else(|| owners.first().cloned()),
            price: prices.get(i).cloned(),
            expires_at: expires_at_list.get(i).copied(),
        });
    }
    Some(out)
}

fn parse_limit_order_placements_from_wasm_attrs(
    attrs: &[Attribute],
) -> Vec<ParsedLimitOrderPlacement> {
    if let Some(columnar) = parse_limit_order_placements_columnar(attrs) {
        return columnar;
    }

    let mut out = Vec::new();
    for (i, a) in attrs.iter().enumerate() {
        if a.key != "action" || a.value != "place_limit_order" {
            continue;
        }
        let Some(contract) = wasm_contract_addr_before(attrs, i) else {
            continue;
        };
        let seg = wasm_kv_map_after_action(attrs, i);
        let oid = seg
            .get("order_id")
            .and_then(|s| s.parse::<i64>().ok())
            .or_else(|| {
                seg.get("limit_order_placed")
                    .and_then(|s| s.parse::<i64>().ok())
            });
        let Some(order_id) = oid else {
            continue;
        };
        let owner = seg.get("owner").map(|x| x.to_string());
        let side = seg.get("side").map(|x| x.to_string());
        let price = seg.get("price").and_then(|x| BigDecimal::from_str(x).ok());
        let expires_at = seg.get("expires_at").and_then(|x| x.parse::<i64>().ok());
        out.push(ParsedLimitOrderPlacement {
            pair_address: contract.to_string(),
            order_id,
            owner,
            side,
            price,
            expires_at,
        });
    }
    out
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
        if !is_wasm_lifecycle_event_type(&event.event_type) {
            continue;
        }
        out.extend(parse_limit_order_placements_from_wasm_attrs(
            &event.attributes,
        ));
    }
    out
}

fn parse_limit_order_cancellations_columnar(
    attrs: &[Attribute],
) -> Option<Vec<ParsedLimitOrderCancellation>> {
    let has_batch = attrs
        .iter()
        .any(|a| a.key == "action" && a.value == "cancel_limit_orders_batch");
    let cancel_count = attrs
        .iter()
        .filter(|a| a.key == "action" && a.value == "cancel_limit_order")
        .count();
    if !has_batch && cancel_count <= 1 {
        return None;
    }
    let contract = attrs
        .iter()
        .find(|a| is_wasm_contract_addr_key(a.key.as_str()))
        .map(|a| a.value.as_str())?;

    let order_ids: Vec<i64> = wasm_attr_values(attrs, "limit_order_cancelled")
        .iter()
        .filter_map(|s| s.parse().ok())
        .collect();
    if order_ids.is_empty() {
        return None;
    }
    let owners: Vec<String> = wasm_attr_values(attrs, "owner")
        .into_iter()
        .map(|s| s.to_string())
        .collect();

    let n = order_ids.len();
    let mut out = Vec::with_capacity(n);
    for i in 0..n {
        out.push(ParsedLimitOrderCancellation {
            pair_address: contract.to_string(),
            order_id: order_ids[i],
            owner: owners
                .get(i)
                .cloned()
                .or_else(|| owners.first().cloned()),
        });
    }
    Some(out)
}

fn parse_limit_order_cancellations_from_wasm_attrs(
    attrs: &[Attribute],
) -> Vec<ParsedLimitOrderCancellation> {
    if let Some(columnar) = parse_limit_order_cancellations_columnar(attrs) {
        return columnar;
    }

    let mut out = Vec::new();
    for (i, a) in attrs.iter().enumerate() {
        if a.key != "action" || a.value != "cancel_limit_order" {
            continue;
        }
        let Some(contract) = wasm_contract_addr_before(attrs, i) else {
            continue;
        };
        let seg = wasm_kv_map_after_action(attrs, i);
        let Some(order_id) = seg
            .get("limit_order_cancelled")
            .and_then(|s| s.parse::<i64>().ok())
        else {
            continue;
        };
        let owner = seg.get("owner").map(|x| x.to_string());
        out.push(ParsedLimitOrderCancellation {
            pair_address: contract.to_string(),
            order_id,
            owner,
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
        if !is_wasm_lifecycle_event_type(&event.event_type) {
            continue;
        }
        out.extend(parse_limit_order_cancellations_from_wasm_attrs(
            &event.attributes,
        ));
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
        p.owner.as_deref(),
        p.side.as_deref(),
        p.price.as_ref(),
        p.expires_at,
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
        c.owner.as_deref(),
    )
    .await?;
    Ok(())
}

fn parse_limit_order_expired_parked(tx: &TxResponse) -> Vec<ParsedLimitOrderExpiredParked> {
    let events: Vec<&crate::lcd::Event> = if let Some(logs) = &tx.logs {
        logs.iter().flat_map(|l| l.events.iter()).collect()
    } else if let Some(evts) = &tx.events {
        evts.iter().collect()
    } else {
        Vec::new()
    };

    let mut out = Vec::new();
    for event in &events {
        if !is_wasm_lifecycle_event_type(&event.event_type) {
            continue;
        }
        out.extend(parse_limit_order_expired_parked_from_wasm_attrs(
            &event.attributes,
        ));
    }
    out
}

fn parse_claim_expired_limit_orders(tx: &TxResponse) -> Vec<ParsedClaimExpiredLimitOrder> {
    let events: Vec<&crate::lcd::Event> = if let Some(logs) = &tx.logs {
        logs.iter().flat_map(|l| l.events.iter()).collect()
    } else if let Some(evts) = &tx.events {
        evts.iter().collect()
    } else {
        Vec::new()
    };

    let mut out = Vec::new();
    for event in &events {
        if !is_wasm_lifecycle_event_type(&event.event_type) {
            continue;
        }
        out.extend(parse_claim_expired_limit_orders_from_wasm_attrs(
            &event.attributes,
        ));
    }
    out
}

async fn process_limit_order_expired_parked(
    pool: &PgPool,
    lcd: &LcdClient,
    p: &ParsedLimitOrderExpiredParked,
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

    limit_order_lifecycle::apply_parked_expired(
        pool,
        pair.id,
        p.order_id,
        height,
        block_time,
        tx_hash,
        &p.remaining,
    )
    .await?;
    Ok(())
}

async fn process_claim_expired_limit_order(
    pool: &PgPool,
    lcd: &LcdClient,
    c: &ParsedClaimExpiredLimitOrder,
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

    limit_order_lifecycle::apply_claim_refund(
        pool, pair.id, c.order_id, height, block_time, tx_hash,
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

    fn wasm_wasm_tx(attrs: Vec<(&str, &str)>) -> TxResponse {
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
            logs: None,
            timestamp: None,
            events: Some(vec![Event {
                event_type: "wasm-wasm".into(),
                attributes,
            }]),
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
        assert_eq!(swaps[0].swap_index, 0);
    }

    #[test]
    fn parse_swaps_assigns_per_pair_swap_index() {
        // GitLab #287: two swaps on the same pair in one tx must get distinct ordinals (0, 1)
        // so they no longer collapse on (tx_hash, pair_id); a swap on a different pair restarts
        // at 0 (the ordinal is per-pair, not per-tx).
        let tx = wasm_tx_multi(vec![
            vec![
                ("_contract_address", "terra1pairA"),
                ("action", "swap"),
                ("sender", "terra1u"),
                ("offer_amount", "100"),
                ("return_amount", "90"),
            ],
            vec![
                ("_contract_address", "terra1pairA"),
                ("action", "swap"),
                ("sender", "terra1u"),
                ("offer_amount", "50"),
                ("return_amount", "45"),
            ],
            vec![
                ("_contract_address", "terra1pairB"),
                ("action", "swap"),
                ("sender", "terra1u"),
                ("offer_amount", "10"),
                ("return_amount", "9"),
            ],
        ]);
        let swaps = parse_swaps(&tx);
        assert_eq!(swaps.len(), 3);
        assert_eq!((swaps[0].pair_address.as_str(), swaps[0].swap_index), ("terra1pairA", 0));
        assert_eq!(
            (swaps[1].pair_address.as_str(), swaps[1].swap_index),
            ("terra1pairA", 1),
            "second swap on the same pair must get ordinal 1"
        );
        assert_eq!(
            (swaps[2].pair_address.as_str(), swaps[2].swap_index),
            ("terra1pairB", 0),
            "a different pair restarts the ordinal"
        );
    }

    #[test]
    fn parse_swaps_hybrid_and_fee_attrs() {
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1pair"),
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
                ("_contract_address", "terra1pair"),
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
                ("_contract_address", "terra1pair"),
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
    fn parse_limit_order_fills_merged_before_swap() {
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1pair"),
            ("action", "limit_order_fill"),
            ("order_id", "101"),
            ("side", "bid"),
            ("maker", "terra1mk1"),
            ("price", "1.0"),
            ("token0_amount", "10"),
            ("token1_amount", "10"),
            ("commission_amount", "0.1"),
            ("action", "limit_order_fill"),
            ("order_id", "102"),
            ("side", "ask"),
            ("maker", "terra1mk2"),
            ("price", "1.1"),
            ("token0_amount", "20"),
            ("token1_amount", "22"),
            ("commission_amount", "0.2"),
            ("action", "limit_order_fill"),
            ("order_id", "103"),
            ("side", "bid"),
            ("maker", "terra1mk3"),
            ("price", "0.9"),
            ("token0_amount", "5"),
            ("token1_amount", "4"),
            ("commission_amount", "0.05"),
            ("action", "limit_order_fill"),
            ("order_id", "104"),
            ("side", "ask"),
            ("maker", "terra1mk4"),
            ("price", "1.2"),
            ("token0_amount", "8"),
            ("token1_amount", "9"),
            ("commission_amount", "0.08"),
            ("action", "limit_order_fill"),
            ("order_id", "105"),
            ("side", "bid"),
            ("maker", "terra1mk5"),
            ("price", "1.05"),
            ("token0_amount", "15"),
            ("token1_amount", "15"),
            ("commission_amount", "0.15"),
            ("action", "swap"),
            ("sender", "terra1taker"),
            ("offer_amount", "100"),
            ("return_amount", "95"),
            ("offer_asset", "tokena"),
            ("ask_asset", "tokenb"),
            ("pool_return_amount", "40"),
            ("book_return_amount", "55"),
            ("limit_book_offer_consumed", "60"),
        ]);
        let fills = parse_limit_order_fills(&tx);
        assert_eq!(fills.len(), 5, "GitLab #254 QA: 5 on-chain fills");
        assert_eq!(fills[0].order_id, 101);
        assert_eq!(fills[4].order_id, 105);
        assert_eq!(fills[0].maker, "terra1mk1");
        assert_eq!(fills[4].commission_amount.to_string(), "0.15");
        let swaps = parse_swaps(&tx);
        assert_eq!(swaps.len(), 1);
        assert_eq!(
            swaps[0].book_return_amount.as_ref().unwrap().to_string(),
            "55"
        );
        // GitLab #316: single swap on the pair -> every fill carries swap_index 0.
        assert!(fills.iter().all(|f| f.swap_index == 0));
    }

    #[test]
    fn parse_limit_order_fills_assigns_swap_index_per_pair_swap() {
        // GitLab #316: two swaps on the SAME pair in one tx arrive as separate wasm events, each
        // with its maker fills before the `swap` action. Each fill must carry the ordinal of the
        // swap that produced it, matching the persisted swap_events.swap_index — not all collapse
        // to swap 0 (the old `ORDER BY id ASC LIMIT 1` linkage bug).
        let tx = wasm_tx_multi(vec![
            // swap 0 on terra1pair: one maker fill, then the swap action.
            vec![
                ("_contract_address", "terra1pair"),
                ("action", "limit_order_fill"),
                ("order_id", "11"),
                ("side", "bid"),
                ("maker", "terra1mkA"),
                ("price", "1.0"),
                ("token0_amount", "10"),
                ("token1_amount", "10"),
                ("commission_amount", "0.1"),
                ("_contract_address", "terra1pair"),
                ("action", "swap"),
                ("sender", "terra1taker"),
                ("offer_amount", "100"),
                ("return_amount", "95"),
            ],
            // swap 1 on the SAME pair: two maker fills, then the swap action.
            vec![
                ("_contract_address", "terra1pair"),
                ("action", "limit_order_fill"),
                ("order_id", "21"),
                ("side", "ask"),
                ("maker", "terra1mkB"),
                ("price", "1.1"),
                ("token0_amount", "5"),
                ("token1_amount", "6"),
                ("commission_amount", "0.05"),
                ("_contract_address", "terra1pair"),
                ("action", "limit_order_fill"),
                ("order_id", "22"),
                ("side", "ask"),
                ("maker", "terra1mkC"),
                ("price", "1.2"),
                ("token0_amount", "7"),
                ("token1_amount", "8"),
                ("commission_amount", "0.07"),
                ("_contract_address", "terra1pair"),
                ("action", "swap"),
                ("sender", "terra1taker"),
                ("offer_amount", "50"),
                ("return_amount", "48"),
            ],
        ]);

        let swaps = parse_swaps(&tx);
        assert_eq!(swaps.len(), 2);
        assert_eq!((swaps[0].swap_index, swaps[1].swap_index), (0, 1));

        let fills = parse_limit_order_fills(&tx);
        assert_eq!(fills.len(), 3);
        // fill of swap 0
        assert_eq!((fills[0].order_id, fills[0].swap_index), (11, 0));
        // both fills of swap 1
        assert_eq!((fills[1].order_id, fills[1].swap_index), (21, 1));
        assert_eq!((fills[2].order_id, fills[2].swap_index), (22, 1));
    }

    #[test]
    fn parse_limit_order_fills_twenty_makers_merged_before_transfer() {
        let mut attributes = vec![Attribute {
            key: "_contract_address".into(),
            value: "terra1pair".into(),
        }];
        for id in 1..=20i64 {
            attributes.extend([
                Attribute {
                    key: "action".into(),
                    value: "limit_order_fill".into(),
                },
                Attribute {
                    key: "order_id".into(),
                    value: id.to_string(),
                },
                Attribute {
                    key: "side".into(),
                    value: if id % 2 == 0 {
                        "ask".into()
                    } else {
                        "bid".into()
                    },
                },
                Attribute {
                    key: "maker".into(),
                    value: format!("terra1mk{id}"),
                },
                Attribute {
                    key: "price".into(),
                    value: "1".into(),
                },
                Attribute {
                    key: "token0_amount".into(),
                    value: "1".into(),
                },
                Attribute {
                    key: "token1_amount".into(),
                    value: "1".into(),
                },
                Attribute {
                    key: "commission_amount".into(),
                    value: "0.01".into(),
                },
            ]);
        }
        attributes.extend([
            Attribute {
                key: "action".into(),
                value: "transfer".into(),
            },
            Attribute {
                key: "recipient".into(),
                value: "terra1taker".into(),
            },
            Attribute {
                key: "amount".into(),
                value: "100".into(),
            },
            Attribute {
                key: "action".into(),
                value: "swap".into(),
            },
            Attribute {
                key: "sender".into(),
                value: "terra1taker".into(),
            },
            Attribute {
                key: "offer_amount".into(),
                value: "500".into(),
            },
            Attribute {
                key: "return_amount".into(),
                value: "480".into(),
            },
            Attribute {
                key: "offer_asset".into(),
                value: "tokena".into(),
            },
            Attribute {
                key: "ask_asset".into(),
                value: "tokenb".into(),
            },
        ]);
        let tx = TxResponse {
            height: "1".into(),
            txhash: "HYBRID20".into(),
            logs: Some(vec![TxLog {
                events: vec![Event {
                    event_type: "wasm".into(),
                    attributes,
                }],
            }]),
            timestamp: None,
            events: None,
        };
        let fills = parse_limit_order_fills(&tx);
        assert_eq!(fills.len(), 20, "GitLab #254 QA: 20 on-chain fills");
        assert_eq!(fills[0].order_id, 1);
        assert_eq!(fills[19].order_id, 20);
        assert_eq!(fills[19].maker, "terra1mk20");
        let swaps = parse_swaps(&tx);
        assert_eq!(swaps.len(), 1);
    }

    #[test]
    fn parse_limit_order_fills_from_wasm_wasm_event_type() {
        let tx = wasm_wasm_tx(vec![
            ("_contract_address", "terra1pair"),
            ("action", "limit_order_fill"),
            ("order_id", "7"),
            ("side", "bid"),
            ("maker", "terra1maker"),
            ("price", "1.5"),
            ("token0_amount", "100"),
            ("token1_amount", "150"),
            ("commission_amount", "1"),
        ]);
        let fills = parse_limit_order_fills(&tx);
        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].pair_address, "terra1pair");
        assert_eq!(fills[0].order_id, 7);
    }

    #[test]
    fn parse_limit_order_fills_skips_malformed_segment_keeps_valid() {
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1pair"),
            ("action", "limit_order_fill"),
            ("order_id", "not-a-number"),
            ("side", "bid"),
            ("maker", "terra1bad"),
            ("action", "limit_order_fill"),
            ("order_id", "42"),
            ("side", "ask"),
            ("maker", "terra1good"),
            ("price", "2"),
            ("token0_amount", "10"),
            ("token1_amount", "20"),
            ("commission_amount", "0"),
            ("action", "swap"),
            ("sender", "terra1taker"),
            ("offer_amount", "1"),
            ("return_amount", "1"),
            ("offer_asset", "a"),
            ("ask_asset", "b"),
        ]);
        let fills = parse_limit_order_fills(&tx);
        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].order_id, 42);
    }

    #[test]
    fn parse_limit_order_fills_pool_only_swap_has_zero_fills() {
        let tx = wasm_tx(vec![
            ("contract_address", "terra1pair"),
            ("action", "swap"),
            ("sender", "terra1user"),
            ("offer_amount", "100"),
            ("return_amount", "99"),
            ("offer_asset", "a"),
            ("ask_asset", "b"),
        ]);
        assert!(parse_limit_order_fills(&tx).is_empty());
    }

    #[test]
    fn parse_limit_order_placements_and_cancellations() {
        let tx = wasm_tx_multi(vec![
            vec![
                ("_contract_address", "terra1pair"),
                ("action", "place_limit_order"),
                ("order_id", "99"),
                ("limit_order_placed", "99"),
                ("side", "bid"),
                ("price", "1.5"),
                ("owner", "terra1maker"),
                ("expires_at", "2000000000"),
            ],
            vec![
                ("_contract_address", "terra1pair"),
                ("action", "cancel_limit_order"),
                ("limit_order_cancelled", "99"),
                ("owner", "terra1maker"),
            ],
        ]);
        let p = parse_limit_order_placements(&tx);
        assert_eq!(p.len(), 1);
        assert_eq!(p[0].order_id, 99);
        assert_eq!(p[0].side.as_deref(), Some("bid"));
        assert_eq!(p[0].owner.as_deref(), Some("terra1maker"));
        assert_eq!(
            p[0].price.as_ref().map(|x| x.to_string()),
            Some("1.5".into())
        );
        assert_eq!(p[0].expires_at, Some(2_000_000_000));
        let c = parse_limit_order_cancellations(&tx);
        assert_eq!(c.len(), 1);
        assert_eq!(c[0].order_id, 99);
        assert_eq!(c[0].owner.as_deref(), Some("terra1maker"));
    }

    // GitLab #285 — forged emitter attack. A malicious contract emits a non-reserved
    // `contract_address` pointing at a victim pair, BEFORE its `action`. wasmd still stamps
    // the real `_contract_address` (the attacker's own contract). The parser must scope the
    // event by the runtime-reserved key only, so the forged value cannot attribute the event
    // to the victim. These would FAIL on the pre-fix existence-only `contract_address` path
    // (which would attribute the row to terra1victimpair).
    #[test]
    fn forged_contract_address_fill_not_attributed_to_victim_pair() {
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1attacker"), // runtime-stamped real emitter
            ("contract_address", "terra1victimpair"), // FORGED, placed before action
            ("action", "limit_order_fill"),
            ("order_id", "7"),
            ("side", "bid"),
            ("maker", "terra1maker"),
            ("price", "1.5"),
            ("token0_amount", "100"),
            ("token1_amount", "150"),
            ("commission_amount", "1"),
        ]);
        let fills = parse_limit_order_fills(&tx);
        assert_eq!(fills.len(), 1);
        assert_eq!(
            fills[0].pair_address, "terra1attacker",
            "forged contract_address must not scope the fill to the victim pair"
        );
        assert_ne!(fills[0].pair_address, "terra1victimpair");
    }

    #[test]
    fn forged_contract_address_cancel_not_attributed_to_victim_pair() {
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1attacker"),
            ("contract_address", "terra1victimpair"), // FORGED
            ("action", "cancel_limit_order"),
            ("limit_order_cancelled", "42"),
            ("owner", "terra1maker"),
        ]);
        let cancels = parse_limit_order_cancellations(&tx);
        assert_eq!(cancels.len(), 1);
        assert_eq!(
            cancels[0].pair_address, "terra1attacker",
            "forged contract_address must not let an attacker mark a victim pair's order cancelled"
        );
        assert_ne!(cancels[0].pair_address, "terra1victimpair");
    }

    #[test]
    fn forged_contract_address_placement_not_attributed_to_victim_pair() {
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1attacker"), // runtime-stamped real emitter
            ("contract_address", "terra1victimpair"), // FORGED, placed before action
            ("action", "place_limit_order"),
            ("order_id", "7"),
            ("limit_order_placed", "7"),
            ("side", "bid"),
            ("price", "1.5"),
            ("owner", "terra1maker"),
            ("expires_at", "2000000000"),
        ]);
        let placements = parse_limit_order_placements(&tx);
        assert_eq!(placements.len(), 1);
        assert_eq!(
            placements[0].pair_address, "terra1attacker",
            "forged contract_address must not let an attacker plant a placement on a victim pair"
        );
        assert_ne!(placements[0].pair_address, "terra1victimpair");
    }

    // No regression: the real on-chain shape — wasmd `_contract_address` before `action`,
    // plus the pair's own convenience `contract_address` after `action` (same value) —
    // still attributes correctly to the pair.
    #[test]
    fn genuine_fill_with_both_contract_address_keys_attributes_to_pair() {
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1pair"),
            ("action", "limit_order_fill"),
            ("order_id", "9"),
            ("side", "ask"),
            ("maker", "terra1maker"),
            ("price", "2"),
            ("token0_amount", "10"),
            ("token1_amount", "20"),
            ("commission_amount", "0"),
            ("contract_address", "terra1pair"), // pair's own convenience attr, after action
        ]);
        let fills = parse_limit_order_fills(&tx);
        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].pair_address, "terra1pair");
        assert_eq!(fills[0].order_id, 9);
    }

    #[test]
    fn parse_limit_order_cancellations_batch_columnar_attrs() {
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1pair"),
            ("action", "cancel_limit_orders_batch"),
            ("batch_count", "3"),
            ("action", "cancel_limit_order"),
            ("action", "cancel_limit_order"),
            ("action", "cancel_limit_order"),
            ("limit_order_cancelled", "10"),
            ("limit_order_cancelled", "11"),
            ("limit_order_cancelled", "12"),
            ("owner", "terra1maker"),
            ("owner", "terra1maker"),
            ("owner", "terra1maker"),
        ]);
        let c = parse_limit_order_cancellations(&tx);
        assert_eq!(c.len(), 3);
        assert_eq!(c[0].order_id, 10);
        assert_eq!(c[2].order_id, 12);
        assert_eq!(c[0].owner.as_deref(), Some("terra1maker"));
    }

    #[test]
    fn parse_claim_expired_limit_orders_batch_columnar_attrs() {
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1pair"),
            ("action", "claim_expired_limit_orders_batch"),
            ("batch_count", "2"),
            ("action", "claim_expired_limit_order"),
            ("action", "claim_expired_limit_order"),
            ("order_id", "5"),
            ("order_id", "6"),
        ]);
        let claims = parse_claim_expired_limit_orders(&tx);
        assert_eq!(claims.len(), 2);
        assert_eq!(claims[0].order_id, 5);
        assert_eq!(claims[1].order_id, 6);
    }

    #[test]
    fn parse_limit_order_expired_parked_event() {
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1pair"),
            ("action", "limit_order_expired_parked"),
            ("order_id", "42"),
            ("maker", "terra1mk"),
            ("side", "bid"),
            ("remaining", "999000"),
        ]);
        let evs = parse_limit_order_expired_parked(&tx);
        assert_eq!(evs.len(), 1);
        assert_eq!(evs[0].pair_address, "terra1pair");
        assert_eq!(evs[0].order_id, 42);
        assert_eq!(evs[0].remaining.to_string(), "999000");
    }

    /// On-chain batch placement uses **columnar** attrs (CosmWasm 1.x); GitLab #206.
    #[test]
    fn parse_limit_order_placements_five_rungs_columnar_attrs() {
        let mut attrs = vec![Attribute {
            key: "_contract_address".into(),
            value: "terra1pair".into(),
        }];
        attrs.push(Attribute {
            key: "action".into(),
            value: "place_limit_order_batch".into(),
        });
        for _ in 0..5 {
            attrs.push(Attribute {
                key: "action".into(),
                value: "place_limit_order".into(),
            });
        }
        attrs.push(Attribute {
            key: "batch_count".into(),
            value: "5".into(),
        });
        for id in 1..=5 {
            attrs.push(Attribute {
                key: "limit_order_placed".into(),
                value: id.to_string(),
            });
        }
        for id in 1..=5 {
            attrs.push(Attribute {
                key: "order_id".into(),
                value: id.to_string(),
            });
        }
        for _ in 0..5 {
            attrs.push(Attribute {
                key: "side".into(),
                value: "bid".into(),
            });
        }
        for _ in 0..5 {
            attrs.push(Attribute {
                key: "owner".into(),
                value: "terra1maker".into(),
            });
        }
        for price in ["0.95", "0.975", "1", "1.025", "1.05"] {
            attrs.push(Attribute {
                key: "price".into(),
                value: price.into(),
            });
        }
        let tx = TxResponse {
            height: "1".into(),
            txhash: "BATCHCOL".into(),
            logs: Some(vec![TxLog {
                events: vec![Event {
                    event_type: "wasm".into(),
                    attributes: attrs,
                }],
            }]),
            timestamp: None,
            events: None,
        };
        let p = parse_limit_order_placements(&tx);
        assert_eq!(p.len(), 5);
        assert_eq!(p[0].order_id, 1);
        assert_eq!(p[4].order_id, 5);
        assert_eq!(
            p[4].price.as_ref().map(|x| x.to_string()),
            Some("1.05".into())
        );
    }

    /// Flattened wasm stream: `action=swap` is **last**; older parsers using `wasm_attr_last` on the
    /// whole slice missed `limit_order_expired_parked` (GitLab #141 / QA note on #141).
    #[test]
    fn parse_limit_order_placements_five_rungs_merged_before_swap() {
        let mut attrs = vec![
            Attribute {
                key: "_contract_address".into(),
                value: "terra1pair".into(),
            },
            Attribute {
                key: "action".into(),
                value: "place_limit_order_batch".into(),
            },
            Attribute {
                key: "batch_count".into(),
                value: "5".into(),
            },
        ];
        for id in 1..=5 {
            attrs.push(Attribute {
                key: "action".into(),
                value: "place_limit_order".into(),
            });
            attrs.push(Attribute {
                key: "order_id".into(),
                value: id.to_string(),
            });
            attrs.push(Attribute {
                key: "side".into(),
                value: "bid".into(),
            });
            attrs.push(Attribute {
                key: "price".into(),
                value: "1".into(),
            });
            attrs.push(Attribute {
                key: "owner".into(),
                value: "terra1maker".into(),
            });
        }
        attrs.push(Attribute {
            key: "action".into(),
            value: "swap".into(),
        });
        attrs.push(Attribute {
            key: "sender".into(),
            value: "terra1taker".into(),
        });
        attrs.push(Attribute {
            key: "offer_amount".into(),
            value: "100".into(),
        });
        attrs.push(Attribute {
            key: "return_amount".into(),
            value: "90".into(),
        });
        attrs.push(Attribute {
            key: "offer_asset".into(),
            value: "tokena".into(),
        });
        attrs.push(Attribute {
            key: "ask_asset".into(),
            value: "tokenb".into(),
        });
        let tx = TxResponse {
            height: "1".into(),
            txhash: "ABCDHASH".into(),
            logs: Some(vec![TxLog {
                events: vec![Event {
                    event_type: "wasm".into(),
                    attributes: attrs,
                }],
            }]),
            timestamp: None,
            events: None,
        };
        let p = parse_limit_order_placements(&tx);
        assert_eq!(p.len(), 5);
        assert_eq!(p[0].order_id, 1);
        assert_eq!(p[4].order_id, 5);
        assert_eq!(p[0].pair_address, "terra1pair");
        let swaps = parse_swaps(&tx);
        assert_eq!(swaps.len(), 1);
    }

    #[test]
    fn parse_limit_order_expired_parked_finds_parked_before_swap_in_merged_wasm_attrs() {
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1pair"),
            ("action", "limit_order_expired_parked"),
            ("order_id", "1"),
            ("maker", "terra1mk"),
            ("side", "bid"),
            ("remaining", "9910"),
            ("action", "swap"),
            ("sender", "terra1taker"),
            ("offer_amount", "5000"),
            ("return_amount", "100"),
            ("offer_asset", "tokena"),
            ("ask_asset", "tokenb"),
        ]);
        let evs = parse_limit_order_expired_parked(&tx);
        assert_eq!(evs.len(), 1);
        assert_eq!(evs[0].pair_address, "terra1pair");
        assert_eq!(evs[0].order_id, 1);
        assert_eq!(evs[0].remaining.to_string(), "9910");
        let swaps = parse_swaps(&tx);
        assert_eq!(swaps.len(), 1);
        assert_eq!(swaps[0].pair_address, "terra1pair");
    }

    /// LocalTerra LCD emits lifecycle CosmWasm rows as `wasm-wasm`, not merged `wasm` (GitLab #141 QA).
    #[test]
    fn parse_limit_order_expired_parked_from_wasm_wasm_event_type() {
        let tx = wasm_wasm_tx(vec![
            ("_contract_address", "terra1pair"),
            ("action", "limit_order_expired_parked"),
            ("contract_address", "terra1pair"),
            ("maker", "terra1mk"),
            ("order_id", "1"),
            ("remaining", "9910000"),
            ("side", "bid"),
        ]);
        let evs = parse_limit_order_expired_parked(&tx);
        assert_eq!(evs.len(), 1);
        assert_eq!(evs[0].pair_address, "terra1pair");
        assert_eq!(evs[0].order_id, 1);
        assert_eq!(evs[0].remaining.to_string(), "9910000");
    }

    #[test]
    fn parse_claim_expired_limit_order_event() {
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1pair"),
            ("action", "claim_expired_limit_order"),
            ("order_id", "42"),
            ("owner", "terra1mk"),
        ]);
        let evs = parse_claim_expired_limit_orders(&tx);
        assert_eq!(evs.len(), 1);
        assert_eq!(evs[0].pair_address, "terra1pair");
        assert_eq!(evs[0].order_id, 42);
    }

    #[test]
    fn parse_claim_expired_limit_order_finds_claim_before_swap_in_merged_wasm_attrs() {
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1pair"),
            ("action", "claim_expired_limit_order"),
            ("order_id", "7"),
            ("owner", "terra1mk"),
            ("action", "swap"),
            ("sender", "terra1user"),
            ("offer_amount", "10"),
            ("return_amount", "9"),
            ("offer_asset", "a"),
            ("ask_asset", "b"),
        ]);
        let claims = parse_claim_expired_limit_orders(&tx);
        assert_eq!(claims.len(), 1);
        assert_eq!(claims[0].order_id, 7);
        assert_eq!(claims[0].pair_address, "terra1pair");
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
            let _ = parse_limit_order_expired_parked(&tx);
            let _ = parse_claim_expired_limit_orders(&tx);
        }
    }
}
