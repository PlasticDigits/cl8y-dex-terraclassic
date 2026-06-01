//! Insert-hint resolver and price-window `limit-book` queries (GitLab #267).

mod common;

use axum_test::TestServer;
use base64::Engine;
use cl8y_dex_indexer::api::limit_book_lcd::{
    resolve_limit_insert_hints, LimitBookInsertHintItem, LIMIT_BOOK_LCD_QUERY_BUDGET,
};
use cl8y_dex_indexer::api::limit_book_price::{compare_positive_decimal_strings, DecimalCmp};
use cl8y_dex_indexer::lcd::LcdClient;
use serde_json::{json, Value};
use serial_test::serial;
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

/// Bid ladder: id 1 @ 2.0 → 2 @ 1.5 → 3 @ 1.5 → 4 @ 1.0
fn tiered_bid_book_responder() -> impl Fn(&Request) -> ResponseTemplate + Send + Sync + 'static {
    let orders = [
        (1u64, "2.0", Some(2u64)),
        (2, "1.5", Some(3)),
        (3, "1.5", Some(4)),
        (4, "1.0", None),
    ];
    move |req: &Request| {
        let q = smart_query_from_request(req);
        let data = if q.get("order_book_head").is_some() {
            json!(1u64)
        } else if q.get("limit_order").is_some() {
            let id = q["limit_order"]["order_id"].as_u64().unwrap();
            orders
                .iter()
                .find(|(oid, _, _)| *oid == id)
                .map(|(oid, price, next)| {
                    json!({
                        "order_id": oid,
                        "owner": "terra1maker",
                        "side": "bid",
                        "price": price,
                        "remaining": "100",
                        "expires_at": null,
                        "prev": null,
                        "next": next.map(Value::from)
                    })
                })
                .unwrap_or(Value::Null)
        } else {
            Value::Null
        };
        ResponseTemplate::new(200).set_body_json(json!({ "data": data }))
    }
}

/// Ask ladder: id 10 @ 1.0 → 11 @ 1.2 → 12 @ 1.2 → 13 @ 1.5
fn tiered_ask_book_responder() -> impl Fn(&Request) -> ResponseTemplate + Send + Sync + 'static {
    let orders = [
        (10u64, "1.0", Some(11u64)),
        (11, "1.2", Some(12)),
        (12, "1.2", Some(13)),
        (13, "1.5", None),
    ];
    move |req: &Request| {
        let q = smart_query_from_request(req);
        let data = if q.get("order_book_head").is_some() {
            json!(10u64)
        } else if q.get("limit_order").is_some() {
            let id = q["limit_order"]["order_id"].as_u64().unwrap();
            orders
                .iter()
                .find(|(oid, _, _)| *oid == id)
                .map(|(oid, price, next)| {
                    json!({
                        "order_id": oid,
                        "owner": "terra1maker",
                        "side": "ask",
                        "price": price,
                        "remaining": "100",
                        "expires_at": null,
                        "prev": null,
                        "next": next.map(Value::from)
                    })
                })
                .unwrap_or(Value::Null)
        } else {
            Value::Null
        };
        ResponseTemplate::new(200).set_body_json(json!({ "data": data }))
    }
}

fn deep_flat_bid_chain(total: u64) -> impl Fn(&Request) -> ResponseTemplate + Send + Sync + 'static {
    move |req: &Request| {
        let q = smart_query_from_request(req);
        let data = if q.get("order_book_head").is_some() {
            json!(1u64)
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

/// Mirrors `frontend-dapp/src/utils/limitBookInsertHint.ts` for parity tests.
fn client_resolve_hint(
    side: &str,
    price: &str,
    orders: &[(u64, &str)],
    has_more: bool,
) -> Option<u64> {
    if orders.is_empty() {
        return None;
    }
    let mut prev: Option<u64> = None;
    for (id, order_price) in orders {
        let cmp = compare_positive_decimal_strings(price, order_price)?;
        if side == "bid" {
            if cmp == DecimalCmp::Gt {
                return prev;
            }
            if cmp == DecimalCmp::Eq || cmp == DecimalCmp::Lt {
                prev = Some(*id);
            }
        } else {
            if cmp == DecimalCmp::Lt {
                return prev;
            }
            if cmp == DecimalCmp::Eq || cmp == DecimalCmp::Gt {
                prev = Some(*id);
            }
        }
    }
    if has_more {
        None
    } else {
        prev
    }
}

fn hint_to_predecessor(item: &LimitBookInsertHintItem) -> Option<u64> {
    if !item.resolved {
        return None;
    }
    item.predecessor_order_id
}

#[tokio::test]
#[serial]
async fn insert_hints_and_price_window_http() {
    // Single test: scenarios share Postgres seed state (same pattern as api_limit_book_lcd_mock).
    {
    let mock = mount_smart_mock(tiered_bid_book_responder()).await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/pairs/{}/limit-book/insert-hints?side=bid&prices=2.1,1.6,1.5,0.9",
        seed.pair_address
    );
    let resp = server.get(&url).await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    let hints = body["hints"].as_array().unwrap();
    assert_eq!(hints.len(), 4);
    assert_eq!(hints[0]["predecessor_order_id"], Value::Null);
    assert_eq!(hints[0]["reason"], "head");
    assert!(hints[0]["resolved"].as_bool().unwrap());
    assert_eq!(hints[1]["predecessor_order_id"], 1);
    assert_eq!(hints[2]["predecessor_order_id"], 3);
    assert_eq!(hints[3]["predecessor_order_id"], 4);
    }
    {
    let mock = mount_smart_mock(tiered_ask_book_responder()).await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/pairs/{}/limit-book/insert-hints?side=ask&prices=0.9,1.1,1.2,1.6",
        seed.pair_address
    );
    let resp = server.get(&url).await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    let hints = body["hints"].as_array().unwrap();
    assert_eq!(hints[0]["predecessor_order_id"], Value::Null);
    assert_eq!(hints[0]["reason"], "head");
    assert_eq!(hints[1]["predecessor_order_id"], 10);
    assert_eq!(hints[2]["predecessor_order_id"], 12);
    assert_eq!(hints[3]["predecessor_order_id"], 13);
    }
    {
    let mock = mount_smart_mock(tiered_bid_book_responder()).await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);
    let prices = (0..101).map(|i| i.to_string()).collect::<Vec<_>>().join(",");
    let url = format!(
        "/api/v1/pairs/{}/limit-book/insert-hints?side=bid&prices={prices}",
        seed.pair_address
    );
    let resp = server.get(&url).await;
    assert_eq!(resp.status_code(), 400);
    }
    {
    let mock = mount_smart_mock(tiered_bid_book_responder()).await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/pairs/{}/limit-book?side=bid&price_from=1.5&price_to=1.0&limit=10",
        seed.pair_address
    );
    let resp = server.get(&url).await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    let orders = body["orders"].as_array().unwrap();
    assert_eq!(orders.len(), 3);
    assert_eq!(orders[0]["order_id"], 2);
    assert_eq!(orders[2]["order_id"], 4);
    assert!(!body["has_more"].as_bool().unwrap());
    }
    {
    let mock = mount_smart_mock(tiered_bid_book_responder()).await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);
    let url = format!(
        "/api/v1/pairs/{}/limit-book?side=bid&price_from=1.5",
        seed.pair_address
    );
    let resp = server.get(&url).await;
    assert_eq!(resp.status_code(), 400);
    }
}

#[tokio::test]
async fn insert_hints_parity_with_client_resolver() {
    let mock = mount_smart_mock(tiered_bid_book_responder()).await;
    let cfg = common::test_config();
    let lcd = LcdClient::new(
        vec![common::lcd_mock::lcd_base_url(&mock)],
        cfg.lcd_timeout_ms,
        cfg.lcd_cooldown_ms,
    );
    let pair = "terra1pair";
    let prices = vec!["2.1".into(), "1.6".into(), "1.5".into(), "0.9".into()];
    let result = resolve_limit_insert_hints(&lcd, pair, "bid", &prices)
        .await
        .unwrap();
    let loaded = [(1u64, "2.0"), (2, "1.5"), (3, "1.5"), (4, "1.0")];
    for (item, price) in result.hints.iter().zip(prices.iter()) {
        let client = client_resolve_hint("bid", price, &loaded, false);
        assert_eq!(hint_to_predecessor(item), client);
    }
}

#[tokio::test]
async fn insert_hints_budget_exhausted_pagination_gap() {
    let total = (LIMIT_BOOK_LCD_QUERY_BUDGET as u64).saturating_add(50);
    let mock = mount_smart_mock(deep_flat_bid_chain(total)).await;
    let cfg = common::test_config();
    let lcd = LcdClient::new(
        vec![common::lcd_mock::lcd_base_url(&mock)],
        cfg.lcd_timeout_ms,
        cfg.lcd_cooldown_ms,
    );
    let result = resolve_limit_insert_hints(
        &lcd,
        "terra1pair",
        "bid",
        &["0.5".to_string()],
    )
    .await
    .unwrap();
    assert!(result.budget_exhausted);
    let h = &result.hints[0];
    assert!(!h.resolved);
    assert_eq!(h.reason.as_deref(), Some("pagination_gap"));
    assert!(h.predecessor_order_id.is_none());
}
