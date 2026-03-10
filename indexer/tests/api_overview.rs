mod common;

use axum_test::TestServer;
use serde_json::Value;

#[tokio::test]
async fn overview_returns_stats() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool);
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/overview").await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert!(body["total_volume_24h"].is_string());
    assert!(body["total_trades_24h"].is_i64());
    assert!(body["pair_count"].is_i64());
    assert!(body["token_count"].is_i64());
    assert!(body["pair_count"].as_i64().unwrap() >= 1);
    assert!(body["token_count"].as_i64().unwrap() >= 2);
}
