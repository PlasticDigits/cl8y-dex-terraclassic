//! /overview global 24h stats: rollup table + BRIN index + response cache (GitLab #281, #333).

mod common;

use axum_test::TestServer;
use chrono::{Duration, Utc};
use cl8y_dex_indexer::db::queries::volume;
use common::{seed_db, setup_pool};
use serde_json::Value;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn swap_events_block_timestamp_brin_index_exists() {
    let pool = setup_pool().await;

    let exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (
           SELECT 1 FROM pg_indexes
           WHERE tablename = 'swap_events'
             AND indexname = 'idx_swaps_block_timestamp_brin'
         )",
    )
    .fetch_one(&pool)
    .await
    .expect("brin index exists query");

    assert!(
        exists,
        "migration must create idx_swaps_block_timestamp_brin on swap_events.block_timestamp (#281)"
    );
}

#[serial]
#[tokio::test]
async fn overview_response_cached_within_ttl() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let first = server.get("/api/v1/overview").await;
    first.assert_status_ok();
    let body1: Value = first.json();

    let second = server.get("/api/v1/overview").await;
    second.assert_status_ok();
    let body2: Value = second.json();

    assert_eq!(
        body1, body2,
        "back-to-back /overview hits within the 60s TTL must return identical JSON (#281 cache)"
    );
}

#[serial]
#[tokio::test]
async fn global_stats_rollup_matches_live_query() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;

    volume::refresh_global_stats(&pool)
        .await
        .expect("refresh global stats");

    let rollup = volume::get_global_stats(&pool)
        .await
        .expect("rollup stats");
    let live = volume::get_global_stats_live(&pool)
        .await
        .expect("live stats");

    assert_eq!(
        rollup.total_volume_24h.normalized(),
        live.total_volume_24h.normalized()
    );
    assert_eq!(
        rollup.total_volume_24h_usd.normalized(),
        live.total_volume_24h_usd.normalized()
    );
    assert_eq!(rollup.total_trades_24h, live.total_trades_24h);
    assert_eq!(rollup.pair_count, live.pair_count);
    assert_eq!(rollup.total_trades_24h, 5, "seed inserts 5 swaps within 24h");
    assert_eq!(rollup.pair_count, 1);
    assert!(seed.pair_id >= 1);
}

#[serial]
#[tokio::test]
async fn global_stats_rollup_excludes_swaps_older_than_24h() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;

    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
         VALUES ($1, 999, $2, 'txold', $3, $4, $5, 9999, 9000, 0.9)",
    )
    .bind(seed.pair_id)
    .bind(Utc::now() - Duration::hours(48))
    .bind(&seed.trader_address)
    .bind(seed.asset_0_id)
    .bind(seed.asset_1_id)
    .execute(&pool)
    .await
    .expect("insert old swap");

    volume::refresh_global_stats(&pool)
        .await
        .expect("refresh global stats");

    let rollup = volume::get_global_stats(&pool)
        .await
        .expect("rollup stats");
    let live = volume::get_global_stats_live(&pool)
        .await
        .expect("live stats");

    assert_eq!(rollup.total_trades_24h, 5);
    assert_eq!(rollup.total_trades_24h, live.total_trades_24h);
}

#[serial]
#[tokio::test]
async fn global_stats_uninitialized_rollup_falls_back_to_live() {
    let pool = setup_pool().await;
    seed_db(&pool).await;

    // Post-migration seed row (zeros) before refresh_global_stats — API can serve /overview
    // while the indexer task is still in pair sync.
    sqlx::query(
        "UPDATE global_stats_24h
         SET total_volume = 0, total_volume_usd = 0, total_trades = 0",
    )
    .execute(&pool)
    .await
    .expect("reset rollup to migration seed");

    let stats = volume::get_global_stats(&pool)
        .await
        .expect("rollup read with fallback");
    let live = volume::get_global_stats_live(&pool)
        .await
        .expect("live stats");

    assert_eq!(
        stats.total_trades_24h, live.total_trades_24h,
        "must not serve seeded zeros when swap_events has 24h data"
    );
    assert_eq!(stats.total_trades_24h, 5);
    assert_eq!(
        stats.total_volume_24h.normalized(),
        live.total_volume_24h.normalized()
    );
    assert_eq!(
        stats.total_volume_24h_usd.normalized(),
        live.total_volume_24h_usd.normalized()
    );
}

#[serial]
#[tokio::test]
async fn global_stats_empty_db_returns_zeros() {
    let pool = setup_pool().await;
    common::clean_db(&pool).await;

    volume::refresh_global_stats(&pool)
        .await
        .expect("refresh global stats on empty db");

    let stats = volume::get_global_stats(&pool)
        .await
        .expect("rollup stats");

    assert_eq!(stats.total_trades_24h, 0);
    assert_eq!(stats.pair_count, 0);
    assert_eq!(stats.total_volume_24h.normalized(), bigdecimal::BigDecimal::from(0).normalized());
    assert_eq!(
        stats.total_volume_24h_usd.normalized(),
        bigdecimal::BigDecimal::from(0).normalized()
    );
}

#[serial]
#[tokio::test]
async fn global_stats_refresh_advances_updated_at() {
    let pool = setup_pool().await;
    seed_db(&pool).await;

    let before: chrono::DateTime<Utc> =
        sqlx::query_scalar("SELECT updated_at FROM global_stats_24h WHERE id = 1")
            .fetch_one(&pool)
            .await
            .expect("updated_at before");

    tokio::time::sleep(std::time::Duration::from_millis(20)).await;

    volume::refresh_global_stats(&pool)
        .await
        .expect("refresh global stats");

    let after: chrono::DateTime<Utc> =
        sqlx::query_scalar("SELECT updated_at FROM global_stats_24h WHERE id = 1")
            .fetch_one(&pool)
            .await
            .expect("updated_at after");

    assert!(
        after >= before,
        "aggregator refresh must advance or preserve global_stats_24h.updated_at"
    );
}

#[serial]
#[tokio::test]
async fn overview_cache_miss_reads_rollup_not_swap_events() {
    let pool = setup_pool().await;
    seed_db(&pool).await;

    let rows: Vec<(String,)> = sqlx::query_as(
        "EXPLAIN (FORMAT TEXT)
         SELECT total_volume, total_volume_usd, total_trades
         FROM global_stats_24h WHERE id = 1",
    )
    .fetch_all(&pool)
    .await
    .expect("explain global stats rollup read");

    let plan: String = rows
        .into_iter()
        .map(|(line,)| line)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        !plan.contains("swap_events"),
        "/overview cache-miss path must read global_stats_24h rollup, not scan swap_events:\n{plan}"
    );
    assert!(
        plan.contains("global_stats_24h"),
        "plan must reference global_stats_24h:\n{plan}"
    );
}
