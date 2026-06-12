//! Event-driven fee-discount tier sync (GitLab #364).

mod common;

use axum_test::TestServer;
use cl8y_dex_indexer::config::Config;
use cl8y_dex_indexer::db::queries::traders;
use cl8y_dex_indexer::indexer::block_indexer;
use cl8y_dex_indexer::indexer::oracle;
use cl8y_dex_indexer::indexer::trader_tracker;
use cl8y_dex_indexer::lcd::LcdClient;
use common::{clean_db, setup_pool, test_config};
use serde_json::json;
use wiremock::matchers::{method, path, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

const FEE_DISCOUNT_ADDR: &str = "terra1feediscountcontractaddress000000";
const TRADER_ADDR: &str = "terra1tiertraderabc1234567890123456789012";

fn tier_sync_config(lcd_url: &str) -> Config {
    Config {
        lcd_urls: vec![lcd_url.to_string()],
        fee_discount_address: Some(FEE_DISCOUNT_ADDR.to_string()),
        ..test_config()
    }
}

fn register_tx(height: i64) -> serde_json::Value {
    json!({
        "height": height.to_string(),
        "txhash": "register_tx_hash",
        "timestamp": "2024-01-01T00:00:00Z",
        "logs": [{ "events": [{ "type": "wasm", "attributes": [
            { "key": "_contract_address", "value": FEE_DISCOUNT_ADDR },
            { "key": "action", "value": "register" },
            { "key": "wallet", "value": TRADER_ADDR },
            { "key": "tier_id", "value": "5" }
        ]}]}]
    })
}

fn block_json(height: i64, hash: &str) -> serde_json::Value {
    json!({
        "block_id": { "hash": hash },
        "block": {
            "header": {
                "height": height.to_string(),
                "time": "2024-01-01T00:00:00Z"
            }
        }
    })
}

async fn mount_register_block(server: &MockServer, height: i64, hash: &str) {
    Mock::given(method("GET"))
        .and(path(format!(
            "/cosmos/base/tendermint/v1beta1/blocks/{}",
            height
        )))
        .respond_with(ResponseTemplate::new(200).set_body_json(block_json(height, hash)))
        .mount(server)
        .await;

    Mock::given(method("GET"))
        .and(path("/cosmos/tx/v1beta1/txs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "tx_responses": [register_tx(height)],
            "pagination": { "total": "1" }
        })))
        .mount(server)
        .await;
}

fn lcd_client(server: &MockServer) -> LcdClient {
    LcdClient::new(
        vec![server.uri().trim_end_matches('/').to_string()],
        5000,
        30000,
    )
}

fn registration_lcd_body(tier_id: u8, registered: bool) -> serde_json::Value {
    if registered {
        json!({
            "registered": true,
            "tier_id": tier_id,
            "tier": { "discount_bps": 5000, "governance_only": false, "min_cl8y_balance": "0" }
        })
    } else {
        json!({ "registered": false })
    }
}

async fn mount_get_registration(server: &MockServer, body: serde_json::Value) {
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({ "data": body })))
        .mount(server)
        .await;
}

#[tokio::test]
async fn registration_event_updates_trader_within_block() {
    let pool = setup_pool().await;
    clean_db(&pool).await;

    let server = MockServer::start().await;
    let height = 1200i64;
    let hash = "HASH1200";
    mount_register_block(&server, height, hash).await;

    let lcd = lcd_client(&server);
    let config = tier_sync_config(&server.uri());
    let price = oracle::new_shared_price();

    block_indexer::index_block(&pool, &lcd, &config, height, &price)
        .await
        .expect("index register block");

    let trader = traders::get_trader(&pool, TRADER_ADDR)
        .await
        .expect("query trader")
        .expect("trader row");
    assert_eq!(trader.tier_id, 5);
    assert_eq!(trader.tier_name, "Tier 5");
    assert!(trader.registered);

    let app = common::build_test_app_with_price_and_config(pool, None, config).await;
    let server = TestServer::new(app);
    let resp = server.get(&format!("/api/v1/traders/{TRADER_ADDR}")).await;
    resp.assert_status_ok();
    let body: serde_json::Value = resp.json();
    assert_eq!(body["tier_id"], 5);
    assert_eq!(body["tier_name"], "Tier 5");
    assert_eq!(body["registered"], true);
}

#[tokio::test]
async fn reconcile_corrects_missed_registration_event() {
    let pool = setup_pool().await;
    clean_db(&pool).await;

    sqlx::query(
        "INSERT INTO traders (address, tier_id, tier_name, registered, total_trades, total_volume)
         VALUES ($1, 0, 'None', false, 1, 100)",
    )
    .bind(TRADER_ADDR)
    .execute(&pool)
    .await
    .expect("seed stale trader");

    let server = MockServer::start().await;
    mount_get_registration(&server, registration_lcd_body(5, true)).await;

    let lcd = lcd_client(&server);
    trader_tracker::reconcile_trader_tiers(&pool, &lcd, FEE_DISCOUNT_ADDR)
        .await
        .expect("reconcile");

    let trader = traders::get_trader(&pool, TRADER_ADDR)
        .await
        .expect("query")
        .expect("row");
    assert_eq!(trader.tier_id, 5);
    assert!(trader.registered);
}

#[tokio::test]
async fn route_solve_uses_tier_after_registration_event() {
    let pool = setup_pool().await;
    clean_db(&pool).await;

    let seed = common::seed_route_solve(&pool).await;
    trader_tracker::apply_registry_tier_events(
        &pool,
        &[trader_tracker::ParsedRegistryTierEvent::Register {
            wallet: TRADER_ADDR.to_string(),
            tier_id: 5,
        }],
    )
    .await
    .expect("apply tier");

    let mock = common::lcd_mock::start_tier_aware_route_optimizer_mock(&[TRADER_ADDR]).await;
    let mut config = common::test_config();
    config.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];
    config.router_address = Some("terra1routertest".to_string());
    config.fee_discount_address = Some(FEE_DISCOUNT_ADDR.to_string());

    let app = common::build_test_app_with_price_and_config(pool, None, config).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1000000&sender={TRADER_ADDR}",
        seed.token_a, seed.token_b
    );
    let body: serde_json::Value = server.get(&url).await.json();
    assert_eq!(
        body["estimated_amount_out"],
        "9777776"
    );
}
