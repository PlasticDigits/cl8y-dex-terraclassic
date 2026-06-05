//! GitLab #319: DB-backed hybrid route solver (Phase 1c).

mod common;

use axum_test::TestServer;
use common::lcd_mock;
use serde_json::Value;
use serial_test::serial;

fn db_hybrid_config(mock: &wiremock::MockServer) -> cl8y_dex_indexer::config::Config {
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![lcd_mock::lcd_base_url(mock)];
    cfg.router_address = Some("terra1routertest".to_string());
    cfg.route_solver_db_hybrid = true;
    cfg
}

#[serial]
#[tokio::test]
async fn route_solve_db_hybrid_no_pair_level_lcd_calls() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_with_mirror(&pool).await;
    let (mock, hybrid_hits) = lcd_mock::start_router_only_route_mock("8888888").await;
    let app = common::build_test_app_with_price_and_config(pool, None, db_hybrid_config(&mock)).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1000000",
        seed.token_a, seed.token_b
    );
    let resp = server.get(&url).await;
    resp.assert_status_ok();
    let j: Value = resp.json();
    assert_eq!(j["solver_version"], "global_v4");
    let kind = j["quote_kind"].as_str().unwrap_or("");
    assert!(
        kind == "indexer_pool_db" || kind == "indexer_hybrid_db",
        "unexpected quote_kind: {kind} notes={:?}",
        j["hybrid_notes"]
    );
    assert_eq!(j["fidelity_check"], "passed");
    assert_eq!(hybrid_hits.load(std::sync::atomic::Ordering::SeqCst), 0);
    assert!(j["db_hybrid_queries"].as_u64().unwrap_or(0) > 0);
}

#[serial]
#[tokio::test]
async fn route_solve_db_hybrid_fidelity_drift_downgrades() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_with_mirror(&pool).await;
    // Router sim returns far less than mirror grid → drift guard.
    let (mock, _) = lcd_mock::start_router_only_route_mock("1").await;
    let mut cfg = db_hybrid_config(&mock);
    cfg.route_fidelity_drift_bps = 1;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in=2000000",
        seed.token_a, seed.token_b
    );
    let j: Value = server.get(&url).await.json();
    assert_eq!(j["fidelity_check"], "drift");
    assert_eq!(j["quote_kind"], "indexer_hybrid_db_degraded");
    assert_eq!(j["estimated_amount_out"], "1");
}
