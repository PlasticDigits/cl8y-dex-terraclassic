mod common;

use bigdecimal::BigDecimal;
use chrono::Utc;
use cl8y_dex_indexer::indexer::candle_builder;
use cl8y_dex_indexer::indexer::pair_price_usd::{CandleUsdLeg, CandleUsdSubject};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn candle_update_skipped_for_zero_price() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;

    let before: i64 = sqlx::query_scalar("SELECT COUNT(*)::bigint FROM candles WHERE pair_id = $1")
        .bind(seed.pair_id)
        .fetch_one(&pool)
        .await
        .expect("count candles");

    let zero = BigDecimal::from(0);
    candle_builder::update_candles_for_swap(
        &pool,
        seed.pair_id,
        Utc::now(),
        &CandleUsdSubject::Usd {
            leg: CandleUsdLeg::Asset0,
            usd: zero,
        },
        &BigDecimal::from(1),
        &BigDecimal::from(1000),
        &BigDecimal::from(950),
    )
    .await
    .expect("update candles");

    let after: i64 = sqlx::query_scalar("SELECT COUNT(*)::bigint FROM candles WHERE pair_id = $1")
        .bind(seed.pair_id)
        .fetch_one(&pool)
        .await
        .expect("count candles");

    assert_eq!(before, after, "zero price must not insert/update candles");
}

#[serial]
#[tokio::test]
async fn candle_update_skipped_when_price_usd_missing() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;

    let before: i64 = sqlx::query_scalar("SELECT COUNT(*)::bigint FROM candles WHERE pair_id = $1")
        .bind(seed.pair_id)
        .fetch_one(&pool)
        .await
        .expect("count candles");

    candle_builder::update_candles_for_swap(
        &pool,
        seed.pair_id,
        Utc::now(),
        &CandleUsdSubject::Skip,
        &BigDecimal::from(21260),
        &BigDecimal::from(1000),
        &BigDecimal::from(950),
    )
    .await
    .expect("update candles");

    let after: i64 = sqlx::query_scalar("SELECT COUNT(*)::bigint FROM candles WHERE pair_id = $1")
        .bind(seed.pair_id)
        .fetch_one(&pool)
        .await
        .expect("count candles");

    assert_eq!(
        before, after,
        "missing price_usd must not write human quote-per-base into USD candles"
    );
}
