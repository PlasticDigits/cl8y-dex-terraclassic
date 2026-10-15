//! Materialized DEX hub USD snapshot (GitLab #556).

mod scheduler;

use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool, Postgres, Transaction};

use crate::indexer::economic_usd::{resolve_economic_marks, EconomicMark};
use crate::indexer::hub_guard::{contain_mark, HubReference};
use crate::indexer::hub_usd::{
    resolve_hub_usd, resolve_lunc_hub_mark, AssetRef, HubMark, HubTicker, HubUsdConfig,
    HubUsdSnapshot, ReservePair,
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
    pub as_of: Option<DateTime<Utc>>,
    pub priced_leg_depth_usd: Option<BigDecimal>,
    pub quality: String,
    pub upstream_available: bool,
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

const RESERVE_INVENTORY_QUERY: &str = "SELECT
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
         LEFT JOIN pair_reserves r ON r.pair_id = p.id";

pub async fn list_reserve_pairs(pool: &PgPool) -> Result<Vec<ReservePair>, sqlx::Error> {
    let rows: Vec<PairReserveJoinRow> = sqlx::query_as(RESERVE_INVENTORY_QUERY)
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

/// Bounded SQL-only scheduled diagnostic census. An overflow is not a complete
/// census and cannot prove a token has no eligible alternative reference.
pub async fn quote_reference_census<'e, E>(
    executor: E,
) -> Result<Option<Vec<ReservePair>>, sqlx::Error>
where
    E: sqlx::Executor<'e, Database = Postgres>,
{
    let rows: Vec<PairReserveJoinRow> = sqlx::query_as(&format!(
        "{RESERVE_INVENTORY_QUERY} ORDER BY p.id LIMIT 513"
    ))
    .fetch_all(executor)
    .await?;
    if rows.len() > 512 {
        return Ok(None);
    }
    Ok(Some(
        rows.into_iter()
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
                    reserve_0: r.reserve_0.unwrap_or_default(),
                    reserve_1: r.reserve_1.unwrap_or_default(),
                    snapshot_at: r.snapshot_at.unwrap_or(DateTime::<Utc>::MIN_UTC),
                })
            })
            .collect(),
    ))
}

/// Full factory inventory for TVL, including missing reserve rows. A missing observation
/// carries the minimum timestamp sentinel and must be classified before zero reserves.
pub async fn list_protocol_reserve_pairs_tx(
    tx: &mut Transaction<'_, Postgres>,
) -> Result<Vec<ReservePair>, sqlx::Error> {
    let rows: Vec<PairReserveJoinRow> = sqlx::query_as(RESERVE_INVENTORY_QUERY)
        .fetch_all(&mut **tx)
        .await?;
    Ok(rows
        .into_iter()
        .map(|r| ReservePair {
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
            snapshot_at: if r.reserve_0.is_some() && r.reserve_1.is_some() {
                r.snapshot_at.unwrap_or(DateTime::<Utc>::MIN_UTC)
            } else {
                DateTime::<Utc>::MIN_UTC
            },
            reserve_0: r.reserve_0.unwrap_or_default(),
            reserve_1: r.reserve_1.unwrap_or_default(),
        })
        .collect())
}

async fn lookup_clunc_asset_id(
    pool: &PgPool,
    cfg: &HubUsdConfig,
) -> Result<Option<i32>, sqlx::Error> {
    sqlx::query_scalar(
        "SELECT id FROM assets
         WHERE is_cw20 AND denom IS NULL AND contract_address = $1
         ORDER BY id ASC
         LIMIT 1",
    )
    .bind(&cfg.clunc_address)
    .fetch_optional(pool)
    .await
}

async fn lookup_custc_asset_id(
    pool: &PgPool,
    cfg: &HubUsdConfig,
) -> Result<Option<i32>, sqlx::Error> {
    let cw20: Option<i32> = sqlx::query_scalar(
        "SELECT id FROM assets
         WHERE is_cw20 AND denom IS NULL AND contract_address = $1
         ORDER BY id ASC
         LIMIT 1",
    )
    .bind(&cfg.custc_address)
    .fetch_optional(pool)
    .await?;
    if cw20.is_some() {
        return Ok(cw20);
    }
    sqlx::query_scalar("SELECT id FROM assets WHERE NOT is_cw20 AND contract_address IS NULL AND denom = 'uusd' ORDER BY id ASC LIMIT 1")
        .fetch_optional(pool)
        .await
}

/// TTL/quality is enforced at every live hub read, including refreshed wrappers.
pub fn row_usable(row: &HubPriceRow, now: DateTime<Utc>) -> bool {
    let ttl = match row.ticker.as_str() {
        "custc" | "lunc" => crate::indexer::oracle::ORACLE_MAX_AGE_SECS,
        "ust1" | "ustr" => live_hub_max_age_seconds(),
        _ => return false,
    };
    row.upstream_available
        && matches!(row.quality.as_str(), "bounded_spot" | "external_quorum")
        && row
            .as_of
            .is_some_and(|at| at <= now && (now - at) <= chrono::Duration::seconds(ttl))
        && row.price_usd > BigDecimal::from(0)
}

pub fn live_hub_max_age_seconds() -> i64 {
    crate::indexer::protocol_tvl::max_staleness_from_env()
        .as_secs()
        .min(i64::MAX as u64) as i64
}

pub async fn get_hub_references(pool: &PgPool) -> Result<Vec<HubReference>, sqlx::Error> {
    sqlx::query_as("SELECT * FROM hub_price_references")
        .fetch_all(pool)
        .await
}

pub async fn get_all_hub_prices<'e, E>(executor: E) -> Result<Vec<HubPriceRow>, sqlx::Error>
where
    E: sqlx::Executor<'e, Database = Postgres>,
{
    sqlx::query_as(
        "SELECT ticker, asset_id, price_usd, source_pair_id, source_pair_address, tvl_usd, updated_at, as_of, priced_leg_depth_usd, quality,
         EXISTS (SELECT 1 FROM oracle_state os
             WHERE os.ticker = CASE WHEN hub_prices.ticker = 'lunc' THEN 'lunc' ELSE 'ustc' END
             AND os.quality = 'fresh' AND cardinality(os.accepted_sources) >= 2
             AND os.price_usd BETWEEN 0.000000000001 AND 1000000
             AND os.as_of BETWEEN NOW() - INTERVAL '300 seconds' AND NOW()) AS upstream_available
         FROM hub_prices
         ORDER BY ticker",
    )
    .fetch_all(executor)
    .await
}

pub async fn get_hub_price(
    pool: &PgPool,
    ticker: HubTicker,
) -> Result<Option<HubPriceRow>, sqlx::Error> {
    sqlx::query_as(
        "SELECT ticker, asset_id, price_usd, source_pair_id, source_pair_address, tvl_usd, updated_at, as_of, priced_leg_depth_usd, quality,
         EXISTS (SELECT 1 FROM oracle_state os
             WHERE os.ticker = CASE WHEN hub_prices.ticker = 'lunc' THEN 'lunc' ELSE 'ustc' END
             AND os.quality = 'fresh' AND cardinality(os.accepted_sources) >= 2
             AND os.price_usd BETWEEN 0.000000000001 AND 1000000
             AND os.as_of BETWEEN NOW() - INTERVAL '300 seconds' AND NOW()) AS upstream_available
         FROM hub_prices
         WHERE ticker = $1",
    )
    .bind(ticker.as_str())
    .fetch_optional(pool)
    .await
}

/// Quote USD for ingest: UST1/USTR from hub table (not $1 / 2.5×) plus
/// factory economic CW20 marks keyed by contract (GitLab #683). Extra
/// `hub_prices` tickers never leak into GET /hub-prices.
pub async fn load_quote_usd(pool: &PgPool) -> Result<HubQuoteUsd, sqlx::Error> {
    let mut connection = pool.acquire().await?;
    load_quote_usd_connection(&mut connection).await
}

async fn load_quote_usd_connection(
    connection: &mut sqlx::PgConnection,
) -> Result<HubQuoteUsd, sqlx::Error> {
    let rows = get_all_hub_prices(&mut *connection).await?;
    let mut hub = HubQuoteUsd::default();
    for row in rows {
        if !row_usable(&row, Utc::now()) {
            continue;
        }
        match row.ticker.as_str() {
            "ust1" => hub.ust1 = Some(row.price_usd),
            "ustr" => hub.ustr = Some(row.price_usd),
            _ => {}
        }
    }
    let econ_rows: Vec<(String, BigDecimal)> = sqlx::query_as(
        "SELECT contract_address, price_usd
         FROM economic_token_marks
         WHERE price_usd > 0 AND as_of BETWEEN NOW() - INTERVAL '20 seconds' AND NOW()
         AND EXISTS (SELECT 1 FROM oracle_state WHERE quality='fresh'
             AND ticker=CASE WHEN economic_token_marks.anchor_ticker='lunc' THEN 'lunc' ELSE 'ustc' END
             AND as_of BETWEEN NOW() - INTERVAL '300 seconds' AND NOW())
         AND (anchor_ticker IN ('custc','lunc') OR EXISTS (SELECT 1 FROM hub_price_references
             WHERE ticker=economic_token_marks.anchor_ticker AND quality='bounded_spot'))",
    )
    .fetch_all(&mut *connection)
    .await?;
    for (contract, price) in econ_rows {
        let key = contract.trim().to_ascii_lowercase();
        if key.starts_with("terra1") && price > BigDecimal::from(0) {
            hub.economic.insert(key, price);
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
    for mark in [&snap.custc, &snap.lunc, &snap.ust1, &snap.ustr]
        .into_iter()
        .flatten()
    {
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
            (ticker, asset_id, price_usd, source_pair_id, source_pair_address, tvl_usd, updated_at, as_of, priced_leg_depth_usd, quality)
         VALUES ($1, $2, $3, $4, $5, $6, NOW(), $7, $8, $9)",
    )
    .bind(mark.ticker.as_str())
    .bind(mark.asset_id)
    .bind(&mark.price_usd)
    .bind(mark.source_pair_id)
    .bind(&mark.source_pair_address)
    .bind(&mark.tvl_usd)
    .bind(mark.as_of)
    .bind(&mark.priced_leg_depth_usd)
    .bind(if mark.source_pair_id.is_some() { "bounded_spot" } else { "external_quorum" })
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn replace_economic_marks(
    tx: &mut Transaction<'_, Postgres>,
    marks: &[EconomicMark],
) -> Result<(), sqlx::Error> {
    sqlx::query("DELETE FROM economic_token_marks")
        .execute(&mut **tx)
        .await?;
    for mark in marks {
        if mark.price_usd <= BigDecimal::from(0) || !fits_numeric_38_18(&mark.price_usd) {
            continue;
        }
        let contract = mark.contract_address.trim().to_ascii_lowercase();
        if !contract.starts_with("terra1") {
            continue;
        }
        sqlx::query(
            "INSERT INTO economic_token_marks
                (contract_address, asset_id, price_usd, source_pair_id, source_pair_address, tvl_usd, updated_at, as_of, anchor_ticker)
             VALUES ($1, $2, $3, $4, $5, $6, NOW(), $7, $8)",
        )
        .bind(&contract)
        .bind(mark.asset_id)
        .bind(&mark.price_usd)
        .bind(mark.source_pair_id)
        .bind(&mark.source_pair_address)
        .bind(&mark.tvl_usd)
        .bind(mark.as_of)
        .bind(&mark.anchor_ticker)
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

async fn certified_oracles<'e, E>(
    executor: E,
    ustc: Option<&BigDecimal>,
    lunc: Option<&BigDecimal>,
) -> Result<(Option<BigDecimal>, Option<BigDecimal>), sqlx::Error>
where
    E: sqlx::Executor<'e, Database = Postgres>,
{
    let certified: Vec<(String, BigDecimal)> = sqlx::query_as(
        "SELECT ticker,price_usd FROM oracle_state
         WHERE quality='fresh' AND cardinality(accepted_sources)>=2
           AND price_usd BETWEEN 0.000000000001 AND 1000000
           AND as_of BETWEEN NOW() - INTERVAL '300 seconds' AND NOW()",
    )
    .fetch_all(executor)
    .await?;
    let eligible = |ticker: &str, value: Option<&BigDecimal>| {
        value
            .filter(|value| {
                certified
                    .iter()
                    .any(|(name, price)| name == ticker && price == *value)
            })
            .cloned()
    };
    Ok((eligible("ustc", ustc), eligible("lunc", lunc)))
}

struct AccountingContext {
    ustc: Option<BigDecimal>,
    lunc: Option<BigDecimal>,
    hub: HubQuoteUsd,
}

async fn accounting_context(
    pool: &PgPool,
    ustc: Option<&BigDecimal>,
    lunc: Option<&BigDecimal>,
) -> Result<AccountingContext, sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ, READ ONLY")
        .execute(&mut *tx)
        .await?;
    let (ustc, lunc) = certified_oracles(&mut *tx, ustc, lunc).await?;
    let hub = load_quote_usd_connection(&mut *tx).await?;
    tx.commit().await?;
    Ok(AccountingContext { ustc, lunc, hub })
}

struct CommittedMarks {
    snapshot: HubUsdSnapshot,
    as_of: DateTime<Utc>,
    ustc: Option<BigDecimal>,
    lunc: Option<BigDecimal>,
}

/// Recompute hub USD from indexed `pairs` + `pair_reserves` + USTC/LUNC oracles (no LCD, no swap scan).
///
/// Does **not** rewrite historical `swap_events.price_usd` or candle USD (GitLab #568).
/// After the snapshot commit, idle mark-to-market upserts only the **current** candle buckets.
pub async fn refresh_hub_prices(
    pool: &PgPool,
    cfg: &HubUsdConfig,
    ustc_oracle: Option<&BigDecimal>,
    lunc_oracle: Option<&BigDecimal>,
) -> Result<HubUsdSnapshot, sqlx::Error> {
    let committed = refresh_reference_marks(pool, cfg, ustc_oracle, lunc_oracle).await?;
    match accounting_context(pool, committed.ustc.as_ref(), committed.lunc.as_ref()).await {
        Ok(context) => refresh_reference_accounting(pool, cfg, committed.as_of, context).await,
        Err(_) => tracing::warn!(
            phase = "reference_accounting",
            "Certified accounting prices unavailable"
        ),
    }
    Ok(committed.snapshot)
}

async fn refresh_reference_marks(
    pool: &PgPool,
    cfg: &HubUsdConfig,
    ustc_oracle: Option<&BigDecimal>,
    lunc_oracle: Option<&BigDecimal>,
) -> Result<CommittedMarks, sqlx::Error> {
    let started = std::time::Instant::now();
    let pairs = list_reserve_pairs(pool).await?;
    let custc_id = lookup_custc_asset_id(pool, cfg).await?;
    let clunc_id = lookup_clunc_asset_id(pool, cfg).await?;
    let now = Utc::now();
    let mut tx = pool.begin().await?;
    // Serialize reference updates independently of protocol liquidity refresh.
    sqlx::query("SELECT pg_advisory_xact_lock(1391)")
        .execute(&mut *tx)
        .await?;
    let states: Vec<HubReference> = sqlx::query_as("SELECT * FROM hub_price_references")
        .fetch_all(&mut *tx)
        .await?;
    let mut references: std::collections::HashMap<String, HubReference> =
        states.into_iter().map(|s| (s.ticker.clone(), s)).collect();
    // Only the exact currently certified external values may establish references.
    // A stale SharedPrice or an uncatalogued caller cannot poison the bootstrap.
    let (certified_ustc, certified_lunc) =
        certified_oracles(&mut *tx, ustc_oracle, lunc_oracle).await?;
    let ustc_oracle = certified_ustc.as_ref();
    let lunc_oracle = certified_lunc.as_ref();
    let mut snap = resolve_hub_usd(now, cfg, ustc_oracle, &pairs, custc_id);
    // Keep an eligible established USTR source instead of switching on tiny depth changes.
    if let Some(reference) = references.get("ustr").filter(|r| r.confirmed) {
        let pinned: Vec<_> = pairs
            .iter()
            .filter(|p| {
                !(crate::indexer::hub_usd::is_hub_ustr(&p.asset_0, cfg)
                    || crate::indexer::hub_usd::is_hub_ustr(&p.asset_1, cfg))
                    || p.pair_address == reference.source_pair_address
            })
            .cloned()
            .collect();
        let preferred = resolve_hub_usd(now, cfg, ustc_oracle, &pinned, custc_id);
        if preferred.ustr.is_some() {
            snap.ustr = preferred.ustr;
        }
    }
    if let Some(ustc) = ustc_oracle {
        let mut state = references.remove("ust1");
        if let Some(mark) = &snap.ust1 {
            if !contain_mark(
                now,
                mark,
                ustc,
                &mut state,
                &format!(
                    "{}|{}|{}",
                    cfg.custc_address, cfg.ust1_address, cfg.ustr_address
                ),
            ) {
                snap.ust1 = None;
            }
        } else if let Some(state) = &mut state {
            state.quality = "unavailable".into();
        }
        if let Some(state) = state {
            references.insert("ust1".into(), state);
        }
        if snap.ust1.is_none() {
            // An unconfirmed/divergent UST1 must not anchor USTR or economic marks.
            let direct: Vec<_> = pairs
                .iter()
                .filter(|p| {
                    !crate::indexer::hub_usd::is_hub_ust1(&p.asset_0, cfg)
                        && !crate::indexer::hub_usd::is_hub_ust1(&p.asset_1, cfg)
                })
                .cloned()
                .collect();
            snap.ustr = resolve_hub_usd(now, cfg, ustc_oracle, &direct, custc_id).ustr;
        }
        let mut state = references.remove("ustr");
        if let Some(mark) = &snap.ustr {
            if !contain_mark(
                now,
                mark,
                ustc,
                &mut state,
                &format!(
                    "{}|{}|{}",
                    cfg.custc_address, cfg.ust1_address, cfg.ustr_address
                ),
            ) {
                snap.ustr = None;
            }
        } else if let Some(state) = &mut state {
            state.quality = "unavailable".into();
        }
        if let Some(state) = state {
            references.insert("ustr".into(), state);
        }
    } else {
        for reference in references.values_mut() {
            reference.quality = "unavailable".into();
        }
    }
    // A USTR reserve refresh cannot renew the original UST1 anchor timestamp.
    if let Some(mark) = &mut snap.ustr {
        if pairs
            .iter()
            .find(|p| Some(p.pair_id) == mark.source_pair_id)
            .is_some_and(|p| {
                crate::indexer::hub_usd::is_hub_ust1(&p.asset_0, cfg)
                    || crate::indexer::hub_usd::is_hub_ust1(&p.asset_1, cfg)
            })
        {
            mark.as_of = mark
                .as_of
                .zip(snap.ust1.as_ref().and_then(|m| m.as_of))
                .map(|(a, b)| a.min(b));
        }
    }
    snap.lunc = resolve_lunc_hub_mark(lunc_oracle, clunc_id);
    // External wrap provenance carries original upstream time, never a refresh timestamp.
    for (ticker, mark) in [("ustc", &mut snap.custc), ("lunc", &mut snap.lunc)] {
        if let Some(mark) = mark {
            mark.as_of = sqlx::query_scalar(
                "SELECT as_of FROM oracle_state WHERE ticker = $1 AND quality = 'fresh'",
            )
            .bind(ticker)
            .fetch_optional(&mut *tx)
            .await?
            .flatten();
        }
    }
    let economic = resolve_economic_marks(now, cfg, &pairs, &snap);
    for reference in references.values() {
        if reference.quality == "divergent" {
            tracing::warn!(ticker = %reference.ticker, divergence_pct = %reference.divergence_pct,
                limit_pct = 10, source_pair = %reference.pending_pair_address,
                "Hub spot divergence rejected (#1391)");
        }
        sqlx::query("INSERT INTO hub_price_references
            (ticker, reference_ratio, reference_at, source_pair_address, pending_pair_address,
             pending_since, last_observed_at, confirmed, quality, divergence_pct, identity_key, last_observed_ratio)
            VALUES ($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12)
            ON CONFLICT (ticker) DO UPDATE SET
             source_pair_address = EXCLUDED.source_pair_address,
             pending_pair_address = EXCLUDED.pending_pair_address,
             pending_since = EXCLUDED.pending_since, last_observed_at = EXCLUDED.last_observed_at,
             last_observed_ratio = EXCLUDED.last_observed_ratio,
             confirmed = EXCLUDED.confirmed, quality = EXCLUDED.quality,
             divergence_pct = EXCLUDED.divergence_pct")
            .bind(&reference.ticker).bind(&reference.reference_ratio).bind(reference.reference_at)
            .bind(&reference.source_pair_address).bind(&reference.pending_pair_address)
            .bind(reference.pending_since).bind(reference.last_observed_at).bind(reference.confirmed)
            .bind(&reference.quality).bind(&reference.divergence_pct)
            .bind(&reference.identity_key).bind(&reference.last_observed_ratio).execute(&mut *tx).await?;
    }
    replace_snapshot(&mut tx, &snap).await?;
    replace_economic_marks(&mut tx, &economic).await?;
    tx.commit().await?;

    tracing::info!(
        phase = "reference_marks",
        economic_marks = economic.len(),
        elapsed_ms = started.elapsed().as_millis() as u64,
        oldest_economic_as_of_ms = economic
            .iter()
            .map(|mark| mark.as_of.timestamp_millis())
            .min(),
        newest_economic_as_of_ms = economic
            .iter()
            .map(|mark| mark.as_of.timestamp_millis())
            .max(),
        "Reference cohort committed"
    );
    Ok(CommittedMarks {
        snapshot: snap,
        as_of: now,
        ustc: ustc_oracle.cloned(),
        lunc: lunc_oracle.cloned(),
    })
}

async fn refresh_reference_accounting(
    pool: &PgPool,
    cfg: &HubUsdConfig,
    now: DateTime<Utc>,
    context: AccountingContext,
) {
    let AccountingContext { ustc, lunc, hub } = context;
    let ustc_oracle = ustc.as_ref();
    let lunc_oracle = lunc.as_ref();
    if let Err(e) = crate::db::queries::protocol_fees::backfill_null_fee_usd(
        pool,
        ustc_oracle,
        lunc_oracle,
        None,
        &hub,
    )
    .await
    {
        tracing::warn!("NULL-only protocol fee USD backfill failed: {}", e);
    } else if let Ok(flags) = crate::db::queries::protocol_fees::get_fee_rollup(pool).await {
        if let Err(e) = crate::db::queries::protocol_fees::refresh_protocol_fees(
            pool,
            flags.wrap_mapper_configured,
            flags.ust1_window_configured,
        )
        .await
        {
            tracing::warn!("Protocol fee rollup after economic marks failed: {}", e);
        }
    }
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
}

pub async fn run_hub_usd_refresh_loop(
    pool: PgPool,
    cfg: HubUsdConfig,
    ustc_price: crate::indexer::oracle::SharedPrice,
    lunc_price: crate::indexer::oracle::SharedPrice,
    interval: std::time::Duration,
) {
    run_hub_usd_refresh_loop_with_signal(pool, cfg, ustc_price, lunc_price, interval, None).await;
}

/// The pool producer can coalesce one wake per completed cycle. A single marks
/// lane preserves atomic qualification and original time; unrelated accounting
/// has its own single nonoverlapping lane at the unchanged timer cadence.
pub async fn run_hub_usd_refresh_loop_with_signal(
    pool: PgPool,
    cfg: HubUsdConfig,
    ustc_price: crate::indexer::oracle::SharedPrice,
    lunc_price: crate::indexer::oracle::SharedPrice,
    interval: std::time::Duration,
    pool_completed: Option<std::sync::Arc<tokio::sync::Notify>>,
) {
    let mark_loop = scheduler::run(interval, pool_completed, || {
        let pool = pool.clone();
        let cfg = cfg.clone();
        let ustc_price = ustc_price.clone();
        let lunc_price = lunc_price.clone();
        async move {
            let ustc = ustc_price.read().await.clone();
            let lunc = lunc_price.read().await.clone();
            if let Err(e) = refresh_reference_marks(&pool, &cfg, ustc.as_ref(), lunc.as_ref()).await
            {
                tracing::warn!("Hub USD refresh failed: {}", e);
            }
        }
    });
    let accounting_loop = scheduler::run(interval, None, || {
        let pool = pool.clone();
        let cfg = cfg.clone();
        let ustc_price = ustc_price.clone();
        let lunc_price = lunc_price.clone();
        async move {
            let ustc = ustc_price.read().await.clone();
            let lunc = lunc_price.read().await.clone();
            let Ok(context) = accounting_context(&pool, ustc.as_ref(), lunc.as_ref()).await else {
                tracing::warn!(
                    phase = "reference_accounting",
                    "Certified accounting prices unavailable"
                );
                return;
            };
            refresh_reference_accounting(&pool, &cfg, Utc::now(), context).await;
            if let Err(e) = crate::indexer::protocol_tvl::refresh_protocol_liquidity(&pool).await {
                tracing::warn!("Protocol liquidity refresh after hub USD failed: {}", e);
            }
        }
    });
    tokio::join!(mark_loop, accounting_loop);
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn missing_source_pair_id_cannot_extend_spot_ttl() {
        let now = Utc::now();
        let mut row = HubPriceRow {
            ticker: "ust1".into(),
            asset_id: None,
            price_usd: BigDecimal::from(1),
            source_pair_id: None,
            source_pair_address: Some("canonical-pair".into()),
            tvl_usd: None,
            updated_at: now,
            as_of: Some(now - chrono::Duration::seconds(live_hub_max_age_seconds() + 1)),
            priced_leg_depth_usd: None,
            quality: "bounded_spot".into(),
            upstream_available: true,
        };
        assert!(
            !row_usable(&row, now),
            "deleted source FK must retain spot TTL"
        );
        row.ticker = "ustr".into();
        assert!(!row_usable(&row, now));
        row.as_of = Some(now);
        assert!(row_usable(&row, now));
    }
}
