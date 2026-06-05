mod common;

use axum_test::TestServer;
use serde_json::Value;
use sqlx::PgPool;

async fn insert_hook(pool: &PgPool, hook_addr: &str, action: &str) {
    sqlx::query(
        "INSERT INTO hook_events (tx_hash, hook_address, action, block_height, block_time)
         VALUES ($1, $2, $3, $4, NOW())",
    )
    .bind(format!("tx_{}", hook_addr))
    .bind(hook_addr)
    .bind(action)
    .bind(1000i64)
    .execute(pool)
    .await
    .expect("insert hook");
}

#[tokio::test]
async fn hooks_list_returns_rows() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    insert_hook(&pool, "terra1hookaddr", "after_swap_burn").await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/hooks").await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert!(!body.is_empty());
    assert_eq!(body[0]["hook_address"], "terra1hookaddr");
    assert_eq!(body[0]["action"], "after_swap_burn");
}

#[tokio::test]
async fn hooks_filter_by_address() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    insert_hook(&pool, "terra1hooka", "after_swap_burn").await;
    insert_hook(&pool, "terra1hookb", "after_swap_tax").await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/hooks?hook_address=terra1hooka").await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert_eq!(body.len(), 1);
    assert_eq!(body[0]["hook_address"], "terra1hooka");
}

#[tokio::test]
async fn hooks_limit_capped_at_200() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    for i in 0..5 {
        insert_hook(&pool, &format!("terra1hook{}", i), "after_swap_burn").await;
    }

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/hooks?limit=9999").await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert!(body.len() <= 200);
}

#[tokio::test]
async fn hooks_negative_and_zero_limit_clamp_to_one_not_500() {
    // GitLab #284: a negative/zero `limit` must clamp to >=1, never reach Postgres as a
    // negative LIMIT (which 500s). The .min()->.clamp(1,MAX) sweep missed hooks.rs, so this
    // endpoint regressed. Status + the <=1 row bound are race-safe under the shared test DB
    // (the clamp guarantees at most one row regardless of what siblings seed/truncate).
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;

    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    for q in ["/api/v1/hooks?limit=-1", "/api/v1/hooks?limit=0"] {
        let resp = server.get(q).await;
        resp.assert_status_ok();
        let body: Vec<Value> = resp.json();
        assert!(body.len() <= 1, "{q} must clamp limit to 1, got {}", body.len());
    }
}
