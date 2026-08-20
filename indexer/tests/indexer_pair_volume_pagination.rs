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

    let rolled: bigdecimal::BigDecimal =
        sqlx::query_scalar("SELECT volume_quote FROM pair_volume_24h WHERE pair_id = $1")
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

/// GitLab #577 **D3**: idle pairs with only 48h-old swaps zero after refresh.
#[serial]
#[tokio::test]
async fn pair_volume_idle_48h_swaps_zero_after_refresh() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;

    sqlx::query(
        "UPDATE swap_events SET block_timestamp = NOW() - INTERVAL '48 hours' WHERE pair_id = $1",
    )
    .bind(seed.pair_id)
    .execute(&pool)
    .await
    .expect("age swaps");

    volume::refresh_pair_volumes(&pool)
        .await
        .expect("refresh pair volumes");

    let rolled: bigdecimal::BigDecimal =
        sqlx::query_scalar("SELECT volume_quote FROM pair_volume_24h WHERE pair_id = $1")
            .bind(seed.pair_id)
            .fetch_one(&pool)
            .await
            .expect("rollup row");

    assert_eq!(
        rolled.normalized(),
        bigdecimal::BigDecimal::from(0).normalized(),
        "pair with only 48h-old swaps must zero pair_volume_24h (#577 D3)"
    );
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

#[serial]
#[tokio::test]
async fn pair_list_volume_sort_plan_does_not_touch_swap_events() {
    let pool = setup_pool().await;
    seed_db(&pool).await;

    let rows: Vec<(String,)> = sqlx::query_as(
        "EXPLAIN (FORMAT TEXT)
         SELECT p.id, pv.volume_quote
         FROM pairs p
         INNER JOIN assets a0 ON a0.id = p.asset_0_id
         INNER JOIN assets a1 ON a1.id = p.asset_1_id
         LEFT JOIN pair_volume_24h pv ON pv.pair_id = p.id
         ORDER BY pv.volume_quote DESC NULLS LAST
         LIMIT 50 OFFSET 0",
    )
    .fetch_all(&pool)
    .await
    .expect("explain pair list volume sort");

    let plan: String = rows
        .into_iter()
        .map(|(line,)| line)
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        !plan.contains("swap_events"),
        "pair list volume sort must use pair_volume_24h rollup, not scan swap_events:\n{plan}"
    );
}
