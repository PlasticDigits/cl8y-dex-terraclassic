//! LCD-proxied limit book endpoints (`order-book-head`, `limit-book-shallow`) against wiremock.

mod common;

use axum_test::TestServer;
use base64::Engine;
use serde_json::{json, Value};
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

fn smart_query_from_request(req: &Request) -> Value {
    let path = req.url.path();
    let b64 = path
        .split("/smart/")
        .nth(1)
        .expect("path should contain /smart/");
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .expect("valid base64 query");
    serde_json::from_slice(&bytes).expect("valid json query")
}

fn limit_book_responder(
    head: Option<u64>,
    chain: &'static str,
) -> impl Fn(&Request) -> ResponseTemplate + Send + Sync {
    move |req: &Request| {
        let q = smart_query_from_request(req);
        let data = if q.get("order_book_head").is_some() {
            match head {
                Some(h) => json!(h),
                None => Value::Null,
            }
        } else if q.get("limit_order").is_some() {
            let id = q["limit_order"]["order_id"].as_u64().unwrap();
            if chain == "two" {
                if id == 7 {
                    json!({
                        "order_id": 7,
                        "owner": "terra1maker",
                        "side": "bid",
                        "price": "1.5",
                        "remaining": "1000",
                        "expires_at": null,
                        "prev": null,
                        "next": 8
                    })
                } else {
                    json!({
                        "order_id": 8,
                        "owner": "terra1maker2",
                        "side": "bid",
                        "price": "1.4",
                        "remaining": "500",
                        "expires_at": null,
                        "prev": 7,
                        "next": null
                    })
                }
            } else {
                json!(null)
            }
        } else {
            json!(null)
        };
        ResponseTemplate::new(200).set_body_json(json!({ "data": data }))
    }
}

async fn mount_smart_mock(
    responder: impl Fn(&Request) -> ResponseTemplate + Send + Sync + 'static,
) -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/.+$"))
        .respond_with(responder)
        .mount(&server)
        .await;
    server
}

#[tokio::test]
async fn limit_book_lcd_proxy_endpoints() {
    // Single test: scenarios share Postgres seed state; avoids parallel insert races on `assets`.
    {
        let mock = mount_smart_mock(limit_book_responder(Some(7u64), "two")).await;
        let mut cfg = common::test_config();
        cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];

        let pool = common::setup_pool().await;
        let seed = common::seed_db(&pool).await;
        let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
        let server = TestServer::new(app);

        let url = format!(
            "/api/v1/pairs/{}/order-book-head?side=bid",
            seed.pair_address
        );
        let resp = server.get(&url).await;
        resp.assert_status_ok();
        let body: Value = resp.json();
        assert_eq!(body["head_order_id"], 7);
    }
    {
        let mock = mount_smart_mock(limit_book_responder(Some(7u64), "two")).await;
        let mut cfg = common::test_config();
        cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];

        let pool = common::setup_pool().await;
        let seed = common::seed_db(&pool).await;
        let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
        let server = TestServer::new(app);

        let url = format!(
            "/api/v1/pairs/{}/limit-book-shallow?side=bid&depth=3",
            seed.pair_address
        );
        let resp = server.get(&url).await;
        resp.assert_status_ok();
        let body: Value = resp.json();
        assert_eq!(body["side"], "bid");
        let orders = body["orders"].as_array().unwrap();
        assert_eq!(orders.len(), 2);
        assert_eq!(orders[0]["order_id"], 7);
        assert_eq!(orders[1]["order_id"], 8);
    }
    {
        let mock = mount_smart_mock(limit_book_responder(None, "empty")).await;
        let mut cfg = common::test_config();
        cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];

        let pool = common::setup_pool().await;
        let seed = common::seed_db(&pool).await;
        let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
        let server = TestServer::new(app);

        let url = format!(
            "/api/v1/pairs/{}/limit-book-shallow?side=ask&depth=5",
            seed.pair_address
        );
        let resp = server.get(&url).await;
        resp.assert_status_ok();
        let body: Value = resp.json();
        assert_eq!(body["orders"].as_array().unwrap().len(), 0);
    }
}
