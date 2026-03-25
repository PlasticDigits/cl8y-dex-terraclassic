mod common;

use axum_test::TestServer;
use serde_json::Value;
use serial_test::serial;

#[serial]
#[tokio::test]
async fn route_solve_returns_hops_and_pool_only_hybrid() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}",
        seed.token_a, seed.token_b
    );
    let resp = server.get(&url).await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    let hops = body["hops"].as_array().expect("hops");
    assert_eq!(hops.len(), 1);
    assert_eq!(hops[0]["pair"], "terra1pairrouteabc");
    assert_eq!(hops[0]["offer_token"], seed.token_a);
    assert_eq!(hops[0]["ask_token"], seed.token_b);

    let ops = body["router_operations"]
        .as_array()
        .expect("router_operations");
    assert_eq!(ops.len(), 1);
    assert!(ops[0]["terra_swap"]["hybrid"].is_null());
    assert!(body["estimated_amount_out"].is_null());
}

#[serial]
#[tokio::test]
async fn route_solve_unknown_token_returns_400() {
    let pool = common::setup_pool().await;
    common::seed_route_solve(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get("/api/v1/route/solve?token_in=terra1notindexed&token_out=terra1routesolvebbb")
        .await;
    resp.assert_status_bad_request();
}

#[serial]
#[tokio::test]
async fn route_solve_no_path_returns_404() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}",
        seed.token_a, seed.token_c
    );
    let resp = server.get(&url).await;
    resp.assert_status_not_found();
    let text = resp.text();
    assert!(
        text.contains("no route") || text.contains("4 hops"),
        "{}",
        text
    );
}
