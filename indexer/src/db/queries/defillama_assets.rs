//! UTC-day UST1 / USTR rollup (GitLab #631). Called from `defillama::refresh_defillama_day`.

use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{FromRow, PgConnection};

use crate::indexer::defillama::{daily_asset_contract, DAILY_ASSET_TICKERS};

/// These snapshots are advisory auxiliary data, separate from ledger accounting.
/// Historical repair preserves them verbatim and never samples today's marks.
#[derive(Clone, Default, FromRow)]
struct AuxiliarySnapshot {
    ticker: String,
    price_usd: Option<BigDecimal>,
    price_as_of: Option<DateTime<Utc>>,
    circulating_raw: Option<BigDecimal>,
}

pub async fn refresh_daily_assets(
    connection: &mut PgConnection,
    day_start: DateTime<Utc>,
    day_end: DateTime<Utc>,
    utc_day: NaiveDate,
    gems: &[String],
    preserve_auxiliary_snapshots: bool,
) -> Result<(), sqlx::Error> {
    #[derive(FromRow)]
    struct VolAgg {
        trade_count: i64,
        unpriced_trade_count: i64,
        volume_usd: Option<BigDecimal>,
    }
    #[derive(FromRow)]
    struct FeeAgg {
        event_count: i64,
        unpriced_count: i64,
        amount_usd: Option<BigDecimal>,
    }

    let previous_snapshots: Vec<AuxiliarySnapshot> = if preserve_auxiliary_snapshots {
        sqlx::query_as(
            "SELECT ticker, price_usd, price_as_of, circulating_raw
             FROM defillama_daily_assets WHERE utc_day = $1",
        )
        .bind(utc_day)
        .fetch_all(&mut *connection)
        .await?
    } else {
        Vec::new()
    };

    sqlx::query("DELETE FROM defillama_daily_assets WHERE utc_day = $1")
        .bind(utc_day)
        .execute(&mut *connection)
        .await?;

    for ticker in DAILY_ASSET_TICKERS {
        let Some(contract) = daily_asset_contract(ticker) else {
            continue;
        };
        let contract_lc = contract.to_ascii_lowercase();

        let vol: VolAgg = sqlx::query_as(
            r#"
            WITH gem_pairs AS (
                SELECT p.id
                FROM pairs p
                JOIN assets a0 ON a0.id = p.asset_0_id
                JOIN assets a1 ON a1.id = p.asset_1_id
                WHERE LOWER(COALESCE(a0.contract_address, '')) = ANY($3)
                   OR LOWER(COALESCE(a1.contract_address, '')) = ANY($3)
            ),
            target AS (
                SELECT id FROM assets
                WHERE LOWER(COALESCE(contract_address, '')) = $4
            )
            SELECT
                COUNT(*)::bigint AS trade_count,
                COUNT(*) FILTER (
                    WHERE volume_usd IS NULL OR volume_usd <= 0
                )::bigint AS unpriced_trade_count,
                COALESCE(
                    SUM(volume_usd) FILTER (
                        WHERE volume_usd IS NOT NULL AND volume_usd > 0
                    ),
                    0
                ) AS volume_usd
            FROM swap_events se
            JOIN pairs p ON p.id = se.pair_id
            WHERE se.block_timestamp >= $1
              AND se.block_timestamp < $2
              AND se.pair_id NOT IN (SELECT id FROM gem_pairs)
              AND (
                  p.asset_0_id IN (SELECT id FROM target)
                  OR p.asset_1_id IN (SELECT id FROM target)
              )
            "#,
        )
        .bind(day_start)
        .bind(day_end)
        .bind(gems)
        .bind(&contract_lc)
        .fetch_one(&mut *connection)
        .await?;

        let include_window = *ticker == "ust1";
        let fee_sql = format!(
            "{}{}",
            include_str!("defillama_fee_scope.sql"),
            r#"
            , target_pairs AS (
                SELECT p.id FROM pairs p
                JOIN assets a0 ON a0.id = p.asset_0_id
                JOIN assets a1 ON a1.id = p.asset_1_id
                WHERE LOWER(COALESCE(a0.contract_address, '')) = $4
                   OR LOWER(COALESCE(a1.contract_address, '')) = $4
            )
            SELECT COUNT(*)::bigint AS event_count,
                   COUNT(*) FILTER (WHERE e.fee_usd IS NULL OR e.fee_usd <= 0)::bigint AS unpriced_count,
                   COALESCE(SUM(e.fee_usd) FILTER (WHERE e.fee_usd > 0), 0) AS amount_usd
            FROM scoped_fee_events e
            WHERE ($5 AND e.source IN ('ust1_mint', 'ust1_redeem'))
               OR (e.source IN ('swap_amm', 'book_take', 'limit_place')
                   AND e.resolved_pair_id IN (SELECT id FROM target_pairs)
                   AND e.resolved_pair_id NOT IN (SELECT id FROM gem_pairs))
            "#
        );
        let fees: FeeAgg = sqlx::query_as(&fee_sql)
            .bind(day_start)
            .bind(day_end)
            .bind(gems)
            .bind(&contract_lc)
            .bind(include_window)
            .fetch_one(&mut *connection)
            .await?;

        let auxiliary = if preserve_auxiliary_snapshots {
            previous_snapshots
                .iter()
                .find(|row| row.ticker == *ticker)
                .cloned()
                .unwrap_or_default()
        } else {
            let price: Option<(BigDecimal, DateTime<Utc>)> = sqlx::query_as(
                "SELECT price_usd, as_of FROM hub_prices WHERE ticker = $1 AND price_usd > 0
             AND quality IN ('bounded_spot','external_quorum')
             AND as_of BETWEEN NOW() - INTERVAL '20 seconds' AND NOW()
             AND EXISTS (SELECT 1 FROM oracle_state os WHERE os.ticker = 'ustc'
                 AND os.quality = 'fresh' AND cardinality(os.accepted_sources) >= 2
                 AND os.as_of BETWEEN NOW() - INTERVAL '300 seconds' AND NOW())",
            )
            .bind(*ticker)
            .fetch_optional(&mut *connection)
            .await?;

            AuxiliarySnapshot {
                ticker: ticker.to_string(),
                price_usd: price.as_ref().map(|p| p.0.clone()),
                price_as_of: price.map(|p| p.1),
                circulating_raw: None,
            }
        };

        sqlx::query(
            r#"
            INSERT INTO defillama_daily_assets
                (utc_day, ticker, volume_usd, trade_count, unpriced_trade_count,
                 fees_usd, fee_event_count, fee_unpriced_count, price_usd, circulating_raw, price_as_of)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
            "#,
        )
        .bind(utc_day)
        .bind(*ticker)
        .bind(vol.volume_usd.unwrap_or_else(|| BigDecimal::from(0)))
        .bind(vol.trade_count)
        .bind(vol.unpriced_trade_count)
        .bind(fees.amount_usd.unwrap_or_else(|| BigDecimal::from(0)))
        .bind(fees.event_count)
        .bind(fees.unpriced_count)
        .bind(auxiliary.price_usd)
        .bind(auxiliary.circulating_raw)
        .bind(auxiliary.price_as_of)
        .execute(&mut *connection)
        .await?;
    }

    Ok(())
}
