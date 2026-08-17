//! GitLab #548 — catalog `volume_usd` ingest, overview USD null, pair-leg token_count.

mod common;

use axum_test::TestServer;
use bigdecimal::BigDecimal;
use chrono::Utc;
use cl8y_dex_indexer::db::queries::{assets, volume};
use serial_test::serial;
use std::str::FromStr;

fn bd(s: &str) -> BigDecimal {
    BigDecimal::from_str(s).unwrap()
}

fn usd_f(v: &BigDecimal) -> f64 {
    use bigdecimal::ToPrimitive;
    v.to_f64().unwrap()
}

async fn insert_oracle(pool: &sqlx::PgPool, ticker: &str, price: &BigDecimal) {
    sqlx::query(
        "INSERT INTO oracle_prices (ticker, price_usd, source, fetched_at)
         VALUES ($1, $2, 'average', NOW())",
    )
    .bind(ticker)
    .bind(price)
    .execute(pool)
    .await
    .expect("oracle");
}

#[serial]
#[tokio::test]
async fn i1_ust1_ustr_volume_usd_and_overview_sum() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let ust1: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1ust1i1', true, 'UST1', 'UST1', 6) RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let ustr: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1ustri1', true, 'USTR', 'USTR', 18) RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let pair_id: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1pairi1', $1, $2, 'terra1lpi1', 30) RETURNING id",
    )
    .bind(ust1)
    .bind(ustr)
    .fetch_one(&pool)
    .await
    .unwrap();

    let ustc = bd("0.004878");
    insert_oracle(&pool, "ustc", &ustc).await;

    let offer = bd("10000000000000000000"); // 10 human USTR
    let ret = bd("1000000");
    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
         VALUES ($1, 5481, $2, 'tx548i1', 'terra1t', $3, $4, $5, $6, 1)",
    )
    .bind(pair_id)
    .bind(Utc::now())
    .bind(ustr)
    .bind(ust1)
    .bind(&offer)
    .bind(&ret)
    .execute(&pool)
    .await
    .unwrap();

    volume::backfill_swap_volume_usd(&pool)
        .await
        .expect("backfill");
    let first: Option<BigDecimal> =
        sqlx::query_scalar("SELECT volume_usd FROM swap_events WHERE tx_hash = 'tx548i1'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let first = first.expect("priced");
    let expected = 10.0 * 2.5 * 0.004878;
    assert!(
        (usd_f(&first) - expected).abs() < 1e-8,
        "got {}",
        usd_f(&first)
    );

    volume::backfill_swap_volume_usd(&pool)
        .await
        .expect("second backfill");
    let second: Option<BigDecimal> =
        sqlx::query_scalar("SELECT volume_usd FROM swap_events WHERE tx_hash = 'tx548i1'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(first.normalized(), second.unwrap().normalized(), "I13/A13");

    volume::refresh_global_stats(&pool).await.unwrap();
    let rollup = volume::get_global_stats(&pool).await.unwrap();
    let live = volume::get_global_stats_live(&pool).await.unwrap();
    assert_eq!(
        rollup.total_volume_24h_usd.normalized(),
        live.total_volume_24h_usd.normalized()
    );
    assert_eq!(rollup.total_trades_24h, 1);
    assert!((usd_f(&rollup.total_volume_24h_usd) - expected).abs() < 1e-8);
}

#[serial]
#[tokio::test]
async fn i9_token_count_is_pair_legs_not_orphan_or_lp() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;

    sqlx::query(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1orphan548', true, 'Orphan', 'ORPH', 6)",
    )
    .execute(&pool)
    .await
    .unwrap();

    let all = assets::get_all_assets(&pool).await.unwrap().len() as i64;
    let legs = assets::count_pair_leg_assets(&pool).await.unwrap();
    assert!(all > legs, "orphan must not increment token_count");
    assert_eq!(legs, 2, "seed pair has two legs");

    cl8y_dex_indexer::api::reset_overview_cache();
    let app = common::build_test_app(pool.clone()).await;
    let server = TestServer::new(app);
    let body: serde_json::Value = server.get("/api/v1/overview").await.json();
    assert_eq!(body["token_count"].as_i64().unwrap(), 2);
    assert_eq!(body["pair_count"].as_i64().unwrap(), 1);
    let _ = seed;
}

#[serial]
#[tokio::test]
async fn a2_overview_usd_is_swap_volume_not_fills() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let ust1: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1ust1a2', true, 'UST1', 'UST1', 6) RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let ustr: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1ustra2', true, 'USTR', 'USTR', 18) RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let pair_id: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1paira2', $1, $2, 'terra1lpa2', 30) RETURNING id",
    )
    .bind(ust1)
    .bind(ustr)
    .fetch_one(&pool)
    .await
    .unwrap();

    let vol = bd("1.25");
    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price, volume_usd)
         VALUES ($1, 5482, $2, 'tx548a2', 'terra1t', $3, $4, 1, 1, 1, $5)",
    )
    .bind(pair_id)
    .bind(Utc::now())
    .bind(ustr)
    .bind(ust1)
    .bind(&vol)
    .execute(&pool)
    .await
    .unwrap();

    let swap_id: i64 = sqlx::query_scalar("SELECT id FROM swap_events WHERE tx_hash = 'tx548a2'")
        .fetch_one(&pool)
        .await
        .unwrap();
    sqlx::query(
        "INSERT INTO limit_order_fills
         (pair_id, swap_event_id, block_height, block_timestamp, tx_hash, order_id, side, maker, price, token0_amount, token1_amount, commission_amount)
         VALUES ($1, $2, 5482, NOW(), 'tx548a2fill', 99, 'bid', 'terra1m', 1.0, 999999, 999999, 0)",
    )
    .bind(pair_id)
    .bind(swap_id)
    .execute(&pool)
    .await
    .unwrap();

    volume::refresh_global_stats(&pool).await.unwrap();
    let stats = volume::get_global_stats(&pool).await.unwrap();
    assert_eq!(stats.total_trades_24h, 1, "C5: fills are not trades");
    assert_eq!(stats.total_volume_24h_usd.normalized(), vol.normalized());
}

#[serial]
#[tokio::test]
async fn i5_unpriced_swaps_overview_usd_is_null() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let gem_a: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1gema', true, 'GEM', 'GEMX', 6) RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let gem_b: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1gemb', true, 'GEM', 'GEMY', 6) RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let pair_id: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1pairgem', $1, $2, 'terra1lpgem', 30) RETURNING id",
    )
    .bind(gem_a)
    .bind(gem_b)
    .fetch_one(&pool)
    .await
    .unwrap();

    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
         VALUES ($1, 5485, $2, 'tx548i5', 'terra1t', $3, $4, 1000000, 1000000, 1)",
    )
    .bind(pair_id)
    .bind(Utc::now())
    .bind(gem_a)
    .bind(gem_b)
    .execute(&pool)
    .await
    .unwrap();

    volume::backfill_swap_volume_usd(&pool).await.unwrap();
    let vol: Option<BigDecimal> =
        sqlx::query_scalar("SELECT volume_usd FROM swap_events WHERE tx_hash = 'tx548i5'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert!(vol.is_none());

    volume::refresh_global_stats(&pool).await.unwrap();
    cl8y_dex_indexer::api::reset_overview_cache();
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let body: serde_json::Value = server.get("/api/v1/overview").await.json();
    assert_eq!(body["total_trades_24h"].as_i64().unwrap(), 1);
    assert!(body["total_volume_24h_usd"].is_null(), "C3 unpriced → null");
    assert!(body["total_volume_24h"].is_string());
}

#[serial]
#[tokio::test]
async fn i11_ustc_price_usd_from_oracle_cache_never_lunc() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    cl8y_dex_indexer::api::reset_overview_cache();
    let ustc = bd("0.004878");
    let lunc = bd("0.00005");
    let app = common::build_test_app_with_prices(pool, Some(ustc.clone()), Some(lunc)).await;
    let server = TestServer::new(app);
    let body: serde_json::Value = server.get("/api/v1/overview").await.json();
    let shown = body["ustc_price_usd"].as_str().expect("ustc present");
    assert!(shown.starts_with("0.004878"));
    assert!(!shown.contains("0.00005"));

    cl8y_dex_indexer::api::reset_overview_cache();
    let pool2 = common::setup_pool().await;
    common::seed_db(&pool2).await;
    let app2 = common::build_test_app(pool2).await;
    let server2 = TestServer::new(app2);
    let body2: serde_json::Value = server2.get("/api/v1/overview").await.json();
    assert!(body2["ustc_price_usd"].is_null());
}

/// Coolify crash: raw 18-decimal SUM(offer_amount) into NUMERIC(38,18) overflows at 10^20.
#[serial]
#[tokio::test]
async fn raw_18_decimal_volume_does_not_overflow_global_stats() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;

    let ust1: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1ust1ovf', true, 'UST1', 'UST1', 6) RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let ustr: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1ustrovf', true, 'USTR', 'USTR', 18) RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .unwrap();
    let pair_id: i32 = sqlx::query_scalar(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1pairovf', $1, $2, 'terra1lpovf', 30) RETURNING id",
    )
    .bind(ust1)
    .bind(ustr)
    .fetch_one(&pool)
    .await
    .unwrap();

    insert_oracle(&pool, "ustc", &bd("0.004878")).await;

    // 1000 human USTR = 10^21 raw — overflows NUMERIC(38,18) integer width (10^20).
    let offer = bd("1000000000000000000000");
    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
         VALUES ($1, 5483, $2, 'tx548ovf', 'terra1t', $3, $4, $5, 1000000, 1)",
    )
    .bind(pair_id)
    .bind(Utc::now())
    .bind(ustr)
    .bind(ust1)
    .bind(&offer)
    .execute(&pool)
    .await
    .unwrap();

    volume::backfill_swap_volume_usd(&pool)
        .await
        .expect("backfill must not numeric-overflow");
    volume::refresh_global_stats(&pool)
        .await
        .expect("rollup must not numeric-overflow");
    volume::refresh_pair_volumes(&pool)
        .await
        .expect("pair rollup must not numeric-overflow");

    let total: BigDecimal =
        sqlx::query_scalar("SELECT total_volume FROM global_stats_24h WHERE id = 1")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(total.normalized(), offer.normalized());

    let vol: Option<BigDecimal> =
        sqlx::query_scalar("SELECT volume_usd FROM swap_events WHERE tx_hash = 'tx548ovf'")
            .fetch_one(&pool)
            .await
            .unwrap();
    let vol = vol.expect("priced USTR");
    let expected = 1000.0 * 2.5 * 0.004878;
    assert!(
        (usd_f(&vol) - expected).abs() < 1e-6,
        "got {}",
        usd_f(&vol)
    );
}
