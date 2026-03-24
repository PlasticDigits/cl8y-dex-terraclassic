mod common;

use axum_test::TestServer;
use serde_json::Value;

#[tokio::test]
async fn health_returns_ok() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/health").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(body["status"], "ok");
}
