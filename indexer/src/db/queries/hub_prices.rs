//! Materialized DEX hub USD snapshot (GitLab #556).

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool, Postgres, Transaction};

use crate::indexer::hub_usd::{
    resolve_hub_usd, AssetRef, HubMark, HubTicker, HubUsdConfig, HubUsdSnapshot, ReservePair,
};
use crate::indexer::pair_price_usd::{fits_numeric_38_18, HubQuoteUsd};

#[derive(Debug, Clone, FromRow)]
struct PairReserveJoinRow {
    pair_id: i32,
    pair_address: String,
    a0_id: i32,
    a0_symbol: String,
    a0_denom: Option<String>,
    a0_contract: Option<String>,
    a0_is_cw20: bool,
    a0_decimals: i16,
    a1_id: i32,
    a1_symbol: String,
    a1_denom: Option<String>,
    a1_contract: Option<String>,
    a1_is_cw20: bool,
    a1_decimals: i16,
    reserve_0: Option<BigDecimal>,
    reserve_1: Option<BigDecimal>,
    snapshot_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow)]
pub struct HubPriceRow {
    pub ticker: String,
    pub asset_id: Option<i32>,
    pub price_usd: BigDecimal,
    pub source_pair_id: Option<i32>,
    pub source_pair_address: Option<String>,
    pub tvl_usd: Option<BigDecimal>,
    pub updated_at: DateTime<Utc>,
}

fn asset_ref(
    id: i32,
    symbol: String,
    denom: Option<String>,
    contract: Option<String>,
    is_cw20: bool,
    decimals: i16,
) -> AssetRef {
    AssetRef {
        id,
        symbol,
        denom,
        contract_address: contract,
        is_cw20,
        decimals,
    }
}

async fn list_reserve_pairs(pool: &PgPool) -> Result<Vec<ReservePair>, sqlx::Error> {
    let rows: Vec<PairReserveJoinRow> = sqlx::query_as(
        "SELECT
            p.id AS pair_id,
            p.contract_address AS pair_address,
            a0.id AS a0_id,
            a0.symbol AS a0_symbol,
            a0.denom AS a0_denom,
            a0.contract_address AS a0_contract,
            a0.is_cw20 AS a0_is_cw20,
            a0.decimals AS a0_decimals,
            a1.id AS a1_id,
            a1.symbol AS a1_symbol,
            a1.denom AS a1_denom,
            a1.contract_address AS a1_contract,
            a1.is_cw20 AS a1_is_cw20,
            a1.decimals AS a1_decimals,
            r.reserve_0,
            r.reserve_1,
            r.snapshot_at
         FROM pairs p
         JOIN assets a0 ON a0.id = p.asset_0_id
         JOIN assets a1 ON a1.id = p.asset_1_id
         LEFT JOIN pair_reserves r ON r.pair_id = p.id",
    )
    .fetch_all(pool)
    .await?;

    Ok(rows
        .into_iter()
        .filter_map(|r| {
            Some(ReservePair {
                pair_id: r.pair_id,
                pair_address: r.pair_address,
                asset_0: asset_ref(
                    r.a0_id,
                    r.a0_symbol,
                    r.a0_denom,
                    r.a0_contract,
                    r.a0_is_cw20,
                    r.a0_decimals,
                ),
                asset_1: asset_ref(
                    r.a1_id,
                    r.a1_symbol,
                    r.a1_denom,
                    r.a1_contract,
                    r.a1_is_cw20,
                    r.a1_decimals,
                ),
                reserve_0: r.reserve_0?,
                reserve_1: r.reserve_1?,
                snapshot_at: r.snapshot_at?,
            })
        })
        .collect())
}

async fn lookup_custc_asset_id(
    pool: &PgPool,
    cfg: &HubUsdConfig,
) -> Result<Option<i32>, sqlx::Error> {
    let cw20: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM assets
         WHERE is_cw20 AND contract_address = $1
         ORDER BY id ASC
         LIMIT 1",
    )
    .bind(&cfg.custc_address)
    .fetch_optional(pool)
    .await?;
    if cw20.is_some() {
        return Ok(cw20);
    }
    sqlx::query_scalar("SELECT id FROM assets WHERE denom = 'uusd' ORDER BY id ASC LIMIT 1")
        .fetch_optional(pool)
        .await
}

pub async fn get_all_hub_prices(pool: &PgPool) -> Result<Vec<HubPriceRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT ticker, asset_id, price_usd, source_pair_id, source_pair_address, tvl_usd, updated_at
         FROM hub_prices
         ORDER BY ticker",
    )
    .fetch_all(pool)
    .await
}

pub async fn get_hub_price(
    pool: &PgPool,
    ticker: HubTicker,
) -> Result<Option<HubPriceRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT ticker, asset_id, price_usd, source_pair_id, source_pair_address, tvl_usd, updated_at
         FROM hub_prices
         WHERE ticker = $1",
    )
    .bind(ticker.as_str())
    .fetch_optional(pool)
    .await
}

/// Quote USD for ingest: UST1/USTR from hub table (not $1 / 2.5×).
pub async fn load_quote_usd(pool: &PgPool) -> Result<HubQuoteUsd, sqlx::Error> {
    let rows = get_all_hub_prices(pool).await?;
    let mut hub = HubQuoteUsd::default();
    for row in rows {
        match row.ticker.as_str() {
            "ust1" => hub.ust1 = Some(row.price_usd),
            "ustr" => hub.ustr = Some(row.price_usd),
            _ => {}
        }
    }
    Ok(hub)
}

async fn replace_snapshot(
    tx: &mut Transaction<'_, Postgres>,
    snap: &HubUsdSnapshot,
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM hub_prices")
        .execute(&mut **tx)
        .await?;
    for mark in [&snap.custc, &snap.ust1, &snap.ustr].into_iter().flatten() {
        insert_mark(tx, mark).await?;
    }
    Ok(())
}

async fn insert_mark(
    tx: &mut Transaction<'_, Postgres>,
    mark: &HubMark,
) -> Result<(), sqlx::Error> {
    if mark.price_usd <= BigDecimal::from(0) || !fits_numeric_38_18(&mark.price_usd) {
        return Ok(());
    }
    sqlx::query(
        "INSERT INTO hub_prices
            (ticker, asset_id, price_usd, source_pair_id, source_pair_address, tvl_usd, updated_at)
         VALUES ($1, $2, $3, $4, $5, $6, NOW())",
    )
    .bind(mark.ticker.as_str())
    .bind(mark.asset_id)
    .bind(&mark.price_usd)
    .bind(mark.source_pair_id)
    .bind(&mark.source_pair_address)
    .bind(&mark.tvl_usd)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

/// Recompute hub USD from indexed `pairs` + `pair_reserves` + USTC oracle (no LCD, no swap scan).
///
/// Does **not** rewrite historical `swap_events.price_usd` or candle USD (GitLab #568).
/// After the snapshot commit, idle mark-to-market upserts only the **current** candle buckets.
pub async fn refresh_hub_prices(
    pool: &PgPool,
    cfg: &HubUsdConfig,
    ustc_oracle: Option<&BigDecimal>,
    lunc_oracle: Option<&BigDecimal>,
) -> Result<HubUsdSnapshot, sqlx::Error> {
    let pairs = list_reserve_pairs(pool).await?;
    let custc_id = lookup_custc_asset_id(pool, cfg).await?;
    let now = Utc::now();
    let snap = resolve_hub_usd(now, cfg, ustc_oracle, &pairs, custc_id);

    let mut tx = pool.begin().await?;
    replace_snapshot(&mut tx, &snap).await?;
    tx.commit().await?;

    let hub = load_quote_usd(pool).await.unwrap_or_default();
    if let Err(e) = crate::indexer::candle_mark::apply_idle_usd_marks(
        pool,
        cfg,
        now,
        ustc_oracle,
        lunc_oracle,
        &hub,
    )
    .await
    {
        tracing::warn!("Idle USD candle marks failed: {}", e);
    }
    Ok(snap)
}

pub async fn run_hub_usd_refresh_loop(
    pool: PgPool,
    cfg: HubUsdConfig,
    ustc_price: crate::indexer::oracle::SharedPrice,
    lunc_price: crate::indexer::oracle::SharedPrice,
    interval: std::time::Duration,
) {
    loop {
        let ustc = ustc_price.read().await.clone();
        let lunc = lunc_price.read().await.clone();
        if let Err(e) = refresh_hub_prices(&pool, &cfg, ustc.as_ref(), lunc.as_ref()).await {
            tracing::warn!("Hub USD refresh failed: {}", e);
        }
        tokio::time::sleep(interval).await;
    }
}
