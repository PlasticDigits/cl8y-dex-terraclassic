//! /overview global 24h stats: BRIN index + response cache (GitLab #281).

mod common;

use axum_test::TestServer;
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
