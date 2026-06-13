mod common;

use axum_test::TestServer;
use serde_json::Value;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

use common::lcd_mock::{lcd_base_url, smart_query_from_request};

#[tokio::test]
async fn fee_discount_health_unconfigured_returns_not_configured() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/health/fee-discount").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(body["configured"], false);
    assert!(body["fee_discount_registry_ok"].is_null());
}

#[tokio::test]
async fn fee_discount_health_reports_lcd_probe_result() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;

    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
        .respond_with(|req: &wiremock::Request| {
            let q = smart_query_from_request(req);
            if q.get("config").is_some() {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "data": {
                        "governance": "terra1gov",
                        "cl8y_token": "terra1cl8y"
                    }
                }))
            } else {
                ResponseTemplate::new(500)
            }
        })
        .mount(&mock)
        .await;

    let mut config = common::test_config();
    config.lcd_urls = vec![lcd_base_url(&mock)];
    config.fee_discount_address = Some("terra1feediscount".to_string());

    let app = common::build_test_app_with_price_and_config(pool, None, config).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/health/fee-discount").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(body["configured"], true);
    assert_eq!(body["fee_discount_registry_ok"], true);
    assert!(body.get("error").is_none());
}
