//! GeckoTerminal Integration API (`/gt/*`) — GitLab #646 / #684.
//!
//! Spec: [Integration API Standards](https://docs.google.com/document/d/1ufjAJUa6rGO9PBGJGwfBMn-XMk9NE0ow3_iMYrS3drk)
//! (`/latest-block`, `/asset`, `/pair`, `/events`). Base URL submitted as
//! `https://indexer.dex.cl8y.com/gt`. `dexKey` is `cl8y`.
//!
//! Not Uniswap-V2 auto-detect and not `/cg/*`. Gem / ALPHA / USTRIX / SpaceUSD
//! pairs are omitted from `/pair` and `/events` (**L639-2**). Event `reserves`
//! are the persisted post-event AMM `RESERVES` (`swap_events.reserve_*` /
//! `liquidity_events.reserve_*`, GitLab #684). Missing columns emit `"0"` —
//! never the live `pair_reserves` snapshot.
//!
//! Per-request cost (#694 / RE-01): inclusive block span is still
//! [`MAX_EVENT_BLOCK_SPAN`]; combined swap+liquidity rows are capped at
//! [`MAX_GT_EVENT_ROWS`]. Over-cap is **400** (no truncation, no live
//! `pair_reserves` scan).

use std::collections::HashMap;

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use bigdecimal::{BigDecimal, Zero};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use utoipa::{IntoParams, ToSchema};

use super::{internal_err, AppState};
use crate::db::queries::{assets, pairs as db_pairs, state};
pub use crate::indexer::listing_exclude::is_excluded_cw20;
use crate::indexer::listing_exclude::listing_excluded_cw20_binds;
use crate::indexer::defillama::COLUMBUS5_GEM_ADDRESSES;

pub const DEX_KEY: &str = "cl8y";
pub const MAX_EVENT_BLOCK_SPAN: i64 = 2_000;
/// Combined swap + join/exit rows per `/gt/events` window (GitLab #694 / RE-01).
pub const MAX_GT_EVENT_ROWS: i64 = 5_000;
/// Stable 400 body when the window's raw event count exceeds [`MAX_GT_EVENT_ROWS`].
pub const GT_EVENT_ROW_CAP_MSG: &str = "event count exceeds 5000";

const CL8Y_CW20: &str = "terra16wtml2q66g82fdkx66tap0qjkahqwp4lwq3ngtygacg5q0kzycgqvhpax3";
const CL8Y_COINGECKO_ID: &str = "ceramicliberty-com";

#[derive(Serialize, ToSchema)]
pub struct GtBlock {
    #[serde(rename = "blockNumber")]
    pub block_number: i64,
    #[serde(rename = "blockTimestamp")]
    pub block_timestamp: i64,
}

#[derive(Serialize, ToSchema)]
pub struct GtLatestBlockResponse {
    pub block: GtBlock,
}

#[derive(Serialize, ToSchema)]
pub struct GtAsset {
    pub id: String,
    pub name: String,
    pub symbol: String,
    pub decimals: i16,
    #[serde(rename = "coinGeckoId", skip_serializing_if = "Option::is_none")]
    pub coin_gecko_id: Option<String>,
}

#[derive(Serialize, ToSchema)]
pub struct GtAssetResponse {
    pub asset: GtAsset,
}

#[derive(Serialize, ToSchema)]
pub struct GtPair {
    pub id: String,
    #[serde(rename = "dexKey")]
    pub dex_key: String,
    #[serde(rename = "asset0Id")]
    pub asset0_id: String,
    #[serde(rename = "asset1Id")]
    pub asset1_id: String,
    #[serde(
        rename = "createdAtBlockNumber",
        skip_serializing_if = "Option::is_none"
    )]
    pub created_at_block_number: Option<i64>,
    #[serde(
        rename = "createdAtBlockTimestamp",
        skip_serializing_if = "Option::is_none"
    )]
    pub created_at_block_timestamp: Option<i64>,
    #[serde(rename = "feeBps", skip_serializing_if = "Option::is_none")]
    pub fee_bps: Option<i16>,
}

#[derive(Serialize, ToSchema)]
pub struct GtPairResponse {
    pub pair: GtPair,
}

#[derive(Serialize, ToSchema)]
pub struct GtReserves {
    pub asset0: String,
    pub asset1: String,
}

#[derive(Serialize, ToSchema)]
#[serde(tag = "eventType")]
pub enum GtEventBody {
    #[serde(rename = "swap")]
    Swap {
        #[serde(rename = "txnId")]
        txn_id: String,
        #[serde(rename = "txnIndex")]
        txn_index: i64,
        #[serde(rename = "eventIndex")]
        event_index: i64,
        maker: String,
        #[serde(rename = "pairId")]
        pair_id: String,
        #[serde(rename = "asset0In", skip_serializing_if = "Option::is_none")]
        asset0_in: Option<String>,
        #[serde(rename = "asset1In", skip_serializing_if = "Option::is_none")]
        asset1_in: Option<String>,
        #[serde(rename = "asset0Out", skip_serializing_if = "Option::is_none")]
        asset0_out: Option<String>,
        #[serde(rename = "asset1Out", skip_serializing_if = "Option::is_none")]
        asset1_out: Option<String>,
        #[serde(rename = "priceNative")]
        price_native: String,
        reserves: GtReserves,
    },
    #[serde(rename = "join")]
    Join {
        #[serde(rename = "txnId")]
        txn_id: String,
        #[serde(rename = "txnIndex")]
        txn_index: i64,
        #[serde(rename = "eventIndex")]
        event_index: i64,
        maker: String,
        #[serde(rename = "pairId")]
        pair_id: String,
        amount0: String,
        amount1: String,
        reserves: GtReserves,
    },
    #[serde(rename = "exit")]
    Exit {
        #[serde(rename = "txnId")]
        txn_id: String,
        #[serde(rename = "txnIndex")]
        txn_index: i64,
        #[serde(rename = "eventIndex")]
        event_index: i64,
        maker: String,
        #[serde(rename = "pairId")]
        pair_id: String,
        amount0: String,
        amount1: String,
        reserves: GtReserves,
    },
}

#[derive(Serialize, ToSchema)]
pub struct GtEvent {
    pub block: GtBlock,
    #[serde(flatten)]
    pub body: GtEventBody,
}

#[derive(Serialize, ToSchema)]
pub struct GtEventsResponse {
    pub events: Vec<GtEvent>,
}

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct GtAssetQuery {
    pub id: String,
}

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct GtPairQuery {
    pub id: String,
}

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct GtEventsQuery {
    #[serde(rename = "fromBlock")]
    pub from_block: Option<i64>,
    #[serde(rename = "toBlock")]
    pub to_block: Option<i64>,
}

#[derive(Clone, FromRow)]
#[allow(dead_code)]
struct EventSwapRow {
    id: i64,
    pair_contract: String,
    pair_id: i32,
    asset_0_id: i32,
    asset_1_id: i32,
    block_height: i64,
    block_timestamp: DateTime<Utc>,
    tx_hash: String,
    sender: String,
    offer_asset_id: i32,
    offer_amount: BigDecimal,
    return_amount: BigDecimal,
    price: BigDecimal,
    reserve_0: Option<BigDecimal>,
    reserve_1: Option<BigDecimal>,
}

#[derive(Clone, FromRow)]
#[allow(dead_code)]
struct EventLiqRow {
    id: i64,
    pair_contract: String,
    pair_id: i32,
    asset_0_id: i32,
    asset_1_id: i32,
    block_height: i64,
    block_timestamp: DateTime<Utc>,
    tx_hash: String,
    provider: String,
    event_type: String,
    asset_0_amount: BigDecimal,
    asset_1_amount: BigDecimal,
    reserve_0: Option<BigDecimal>,
    reserve_1: Option<BigDecimal>,
}

fn event_reserves(
    r0: Option<&BigDecimal>,
    r1: Option<&BigDecimal>,
    dec0: i16,
    dec1: i16,
) -> GtReserves {
    match (r0, r1) {
        (Some(a), Some(b)) => GtReserves {
            asset0: format_dec(&decimalize(a, dec0)),
            asset1: format_dec(&decimalize(b, dec1)),
        },
        _ => GtReserves {
            asset0: "0".into(),
            asset1: "0".into(),
        },
    }
}

pub fn gt_asset_id(asset: &assets::AssetRow) -> Option<String> {
    if asset.is_cw20 {
        asset.contract_address.clone()
    } else {
        asset.denom.clone()
    }
}

fn pair_is_excluded(a0: &assets::AssetRow, a1: &assets::AssetRow) -> bool {
    [a0, a1]
        .iter()
        .any(|a| a.contract_address.as_deref().is_some_and(is_excluded_cw20))
}

fn ten_pow(decimals: i16) -> BigDecimal {
    BigDecimal::from(10u64.pow(decimals.clamp(0, 18) as u32))
}

pub fn decimalize(raw: &BigDecimal, decimals: i16) -> BigDecimal {
    if decimals <= 0 {
        return raw.clone();
    }
    raw / ten_pow(decimals)
}

pub fn format_dec(v: &BigDecimal) -> String {
    let s = v.normalized().to_string();
    if s.contains('e') || s.contains('E') {
        let scaled = v.with_scale(50);
        let plain = scaled.normalized().to_string();
        if plain.contains('e') || plain.contains('E') {
            return v.to_string();
        }
        return plain;
    }
    s
}

fn coin_gecko_id_for(asset: &assets::AssetRow) -> Option<String> {
    let addr = asset.contract_address.as_deref()?.to_ascii_lowercase();
    if addr == CL8Y_CW20 {
        return Some(CL8Y_COINGECKO_ID.to_string());
    }
    asset.coingecko_id.clone().filter(|id| !id.is_empty())
}

#[utoipa::path(
    get,
    path = "/gt/latest-block",
    responses((status = 200, description = "Latest indexed block", body = GtLatestBlockResponse)),
    tag = "GeckoTerminal"
)]
pub async fn gt_latest_block(
    State(state): State<AppState>,
) -> Result<Json<GtLatestBlockResponse>, (StatusCode, String)> {
    let height = state::get_last_indexed_height(&state.pool)
        .await
        .map_err(internal_err)?;
    let ts: i64 = sqlx::query_scalar(
        r#"
        SELECT COALESCE(
            (SELECT EXTRACT(EPOCH FROM MAX(block_timestamp))::bigint
             FROM swap_events WHERE block_height <= $1),
            (SELECT EXTRACT(EPOCH FROM MAX(block_timestamp))::bigint
             FROM liquidity_events WHERE block_height <= $1),
            (SELECT EXTRACT(EPOCH FROM updated_at)::bigint
             FROM indexer_state WHERE key = 'last_indexed_height'),
            0
        )
        "#,
    )
    .bind(height)
    .fetch_one(&state.pool)
    .await
    .map_err(internal_err)?;

    Ok(Json(GtLatestBlockResponse {
        block: GtBlock {
            block_number: height,
            block_timestamp: ts,
        },
    }))
}

#[utoipa::path(
    get,
    path = "/gt/asset",
    params(GtAssetQuery),
    responses(
        (status = 200, description = "Asset metadata", body = GtAssetResponse),
        (status = 400, description = "Missing id"),
        (status = 404, description = "Unknown asset"),
    ),
    tag = "GeckoTerminal"
)]
pub async fn gt_asset(
    State(state): State<AppState>,
    Query(q): Query<GtAssetQuery>,
) -> Result<Json<GtAssetResponse>, (StatusCode, String)> {
    let id = q.id.trim();
    if id.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "id is required".into()));
    }
    let row = if id.starts_with("terra") {
        assets::get_asset_by_contract(&state.pool, id)
            .await
            .map_err(internal_err)?
    } else {
        assets::get_asset_by_denom(&state.pool, id)
            .await
            .map_err(internal_err)?
    }
    .ok_or_else(|| (StatusCode::NOT_FOUND, format!("asset not found: {id}")))?;

    let public_id = gt_asset_id(&row).unwrap_or_else(|| id.to_string());
    Ok(Json(GtAssetResponse {
        asset: GtAsset {
            id: public_id,
            name: row.name.clone(),
            symbol: row.symbol.clone(),
            decimals: row.decimals,
            coin_gecko_id: coin_gecko_id_for(&row),
        },
    }))
}

#[utoipa::path(
    get,
    path = "/gt/pair",
    params(GtPairQuery),
    responses(
        (status = 200, description = "Pair metadata", body = GtPairResponse),
        (status = 400, description = "Missing id"),
        (status = 404, description = "Unknown or excluded pair"),
    ),
    tag = "GeckoTerminal"
)]
pub async fn gt_pair(
    State(state): State<AppState>,
    Query(q): Query<GtPairQuery>,
) -> Result<Json<GtPairResponse>, (StatusCode, String)> {
    let id = q.id.trim();
    if id.is_empty() {
        return Err((StatusCode::BAD_REQUEST, "id is required".into()));
    }
    let pair = db_pairs::get_pair_by_address(&state.pool, id)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("pair not found: {id}")))?;
    let a0 = assets::get_asset_by_id(&state.pool, pair.asset_0_id)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "pair asset0 missing".into()))?;
    let a1 = assets::get_asset_by_id(&state.pool, pair.asset_1_id)
        .await
        .map_err(internal_err)?
        .ok_or_else(|| (StatusCode::NOT_FOUND, "pair asset1 missing".into()))?;
    if pair_is_excluded(&a0, &a1) {
        return Err((StatusCode::NOT_FOUND, format!("pair not found: {id}")));
    }
    let asset0_id = gt_asset_id(&a0).ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "asset0 has no public id".into(),
        )
    })?;
    let asset1_id = gt_asset_id(&a1).ok_or_else(|| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            "asset1 has no public id".into(),
        )
    })?;

    Ok(Json(GtPairResponse {
        pair: GtPair {
            id: pair.contract_address,
            dex_key: DEX_KEY.to_string(),
            asset0_id,
            asset1_id,
            created_at_block_number: pair.created_at_block,
            created_at_block_timestamp: Some(pair.created_at.timestamp()),
            fee_bps: pair.fee_bps,
        },
    }))
}

#[utoipa::path(
    get,
    path = "/gt/events",
    params(GtEventsQuery),
    responses(
        (status = 200, description = "Swap and join/exit events in the block range", body = GtEventsResponse),
        (status = 400, description = "Invalid block range or event count exceeds cap"),
    ),
    tag = "GeckoTerminal"
)]
pub async fn gt_events(
    State(state): State<AppState>,
    Query(q): Query<GtEventsQuery>,
) -> Result<Json<GtEventsResponse>, (StatusCode, String)> {
    let from = q
        .from_block
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "fromBlock is required".into()))?;
    let to = q
        .to_block
        .ok_or_else(|| (StatusCode::BAD_REQUEST, "toBlock is required".into()))?;
    if from < 0 || to < 0 {
        return Err((StatusCode::BAD_REQUEST, "block numbers must be >= 0".into()));
    }
    if to < from {
        return Err((
            StatusCode::BAD_REQUEST,
            "toBlock must be >= fromBlock".into(),
        ));
    }
    if to - from + 1 > MAX_EVENT_BLOCK_SPAN {
        return Err((
            StatusCode::BAD_REQUEST,
            format!("block range exceeds {MAX_EVENT_BLOCK_SPAN}"),
        ));
    }

    let latest = state::get_last_indexed_height(&state.pool)
        .await
        .map_err(internal_err)?;
    if from > latest {
        return Ok(Json(GtEventsResponse { events: Vec::new() }));
    }
    let to = to.min(latest);

    let excluded = listing_excluded_cw20_binds();

    // RE-01: count before materializing rows so a busy 2k-block window cannot
    // return unbounded JSON. Do not SELECT pair_reserves here (#684 / #694).
    let event_count: i64 = sqlx::query_scalar(
        r#"
        SELECT
          (SELECT COUNT(*) FROM swap_events se
           JOIN pairs p ON p.id = se.pair_id
           JOIN assets a0 ON a0.id = p.asset_0_id
           JOIN assets a1 ON a1.id = p.asset_1_id
           WHERE se.block_height >= $1 AND se.block_height <= $2
             AND LOWER(COALESCE(a0.contract_address, '')) <> ALL($3)
             AND LOWER(COALESCE(a1.contract_address, '')) <> ALL($3)
             AND se.price > 0
             AND se.offer_amount > 0
             AND se.return_amount > 0)
        + (SELECT COUNT(*) FROM liquidity_events le
           JOIN pairs p ON p.id = le.pair_id
           JOIN assets a0 ON a0.id = p.asset_0_id
           JOIN assets a1 ON a1.id = p.asset_1_id
           WHERE le.block_height >= $1 AND le.block_height <= $2
             AND LOWER(COALESCE(a0.contract_address, '')) <> ALL($3)
             AND LOWER(COALESCE(a1.contract_address, '')) <> ALL($3)
             AND le.event_type IN ('add', 'remove'))
        "#,
    )
    .bind(from)
    .bind(to)
    .bind(&excluded)
    .fetch_one(&state.pool)
    .await
    .map_err(internal_err)?;
    if event_count > MAX_GT_EVENT_ROWS {
        return Err((StatusCode::BAD_REQUEST, GT_EVENT_ROW_CAP_MSG.to_string()));
    }

    let swaps: Vec<EventSwapRow> = sqlx::query_as(
        r#"
        SELECT se.id, p.contract_address AS pair_contract, se.pair_id, p.asset_0_id, p.asset_1_id,
               se.block_height, se.block_timestamp, se.tx_hash, se.sender,
               se.offer_asset_id, se.offer_amount, se.return_amount, se.price,
               se.reserve_0, se.reserve_1
        FROM swap_events se
        JOIN pairs p ON p.id = se.pair_id
        JOIN assets a0 ON a0.id = p.asset_0_id
        JOIN assets a1 ON a1.id = p.asset_1_id
        WHERE se.block_height >= $1 AND se.block_height <= $2
          AND LOWER(COALESCE(a0.contract_address, '')) <> ALL($3)
          AND LOWER(COALESCE(a1.contract_address, '')) <> ALL($3)
          AND se.price > 0
          AND se.offer_amount > 0
          AND se.return_amount > 0
        ORDER BY se.block_height, se.tx_hash, se.id
        "#,
    )
    .bind(from)
    .bind(to)
    .bind(&excluded)
    .fetch_all(&state.pool)
    .await
    .map_err(internal_err)?;

    let liqs: Vec<EventLiqRow> = sqlx::query_as(
        r#"
        SELECT le.id, p.contract_address AS pair_contract, le.pair_id, p.asset_0_id, p.asset_1_id,
               le.block_height, le.block_timestamp, le.tx_hash, le.provider,
               le.event_type,
               COALESCE(le.asset_0_amount, 0) AS asset_0_amount,
               COALESCE(le.asset_1_amount, 0) AS asset_1_amount,
               le.reserve_0, le.reserve_1
        FROM liquidity_events le
        JOIN pairs p ON p.id = le.pair_id
        JOIN assets a0 ON a0.id = p.asset_0_id
        JOIN assets a1 ON a1.id = p.asset_1_id
        WHERE le.block_height >= $1 AND le.block_height <= $2
          AND LOWER(COALESCE(a0.contract_address, '')) <> ALL($3)
          AND LOWER(COALESCE(a1.contract_address, '')) <> ALL($3)
          AND le.event_type IN ('add', 'remove')
        ORDER BY le.block_height, le.tx_hash, le.id
        "#,
    )
    .bind(from)
    .bind(to)
    .bind(&excluded)
    .fetch_all(&state.pool)
    .await
    .map_err(internal_err)?;

    let asset_map = super::build_asset_map(&state.pool)
        .await
        .map_err(internal_err)?;

    #[derive(Clone)]
    enum RawEvent {
        Swap(EventSwapRow),
        Liq(EventLiqRow),
    }

    let mut raw: Vec<RawEvent> = Vec::with_capacity(swaps.len() + liqs.len());
    raw.extend(swaps.into_iter().map(RawEvent::Swap));
    raw.extend(liqs.into_iter().map(RawEvent::Liq));
    raw.sort_by(|a, b| {
        let (ha, txa, ida) = match a {
            RawEvent::Swap(s) => (s.block_height, s.tx_hash.as_str(), s.id),
            RawEvent::Liq(l) => (l.block_height, l.tx_hash.as_str(), l.id),
        };
        let (hb, txb, idb) = match b {
            RawEvent::Swap(s) => (s.block_height, s.tx_hash.as_str(), s.id),
            RawEvent::Liq(l) => (l.block_height, l.tx_hash.as_str(), l.id),
        };
        ha.cmp(&hb).then(txa.cmp(txb)).then(ida.cmp(&idb))
    });

    let mut events = Vec::with_capacity(raw.len());
    let mut txn_index_by_block: HashMap<(i64, String), i64> = HashMap::new();
    let mut event_index_by_tx: HashMap<(i64, String), i64> = HashMap::new();
    let mut next_txn_in_block: HashMap<i64, i64> = HashMap::new();

    for ev in raw {
        let (height, ts, tx_hash) = match &ev {
            RawEvent::Swap(s) => (s.block_height, s.block_timestamp, s.tx_hash.clone()),
            RawEvent::Liq(l) => (l.block_height, l.block_timestamp, l.tx_hash.clone()),
        };
        let txn_key = (height, tx_hash.clone());
        let txn_index = *txn_index_by_block
            .entry(txn_key.clone())
            .or_insert_with(|| {
                let n = next_txn_in_block.entry(height).or_insert(0);
                let assigned = *n;
                *n += 1;
                assigned
            });
        let event_index = {
            let slot = event_index_by_tx.entry(txn_key).or_insert(0);
            let assigned = *slot;
            *slot += 1;
            assigned
        };

        match ev {
            RawEvent::Swap(s) => {
                let Some(a0) = asset_map.get(&s.asset_0_id) else {
                    continue;
                };
                let Some(a1) = asset_map.get(&s.asset_1_id) else {
                    continue;
                };
                let offer_is_base = s.offer_asset_id == s.asset_0_id;
                let offer_dec = decimalize(
                    &s.offer_amount,
                    if offer_is_base {
                        a0.decimals
                    } else {
                        a1.decimals
                    },
                );
                let return_dec = decimalize(
                    &s.return_amount,
                    if offer_is_base {
                        a1.decimals
                    } else {
                        a0.decimals
                    },
                );
                if offer_dec.is_zero() || return_dec.is_zero() || s.price.is_zero() {
                    continue;
                }
                let (asset0_in, asset1_in, asset0_out, asset1_out) = if offer_is_base {
                    (
                        Some(format_dec(&offer_dec)),
                        None,
                        None,
                        Some(format_dec(&return_dec)),
                    )
                } else {
                    (
                        None,
                        Some(format_dec(&offer_dec)),
                        Some(format_dec(&return_dec)),
                        None,
                    )
                };
                events.push(GtEvent {
                    block: GtBlock {
                        block_number: height,
                        block_timestamp: ts.timestamp(),
                    },
                    body: GtEventBody::Swap {
                        txn_id: s.tx_hash,
                        txn_index,
                        event_index,
                        maker: s.sender,
                        pair_id: s.pair_contract,
                        asset0_in,
                        asset1_in,
                        asset0_out,
                        asset1_out,
                        price_native: format_dec(&s.price),
                        reserves: event_reserves(
                            s.reserve_0.as_ref(),
                            s.reserve_1.as_ref(),
                            a0.decimals,
                            a1.decimals,
                        ),
                    },
                });
            }
            RawEvent::Liq(l) => {
                let Some(a0) = asset_map.get(&l.asset_0_id) else {
                    continue;
                };
                let Some(a1) = asset_map.get(&l.asset_1_id) else {
                    continue;
                };
                let amount0 = format_dec(&decimalize(&l.asset_0_amount, a0.decimals));
                let amount1 = format_dec(&decimalize(&l.asset_1_amount, a1.decimals));
                let reserves = event_reserves(
                    l.reserve_0.as_ref(),
                    l.reserve_1.as_ref(),
                    a0.decimals,
                    a1.decimals,
                );
                let body = if l.event_type == "add" {
                    GtEventBody::Join {
                        txn_id: l.tx_hash,
                        txn_index,
                        event_index,
                        maker: l.provider,
                        pair_id: l.pair_contract,
                        amount0,
                        amount1,
                        reserves,
                    }
                } else {
                    GtEventBody::Exit {
                        txn_id: l.tx_hash,
                        txn_index,
                        event_index,
                        maker: l.provider,
                        pair_id: l.pair_contract,
                        amount0,
                        amount1,
                        reserves,
                    }
                };
                events.push(GtEvent {
                    block: GtBlock {
                        block_number: height,
                        block_timestamp: ts.timestamp(),
                    },
                    body,
                });
            }
        }
    }

    Ok(Json(GtEventsResponse { events }))
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use std::str::FromStr;

    #[test]
    fn decimalize_six_dp() {
        let raw = BigDecimal::from_str("1000000").unwrap();
        assert_eq!(format_dec(&decimalize(&raw, 6)), "1");
    }

    #[test]
    fn missing_event_reserves_emit_zero_not_snapshot() {
        let z = event_reserves(None, None, 6, 6);
        assert_eq!(z.asset0, "0");
        assert_eq!(z.asset1, "0");
        let raw = BigDecimal::from_str("2000000").unwrap();
        let filled = event_reserves(Some(&raw), Some(&raw), 6, 6);
        assert_eq!(filled.asset0, "2");
        assert_eq!(filled.asset1, "2");
    }

    #[test]
    fn decimalize_eighteen_dp_no_scientific() {
        let raw = BigDecimal::from_str("1000000000000000000").unwrap();
        let s = format_dec(&decimalize(&raw, 18));
        assert_eq!(s, "1");
        assert!(!s.contains('e') && !s.contains('E'));
    }

    #[test]
    fn excluded_gems_and_spaceusd() {
        assert!(is_excluded_cw20(
            "terra1dmuruhht32x8f47nvm73pwp6q7uf2jtfhdt3nxcql4mmqkyfsraqn2dt94"
        ));
        assert!(is_excluded_cw20(
            "terra1cvd5cgrs8rrl96hte34n57497u5f9cwuv3e6ztxgetkx4uzmcdyswv79zl"
        ));
        assert!(!is_excluded_cw20(CL8Y_CW20));
    }

    #[test]
    fn gem_const_lockstep() {
        for addr in COLUMBUS5_GEM_ADDRESSES {
            assert!(is_excluded_cw20(addr), "{addr}");
        }
    }

    #[test]
    fn event_row_cap_is_5000() {
        assert_eq!(MAX_GT_EVENT_ROWS, 5_000);
        assert_eq!(GT_EVENT_ROW_CAP_MSG, "event count exceeds 5000");
        assert_eq!(MAX_EVENT_BLOCK_SPAN, 2_000);
    }
}
