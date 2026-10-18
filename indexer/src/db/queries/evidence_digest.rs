//! #1206: independent, bounded protocol aggregates; never select actor/transaction columns.
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::{FromRow, PgPool};

pub const COUNTS_SQL: &str = r#"
SELECT 'swap' AS surface, 'swap' AS kind, COUNT(*) AS event_count FROM swap_events WHERE block_timestamp >= $1 AND block_timestamp < $2
UNION ALL SELECT 'lp', event_type, COUNT(*) FROM liquidity_events WHERE block_timestamp >= $1 AND block_timestamp < $2 GROUP BY event_type
UNION ALL SELECT 'limit', 'place', COUNT(*) FROM limit_order_placements WHERE block_timestamp >= $1 AND block_timestamp < $2
UNION ALL SELECT 'limit', 'cancel', COUNT(*) FROM limit_order_cancellations WHERE block_timestamp >= $1 AND block_timestamp < $2
UNION ALL SELECT 'limit', 'fill', COUNT(*) FROM limit_order_fills WHERE block_timestamp >= $1 AND block_timestamp < $2
"#;
pub const SOURCES_SQL: &str = r#"
SELECT source, COUNT(*) AS event_count, COUNT(fee_usd) AS priced_count, SUM(fee_usd) AS priced_usd
FROM protocol_fee_events WHERE block_timestamp >= $1 AND block_timestamp < $2 AND source = ANY($3)
GROUP BY source
"#;
// Group in PostgreSQL, return only top eight protocol assets plus one mixed-unit bucket.
// No unbounded asset rows or metadata strings reach the response.
pub const TOKENS_SQL: &str = r#"
WITH grouped AS (
 SELECT asset_id, COUNT(*) AS event_count, COUNT(fee_usd) AS priced_count, SUM(fee_usd) AS priced_usd
 FROM protocol_fee_events WHERE block_timestamp >= $1 AND block_timestamp < $2 AND source = ANY($3)
 GROUP BY asset_id
), ranked AS (
 SELECT *, ROW_NUMBER() OVER (ORDER BY priced_usd DESC NULLS LAST, asset_id) AS rank FROM grouped
), bounded AS (
 SELECT asset_id, event_count, priced_count, priced_usd, rank, false AS is_other FROM ranked WHERE rank <= 8
 UNION ALL
 SELECT NULL, SUM(event_count)::bigint, SUM(priced_count)::bigint, SUM(priced_usd), 9, true FROM ranked WHERE rank > 8 HAVING COUNT(*) > 0
)
SELECT a.contract_address, a.denom, b.event_count, b.priced_count, b.priced_usd, b.is_other
FROM bounded b LEFT JOIN assets a ON a.id = b.asset_id ORDER BY b.rank
"#;

#[derive(FromRow)]
pub struct CountRow {
    pub surface: String,
    pub kind: String,
    pub event_count: i64,
}
#[derive(FromRow)]
pub struct SourceRow {
    pub source: String,
    pub event_count: i64,
    pub priced_count: i64,
    pub priced_usd: Option<BigDecimal>,
}
#[derive(FromRow)]
pub struct TokenRow {
    pub contract_address: Option<String>,
    pub denom: Option<String>,
    pub event_count: i64,
    pub priced_count: i64,
    pub priced_usd: Option<BigDecimal>,
    pub is_other: bool,
}

pub async fn aggregate(
    pool: &PgPool,
    start: DateTime<Utc>,
    end: DateTime<Utc>,
    sources: &[&str],
) -> Result<(Vec<CountRow>, Vec<SourceRow>, Vec<TokenRow>), sqlx::Error> {
    let mut tx = pool.begin().await?;
    sqlx::query("SET TRANSACTION ISOLATION LEVEL REPEATABLE READ READ ONLY")
        .execute(&mut *tx)
        .await?;
    // Entire HTTP owner also has a three-second deadline and two-export admission cap.
    sqlx::query("SET LOCAL statement_timeout = '700ms'")
        .execute(&mut *tx)
        .await?;
    sqlx::query("SET LOCAL lock_timeout = '200ms'")
        .execute(&mut *tx)
        .await?;
    let counts = sqlx::query_as(COUNTS_SQL)
        .bind(start)
        .bind(end)
        .fetch_all(&mut *tx)
        .await?;
    let fees = sqlx::query_as(SOURCES_SQL)
        .bind(start)
        .bind(end)
        .bind(sources)
        .fetch_all(&mut *tx)
        .await?;
    let tokens = sqlx::query_as(TOKENS_SQL)
        .bind(start)
        .bind(end)
        .bind(sources)
        .fetch_all(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok((counts, fees, tokens))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sql_has_no_actor_or_tape_dependency() {
        for sql in [COUNTS_SQL, SOURCES_SQL, TOKENS_SQL] {
            let sql = sql.to_ascii_lowercase();
            for forbidden in [
                "traders",
                "sender",
                "maker",
                "owner",
                "provider",
                "tx_hash",
                "pair_reserves",
                "count(distinct",
                "select * from",
            ] {
                assert!(
                    !sql.contains(forbidden),
                    "forbidden SQL dependency: {forbidden}"
                );
            }
            assert!(sql.contains("block_timestamp >= $1 and block_timestamp < $2"));
        }
    }
}
