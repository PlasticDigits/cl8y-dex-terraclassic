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

    let rollup = volume::get_global_stats(&pool).await.expect("rollup stats");
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
    assert_eq!(
        rollup.total_trades_24h, 5,
        "seed inserts 5 swaps within 24h"
    );
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

    let rollup = volume::get_global_stats(&pool).await.expect("rollup stats");
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

    let stats = volume::get_global_stats(&pool).await.expect("rollup stats");

    assert_eq!(stats.total_trades_24h, 0);
    assert_eq!(stats.pair_count, 0);
    assert_eq!(
        stats.total_volume_24h.normalized(),
        bigdecimal::BigDecimal::from(0).normalized()
    );
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

#[serial]
#[tokio::test]
async fn overview_cache_miss_window_read_does_not_scan_swap_events() {
    let pool = setup_pool().await;
    seed_db(&pool).await;

    let rows: Vec<(String,)> = sqlx::query_as(
        "EXPLAIN (FORMAT TEXT)
         SELECT total_volume, total_volume_usd, total_trades,
                total_volume_7d_usd, total_volume_30d_usd,
                total_trades_7d, total_trades_30d,
                active_pairs_24h, unique_traders_24h,
                total_liquidity_usd, liquidity_change_24h_pct, liquidity_change_30d_pct,
                priced_pair_count, unpriced_pair_count
         FROM global_stats_24h WHERE id = 1",
    )
    .fetch_all(&pool)
    .await
    .expect("explain #550 rollup read");

    let plan: String = rows
        .into_iter()
        .map(|(line,)| line)
        .collect::<Vec<_>>()
        .join("\n");

    assert!(
        !plan.contains("swap_events"),
        "/overview cache-miss must not scan swap_events for 7d/30d:\n{plan}"
    );
}

#[serial]
#[tokio::test]
async fn listing_census_counts_do_not_scan_swap_events() {
    let pool = setup_pool().await;
    seed_db(&pool).await;

    for sql in [
        "EXPLAIN (FORMAT TEXT) SELECT COUNT(*) FROM assets",
        "EXPLAIN (FORMAT TEXT) SELECT COUNT(*) FROM assets WHERE created_at >= NOW() - INTERVAL '30 days'",
        "EXPLAIN (FORMAT TEXT) SELECT COUNT(*) FROM pairs WHERE created_at >= NOW() - INTERVAL '30 days'",
    ] {
        let rows: Vec<(String,)> = sqlx::query_as(sql).fetch_all(&pool).await.expect(sql);
        let plan: String = rows.into_iter().map(|(line,)| line).collect::<Vec<_>>().join("\n");
        assert!(
            !plan.contains("swap_events"),
            "census query must not touch swap_events:\n{sql}\n{plan}"
        );
    }
}

#[serial]
#[tokio::test]
async fn global_stats_windows_respect_cutoffs() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;

    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price, volume_usd)
         VALUES ($1, 2001, $2, 'tx8d', $3, $4, $5, 100, 90, 0.9, 80)",
    )
    .bind(seed.pair_id)
    .bind(Utc::now() - Duration::days(8))
    .bind(&seed.trader_address)
    .bind(seed.asset_0_id)
    .bind(seed.asset_1_id)
    .execute(&pool)
    .await
    .expect("insert 8d swap");

    sqlx::query(
        "INSERT INTO swap_events
         (pair_id, block_height, block_timestamp, tx_hash, sender,
          offer_asset_id, ask_asset_id, offer_amount, return_amount, price, volume_usd)
         VALUES ($1, 2002, $2, 'tx25h', $3, $4, $5, 50, 45, 0.9, 40)",
    )
    .bind(seed.pair_id)
    .bind(Utc::now() - Duration::hours(25))
    .bind(&seed.trader_address)
    .bind(seed.asset_0_id)
    .bind(seed.asset_1_id)
    .execute(&pool)
    .await
    .expect("insert 25h swap");

    volume::refresh_global_stats(&pool).await.expect("refresh");

    let stats = volume::get_global_stats(&pool).await.expect("stats");
    assert_eq!(stats.total_trades_24h, 5, "25h swap excluded from 24h");
    assert_eq!(
        stats.total_trades_7d, 6,
        "25h swap in 7d; 8d swap not in 7d"
    );
    assert_eq!(
        stats.total_trades_30d, 7,
        "8d swap in 30d along with 24h+25h"
    );
}

#[serial]
#[tokio::test]
async fn global_stats_active_pairs_excludes_idle() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;

    let idle_asset: i32 = sqlx::query_scalar(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals)
         VALUES ('terra1idleasset', true, 'Idle', 'IDLE', 6)
         RETURNING id",
    )
    .fetch_one(&pool)
    .await
    .expect("idle asset");

    sqlx::query(
        "INSERT INTO pairs (contract_address, asset_0_id, asset_1_id, lp_token, fee_bps)
         VALUES ('terra1idlepair', $1, $2, 'terra1idlelp', 30)",
    )
    .bind(seed.asset_0_id)
    .bind(idle_asset)
    .execute(&pool)
    .await
    .expect("idle pair");

    volume::refresh_global_stats(&pool).await.expect("refresh");

    let stats = volume::get_global_stats(&pool).await.expect("stats");
    assert_eq!(stats.pair_count, 2);
    assert_eq!(stats.active_pairs_24h, 1, "idle pair has no 24h swaps");
    assert_eq!(stats.unique_traders_24h, 1);
}

#[serial]
#[tokio::test]
async fn tokens_and_pairs_added_30d_use_created_at() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;

    sqlx::query("UPDATE assets SET created_at = NOW() - INTERVAL '40 days' WHERE id = $1")
        .bind(seed.asset_0_id)
        .execute(&pool)
        .await
        .expect("age asset");

    sqlx::query("UPDATE pairs SET created_at = NOW() - INTERVAL '40 days' WHERE id = $1")
        .bind(seed.pair_id)
        .execute(&pool)
        .await
        .expect("age pair");

    let cutoff = Utc::now() - Duration::days(30);
    let tokens_new =
        cl8y_dex_indexer::db::queries::assets::count_assets_created_since(&pool, cutoff)
            .await
            .expect("tokens 30d");
    let pairs_new = cl8y_dex_indexer::db::queries::pairs::count_pairs_created_since(&pool, cutoff)
        .await
        .expect("pairs 30d");
    let token_count = cl8y_dex_indexer::db::queries::assets::count_assets(&pool)
        .await
        .expect("token count");

    assert_eq!(token_count, 2);
    assert_eq!(tokens_new, 1, "one asset still inside 30d");
    assert_eq!(pairs_new, 0, "only pair aged out of 30d");
}

#[serial]
#[tokio::test]
async fn overview_api_exposes_additive_fields_and_pair_leg_tokens() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    volume::refresh_global_stats(&pool).await.expect("refresh");

    let app = common::build_test_app(pool.clone()).await;
    let server = TestServer::new(app);
    let resp = server.get("/api/v1/overview").await;
    resp.assert_status_ok();
    let body: Value = resp.json();

    assert!(body["total_volume_24h"].is_string());
    assert!(
        body["total_volume_24h_usd"].is_null() || body["total_volume_24h_usd"].is_string(),
        "unpriced 24h activity is JSON null (#548 C3); priced/idle is a decimal string"
    );
    assert!(body["total_trades_24h"].is_i64());
    assert!(body["pair_count"].is_i64());
    assert!(body["token_count"].is_i64());
    assert_eq!(body["token_count"].as_i64().unwrap(), 2);
    assert!(body["total_volume_7d_usd"].is_string());
    assert!(body["total_volume_30d_usd"].is_string());
    assert!(body["tokens_added_30d"].as_i64().unwrap() >= 2);
    assert!(body["pairs_added_30d"].as_i64().unwrap() >= 1);
    assert_eq!(body["active_pairs_24h"].as_i64().unwrap(), 1);
    assert!(body["total_liquidity_usd"].is_string());
    assert!(body.get("liquidity_change_24h_pct").is_some());
}

#[serial]
#[tokio::test]
async fn global_stats_empty_db_window_fields_are_zero() {
    let pool = setup_pool().await;
    common::clean_db(&pool).await;

    volume::refresh_global_stats(&pool)
        .await
        .expect("refresh empty");

    let stats = volume::get_global_stats(&pool).await.expect("stats");
    assert_eq!(stats.total_trades_7d, 0);
    assert_eq!(stats.total_trades_30d, 0);
    assert_eq!(stats.active_pairs_24h, 0);
    assert_eq!(stats.unique_traders_24h, 0);
    assert_eq!(
        stats.total_volume_7d_usd.normalized(),
        bigdecimal::BigDecimal::from(0).normalized()
    );
    assert_eq!(
        stats.total_volume_30d_usd.normalized(),
        bigdecimal::BigDecimal::from(0).normalized()
    );
}
