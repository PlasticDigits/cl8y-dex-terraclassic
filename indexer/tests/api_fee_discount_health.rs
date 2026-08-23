//! GitLab #373: narrow fee-discount registry health API.

mod common;

use axum::Router;
use axum_test::TestServer;
use cl8y_dex_indexer::api::{build_router, AppState};
use cl8y_dex_indexer::indexer::fee_discount_registry_health::{
    probe_fee_discount_registry_once, FeeDiscountRegistryHealth,
};
use cl8y_dex_indexer::lcd::LcdClient;
use serde_json::Value;
use serial_test::serial;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

async fn build_app_with_health(
    pool: sqlx::PgPool,
    config: cl8y_dex_indexer::config::Config,
    health: FeeDiscountRegistryHealth,
) -> Router {
    let lcd = LcdClient::new(
        config.lcd_urls.clone(),
        config.lcd_timeout_ms,
        config.lcd_cooldown_ms,
    );
    let oracle_prices = cl8y_dex_indexer::indexer::oracle::OraclePriceHandles::new();
    let state = AppState {
        pool,
        lcd,
        oracle_prices,
        venus_vfdusd: cl8y_dex_indexer::indexer::venus_vfdusd::new_shared_venus(),
        ticker_map_cache: cl8y_dex_indexer::api::TickerMapCache::default(),
        orderbook_cache: cl8y_dex_indexer::api::orderbook_sim::OrderbookCache::default(),
        router_address: config.router_address.clone(),
        factory_address: Some(config.factory_address.clone()),
        fee_discount_address: config.fee_discount_address.clone(),
        fee_discount_registry_health: health,
        route_solver_db_hybrid: config.route_solver_db_hybrid,
        book_snapshot_max_staleness_ms: config.book_snapshot_max_staleness_ms(),
        route_fidelity_drift_bps: config.route_fidelity_drift_bps,
        hub_usd: cl8y_dex_indexer::indexer::hub_usd::HubUsdConfig::from_indexer_config(&config),
        community_tax: cl8y_dex_indexer::config::CommunityTaxCatalogConfig::from_indexer_config(
            &config,
        ),
    };
    build_router(state, &config)
}

#[tokio::test]
#[serial]
async fn generic_health_unchanged() {
    let pool = common::setup_pool().await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/health").await;
    resp.assert_status_ok();
    assert_eq!(resp.json::<Value>(), serde_json::json!({ "status": "ok" }));
}

#[tokio::test]
#[serial]
async fn unconfigured_fee_discount_health_returns_null_ok() {
    let pool = common::setup_pool().await;
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
#[serial]
async fn lcd_failure_increments_counter_without_leaking_upstream_text() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/"))
        .respond_with(
            ResponseTemplate::new(500)
                .set_body_string("All LCD endpoints failed; cosmwasm http://127.0.0.1:1317 leak"),
        )
        .mount(&mock)
        .await;

    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];
    cfg.fee_discount_address = Some("terra1feediscount".to_string());

    let pool = common::setup_pool().await;
    let health = FeeDiscountRegistryHealth::configured();
    let lcd = LcdClient::new(
        cfg.lcd_urls.clone(),
        cfg.lcd_timeout_ms,
        cfg.lcd_cooldown_ms,
    );

    probe_fee_discount_registry_once(&lcd, "terra1feediscount", &health).await;
    probe_fee_discount_registry_once(&lcd, "terra1feediscount", &health).await;

    let app = build_app_with_health(pool, cfg, health).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/health/fee-discount").await;
    resp.assert_status_ok();
    let text = resp.text();
    let body: Value = serde_json::from_str(&text).expect("json body");

    assert_eq!(body["configured"], true);
    assert_eq!(body["fee_discount_registry_ok"], false);
    assert!(body["consecutive_lcd_failures"].as_u64().unwrap() >= 2);

    let lower = text.to_lowercase();
    for forbidden in [
        "cosmwasm",
        "http://",
        "https://",
        "all lcd endpoints failed",
        "127.0.0.1",
        "leak",
    ] {
        assert!(
            !lower.contains(forbidden),
            "response must not leak upstream detail: found {forbidden} in {text}"
        );
    }
}

#[tokio::test]
#[serial]
async fn configured_successful_probe_reports_ok() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "data": {
                "governance": "terra1gov",
                "cl8y_token": "terra1cl8y"
            }
        })))
        .mount(&mock)
        .await;

    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];
    cfg.fee_discount_address = Some("terra1feediscount".to_string());

    let pool = common::setup_pool().await;
    let health = FeeDiscountRegistryHealth::configured();
    let lcd = LcdClient::new(
        cfg.lcd_urls.clone(),
        cfg.lcd_timeout_ms,
        cfg.lcd_cooldown_ms,
    );
    probe_fee_discount_registry_once(&lcd, "terra1feediscount", &health).await;

    let app = build_app_with_health(pool, cfg, health).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/health/fee-discount").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(body["configured"], true);
    assert_eq!(body["fee_discount_registry_ok"], true);
    assert_eq!(body["consecutive_lcd_failures"], 0);
}

#[tokio::test]
#[serial]
async fn endpoint_has_no_trader_query_params() {
    let pool = common::setup_pool().await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get("/api/v1/health/fee-discount?trader=terra1attacker")
        .await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert!(body.get("trader").is_none());
    assert!(body.get("address").is_none());
    assert!(!body["configured"].as_bool().unwrap());
}
