//! UTC-day UST1 / USTR rollup (GitLab #631). Called from `defillama::refresh_defillama_day`.

use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDate, Utc};
use sqlx::{FromRow, PgPool};

use crate::indexer::defillama::{daily_asset_contract, DAILY_ASSET_TICKERS};

pub async fn refresh_daily_assets(
    pool: &PgPool,
    day_start: DateTime<Utc>,
    day_end: DateTime<Utc>,
    utc_day: NaiveDate,
    gems: &[String],
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

    sqlx::query("DELETE FROM defillama_daily_assets WHERE utc_day = $1")
        .bind(utc_day)
        .execute(pool)
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
        .fetch_one(pool)
        .await?;

        let include_window = *ticker == "ust1";
        let fees: FeeAgg = sqlx::query_as(
            r#"
            WITH gem_pairs AS (
                SELECT p.id
                FROM pairs p
                JOIN assets a0 ON a0.id = p.asset_0_id
                JOIN assets a1 ON a1.id = p.asset_1_id
                WHERE LOWER(COALESCE(a0.contract_address, '')) = ANY($3)
                   OR LOWER(COALESCE(a1.contract_address, '')) = ANY($3)
            ),
            target_pairs AS (
                SELECT p.id
                FROM pairs p
                JOIN assets a0 ON a0.id = p.asset_0_id
                JOIN assets a1 ON a1.id = p.asset_1_id
                WHERE LOWER(COALESCE(a0.contract_address, '')) = $4
                   OR LOWER(COALESCE(a1.contract_address, '')) = $4
            )
            SELECT
                COUNT(*)::bigint AS event_count,
                COUNT(*) FILTER (
                    WHERE e.fee_usd IS NULL OR e.fee_usd <= 0
                )::bigint AS unpriced_count,
                COALESCE(
                    SUM(e.fee_usd) FILTER (
                        WHERE e.fee_usd IS NOT NULL AND e.fee_usd > 0
                    ),
                    0
                ) AS amount_usd
            FROM protocol_fee_events e
            WHERE e.block_timestamp >= $1
              AND e.block_timestamp < $2
              AND (
                  ($5 AND e.source IN ('ust1_mint', 'ust1_redeem'))
                  OR (
                      e.source IN ('swap_amm', 'book_take', 'limit_place')
                      AND NOT EXISTS (
                          SELECT 1 FROM gem_pairs gp
                          WHERE
                              (e.source = 'swap_amm' AND EXISTS (
                                  SELECT 1 FROM swap_events se
                                  WHERE se.tx_hash = e.tx_hash AND se.pair_id = gp.id
                              ))
                              OR (e.source = 'book_take' AND EXISTS (
                                  SELECT 1 FROM limit_order_fills f
                                  WHERE f.tx_hash = e.tx_hash AND f.pair_id = gp.id
                              ))
                              OR (e.source = 'limit_place' AND EXISTS (
                                  SELECT 1 FROM limit_order_placements pl
                                  WHERE pl.tx_hash = e.tx_hash AND pl.pair_id = gp.id
                              ))
                      )
                      AND (
                          (e.source = 'swap_amm' AND EXISTS (
                              SELECT 1 FROM swap_events se
                              WHERE se.tx_hash = e.tx_hash
                                AND se.pair_id IN (SELECT id FROM target_pairs)
                          ))
                          OR (e.source = 'book_take' AND EXISTS (
                              SELECT 1 FROM limit_order_fills f
                              WHERE f.tx_hash = e.tx_hash
                                AND f.pair_id IN (SELECT id FROM target_pairs)
                          ))
                          OR (e.source = 'limit_place' AND EXISTS (
                              SELECT 1 FROM limit_order_placements pl
                              WHERE pl.tx_hash = e.tx_hash
                                AND pl.pair_id IN (SELECT id FROM target_pairs)
                          ))
                      )
                  )
              )
            "#,
        )
        .bind(day_start)
        .bind(day_end)
        .bind(gems)
        .bind(&contract_lc)
        .bind(include_window)
        .fetch_one(pool)
        .await?;

        let price: Option<BigDecimal> = sqlx::query_scalar(
            "SELECT price_usd FROM hub_prices WHERE ticker = $1 AND price_usd > 0",
        )
        .bind(*ticker)
        .fetch_optional(pool)
        .await?;

        sqlx::query(
            r#"
            INSERT INTO defillama_daily_assets
                (utc_day, ticker, volume_usd, trade_count, unpriced_trade_count,
                 fees_usd, fee_event_count, fee_unpriced_count, price_usd, circulating_raw)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NULL)
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
        .bind(price)
        .execute(pool)
        .await?;
    }

    Ok(())
}
