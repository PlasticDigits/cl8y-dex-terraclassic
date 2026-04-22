//! Deep `limit-book` pagination and light concurrent fetch stress (wiremock LCD).

mod common;

use axum_test::TestServer;
use base64::Engine;
use serde_json::{json, Value};
use serial_test::serial;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
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

/// Linear bid book: head order_id = 1, `next` increments until `total` (inclusive).
fn deep_bid_chain_responder(
    total: u64,
) -> impl Fn(&Request) -> ResponseTemplate + Send + Sync + 'static {
    move |req: &Request| {
        let q = smart_query_from_request(req);
        let data = if q.get("order_book_head").is_some() {
            if total == 0 {
                Value::Null
            } else {
                json!(1u64)
            }
        } else if q.get("limit_order").is_some() {
            let id = q["limit_order"]["order_id"].as_u64().unwrap();
            if (1..=total).contains(&id) {
                let next = if id < total {
                    json!(id + 1)
                } else {
                    Value::Null
                };
                json!({
                    "order_id": id,
                    "owner": "terra1maker",
                    "side": "bid",
                    "price": "1.0",
                    "remaining": "100",
                    "expires_at": null,
                    "prev": null,
                    "next": next
                })
            } else {
                Value::Null
            }
        } else {
            Value::Null
        };
        ResponseTemplate::new(200).set_body_json(json!({ "data": data }))
    }
}

/// First page is bid; order 51 is an ask (wrong side) to trigger 400 mid-walk.
fn broken_side_chain_responder() -> impl Fn(&Request) -> ResponseTemplate + Send + Sync + 'static {
    move |req: &Request| {
        let q = smart_query_from_request(req);
        let data = if q.get("order_book_head").is_some() {
            json!(1u64)
        } else if q.get("limit_order").is_some() {
            let id = q["limit_order"]["order_id"].as_u64().unwrap();
            let side = if id == 51 { "ask" } else { "bid" };
            let next = if id < 100 { json!(id + 1) } else { Value::Null };
            json!({
                "order_id": id,
                "owner": "terra1maker",
                "side": side,
                "price": "1.0",
                "remaining": "100",
                "expires_at": null,
                "prev": null,
                "next": next
            })
        } else {
            Value::Null
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
#[serial]
async fn limit_book_paginates_deep_chain() {
    const TOTAL: u64 = 237;
    let mock = mount_smart_mock(deep_bid_chain_responder(TOTAL)).await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];

    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let mut after: Option<u64> = None;
    let mut seen = Vec::new();
    loop {
        let q = match after {
            None => format!(
                "/api/v1/pairs/{}/limit-book?side=bid&limit=50",
                seed.pair_address
            ),
            Some(id) => format!(
                "/api/v1/pairs/{}/limit-book?side=bid&limit=50&after_order_id={}",
                seed.pair_address, id
            ),
        };
        let resp = server.get(&q).await;
        resp.assert_status_ok();
        let body: Value = resp.json();
        let orders = body["orders"].as_array().unwrap();
        for o in orders {
            seen.push(o["order_id"].as_u64().unwrap());
        }
        let has_more = body["has_more"].as_bool().unwrap();
        let next = body["next_after_order_id"].as_u64();
        if !has_more {
            assert!(next.is_none());
            break;
        }
        assert_eq!(next, orders.last().map(|o| o["order_id"].as_u64().unwrap()));
        after = next;
    }

    assert_eq!(seen.len() as u64, TOTAL);
    for (i, id) in seen.iter().enumerate() {
        assert_eq!(*id, (i + 1) as u64);
    }
}

#[tokio::test]
#[serial]
async fn limit_book_invalid_cursor_400() {
    let mock = mount_smart_mock(deep_bid_chain_responder(10)).await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];

    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/pairs/{}/limit-book?side=bid&limit=10&after_order_id=9999",
        seed.pair_address
    );
    let resp = server.get(&url).await;
    resp.assert_status_bad_request();
}

#[tokio::test]
#[serial]
async fn limit_book_side_mismatch_400() {
    let mock = mount_smart_mock(broken_side_chain_responder()).await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];

    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/pairs/{}/limit-book?side=bid&limit=100",
        seed.pair_address
    );
    let resp = server.get(&url).await;
    resp.assert_status_bad_request();
}

#[tokio::test]
#[serial]
async fn limit_book_concurrent_pages_stress() {
    const TOTAL: u64 = 120;
    let lcd_hits = Arc::new(AtomicUsize::new(0));
    let hits = Arc::clone(&lcd_hits);

    let mock = mount_smart_mock(move |req: &Request| {
        hits.fetch_add(1, Ordering::Relaxed);
        deep_bid_chain_responder(TOTAL)(req)
    })
    .await;

    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];

    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);
    let pair = seed.pair_address.clone();

    let url0 = format!("/api/v1/pairs/{pair}/limit-book?side=bid&limit=40");
    let url1 = format!("/api/v1/pairs/{pair}/limit-book?side=bid&limit=40&after_order_id=40");
    let url2 = format!("/api/v1/pairs/{pair}/limit-book?side=bid&limit=40&after_order_id=80");

    let (r0, r1, r2) = tokio::join!(server.get(&url0), server.get(&url1), server.get(&url2),);
    r0.assert_status_ok();
    r1.assert_status_ok();
    r2.assert_status_ok();
    let bodies = [r0.json::<Value>(), r1.json::<Value>(), r2.json::<Value>()];

    let mut all_ids: Vec<u64> = Vec::new();
    for (i, body) in bodies.iter().enumerate() {
        let orders = body["orders"].as_array().unwrap();
        assert!(!orders.is_empty());
        for o in orders {
            all_ids.push(o["order_id"].as_u64().unwrap());
        }
        let has_more = body["has_more"].as_bool().unwrap();
        if i < 2 {
            assert!(has_more);
        }
    }
    all_ids.sort_unstable();
    all_ids.dedup();
    assert_eq!(all_ids.len(), 120);
    assert!(lcd_hits.load(Ordering::Relaxed) >= 10);
}
