mod common;

use axum_test::TestServer;
use cl8y_dex_indexer::api::{build_router, AppState};
use cl8y_dex_indexer::config::Config;
use cl8y_dex_indexer::indexer::fee_discount_registry_health::FeeDiscountRegistryHealth;
use cl8y_dex_indexer::lcd::LcdClient;
use serde_json::Value;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn build_app_with_fee_discount_health(
    pool: sqlx::PgPool,
    lcd_urls: Vec<String>,
    health: FeeDiscountRegistryHealth,
) -> axum::Router {
    let config = common::test_config();
    let lcd = LcdClient::new(lcd_urls, config.lcd_timeout_ms, config.lcd_cooldown_ms);
    let state = AppState {
        pool,
        lcd,
        ustc_price: cl8y_dex_indexer::indexer::oracle::new_shared_price(),
        ticker_map_cache: cl8y_dex_indexer::api::TickerMapCache::default(),
        orderbook_cache: cl8y_dex_indexer::api::orderbook_sim::OrderbookCache::default(),
        router_address: None,
        factory_address: Some(config.factory_address.clone()),
        fee_discount_address: Some("terra1feediscount".to_string()),
        fee_discount_registry_health: health,
        route_solver_db_hybrid: false,
        book_snapshot_max_staleness_ms: config.book_snapshot_max_staleness_ms(),
        route_fidelity_drift_bps: config.route_fidelity_drift_bps,
    };
    build_router(state, &Config {
        fee_discount_address: Some("terra1feediscount".to_string()),
        ..config
    })
}

#[tokio::test]
async fn fee_discount_health_unconfigured_returns_null_ok() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/health/fee-discount").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(body["configured"], false);
    assert!(body["fee_discount_registry_ok"].is_null());
    assert_eq!(body["consecutive_lcd_failures"], 0);
}

#[tokio::test]
async fn fee_discount_health_reflects_probe_state_without_lcd_stack_traces() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;

    let lcd_server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/.+$"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&lcd_server)
        .await;

    let health = FeeDiscountRegistryHealth::new(true);
    let lcd = LcdClient::new(vec![lcd_server.uri()], 5000, 30000);
    for _ in 0..2 {
        cl8y_dex_indexer::indexer::fee_discount_registry_health::probe_fee_discount_registry(
            &lcd,
            "terra1feediscount",
            &health,
        )
        .await;
    }

    let app = build_app_with_fee_discount_health(pool, vec![lcd_server.uri()], health)
    .await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/health/fee-discount").await;
    resp.assert_status_ok();
    let text = resp.text();
    assert!(!text.contains("cosmwasm"));
    assert!(!text.contains("http://"));
    assert!(!text.contains("All LCD endpoints failed"));
    let body: Value = serde_json::from_str(&text).expect("json body");
    assert_eq!(body["configured"], true);
    assert_eq!(body["fee_discount_registry_ok"], false);
    assert_eq!(body["consecutive_lcd_failures"], 2);
}
