//! Tx log parsing for swaps, liquidity, and hooks.
//!
//! **Invariants:** duplicate wasm attribute keys use the last value within an action where a
//! single logical value is intended. Swap, liquidity, hook, and **lifecycle wasm** (`limit_order_expired_parked`,
//! `claim_expired_limit_order`, `limit_order_fill`) may share one `wasm` event with other actions
//! when LCD output flattens attribute streams — those parsers scan **every** `action` occurrence
//! (GitLab #141, #269). Wrap-mapper fees (`notify_deposit` / `unwrap`) use the same per-action
//! scan in [`super::protocol_fees`] (GitLab #613).
//! LocalTerra also emits standalone lifecycle rows as **`wasm-wasm`** events (same attribute keys).
//! Parsing must not panic on adversarial attribute lists (see stress tests in `#[cfg(test)]`).
//! #1395: raw quantities are bounded unsigned integers; malformed mandatory or present
//! optional amounts reject their complete action, never become zero. Financial fields and
//! identities are scoped to the same action/emitter. Columnar arrays retain row indices;
//! missing entries reject an ambiguous batch. Rejected emissions retain replay ordinals.
//! Full matrix: `docs/indexer-invariants.md`.

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::PgPool;

use crate::config::Config;
use crate::db::queries::{
    assets, limit_order_fills, limit_order_lifecycle, liquidity, oracle as db_oracle,
    protocol_fees as fee_events, swap_events,
};
use crate::lcd::{Attribute, LcdClient, TxResponse};

use super::{
    asset_resolver, candle_builder,
    event_amount::{
        parse_optional_raw_amount, parse_price, parse_raw_amount, report_amount_rejection,
        AmountError,
    },
    gt_event_reserves, oracle, pair_discovery, pair_price_usd, position_tracker, protocol_fees,
    swap_orientation, trader_tracker,
};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Keep error metadata fixed and bounded; reject complete logical events on conversion errors.
fn required_amount(
    kind: &'static str,
    field: &'static str,
    value: Option<&str>,
) -> Option<BigDecimal> {
    parse_raw_amount(value)
        .map_err(|e| report_amount_rejection(kind, field, e))
        .ok()
}

fn optional_amount(
    kind: &'static str,
    field: &'static str,
    value: Option<&str>,
) -> Result<Option<BigDecimal>, AmountError> {
    parse_optional_raw_amount(value).inspect_err(|e| report_amount_rejection(kind, field, *e))
}

fn optional_integer(
    kind: &'static str,
    field: &'static str,
    value: Option<&str>,
    max: i32,
) -> Result<Option<i32>, AmountError> {
    let Some(value) = value else {
        return Ok(None);
    };
    parse_raw_amount(Some(value))
        .and_then(|_| value.parse::<i32>().map_err(|_| AmountError::Overflow))
        .and_then(|value| {
            if value <= max {
                Ok(Some(value))
            } else {
                Err(AmountError::Overflow)
            }
        })
        .inspect_err(|e| report_amount_rejection(kind, field, *e))
}

fn required_price(kind: &'static str, value: Option<&str>) -> Option<BigDecimal> {
    parse_price(value)
        .map_err(|e| report_amount_rejection(kind, "price", e))
        .ok()
}

fn optional_price(
    kind: &'static str,
    value: Option<&str>,
) -> Result<Option<BigDecimal>, AmountError> {
    value
        .map(|v| parse_price(Some(v)))
        .transpose()
        .inspect_err(|e| report_amount_rejection(kind, "price", *e))
}

fn is_wasm_contract_addr_key(key: &str) -> bool {
    // #285: `_contract_address` is runtime-reserved and authenticated. The unreserved
    // `contract_address` is caller-controlled data and never establishes an emitter.
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

/// Authenticated logical actions, in wire order. Financial fields never cross the next
/// action or runtime emitter boundary, including multiple same-family actions in one event.
type WasmAction<'a> = (
    usize,
    &'a str,
    &'a str,
    std::collections::HashMap<&'a str, &'a str>,
);

fn wasm_actions(attrs: &[Attribute]) -> impl Iterator<Item = WasmAction<'_>> {
    let mut emitter = None;
    attrs.iter().enumerate().filter_map(move |(i, a)| {
        if is_wasm_contract_addr_key(&a.key) {
            emitter = Some(a.value.as_str());
            return None;
        }
        if a.key != "action" {
            return None;
        }
        let contract = emitter?;
        Some((
            i,
            a.value.as_str(),
            contract,
            wasm_kv_map_after_action(attrs, i),
        ))
    })
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
        let Some(remaining) = required_amount(
            "limit_order_expired_parked",
            "remaining",
            seg.get("remaining").copied(),
        ) else {
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
    /// Optional pair wasm post-event `RESERVES` (GitLab #684). Taken only from this
    /// event's runtime-reserved `_contract_address` (GitLab #285).
    pub reserve_0: Option<BigDecimal>,
    pub reserve_1: Option<BigDecimal>,
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
    /// GitLab #316 / #331: 0-based ordinal of the swap (on this pair, within the tx) that produced
    /// this fill. Post-upgrade contracts emit `swap_index` on the wasm event; legacy txs rely on
    /// parser walk-order inference in [`parse_limit_order_fills`].
    swap_index: i32,
    /// When set, taken from on-chain `swap_index` wasm attr (GitLab #331).
    swap_index_from_chain: Option<i32>,
    /// Attribute offset of this fill's action within its physical LCD event. Used only
    /// to infer legacy swap ordinals when several swaps share one flattened event.
    action_pos: usize,
}

#[derive(Debug, Clone)]
struct ParsedLimitOrderPlacement {
    pair_address: String,
    order_id: i64,
    owner: Option<String>,
    side: Option<String>,
    price: Option<BigDecimal>,
    expires_at: Option<i64>,
    maker_fee_amount: Option<BigDecimal>,
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
    let mut reserve_cache: std::collections::HashMap<i32, (BigDecimal, BigDecimal)> =
        std::collections::HashMap::new();
    for tx in txs {
        if let Some(ref fee_addr) = config.fee_discount_address {
            if !fee_addr.is_empty() {
                let tier_events = trader_tracker::parse_fee_discount_registry_events(tx, fee_addr);
                if !tier_events.is_empty() {
                    trader_tracker::apply_registry_tier_events(pool, &tier_events).await?;
                }
            }
        }

        let swaps = parse_swaps(tx);
        for swap in &swaps {
            process_swap(
                pool,
                lcd,
                config,
                swap,
                height,
                block_time,
                &tx.txhash,
                ustc_price,
                &mut reserve_cache,
            )
            .await?;
        }

        let liq_events = parse_liquidity_events(tx);
        let factory_addr = config.factory_address.as_str();
        for liq in &liq_events {
            process_liquidity_event(
                pool,
                lcd,
                factory_addr,
                liq,
                height,
                block_time,
                &tx.txhash,
                &mut reserve_cache,
            )
            .await?;
        }

        let lo_fills = parse_limit_order_fills(tx);
        for fill in &lo_fills {
            process_limit_order_fill(
                pool,
                lcd,
                config,
                factory_addr,
                fill,
                height,
                block_time,
                &tx.txhash,
                ustc_price,
            )
            .await?;
        }

        let placements = parse_limit_order_placements(tx);
        for p in &placements {
            process_limit_order_placement(
                pool,
                lcd,
                config,
                factory_addr,
                p,
                height,
                block_time,
                &tx.txhash,
                ustc_price,
            )
            .await?;
        }

        let cancellations = parse_limit_order_cancellations(tx);
        for c in &cancellations {
            process_limit_order_cancellation(
                pool,
                lcd,
                factory_addr,
                c,
                height,
                block_time,
                &tx.txhash,
            )
            .await?;
        }

        let parked = parse_limit_order_expired_parked(tx);
        for p in &parked {
            process_limit_order_expired_parked(
                pool,
                lcd,
                factory_addr,
                p,
                height,
                block_time,
                &tx.txhash,
            )
            .await?;
        }

        let claims = parse_claim_expired_limit_orders(tx);
        for c in &claims {
            process_claim_expired_limit_order(
                pool,
                lcd,
                factory_addr,
                c,
                height,
                block_time,
                &tx.txhash,
            )
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

        for ev in crate::indexer::community_tokens::parse_community_events_from_tx(tx) {
            crate::indexer::community_tokens::ingest_event(pool, config, &ev, &tx.txhash, height)
                .await?;
        }

        if let Some(ref mapper) = config.wrap_mapper_address {
            for fee in protocol_fees::parse_wrap_fees(tx, mapper) {
                process_wrap_fee(
                    pool, lcd, config, &fee, height, block_time, &tx.txhash, ustc_price,
                )
                .await?;
            }
        }

        if let Some(ref window) = config.ust1_window_address {
            for fee in protocol_fees::parse_ust1_window_fees(tx, window) {
                process_wrap_fee(
                    pool, lcd, config, &fee, height, block_time, &tx.txhash, ustc_price,
                )
                .await?;
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
    reserve_cache: &mut std::collections::HashMap<i32, (BigDecimal, BigDecimal)>,
) -> Result<(), BoxError> {
    let Some(pair) = pair_discovery::get_or_discover_pair(
        pool,
        lcd,
        &config.factory_address,
        &swap.pair_address,
    )
    .await?
    else {
        return Ok(());
    };

    if swap_events::trade_exists(pool, tx_hash, pair.id, swap.swap_index).await? {
        // Replay skips swap insert (#287) but must still try fee ingest: unique
        // (tx, swap_amm, pair_id, ordinal) heals hops dropped by the old 3-column key
        // without double-counting (GitLab #1269).
        let ask_asset_id = asset_resolver::resolve_asset_str(pool, lcd, &swap.ask_asset).await?;
        ingest_swap_amm_fee(
            pool,
            lcd,
            config,
            pair.id,
            ask_asset_id,
            swap,
            height,
            block_time,
            tx_hash,
            ustc_price,
        )
        .await?;
        return Ok(());
    }

    let offer_asset_id = asset_resolver::resolve_asset_str(pool, lcd, &swap.offer_asset).await?;
    let ask_asset_id = asset_resolver::resolve_asset_str(pool, lcd, &swap.ask_asset).await?;

    let base_asset = assets::get_asset_by_id(pool, pair.asset_0_id)
        .await
        .ok()
        .flatten();
    let quote_asset = assets::get_asset_by_id(pool, pair.asset_1_id)
        .await
        .ok()
        .flatten();
    let decimals_base = base_asset.as_ref().map(|a| a.decimals).unwrap_or(6);
    let decimals_quote = quote_asset.as_ref().map(|a| a.decimals).unwrap_or(6);

    let oriented = swap_orientation::orient_swap_leg(
        pair.asset_0_id,
        offer_asset_id,
        &swap.offer_amount,
        &swap.return_amount,
        decimals_base,
        decimals_quote,
    );
    // A syntactically valid raw amount can still produce an unrepresentable derived price.
    // Reject before any economic mutation rather than wedge the block with a SQL overflow.
    if !pair_price_usd::fits_numeric_38_18(
        &oriented
            .price
            .with_scale_round(18, bigdecimal::RoundingMode::HalfUp),
    ) {
        report_amount_rejection("swap", "derived_price", AmountError::Overflow);
        return Ok(());
    }

    let ustc_usd = ustc_price.read().await.clone();
    let lunc_usd = db_oracle::get_latest_average_price(pool, oracle::OracleTicker::Lunc)
        .await
        .ok()
        .flatten();
    let hub = crate::db::queries::hub_prices::load_quote_usd(pool)
        .await
        .ok();
    let price_usd = quote_asset.as_ref().and_then(|quote| {
        pair_price_usd::price_usd_for_human_quote_per_base_pinned(
            quote,
            &oriented.price,
            ustc_usd.as_ref(),
            lunc_usd.as_ref(),
            hub.as_ref(),
            config.ustc_denom.as_deref(),
            Some(config.usdt_cw20_address.as_str()),
        )
    });

    let offer_asset = assets::get_asset_by_id(pool, offer_asset_id)
        .await
        .ok()
        .flatten();
    let ask_asset = assets::get_asset_by_id(pool, ask_asset_id)
        .await
        .ok()
        .flatten();
    let volume_usd = match (
        offer_asset.as_ref(),
        ask_asset.as_ref(),
        quote_asset.as_ref(),
    ) {
        (Some(offer), Some(ask), Some(quote)) => pair_price_usd::volume_usd_for_swap_pinned(
            offer,
            ask,
            &swap.offer_amount,
            &swap.return_amount,
            quote,
            ustc_usd.as_ref(),
            lunc_usd.as_ref(),
            config.ustc_denom.as_deref(),
            hub.as_ref(),
            Some(config.usdt_cw20_address.as_str()),
        ),
        _ => None,
    };

    let post_reserves = if let (Some(r0), Some(r1)) = (&swap.reserve_0, &swap.reserve_1) {
        Some((r0.clone(), r1.clone()))
    } else {
        match gt_event_reserves::resolve_pre_reserves(pool, reserve_cache, pair.id).await? {
            Some((pre0, pre1)) => {
                let offer_is_asset_0 = offer_asset_id == pair.asset_0_id;
                let delta = gt_event_reserves::swap_pool_delta(
                    offer_is_asset_0,
                    &swap.offer_amount,
                    &swap.return_amount,
                    swap.pool_return_amount.as_ref(),
                    swap.book_return_amount.as_ref(),
                    swap.limit_book_offer_consumed.as_ref(),
                    swap.commission_amount.as_ref(),
                );
                gt_event_reserves::apply_delta(&pre0, &pre1, &delta)
            }
            None => None,
        }
    };
    if !post_reserves_fit(&post_reserves) {
        report_amount_rejection("swap", "derived_reserves", AmountError::Overflow);
        return Ok(());
    }
    let (post_r0, post_r1) = match &post_reserves {
        Some((a, b)) => (Some(a), Some(b)),
        None => (None, None),
    };

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
        &oriented.price,
        price_usd.as_ref(),
        volume_usd.as_ref(),
        swap.pool_return_amount.as_ref(),
        swap.book_return_amount.as_ref(),
        swap.limit_book_offer_consumed.as_ref(),
        post_r0,
        post_r1,
    )
    .await?;
    if inserted.is_none() {
        return Ok(());
    }
    if let Some(post) = post_reserves {
        reserve_cache.insert(pair.id, post);
    }

    ingest_swap_amm_fee(
        pool,
        lcd,
        config,
        pair.id,
        ask_asset_id,
        swap,
        height,
        block_time,
        tx_hash,
        ustc_price,
    )
    .await?;

    let subject = match (base_asset.as_ref(), quote_asset.as_ref()) {
        (Some(base), Some(quote)) => pair_price_usd::candle_usd_subject(
            base,
            quote,
            &oriented.price,
            pair_price_usd::CandleUsdPrints {
                ustc_usd: ustc_usd.as_ref(),
                lunc_usd: lunc_usd.as_ref(),
                hub: hub.as_ref(),
                configured_ustc_denom: config.ustc_denom.as_deref(),
                configured_usdt_address: Some(config.usdt_cw20_address.as_str()),
            },
        ),
        _ => pair_price_usd::CandleUsdSubject::Skip,
    };

    candle_builder::update_candles_for_swap(
        pool,
        pair.id,
        block_time,
        &subject,
        &oriented.price,
        &oriented.volume_base,
        &oriented.volume_quote,
    )
    .await?;

    trader_tracker::update_trader_on_swap(
        pool,
        lcd,
        config.fee_discount_address.as_deref(),
        &swap.sender,
        &swap.offer_amount,
        volume_usd.as_ref(),
    )
    .await?;

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

async fn ingest_swap_amm_fee(
    pool: &PgPool,
    lcd: &LcdClient,
    config: &Config,
    pair_id: i32,
    ask_asset_id: i32,
    swap: &ParsedSwap,
    height: i64,
    block_time: DateTime<Utc>,
    tx_hash: &str,
    ustc_price: &oracle::SharedPrice,
) -> Result<(), BoxError> {
    let Some(commission) = swap.commission_amount.as_ref() else {
        return Ok(());
    };
    if *commission <= BigDecimal::from(0) {
        return Ok(());
    }
    ingest_protocol_fee(
        pool,
        lcd,
        config,
        protocol_fees::FeeSource::SwapAmm,
        i64::from(swap.swap_index),
        Some(pair_id),
        ask_asset_id,
        commission,
        height,
        block_time,
        tx_hash,
        ustc_price,
    )
    .await
}

async fn ingest_protocol_fee(
    pool: &PgPool,
    _lcd: &LcdClient,
    config: &Config,
    source: protocol_fees::FeeSource,
    ordinal: i64,
    pair_id: Option<i32>,
    asset_id: i32,
    amount_raw: &BigDecimal,
    height: i64,
    block_time: DateTime<Utc>,
    tx_hash: &str,
    ustc_price: &oracle::SharedPrice,
) -> Result<(), BoxError> {
    if *amount_raw <= BigDecimal::from(0) {
        return Ok(());
    }
    let Some(asset) = assets::get_asset_by_id(pool, asset_id).await.ok().flatten() else {
        return Ok(());
    };
    let ustc_usd = ustc_price.read().await.clone();
    let lunc_usd = db_oracle::get_latest_average_price(pool, oracle::OracleTicker::Lunc)
        .await
        .ok()
        .flatten();
    let hub = crate::db::queries::hub_prices::load_quote_usd(pool)
        .await
        .ok();
    let fee_usd = protocol_fees::fee_usd_for_raw(
        &asset,
        amount_raw,
        ustc_usd.as_ref(),
        lunc_usd.as_ref(),
        config.ustc_denom.as_deref(),
        hub.as_ref(),
    );
    let draft = protocol_fees::FeeEventDraft {
        block_height: height,
        block_timestamp: block_time,
        tx_hash: tx_hash.to_string(),
        source,
        ordinal,
        pair_id,
        asset_id,
        amount_raw: amount_raw.clone(),
        decimals: asset.decimals,
        fee_usd,
    };
    fee_events::insert_fee_event(pool, &draft).await?;
    Ok(())
}

async fn process_wrap_fee(
    pool: &PgPool,
    lcd: &LcdClient,
    config: &Config,
    fee: &protocol_fees::ParsedWrapFee,
    height: i64,
    block_time: DateTime<Utc>,
    tx_hash: &str,
    ustc_price: &oracle::SharedPrice,
) -> Result<(), BoxError> {
    let asset_id = asset_resolver::resolve_asset_str(pool, lcd, &fee.token).await?;
    ingest_protocol_fee(
        pool,
        lcd,
        config,
        fee.source,
        fee.ordinal,
        None,
        asset_id,
        &fee.amount_raw,
        height,
        block_time,
        tx_hash,
        ustc_price,
    )
    .await
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

        for (_, action, contract, seg) in wasm_actions(attrs) {
            if action != "swap" {
                continue;
            }
            let Some(sender) = seg.get("sender").copied() else {
                continue;
            };
            // Reserve each authenticated emission's ordinal before validating amounts.
            // A rejected hop must not change a following valid hop's replay identity.
            let swap_index = {
                let c = per_pair_swap_index.entry(contract.to_string()).or_insert(0);
                let i = *c;
                *c += 1;
                i
            };
            if let Some(swap) = parse_swap_amounts(&seg, contract, sender, swap_index) {
                swaps.push(swap);
            }
        }
    }

    swaps
}

fn parse_swap_amounts(
    seg: &std::collections::HashMap<&str, &str>,
    contract: &str,
    sender: &str,
    swap_index: i32,
) -> Option<ParsedSwap> {
    // Last duplicate wins only within this authenticated logical action (#285).
    let required = |field| required_amount("swap", field, seg.get(field).copied());
    let optional = |field| optional_amount("swap", field, seg.get(field).copied()).ok();
    Some(ParsedSwap {
        pair_address: contract.to_string(),
        swap_index,
        sender: sender.to_string(),
        receiver: seg.get("receiver").map(|s| s.to_string()),
        offer_asset: seg.get("offer_asset").copied().unwrap_or("").to_string(),
        ask_asset: seg.get("ask_asset").copied().unwrap_or("").to_string(),
        offer_amount: required("offer_amount")?,
        return_amount: required("return_amount")?,
        spread_amount: optional("spread_amount")?,
        commission_amount: optional("commission_amount")?,
        effective_fee_bps: optional_integer(
            "swap",
            "effective_fee_bps",
            seg.get("effective_fee_bps").copied(),
            10_000,
        )
        .ok()?
        .map(|v| v as i16),
        pool_return_amount: optional("pool_return_amount")?,
        book_return_amount: optional("book_return_amount")?,
        limit_book_offer_consumed: optional("limit_book_offer_consumed")?,
        reserve_0: optional("reserve_0")?,
        reserve_1: optional("reserve_1")?,
    })
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
    let price = required_price("limit_order_fill", seg.get("price").copied())?;
    let token0_amount = required_amount(
        "limit_order_fill",
        "token0_amount",
        seg.get("token0_amount").copied(),
    )?;
    let token1_amount = required_amount(
        "limit_order_fill",
        "token1_amount",
        seg.get("token1_amount").copied(),
    )?;
    let commission_amount = required_amount(
        "limit_order_fill",
        "commission_amount",
        seg.get("commission_amount").copied(),
    )?;
    let swap_index_from_chain = optional_integer(
        "limit_order_fill",
        "swap_index",
        seg.get("swap_index").copied(),
        i32::MAX,
    )
    .ok()?;
    Some(ParsedLimitOrderFill {
        pair_address: contract.to_string(),
        order_id,
        side: side.to_string(),
        maker: maker.to_string(),
        price,
        token0_amount,
        token1_amount,
        commission_amount,
        swap_index: swap_index_from_chain.unwrap_or(0),
        swap_index_from_chain,
        action_pos: 0,
    })
}

/// Merged LCD streams may list multiple `limit_order_fill` actions with columnar field keys.
fn parse_limit_order_fills_columnar(
    contract: &str,
    attrs: &[Attribute],
) -> Option<Vec<ParsedLimitOrderFill>> {
    let fill_count = attrs
        .iter()
        .filter(|a| a.key == "action" && a.value == "limit_order_fill")
        .count();
    if fill_count <= 1 {
        return None;
    }
    // Only columnar streams have consecutive action headers. Interleaved streams need
    // per-action provenance and segments, even when all columns happen to have equal lengths.
    if !has_columnar_actions(attrs, "limit_order_fill") {
        return None;
    }
    let keys = [
        "order_id",
        "side",
        "maker",
        "price",
        "token0_amount",
        "token1_amount",
        "commission_amount",
        "swap_index",
    ];
    let columns: Vec<Vec<&str>> = keys.iter().map(|k| wasm_attr_values(attrs, k)).collect();
    // Missing column entries carry no row index. Quarantine this ambiguous batch rather
    // than shift later values onto earlier identities; explicit invalid entries retain indexes.
    for (key, column) in keys.iter().zip(&columns) {
        let valid_length = match *key {
            "side" | "maker" => column.len() == 1 || column.len() == fill_count,
            "swap_index" => column.is_empty() || column.len() == fill_count,
            _ => column.len() == fill_count,
        };
        if !valid_length {
            report_amount_rejection("limit_order_fill", key, AmountError::Missing);
            return Some(Vec::new());
        }
    }
    let mut out = Vec::new();
    for i in 0..fill_count {
        let mut seg = std::collections::HashMap::new();
        for (key, column) in keys.iter().zip(&columns) {
            let value = if matches!(*key, "side" | "maker") {
                column.get(i).or_else(|| column.first())
            } else {
                column.get(i)
            };
            if let Some(value) = value {
                seg.insert(*key, *value);
            }
        }
        if let Some(fill) = parse_limit_order_fill_segment(contract, &seg) {
            out.push(fill);
        }
    }
    Some(out)
}

/// Detect the columnar wire format without mistaking interleaved actions for columns.
fn has_columnar_actions(attrs: &[Attribute], action: &str) -> bool {
    attrs.windows(2).any(|w| {
        w[0].key == "action" && w[0].value == action && w[1].key == "action" && w[1].value == action
    })
}

fn parse_limit_order_fills_from_wasm_attrs(attrs: &[Attribute]) -> Vec<ParsedLimitOrderFill> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < attrs.len() {
        if attrs[i].key != "action" || attrs[i].value != "limit_order_fill" {
            i += 1;
            continue;
        }
        let Some(contract) = wasm_contract_addr_before(attrs, i) else {
            i += 1;
            continue;
        };
        let mut headers_end = i + 1;
        while headers_end < attrs.len()
            && attrs[headers_end].key == "action"
            && attrs[headers_end].value == "limit_order_fill"
        {
            headers_end += 1;
        }
        if headers_end > i + 1 {
            // A columnar batch ends before another action or authenticated emitter. This
            // prevents later swap fee columns or a foreign batch from contaminating rows.
            let end = attrs[headers_end..]
                .iter()
                .position(|a| a.key == "action" || is_wasm_contract_addr_key(&a.key))
                .map(|offset| headers_end + offset)
                .unwrap_or(attrs.len());
            if let Some(mut batch) = parse_limit_order_fills_columnar(contract, &attrs[i..end]) {
                for fill in &mut batch {
                    fill.action_pos = i;
                }
                out.extend(batch);
            }
            i = end;
        } else {
            let seg = wasm_kv_map_after_action(attrs, i);
            if let Some(mut event) = parse_limit_order_fill_segment(contract, &seg) {
                event.action_pos = i;
                out.push(event);
            }
            i += 1;
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

    // Maker fills precede their producing swap. Count all authenticated swap emissions,
    // including rejected amounts, before each fill's physical action offset. Explicit
    // on-chain indices retain priority; legacy flattened swaps share the same inference.
    let mut per_pair_swap_seen: std::collections::HashMap<String, i32> =
        std::collections::HashMap::new();
    let mut out = Vec::new();
    for event in &events {
        if !is_wasm_lifecycle_event_type(&event.event_type) {
            continue;
        }
        let attrs = &event.attributes;
        let swap_positions: Vec<(usize, &str)> = if event.event_type == "wasm" {
            wasm_actions(attrs)
                .filter_map(|(pos, action, contract, seg)| {
                    (action == "swap" && seg.contains_key("sender")).then_some((pos, contract))
                })
                .collect()
        } else {
            Vec::new()
        };
        let mut fills = parse_limit_order_fills_from_wasm_attrs(attrs);
        for fill in &mut fills {
            let prior_event_count = per_pair_swap_seen
                .get(&fill.pair_address)
                .copied()
                .unwrap_or(0);
            let preceding_in_event = swap_positions
                .iter()
                .filter(|(pos, contract)| *pos < fill.action_pos && *contract == fill.pair_address)
                .count() as i32;
            fill.swap_index = fill
                .swap_index_from_chain
                .unwrap_or(prior_event_count + preceding_in_event);
        }
        out.append(&mut fills);
        for (_, contract) in swap_positions {
            *per_pair_swap_seen.entry(contract.to_string()).or_insert(0) += 1;
        }
    }
    out
}

async fn process_limit_order_fill(
    pool: &PgPool,
    lcd: &LcdClient,
    config: &Config,
    factory_addr: &str,
    fill: &ParsedLimitOrderFill,
    height: i64,
    block_time: DateTime<Utc>,
    tx_hash: &str,
    ustc_price: &oracle::SharedPrice,
) -> Result<(), BoxError> {
    let Some(pair) =
        pair_discovery::get_or_discover_pair(pool, lcd, factory_addr, &fill.pair_address).await?
    else {
        return Ok(());
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

    if fill.commission_amount > BigDecimal::from(0) {
        if let Some(asset_id) =
            protocol_fees::offer_asset_id_for_side(&fill.side, pair.asset_0_id, pair.asset_1_id)
        {
            ingest_protocol_fee(
                pool,
                lcd,
                config,
                protocol_fees::FeeSource::BookTake,
                fill.order_id,
                None,
                asset_id,
                &fill.commission_amount,
                height,
                block_time,
                tx_hash,
                ustc_price,
            )
            .await?;
        }
    }

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
    contract: &str,
    attrs: &[Attribute],
) -> Option<Vec<ParsedLimitOrderPlacement>> {
    if !has_columnar_actions(attrs, "place_limit_order") {
        return None;
    }
    let keys = [
        "order_id",
        "limit_order_placed",
        "side",
        "owner",
        "price",
        "expires_at",
        "maker_fee_amount",
    ];
    let columns: Vec<Vec<&str>> = keys.iter().map(|k| wasm_attr_values(attrs, k)).collect();
    let n = attrs
        .iter()
        .filter(|a| a.key == "action" && a.value == "place_limit_order")
        .count();
    for (key, column) in keys.iter().zip(&columns) {
        let valid_length = match *key {
            "side" | "owner" => column.is_empty() || column.len() == 1 || column.len() == n,
            _ => column.is_empty() || column.len() == n,
        };
        if !valid_length {
            report_amount_rejection("place_limit_order", key, AmountError::Missing);
            return Some(Vec::new());
        }
    }
    let mut out = Vec::new();
    for i in 0..n {
        let mut seg = std::collections::HashMap::new();
        for (key, column) in keys.iter().zip(&columns) {
            let value = if matches!(*key, "side" | "owner") {
                column.get(i).or_else(|| column.first())
            } else {
                column.get(i)
            };
            if let Some(value) = value {
                seg.insert(*key, *value);
            }
        }
        if let Some(p) = parse_limit_order_placement_segment(contract, &seg) {
            out.push(p);
        }
    }
    Some(out)
}

fn parse_limit_order_placement_segment(
    contract: &str,
    seg: &std::collections::HashMap<&str, &str>,
) -> Option<ParsedLimitOrderPlacement> {
    let order_id = seg
        .get("order_id")
        .or_else(|| seg.get("limit_order_placed"))?
        .parse::<i64>()
        .ok()?;
    Some(ParsedLimitOrderPlacement {
        pair_address: contract.to_string(),
        order_id,
        owner: seg.get("owner").map(|s| s.to_string()),
        side: seg.get("side").map(|s| s.to_string()),
        price: optional_price("place_limit_order", seg.get("price").copied()).ok()?,
        expires_at: seg.get("expires_at").and_then(|s| s.parse().ok()),
        maker_fee_amount: optional_amount(
            "place_limit_order",
            "maker_fee_amount",
            seg.get("maker_fee_amount").copied(),
        )
        .ok()?,
    })
}

fn parse_limit_order_placements_from_wasm_attrs(
    attrs: &[Attribute],
) -> Vec<ParsedLimitOrderPlacement> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < attrs.len() {
        if attrs[i].key != "action" || attrs[i].value != "place_limit_order" {
            i += 1;
            continue;
        }
        let Some(contract) = wasm_contract_addr_before(attrs, i) else {
            i += 1;
            continue;
        };
        let mut headers_end = i + 1;
        while headers_end < attrs.len()
            && attrs[headers_end].key == "action"
            && attrs[headers_end].value == "place_limit_order"
        {
            headers_end += 1;
        }
        if headers_end > i + 1 {
            // A columnar batch ends before another action or authenticated emitter. This
            // prevents later swap fee columns or a foreign batch from contaminating rows.
            let end = attrs[headers_end..]
                .iter()
                .position(|a| a.key == "action" || is_wasm_contract_addr_key(&a.key))
                .map(|offset| headers_end + offset)
                .unwrap_or(attrs.len());
            if let Some(batch) = parse_limit_order_placements_columnar(contract, &attrs[i..end]) {
                out.extend(batch);
            }
            i = end;
        } else {
            let seg = wasm_kv_map_after_action(attrs, i);
            if let Some(event) = parse_limit_order_placement_segment(contract, &seg) {
                out.push(event);
            }
            i += 1;
        }
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
            owner: owners.get(i).cloned().or_else(|| owners.first().cloned()),
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
    config: &Config,
    factory_addr: &str,
    p: &ParsedLimitOrderPlacement,
    height: i64,
    block_time: DateTime<Utc>,
    tx_hash: &str,
    ustc_price: &oracle::SharedPrice,
) -> Result<(), BoxError> {
    let Some(pair) =
        pair_discovery::get_or_discover_pair(pool, lcd, factory_addr, &p.pair_address).await?
    else {
        return Ok(());
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

    if let Some(fee) = p.maker_fee_amount.as_ref() {
        if *fee > BigDecimal::from(0) {
            if let Some(side) = p.side.as_deref() {
                if let Some(asset_id) =
                    protocol_fees::offer_asset_id_for_side(side, pair.asset_0_id, pair.asset_1_id)
                {
                    let _ = sqlx::query(
                        "UPDATE limit_order_placements SET maker_fee_amount = $1 \
                         WHERE tx_hash = $2 AND pair_id = $3 AND order_id = $4",
                    )
                    .bind(fee)
                    .bind(tx_hash)
                    .bind(pair.id)
                    .bind(p.order_id)
                    .execute(pool)
                    .await;
                    ingest_protocol_fee(
                        pool,
                        lcd,
                        config,
                        protocol_fees::FeeSource::LimitPlace,
                        p.order_id,
                        None,
                        asset_id,
                        fee,
                        height,
                        block_time,
                        tx_hash,
                        ustc_price,
                    )
                    .await?;
                }
            }
        }
    }
    Ok(())
}

async fn process_limit_order_cancellation(
    pool: &PgPool,
    lcd: &LcdClient,
    factory_addr: &str,
    c: &ParsedLimitOrderCancellation,
    height: i64,
    block_time: DateTime<Utc>,
    tx_hash: &str,
) -> Result<(), BoxError> {
    let Some(pair) =
        pair_discovery::get_or_discover_pair(pool, lcd, factory_addr, &c.pair_address).await?
    else {
        return Ok(());
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
    factory_addr: &str,
    p: &ParsedLimitOrderExpiredParked,
    height: i64,
    block_time: DateTime<Utc>,
    tx_hash: &str,
) -> Result<(), BoxError> {
    let Some(pair) =
        pair_discovery::get_or_discover_pair(pool, lcd, factory_addr, &p.pair_address).await?
    else {
        return Ok(());
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
    factory_addr: &str,
    c: &ParsedClaimExpiredLimitOrder,
    height: i64,
    block_time: DateTime<Utc>,
    tx_hash: &str,
) -> Result<(), BoxError> {
    let Some(pair) =
        pair_discovery::get_or_discover_pair(pool, lcd, factory_addr, &c.pair_address).await?
    else {
        return Ok(());
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
    reserve_0: Option<BigDecimal>,
    reserve_1: Option<BigDecimal>,
}

/// Derived optional reserves must fit the same raw storage domain before ledger mutation.
fn post_reserves_fit(reserves: &Option<(BigDecimal, BigDecimal)>) -> bool {
    let upper = pair_price_usd::ten_pow_i32(38);
    reserves.as_ref().is_none_or(|(r0, r1)| {
        r0 >= &BigDecimal::from(0) && r0 < &upper && r1 >= &BigDecimal::from(0) && r1 < &upper
    })
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
    for event in events {
        if event.event_type != "wasm" {
            continue;
        }
        for (_, action, contract, seg) in wasm_actions(&event.attributes) {
            let (event_type, assets_key, lp_key) = match action {
                "provide_liquidity" => ("add", "assets", "share"),
                "withdraw_liquidity" => ("remove", "refund_assets", "withdrawn_share"),
                _ => continue,
            };
            let Some(provider) = seg.get("sender").copied() else {
                continue;
            };
            let parsed = (|| {
                let (a0, a1) = parse_asset_amounts(seg.get(assets_key).copied(), assets_key)?;
                let lp = required_amount("liquidity", lp_key, seg.get(lp_key).copied())?;
                Some(ParsedLiquidityEvent {
                    pair_address: contract.to_string(),
                    provider: provider.to_string(),
                    event_type: event_type.to_string(),
                    asset_0_amount: a0,
                    asset_1_amount: a1,
                    lp_amount: lp,
                    reserve_0: optional_amount(
                        "liquidity",
                        "reserve_0",
                        seg.get("reserve_0").copied(),
                    )
                    .ok()?,
                    reserve_1: optional_amount(
                        "liquidity",
                        "reserve_1",
                        seg.get("reserve_1").copied(),
                    )
                    .ok()?,
                })
            })();
            if let Some(parsed) = parsed {
                out.push(parsed);
            }
        }
    }
    out
}

/// Parse the stringified assets attribute emitted by pair contracts.
/// Format: `"<addr_or_denom> <amount>, <addr_or_denom> <amount>"`
fn parse_asset_amounts(
    assets: Option<&str>,
    field: &'static str,
) -> Option<(BigDecimal, BigDecimal)> {
    let Some(assets) = assets else {
        report_amount_rejection("liquidity", field, AmountError::Missing);
        return None;
    };
    // Two raw 38-digit values plus chain identifiers fit this bounded representation.
    // This bound is checked before splitting; no unbounded Vec allocation from attacker text.
    if assets.len() > 512 {
        report_amount_rejection("liquidity", field, AmountError::Overflow);
        return None;
    }
    let mut parts = assets.split(", ");
    let parsed = (|| {
        let (asset0, raw0) = parts.next()?.split_once(' ')?;
        let (asset1, raw1) = parts.next()?.split_once(' ')?;
        if asset0.is_empty() || asset1.is_empty() || parts.next().is_some() {
            return None;
        }
        Some((raw0, raw1))
    })();
    let Some((raw0, raw1)) = parsed else {
        report_amount_rejection("liquidity", field, AmountError::Invalid);
        return None;
    };
    Some((
        required_amount("liquidity", field, Some(raw0))?,
        required_amount("liquidity", field, Some(raw1))?,
    ))
}

async fn process_liquidity_event(
    pool: &PgPool,
    lcd: &LcdClient,
    factory_addr: &str,
    event: &ParsedLiquidityEvent,
    height: i64,
    block_time: DateTime<Utc>,
    tx_hash: &str,
    reserve_cache: &mut std::collections::HashMap<i32, (BigDecimal, BigDecimal)>,
) -> Result<(), BoxError> {
    let Some(pair) =
        pair_discovery::get_or_discover_pair(pool, lcd, factory_addr, &event.pair_address).await?
    else {
        return Ok(());
    };

    if liquidity::liquidity_event_exists(pool, tx_hash, pair.id, &event.event_type).await? {
        return Ok(());
    }

    let post_reserves = if let (Some(r0), Some(r1)) = (&event.reserve_0, &event.reserve_1) {
        Some((r0.clone(), r1.clone()))
    } else {
        match gt_event_reserves::resolve_pre_reserves(pool, reserve_cache, pair.id).await? {
            Some((pre0, pre1)) => {
                let delta = gt_event_reserves::liquidity_delta(
                    event.event_type == "add",
                    &event.asset_0_amount,
                    &event.asset_1_amount,
                );
                gt_event_reserves::apply_delta(&pre0, &pre1, &delta)
            }
            None => None,
        }
    };
    if !post_reserves_fit(&post_reserves) {
        report_amount_rejection("liquidity", "derived_reserves", AmountError::Overflow);
        return Ok(());
    }
    let (post_r0, post_r1) = match &post_reserves {
        Some((a, b)) => (Some(a), Some(b)),
        None => (None, None),
    };

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
        post_r0,
        post_r1,
    )
    .await?;
    if let Some(post) = post_reserves {
        reserve_cache.insert(pair.id, post);
    }

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
    for event in events {
        if event.event_type != "wasm" {
            continue;
        }
        for (_, action, contract, seg) in wasm_actions(&event.attributes) {
            if !action.starts_with("after_swap_") {
                continue;
            }
            let Ok(burn_amount) =
                optional_amount("hook", "burn_amount", seg.get("burn_amount").copied())
            else {
                continue;
            };
            let Ok(tax_amount) =
                optional_amount("hook", "tax_amount", seg.get("tax_amount").copied())
            else {
                continue;
            };
            hooks.push(ParsedHookEvent {
                hook_address: contract.to_string(),
                action: action.to_string(),
                amount: burn_amount.or(tax_amount),
                token: seg
                    .get("burn_token")
                    .or_else(|| seg.get("tax_token"))
                    .or_else(|| seg.get("lp_token"))
                    .map(|s| s.to_string()),
                skipped: seg.get("skipped").map(|s| s.to_string()),
                warning: seg.get("warning").map(|s| s.to_string()),
            });
        }
    }
    hooks
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lcd::{Attribute, Event, TxLog, TxResponse};

    #[test]
    fn issue1395_many_poison_actions_keep_ordinals_and_forward_emitter_scope() {
        let mut attrs = vec![("_contract_address", "terra1first")];
        for _ in 0..10_000 {
            attrs.extend([("action", "swap"), ("sender", "terra1synthetic")]);
        }
        attrs.extend([
            ("action", "swap"),
            ("sender", "terra1synthetic"),
            ("offer_amount", "2"),
            ("return_amount", "1"),
            ("_contract_address", "terra1second"),
            ("action", "swap"),
            ("sender", "terra1synthetic"),
            ("offer_amount", "3"),
            ("return_amount", "2"),
        ]);
        let swaps = parse_swaps(&wasm_tx(attrs));
        assert_eq!(swaps.len(), 2);
        assert_eq!(swaps[0].pair_address, "terra1first");
        assert_eq!(swaps[0].swap_index, 10_000);
        assert_eq!(swaps[1].pair_address, "terra1second");
        assert_eq!(swaps[1].swap_index, 0);
    }

    /// Every raw economic family uses the same strict integer domain; absent optional
    /// fields are preserved as NULL, never confused with malformed present values.
    #[test]
    fn issue1395_economic_amount_matrix() {
        let max = "9".repeat(38);
        let huge = "1".repeat(1_000_000);
        let cases = [
            Some("42"),
            Some("0"),
            Some(max.as_str()),
            None,
            Some("-1"),
            Some(" 1"),
            Some("1 "),
            Some("1.5"),
            Some("1e3"),
            Some("bad"),
            Some(""),
            Some(huge.as_str()),
        ];
        let families = [
            ("swap", "offer_amount", false),
            ("swap", "return_amount", false),
            ("swap", "commission_amount", true),
            ("swap", "spread_amount", true),
            ("swap", "pool_return_amount", true),
            ("swap", "book_return_amount", true),
            ("swap", "limit_book_offer_consumed", true),
            ("swap", "reserve_0", true),
            ("swap", "reserve_1", true),
            ("provide_liquidity", "share", false),
            ("withdraw_liquidity", "withdrawn_share", false),
            ("provide_liquidity", "reserve_0", true),
            ("limit_order_fill", "token0_amount", false),
            ("limit_order_fill", "token1_amount", false),
            ("limit_order_fill", "commission_amount", false),
            ("place_limit_order", "maker_fee_amount", true),
            ("limit_order_expired_parked", "remaining", false),
            ("after_swap_burn", "burn_amount", true),
            ("after_swap_tax", "tax_amount", true),
        ];
        for (action, field, optional) in families {
            for value in cases {
                let mut attrs = vec![
                    ("_contract_address", "terra1syntheticpair"),
                    ("action", action),
                    ("sender", "terra1synthetictrader"),
                    ("offer_amount", "10"),
                    ("return_amount", "9"),
                    ("order_id", "7"),
                    ("maker", "terra1syntheticmaker"),
                    ("side", "bid"),
                    ("price", "1.000000000000000001"),
                    ("token0_amount", "10"),
                    ("token1_amount", "10"),
                    ("commission_amount", "0"),
                    ("assets", "uluna 10, uusd 10"),
                    ("share", "10"),
                    ("refund_assets", "uluna 10, uusd 10"),
                    ("withdrawn_share", "10"),
                    ("remaining", "10"),
                ];
                attrs.retain(|(key, _)| *key != field);
                if let Some(value) = value {
                    attrs.push((field, value));
                }
                let tx = wasm_tx(attrs);
                let count = match action {
                    "swap" => parse_swaps(&tx).len(),
                    "provide_liquidity" | "withdraw_liquidity" => parse_liquidity_events(&tx).len(),
                    "limit_order_fill" => parse_limit_order_fills(&tx).len(),
                    "place_limit_order" => parse_limit_order_placements(&tx).len(),
                    "limit_order_expired_parked" => parse_limit_order_expired_parked(&tx).len(),
                    _ => parse_hook_events(&tx).len(),
                };
                let valid = value.is_some_and(|v| matches!(v, "42" | "0") || v == max)
                    || (optional && value.is_none());
                assert_eq!(
                    count,
                    usize::from(valid),
                    "{action}.{field}: {}",
                    value
                        .map(|v| if v.len() > 100 { "<oversized>" } else { v })
                        .unwrap_or("<missing>")
                );
            }
        }
    }

    #[test]
    fn issue1395_price_and_fee_metadata_matrix() {
        for value in [
            None,
            Some("0"),
            Some("1.000000000000000001"),
            Some("99999999999999999999.999999999999999999"),
            Some("-1"),
            Some(" 1"),
            Some("1e2"),
            Some("100000000000000000000"),
            Some("0.0000000000000000001"),
        ] {
            for action in ["limit_order_fill", "place_limit_order"] {
                let mut attrs = vec![
                    ("_contract_address", "terra1synthetic"),
                    ("action", action),
                    ("order_id", "1"),
                    ("side", "bid"),
                    ("maker", "terra1maker"),
                    ("token0_amount", "1"),
                    ("token1_amount", "1"),
                    ("commission_amount", "0"),
                ];
                if let Some(value) = value {
                    attrs.push(("price", value));
                }
                let valid = value.is_some_and(|v| {
                    v == "0"
                        || v == "1.000000000000000001"
                        || v == "99999999999999999999.999999999999999999"
                }) || (action == "place_limit_order" && value.is_none());
                let tx = wasm_tx(attrs);
                let count = if action == "limit_order_fill" {
                    parse_limit_order_fills(&tx).len()
                } else {
                    parse_limit_order_placements(&tx).len()
                };
                assert_eq!(count, usize::from(valid), "{action}.{value:?}");
            }
        }
        for value in ["bad", "-1", " 1", "1.5", "1e2", "10001"] {
            let tx = wasm_tx(vec![
                ("_contract_address", "terra1synthetic"),
                ("action", "swap"),
                ("sender", "terra1user"),
                ("offer_amount", "1"),
                ("return_amount", "1"),
                ("effective_fee_bps", value),
            ]);
            assert!(parse_swaps(&tx).is_empty(), "invalid effective fee {value}");
        }
    }

    #[test]
    fn issue1395_liquidity_asset_matrix() {
        for action in ["provide_liquidity", "withdraw_liquidity"] {
            let (asset_field, lp_field) = if action == "provide_liquidity" {
                ("assets", "share")
            } else {
                ("refund_assets", "withdrawn_share")
            };
            for asset_value in [
                None,
                Some(""),
                Some("uluna -1, uusd 1"),
                Some("uluna 1.5, uusd 1"),
                Some("uluna 1e2, uusd 1"),
                Some("uluna  1, uusd 1"),
                Some("uluna 1, uusd 1, ux 1"),
                Some("uluna 1"),
                Some("uluna 1, uusd 0"),
            ] {
                let mut attrs = vec![
                    ("_contract_address", "terra1pair"),
                    ("action", action),
                    ("sender", "terra1synthetic"),
                    (lp_field, "0"),
                ];
                if let Some(value) = asset_value {
                    attrs.push((asset_field, value));
                }
                assert_eq!(
                    parse_liquidity_events(&wasm_tx(attrs)).len(),
                    usize::from(asset_value == Some("uluna 1, uusd 0"))
                );
            }
        }
    }

    #[test]
    fn issue1395_rejected_swap_preserves_following_fill_and_swap_ordinals() {
        let tx = wasm_tx_multi(vec![
            vec![
                ("_contract_address", "terra1pair"),
                ("action", "swap"),
                ("sender", "terra1synthetic"),
                ("offer_amount", "bad"),
                ("return_amount", "1"),
            ],
            vec![
                ("_contract_address", "terra1pair"),
                ("action", "limit_order_fill"),
                ("order_id", "3"),
                ("side", "bid"),
                ("maker", "terra1maker"),
                ("price", "1"),
                ("token0_amount", "1"),
                ("token1_amount", "1"),
                ("commission_amount", "0"),
                ("action", "swap"),
                ("sender", "terra1synthetic"),
                ("offer_amount", "2"),
                ("return_amount", "2"),
            ],
        ]);
        for _ in 0..3 {
            let swaps = parse_swaps(&tx);
            assert_eq!(swaps.len(), 1);
            assert_eq!(swaps[0].swap_index, 1);
            let fills = parse_limit_order_fills(&tx);
            assert_eq!(fills.len(), 1);
            assert_eq!(fills[0].swap_index, 1);
        }
    }

    #[test]
    fn issue1395_columnar_poison_does_not_shift_valid_sibling_amounts() {
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1syntheticpair"),
            ("action", "limit_order_fill"),
            ("action", "limit_order_fill"),
            ("action", "limit_order_fill"),
            ("order_id", "1"),
            ("order_id", "2"),
            ("order_id", "3"),
            ("side", "bid"),
            ("maker", "terra1maker"),
            ("price", "1"),
            ("price", "1"),
            ("price", "1"),
            ("token0_amount", "10"),
            ("token0_amount", "bad"),
            ("token0_amount", "30"),
            ("token1_amount", "11"),
            ("token1_amount", "22"),
            ("token1_amount", "33"),
            ("commission_amount", "1"),
            ("commission_amount", "2"),
            ("commission_amount", "3"),
        ]);
        let fills = parse_limit_order_fills(&tx);
        assert_eq!(fills.len(), 2);
        assert_eq!(fills[0].order_id, 1);
        assert_eq!(fills[1].order_id, 3);
        assert_eq!(fills[1].token0_amount, BigDecimal::from(30));
        assert_eq!(fills[1].token1_amount, BigDecimal::from(33));
        assert_eq!(fills[1].commission_amount, BigDecimal::from(3));

        let tx = wasm_tx(vec![
            ("_contract_address", "terra1syntheticpair"),
            ("action", "place_limit_order"),
            ("action", "place_limit_order"),
            ("action", "place_limit_order"),
            ("order_id", "1"),
            ("order_id", "2"),
            ("order_id", "3"),
            ("maker_fee_amount", "1"),
            ("maker_fee_amount", "bad"),
            ("maker_fee_amount", "3"),
            ("price", "1.1"),
            ("price", "2.2"),
            ("price", "3.3"),
        ]);
        let places = parse_limit_order_placements(&tx);
        assert_eq!(places.len(), 2);
        assert_eq!(places[1].order_id, 3);
        assert_eq!(places[1].maker_fee_amount, Some(BigDecimal::from(3)));
        assert_eq!(places[1].price.as_ref().unwrap().to_string(), "3.3");
    }

    #[test]
    fn issue1395_malformed_fill_preserves_flattened_valid_swap() {
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1syntheticpair"),
            ("action", "limit_order_fill"),
            ("order_id", "1"),
            ("side", "bid"),
            ("maker", "terra1maker"),
            ("price", "1"),
            ("token0_amount", "bad"),
            ("token1_amount", "1"),
            ("commission_amount", "bad"),
            ("action", "swap"),
            ("sender", "terra1synthetic"),
            ("offer_amount", "1"),
            ("return_amount", "1"),
            ("commission_amount", "0"),
        ]);
        assert!(parse_limit_order_fills(&tx).is_empty());
        let swaps = parse_swaps(&tx);
        assert_eq!(swaps.len(), 1);
        assert_eq!(swaps[0].commission_amount, Some(BigDecimal::from(0)));
    }

    #[test]
    fn issue1395_rejection_preserves_authenticated_provenance() {
        let tx = wasm_tx(vec![
            ("contract_address", "terra1spoof"),
            ("action", "swap"),
            ("sender", "terra1synthetic"),
            ("offer_amount", "1"),
            ("return_amount", "1"),
        ]);
        assert!(parse_swaps(&tx).is_empty());
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1authenticated"),
            ("contract_address", "terra1spoof"),
            ("action", "swap"),
            ("sender", "terra1synthetic"),
            ("offer_amount", "1"),
            ("return_amount", "1"),
        ]);
        assert_eq!(parse_swaps(&tx)[0].pair_address, "terra1authenticated");
    }

    #[test]
    fn issue1395_flattened_amounts_never_inherit_from_previous_actions() {
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1previous"),
            ("action", "provide_liquidity"),
            ("sender", "terra1previoussender"),
            ("assets", "uluna 1, uusd 1"),
            ("share", "1"),
            ("_contract_address", "terra1final"),
            ("action", "provide_liquidity"),
            ("sender", "terra1finalsender"),
        ]);
        let liquidity = parse_liquidity_events(&tx);
        assert_eq!(liquidity.len(), 1);
        assert_eq!(liquidity[0].pair_address, "terra1previous");
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1previous"),
            ("action", "after_swap_burn"),
            ("burn_amount", "bad"),
            ("burn_token", "terra1previoustoken"),
            ("_contract_address", "terra1final"),
            ("action", "after_swap_tax"),
            ("tax_amount", "2"),
            ("tax_token", "terra1finaltoken"),
        ]);
        let hooks = parse_hook_events(&tx);
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].hook_address, "terra1final");
        assert_eq!(hooks[0].amount, Some(BigDecimal::from(2)));
        assert_eq!(hooks[0].token.as_deref(), Some("terra1finaltoken"));
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1previous"),
            ("action", "swap"),
            ("sender", "terra1previoussender"),
            ("offer_amount", "1"),
            ("return_amount", "1"),
            ("_contract_address", "terra1final"),
            ("action", "swap"),
            ("offer_amount", "2"),
            ("return_amount", "2"),
        ]);
        let swaps = parse_swaps(&tx);
        assert_eq!(
            swaps.len(),
            1,
            "missing sender rejects only the final action"
        );
        assert_eq!(swaps[0].pair_address, "terra1previous");
    }

    #[test]
    fn issue1395_columnar_batch_ends_at_action_and_authenticated_emitter() {
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1pairA"),
            ("action", "limit_order_fill"),
            ("action", "limit_order_fill"),
            ("order_id", "1"),
            ("order_id", "2"),
            ("side", "bid"),
            ("maker", "terra1maker"),
            ("price", "1"),
            ("price", "1"),
            ("token0_amount", "10"),
            ("token0_amount", "20"),
            ("token1_amount", "11"),
            ("token1_amount", "22"),
            ("commission_amount", "1"),
            ("commission_amount", "2"),
            ("action", "swap"),
            ("sender", "terra1synthetic"),
            ("offer_amount", "30"),
            ("return_amount", "33"),
            ("commission_amount", "3"),
            ("_contract_address", "terra1pairB"),
            ("action", "limit_order_fill"),
            ("order_id", "3"),
            ("side", "bid"),
            ("maker", "terra1maker"),
            ("price", "1"),
            ("token0_amount", "30"),
            ("token1_amount", "33"),
            ("commission_amount", "4"),
        ]);
        let fills = parse_limit_order_fills(&tx);
        assert_eq!(fills.len(), 3);
        assert_eq!(fills[0].pair_address, "terra1pairA");
        assert_eq!(fills[1].pair_address, "terra1pairA");
        assert_eq!(fills[2].pair_address, "terra1pairB");
        assert_eq!(fills[1].commission_amount, BigDecimal::from(2));
        assert_eq!(fills[2].commission_amount, BigDecimal::from(4));

        let tx = wasm_tx(vec![
            ("_contract_address", "terra1pairA"),
            ("action", "place_limit_order"),
            ("action", "place_limit_order"),
            ("order_id", "1"),
            ("order_id", "2"),
            ("price", "1.1"),
            ("price", "2.2"),
            ("maker_fee_amount", "1"),
            ("maker_fee_amount", "2"),
            ("action", "swap"),
            ("sender", "terra1synthetic"),
            ("offer_amount", "1"),
            ("return_amount", "1"),
            ("price", "bad"),
            ("_contract_address", "terra1pairB"),
            ("action", "place_limit_order"),
            ("order_id", "3"),
            ("price", "3.3"),
            ("maker_fee_amount", "3"),
        ]);
        let placements = parse_limit_order_placements(&tx);
        assert_eq!(placements.len(), 3);
        assert_eq!(placements[0].pair_address, "terra1pairA");
        assert_eq!(placements[2].pair_address, "terra1pairB");
        assert_eq!(placements[2].maker_fee_amount, Some(BigDecimal::from(3)));
    }

    #[test]
    fn issue1395_same_family_valid_siblings_survive_malformed_later_actions() {
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1syntheticpair"),
            ("action", "swap"),
            ("sender", "terra1synthetic"),
            ("offer_amount", "10"),
            ("return_amount", "9"),
            ("action", "swap"),
            ("sender", "terra1synthetic"),
            ("offer_amount", "bad"),
            ("return_amount", "9"),
            ("action", "swap"),
            ("sender", "terra1synthetic"),
            ("offer_amount", "30"),
            ("return_amount", "27"),
        ]);
        let swaps = parse_swaps(&tx);
        assert_eq!(swaps.len(), 2);
        assert_eq!((swaps[0].swap_index, swaps[1].swap_index), (0, 2));
        assert_eq!(swaps[0].offer_amount, BigDecimal::from(10));
        assert_eq!(swaps[1].offer_amount, BigDecimal::from(30));

        let tx = wasm_tx(vec![
            ("_contract_address", "terra1syntheticpair"),
            ("action", "provide_liquidity"),
            ("sender", "terra1synthetic"),
            ("assets", "uluna 1, uusd 2"),
            ("share", "3"),
            ("action", "provide_liquidity"),
            ("sender", "terra1synthetic"),
            ("assets", "uluna bad, uusd 2"),
            ("share", "4"),
        ]);
        let liquidity = parse_liquidity_events(&tx);
        assert_eq!(liquidity.len(), 1);
        assert_eq!(liquidity[0].lp_amount, BigDecimal::from(3));

        let tx = wasm_tx(vec![
            ("_contract_address", "terra1synthetichook"),
            ("action", "after_swap_burn"),
            ("burn_amount", "5"),
            ("action", "after_swap_burn"),
            ("burn_amount", "bad"),
        ]);
        let hooks = parse_hook_events(&tx);
        assert_eq!(hooks.len(), 1);
        assert_eq!(hooks[0].amount, Some(BigDecimal::from(5)));
    }

    #[test]
    fn issue1395_flattened_swaps_and_fills_share_emission_ordinals() {
        let mut attrs = vec![("_contract_address", "terra1syntheticpair")];
        for (id, raw_offer) in [("1", "10"), ("2", "bad"), ("3", "30")] {
            attrs.extend([
                ("action", "limit_order_fill"),
                ("order_id", id),
                ("side", "bid"),
                ("maker", "terra1syntheticmaker"),
                ("price", "1"),
                ("token0_amount", "1"),
                ("token1_amount", "1"),
                ("commission_amount", "0"),
                ("action", "swap"),
                ("sender", "terra1synthetictrader"),
                ("offer_amount", raw_offer),
                ("return_amount", "9"),
            ]);
        }
        let tx = wasm_tx(attrs);
        for _ in 0..3 {
            let swaps = parse_swaps(&tx);
            assert_eq!(
                swaps.iter().map(|s| s.swap_index).collect::<Vec<_>>(),
                vec![0, 2]
            );
            let fills = parse_limit_order_fills(&tx);
            assert_eq!(
                fills
                    .iter()
                    .map(|f| (f.order_id, f.swap_index))
                    .collect::<Vec<_>>(),
                vec![(1, 0), (2, 1), (3, 2)]
            );
        }
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1syntheticpair"),
            ("action", "swap"),
            ("sender", "terra1synthetic"),
            ("offer_amount", "1"),
            ("return_amount", "1"),
            ("_contract_address", "terra1differentpair"),
            ("action", "swap"),
            ("sender", "terra1synthetic"),
            ("offer_amount", "1"),
            ("return_amount", "1"),
            ("_contract_address", "terra1syntheticpair"),
            ("action", "limit_order_fill"),
            ("order_id", "7"),
            ("side", "bid"),
            ("maker", "terra1maker"),
            ("price", "1"),
            ("token0_amount", "1"),
            ("token1_amount", "1"),
            ("commission_amount", "0"),
            ("swap_index", "9"),
        ]);
        assert_eq!(
            parse_limit_order_fills(&tx)[0].swap_index,
            9,
            "explicit chain index retains priority"
        );
    }

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
    fn parse_swaps_reads_optional_reserve_attrs_from_pair_contract() {
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1pair"),
            ("action", "swap"),
            ("sender", "terra1user"),
            ("offer_amount", "100"),
            ("return_amount", "95"),
            ("reserve_0", "1000"),
            ("reserve_1", "2000"),
        ]);
        let swaps = parse_swaps(&tx);
        assert_eq!(swaps[0].reserve_0.as_ref().unwrap().to_string(), "1000");
        assert_eq!(swaps[0].reserve_1.as_ref().unwrap().to_string(), "2000");
    }

    #[test]
    fn parse_swaps_ignores_forged_reserve_attrs_on_foreign_contract() {
        // GitLab #285 / #684 A2: only the pair's runtime-reserved `_contract_address`
        // scopes attrs. A second wasm event from another contract must not attach.
        let tx = wasm_tx_multi(vec![
            vec![
                ("_contract_address", "terra1pair"),
                ("action", "swap"),
                ("sender", "terra1user"),
                ("offer_amount", "100"),
                ("return_amount", "95"),
            ],
            vec![
                ("_contract_address", "terra1attacker"),
                ("action", "swap"),
                ("sender", "terra1user"),
                ("offer_amount", "1"),
                ("return_amount", "1"),
                ("reserve_0", "999999"),
                ("reserve_1", "999999"),
            ],
        ]);
        let swaps = parse_swaps(&tx);
        assert_eq!(swaps.len(), 2);
        assert!(swaps[0].reserve_0.is_none());
        assert!(swaps[0].reserve_1.is_none());
        assert_eq!(swaps[1].pair_address, "terra1attacker");
        assert_eq!(swaps[1].reserve_0.as_ref().unwrap().to_string(), "999999");
    }

    #[test]
    fn parse_swaps_ignores_non_reserved_contract_address_key() {
        let tx = wasm_tx(vec![
            ("contract_address", "terra1forged"),
            ("_contract_address", "terra1pair"),
            ("action", "swap"),
            ("sender", "terra1user"),
            ("offer_amount", "100"),
            ("return_amount", "95"),
            ("reserve_0", "5"),
            ("reserve_1", "6"),
        ]);
        let swaps = parse_swaps(&tx);
        assert_eq!(swaps[0].pair_address, "terra1pair");
        assert_eq!(swaps[0].reserve_0.as_ref().unwrap().to_string(), "5");
    }

    #[test]
    fn parse_swaps_assigns_per_pair_swap_index() {
        // GitLab #287: two swaps on the same pair in one tx must get distinct ordinals (0, 1)
        // so they no longer collapse on (tx_hash, pair_id); a swap on a different pair restarts
        // at 0 (the ordinal is per-pair, not per-tx).
        // GitLab #1269: protocol_fee_events must still persist hop B at swap_index 0 via
        // (tx_hash, source, pair_id, ordinal) — do not reuse this per-pair index as a
        // per-tx fee ordinal.
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
        assert_eq!(
            (swaps[0].pair_address.as_str(), swaps[0].swap_index),
            ("terra1pairA", 0)
        );
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
    fn parse_swaps_pool_commission_ignores_book_commission_amount_l7() {
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1pair"),
            ("action", "swap"),
            ("sender", "terra1user"),
            ("offer_amount", "100"),
            ("return_amount", "95"),
            ("offer_asset", "uluna"),
            ("ask_asset", "uusd"),
            ("commission_amount", "3"),
            ("book_commission_amount", "7"),
        ]);
        let swaps = parse_swaps(&tx);
        assert_eq!(swaps.len(), 1);
        assert_eq!(
            swaps[0].commission_amount.as_ref().unwrap().to_string(),
            "3",
            "L7 / PFee-5: swap_amm uses pool commission_amount only; never book_commission_amount"
        );
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
            ("commission_amount", "1"),
            ("action", "limit_order_fill"),
            ("order_id", "102"),
            ("side", "ask"),
            ("maker", "terra1mk2"),
            ("price", "1.1"),
            ("token0_amount", "20"),
            ("token1_amount", "22"),
            ("commission_amount", "2"),
            ("action", "limit_order_fill"),
            ("order_id", "103"),
            ("side", "bid"),
            ("maker", "terra1mk3"),
            ("price", "0.9"),
            ("token0_amount", "5"),
            ("token1_amount", "4"),
            ("commission_amount", "5"),
            ("action", "limit_order_fill"),
            ("order_id", "104"),
            ("side", "ask"),
            ("maker", "terra1mk4"),
            ("price", "1.2"),
            ("token0_amount", "8"),
            ("token1_amount", "9"),
            ("commission_amount", "8"),
            ("action", "limit_order_fill"),
            ("order_id", "105"),
            ("side", "bid"),
            ("maker", "terra1mk5"),
            ("price", "1.05"),
            ("token0_amount", "15"),
            ("token1_amount", "15"),
            ("commission_amount", "15"),
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
        assert_eq!(fills[4].commission_amount.to_string(), "15");
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
                ("commission_amount", "1"),
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
                ("commission_amount", "5"),
                ("_contract_address", "terra1pair"),
                ("action", "limit_order_fill"),
                ("order_id", "22"),
                ("side", "ask"),
                ("maker", "terra1mkC"),
                ("price", "1.2"),
                ("token0_amount", "7"),
                ("token1_amount", "8"),
                ("commission_amount", "7"),
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
    fn parse_limit_order_fills_prefers_on_chain_swap_index_attr() {
        // GitLab #331: post-upgrade fills carry `swap_index` on wasm attrs; parser uses the emitted
        // value instead of walk-order inference (legacy txs without the attr still infer).
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1pair"),
            ("action", "limit_order_fill"),
            ("order_id", "42"),
            ("side", "bid"),
            ("maker", "terra1mk"),
            ("price", "1.0"),
            ("token0_amount", "10"),
            ("token1_amount", "10"),
            ("commission_amount", "1"),
            ("swap_index", "1"),
            ("action", "swap"),
            ("sender", "terra1taker"),
            ("offer_amount", "100"),
            ("return_amount", "95"),
        ]);
        let fills = parse_limit_order_fills(&tx);
        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].swap_index, 1);
        assert_eq!(fills[0].swap_index_from_chain, Some(1));
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
                    value: "1".into(),
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
                ("maker_fee_amount", "2500"),
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
            p[0].maker_fee_amount
                .as_ref()
                .map(|x| x.to_string())
                .as_deref(),
            Some("2500")
        );
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
        let (a0, a1) = parse_asset_amounts(Some("uluna 1000, uusd 950"), "assets").unwrap();
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
    fn wasm_action_map_finds_last_duplicate() {
        let attrs = vec![
            Attribute {
                key: "action".into(),
                value: "synthetic".into(),
            },
            Attribute {
                key: "k".into(),
                value: "first".into(),
            },
            Attribute {
                key: "k".into(),
                value: "second".into(),
            },
        ];
        assert_eq!(
            wasm_kv_map_after_action(&attrs, 0).get("k").copied(),
            Some("second")
        );
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
            let _ = parse_asset_amounts(Some(&s), "assets");
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
