//! `/metrics` route tests using a lazy Postgres pool (no live DB required).

mod common;

use axum_test::TestServer;
use cl8y_dex_indexer::api::{build_router, AppState};
use cl8y_dex_indexer::config::Config;
use cl8y_dex_indexer::lcd::LcdClient;
use sqlx::postgres::PgPoolOptions;

fn lazy_pool() -> sqlx::PgPool {
    PgPoolOptions::new()
        .max_connections(1)
        .connect_lazy("postgres://127.0.0.1:1/metrics_route_test")
        .expect("lazy pool")
}

fn test_app(config: Config) -> axum::Router {
    let pool = lazy_pool();
    let lcd = LcdClient::new(
        config.lcd_urls.clone(),
        config.lcd_timeout_ms,
        config.lcd_cooldown_ms,
    );
    let ustc = cl8y_dex_indexer::indexer::oracle::new_shared_price();
    let state = AppState {
        pool,
        lcd,
        ustc_price: ustc,
        ticker_map_cache: cl8y_dex_indexer::api::TickerMapCache::default(),
        orderbook_cache: cl8y_dex_indexer::api::orderbook_sim::OrderbookCache::default(),
        router_address: config.router_address.clone(),
    };
    build_router(state, &config)
}

#[tokio::test]
async fn prometheus_metrics_not_exposed_by_default() {
    let mut config = common::test_config();
    config.metrics_enabled = false;
    let server = TestServer::new(test_app(config));
    server.get("/metrics").await.assert_status_not_found();
}

#[tokio::test]
async fn prometheus_metrics_when_enabled() {
    let mut config = common::test_config();
    config.metrics_enabled = true;
    let server = TestServer::new(test_app(config));
    let resp = server.get("/metrics").await;
    resp.assert_status_ok();
    let text = resp.text();
    assert!(
        text.contains("indexer_blocks_processed_total"),
        "expected Prometheus text, got: {}",
        text
    );
}
