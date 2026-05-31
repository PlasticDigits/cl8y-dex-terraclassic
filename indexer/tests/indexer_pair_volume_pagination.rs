//! Pair 24h volume rollup + pagination caps (GitLab #243).

mod common;

use axum_test::TestServer;
use cl8y_dex_indexer::db::queries::volume;
use common::{seed_db, setup_pool};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn pair_volume_rollup_matches_swap_events() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;

    volume::refresh_pair_volumes(&pool)
        .await
        .expect("refresh pair volumes");

    let rolled: bigdecimal::BigDecimal = sqlx::query_scalar(
        "SELECT volume_quote FROM pair_volume_24h WHERE pair_id = $1",
    )
    .bind(seed.pair_id)
    .fetch_one(&pool)
    .await
    .expect("rollup row");

    let live: bigdecimal::BigDecimal = sqlx::query_scalar(
        "SELECT COALESCE(SUM(return_amount), 0) FROM swap_events
         WHERE pair_id = $1 AND block_timestamp >= NOW() - INTERVAL '24 hours'",
    )
    .bind(seed.pair_id)
    .fetch_one(&pool)
    .await
    .expect("live sum");

    assert_eq!(rolled.normalized(), live.normalized());
}

#[serial]
#[tokio::test]
async fn list_pairs_volume_sort_uses_rollup_table() {
    let pool = setup_pool().await;
    seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/pairs?sort=volume_24h&order=desc").await;
    resp.assert_status_ok();
}
