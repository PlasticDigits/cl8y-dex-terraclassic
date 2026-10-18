//! Materialized public cohort totals only; actor columns never reach this module's rows.
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use sqlx::FromRow;

#[derive(Debug, Clone, FromRow)]
pub struct FeeCohortStat {
    pub cohort: String,
    pub amount_usd: Option<BigDecimal>,
    pub share_pct: Option<BigDecimal>,
    pub event_count: i64,
}

/// One finite aggregate per scheduled window. Missing legacy attribution is unknown.
/// Atomic upserts preserve the prior complete three-row snapshot on any failure.
pub async fn refresh<'e>(
    pool: impl sqlx::Executor<'e, Database = sqlx::Postgres>,
    window: &str,
    cutoff: DateTime<Utc>,
) -> Result<(), sqlx::Error> {
    sqlx::query(r#"
        INSERT INTO protocol_fee_stats_by_cohort("window",cohort,amount_usd,event_count,share_pct,updated_at)
        WITH totals AS (
            SELECT COALESCE(a.cohort,'unknown') AS cohort, COUNT(*) AS event_count,
                SUM(e.fee_usd) FILTER(WHERE e.fee_usd IS NOT NULL) AS usd
            FROM protocol_fee_events e LEFT JOIN protocol_fee_attributions a ON a.fee_event_id=e.id
            WHERE e.block_timestamp >= $2 GROUP BY COALESCE(a.cohort,'unknown')
        ), priced AS (SELECT SUM(usd) AS usd FROM totals)
        SELECT $1,c.cohort,
            CASE WHEN COALESCE(t.event_count,0)=0 THEN 0
                 WHEN t.usd>0 AND t.usd<1e20::numeric THEN t.usd ELSE NULL END,
            COALESCE(t.event_count,0),
            CASE WHEN p.usd>0 AND ((t.usd>0 AND t.usd<1e20::numeric) OR COALESCE(t.event_count,0)=0) THEN COALESCE(t.usd,0)/p.usd*100 ELSE NULL END,
            now()
        FROM (VALUES ('retail'),('mm'),('unknown')) AS c(cohort)
        LEFT JOIN totals t ON t.cohort=c.cohort CROSS JOIN priced p
        ON CONFLICT("window",cohort) DO UPDATE SET amount_usd=EXCLUDED.amount_usd,
            event_count=EXCLUDED.event_count,share_pct=EXCLUDED.share_pct,updated_at=EXCLUDED.updated_at
    "#).bind(window).bind(cutoff).execute(pool).await?;
    Ok(())
}

pub async fn get<'e>(
    pool: impl sqlx::Executor<'e, Database = sqlx::Postgres>,
    window: &str,
) -> Result<Vec<FeeCohortStat>, sqlx::Error> {
    // Exactly three rows, including configured idle; no fee-event scan on GET.
    sqlx::query_as(r#"SELECT c.cohort,COALESCE(s.event_count,0) AS event_count,
        CASE WHEN s.cohort IS NULL THEN 0 ELSE s.amount_usd END AS amount_usd,s.share_pct
        FROM (VALUES ('retail',0),('mm',1),('unknown',2)) AS c(cohort,rank)
        LEFT JOIN protocol_fee_stats_by_cohort s ON s.cohort=c.cohort AND s."window"=$1 ORDER BY c.rank"#)
        .bind(window).fetch_all(pool).await
}
