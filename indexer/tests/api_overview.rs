mod common;

use axum_test::TestServer;
use serde_json::Value;

#[tokio::test]
async fn overview_returns_stats() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/overview").await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert!(body["total_volume_24h"].is_string());
    assert!(body.get("total_volume_24h_usd").is_some());
    assert!(body["total_trades_24h"].is_i64());
    assert!(body["pair_count"].is_i64());
    assert!(body["token_count"].is_i64());
    assert!(body["pair_count"].as_i64().unwrap() >= 1);
    assert!(body["token_count"].as_i64().unwrap() >= 2);
    assert!(body["total_volume_7d_usd"].is_string());
    assert!(body["total_volume_30d_usd"].is_string());
    assert!(body["tokens_added_30d"].is_i64());
    assert!(body["pairs_added_30d"].is_i64());
    assert!(body["active_pairs_24h"].is_i64());
    assert!(body["unique_traders_24h"].is_i64());
    assert!(body["total_trades_7d"].is_i64());
    assert!(body["total_trades_30d"].is_i64());
}
