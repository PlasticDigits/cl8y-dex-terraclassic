use std::str::FromStr;

use bigdecimal::BigDecimal;
use chrono::{DateTime, Duration, Utc};
use rand::Rng;
use sqlx::PgPool;

use crate::db::queries::candles;

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const INTERVALS: &[&str] = &["1m", "5m", "15m", "1h", "4h", "1d", "1w"];
const SEED_SENDER: &str = "terra1seedqa000000000000000000000000000000000";
const TX_PREFIX: &str = "SEEDQA_";

pub struct SeedQaConfig {
    pub span_weeks: u32,
    pub swaps_per_day: u32,
}

impl Default for SeedQaConfig {
    fn default() -> Self {
        Self {
            span_weeks: 4,
            swaps_per_day: 24,
        }
    }
}

pub async fn run(pool: &PgPool, config: SeedQaConfig) -> Result<(), BoxError> {
    #[derive(sqlx::FromRow)]
    struct PairRow {
        id: i32,
        asset_0_id: i32,
        asset_1_id: i32,
    }

    let pairs: Vec<PairRow> = sqlx::query_as("SELECT id, asset_0_id, asset_1_id FROM pairs")
        .fetch_all(pool)
        .await?;

    if pairs.is_empty() {
        return Err("No pairs found. Deploy contracts first with `make deploy-local`.".into());
    }

    tracing::info!(
        "Seeding QA data: {} week(s), {} swaps/pair/day across {} pair(s)",
        config.span_weeks,
        config.swaps_per_day,
        pairs.len(),
    );

    let now = Utc::now();
    let span = Duration::weeks(config.span_weeks as i64);
    let start = now - span;
    let total_days = config.span_weeks * 7;
    let total_swaps = total_days * config.swaps_per_day;
    let step_secs = span.num_seconds() / total_swaps as i64;

    let mut rng = rand::thread_rng();

    for pair in &pairs {
        let mut price: f64 = 0.3 + rng.gen::<f64>() * 1.7;
        let mut block_height: i64 = 900_000;

        for i in 0..total_swaps {
            let timestamp: DateTime<Utc> = start
                + Duration::seconds(step_secs * i as i64)
                + Duration::seconds(rng.gen_range(0..step_secs.max(1)));

            // Random walk: ±3% per step with slight upward drift
            let pct_change = (rng.gen::<f64>() - 0.48) * 0.06;
            price = (price * (1.0 + pct_change)).clamp(0.001, 999.0);

            let offer_amount: i64 = rng.gen_range(500..20_000);
            let return_amount: i64 = (offer_amount as f64 * price).max(1.0) as i64;

            let price_bd = BigDecimal::from_str(&format!("{:.18}", price))?;
            let offer_bd = BigDecimal::from(offer_amount);
            let return_bd = BigDecimal::from(return_amount);

            let tx_hash = format!("{}{}_{}_{}", TX_PREFIX, pair.id, i, timestamp.timestamp());

            sqlx::query(
                "INSERT INTO swap_events
                 (pair_id, block_height, block_timestamp, tx_hash, sender,
                  offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            )
            .bind(pair.id)
            .bind(block_height)
            .bind(timestamp)
            .bind(&tx_hash)
            .bind(SEED_SENDER)
            .bind(pair.asset_0_id)
            .bind(pair.asset_1_id)
            .bind(&offer_bd)
            .bind(&return_bd)
            .bind(&price_bd)
            .execute(pool)
            .await?;

            block_height += 1;
        }

        for &interval in INTERVALS {
            candles::rebuild_candles_from_swaps(pool, pair.id, interval, start).await?;
        }

        tracing::info!(
            "  pair {} — {} swap events inserted, candles rebuilt",
            pair.id,
            total_swaps,
        );
    }

    tracing::info!(
        "QA seed complete. Total swap events: {}",
        total_swaps as usize * pairs.len()
    );
    Ok(())
}

pub async fn clean(pool: &PgPool) -> Result<(), BoxError> {
    let deleted_swaps = sqlx::query_scalar::<_, i64>(&format!(
        "WITH d AS (DELETE FROM swap_events WHERE tx_hash LIKE '{}%' RETURNING 1) SELECT COUNT(*) FROM d",
        TX_PREFIX,
    ))
    .fetch_one(pool)
    .await?;

    tracing::info!("Removed {} seeded swap events", deleted_swaps);

    // Rebuild candles from remaining (real) swaps
    #[derive(sqlx::FromRow)]
    struct PairId {
        id: i32,
    }

    let pairs: Vec<PairId> = sqlx::query_as("SELECT id FROM pairs")
        .fetch_all(pool)
        .await?;

    sqlx::query("DELETE FROM candles").execute(pool).await?;

    let earliest: Option<DateTime<Utc>> =
        sqlx::query_scalar("SELECT MIN(block_timestamp) FROM swap_events")
            .fetch_one(pool)
            .await?;

    if let Some(from) = earliest {
        for pair in &pairs {
            for &interval in INTERVALS {
                candles::rebuild_candles_from_swaps(pool, pair.id, interval, from).await?;
            }
        }
        tracing::info!("Candles rebuilt from remaining swap data");
    } else {
        tracing::info!("No swap events remain; candles table cleared");
    }

    Ok(())
}
