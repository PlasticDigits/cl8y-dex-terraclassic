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

/// GitLab #270 — bid ladder with a BELOW-band tail: 1@2.0 → 2@1.5 → 3@1.5 → 4@1.0 → 5@0.5.
/// Window [1.5,1.0] returns 2,3,4 and the chain continues below the band at 5@0.5.
fn tiered_bid_book_below_band_tail_responder(
) -> impl Fn(&Request) -> ResponseTemplate + Send + Sync + 'static {
    let orders = [
        (1u64, "2.0", Some(2u64)),
        (2, "1.5", Some(3)),
        (3, "1.5", Some(4)),
        (4, "1.0", Some(5)),
        (5, "0.5", None),
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

/// GitLab #270 — ask ladder with an ABOVE-band tail: 10@1.0 → 11@1.2 → 12@1.2 → 13@1.5 → 14@2.0.
/// Window [1.2,1.5] returns 11,12,13; the chain continues above the band at 14@2.0.
fn tiered_ask_book_above_band_tail_responder(
) -> impl Fn(&Request) -> ResponseTemplate + Send + Sync + 'static {
    let orders = [
        (10u64, "1.0", Some(11u64)),
        (11, "1.2", Some(12)),
        (12, "1.2", Some(13)),
        (13, "1.5", Some(14)),
        (14, "2.0", None),
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

/// GitLab #270 — set up a fresh app over `responder`, issue one `limit-book` price-window request
/// (`query` is the raw query string after the `?`), and return its JSON body plus the number of
/// book-walk LCD queries the request actually made (`order_book_head` + `limit_order` smart queries).
/// The query count makes the per-page LCD budget guardrail observable (past_band adds zero peek
/// queries; a page-full peek adds exactly one).
async fn price_window_body_and_book_queries(
    responder: impl Fn(&Request) -> ResponseTemplate + Send + Sync + 'static,
    query: &str,
) -> (Value, usize) {
    let mock = mount_smart_mock(responder).await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);
    let url = format!("/api/v1/pairs/{}/limit-book?{}", seed.pair_address, query);
    let resp = server.get(&url).await;
    resp.assert_status_ok();
    let body = resp.json();
    let book_queries = mock
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|r| {
            let q = smart_query_from_request(r);
            q.get("limit_order").is_some() || q.get("order_book_head").is_some()
        })
        .count();
    (body, book_queries)
}

/// Thin wrapper for scenarios that don't assert the LCD query count.
async fn price_window_body(
    responder: impl Fn(&Request) -> ResponseTemplate + Send + Sync + 'static,
    query: &str,
) -> Value {
    price_window_body_and_book_queries(responder, query).await.0
}

fn order_ids(body: &Value) -> Vec<u64> {
    body["orders"]
        .as_array()
        .unwrap()
        .iter()
        .map(|o| o["order_id"].as_u64().unwrap())
        .collect()
}

/// GitLab #270 — price-window `has_more` reflects remaining IN-BAND rows, not that the FIFO chain
/// continues outside the band. Regression for the band-floor false positive on `#267` windows.
#[tokio::test]
#[serial]
async fn price_window_has_more_excludes_out_of_band_tail() {
    // 1) Bid window, in-band rows then a below-band tail (5@0.5), limit >= in-band depth:
    //    all in-band orders returned; has_more = false despite the chain continuing below the band.
    //    Query-count guardrail: the past_band exit adds NO peek query — head(1) + walk of
    //    1@2.0(skip),2,3,4(in-band),5@0.5(past) = 6 book queries, no extra.
    {
        let (body, book_queries) = price_window_body_and_book_queries(
            tiered_bid_book_below_band_tail_responder(),
            "side=bid&price_from=1.5&price_to=1.0&limit=10",
        )
        .await;
        assert_eq!(order_ids(&body), vec![2, 3, 4]);
        assert!(
            !body["has_more"].as_bool().unwrap(),
            "below-band tail must not set has_more"
        );
        assert!(body["next_after_order_id"].is_null());
        assert_eq!(
            book_queries, 6,
            "past_band exit must not add a peek LCD query"
        );
    }

    // 2) Symmetric ask window, in-band rows then an above-band tail (14@2.0).
    {
        let body = price_window_body(
            tiered_ask_book_above_band_tail_responder(),
            "side=ask&price_from=1.2&price_to=1.5&limit=10",
        )
        .await;
        assert_eq!(order_ids(&body), vec![11, 12, 13]);
        assert!(
            !body["has_more"].as_bool().unwrap(),
            "above-band tail must not set has_more"
        );
        assert!(body["next_after_order_id"].is_null());
    }

    // 3) cap-at-floor: limit == in-band depth and the next node is out-of-band -> has_more = false.
    //    (The page-full exit path: the walk stops with `current` pointing at the below-band tail.)
    {
        let body = price_window_body(
            tiered_bid_book_below_band_tail_responder(),
            "side=bid&price_from=1.5&price_to=1.0&limit=3",
        )
        .await;
        assert_eq!(order_ids(&body), vec![2, 3, 4]);
        assert!(
            !body["has_more"].as_bool().unwrap(),
            "cap exactly at the band floor must not set has_more"
        );
        assert!(body["next_after_order_id"].is_null());
    }

    // 4) in-band depth > limit -> page 1 has_more=true with a valid cursor; page 2 (after_order_id)
    //    returns only the remaining in-band rows, then has_more=false at the band floor.
    {
        // Page 1 query-count guardrail: head(1) + 1@2.0(skip),2,3(in-band, fills cap=2) = 4 walk
        // queries, then exactly ONE peek of successor 4 = 5 book queries total.
        let (p1, p1_queries) = price_window_body_and_book_queries(
            tiered_bid_book_below_band_tail_responder(),
            "side=bid&price_from=1.5&price_to=1.0&limit=2",
        )
        .await;
        assert_eq!(order_ids(&p1), vec![2, 3]);
        assert!(
            p1["has_more"].as_bool().unwrap(),
            "in-band depth beyond limit -> has_more true"
        );
        assert_eq!(p1["next_after_order_id"].as_u64().unwrap(), 3);
        assert_eq!(
            p1_queries, 5,
            "page-full in-band successor must add exactly one peek LCD query"
        );

        let p2 = price_window_body(
            tiered_bid_book_below_band_tail_responder(),
            "side=bid&price_from=1.5&price_to=1.0&limit=2&after_order_id=3",
        )
        .await;
        assert_eq!(order_ids(&p2), vec![4]);
        assert!(
            !p2["has_more"].as_bool().unwrap(),
            "page 2 reaches the band floor -> has_more false"
        );
        assert!(p2["next_after_order_id"].is_null());
    }

    // 5) Empty band (no in-band rows): the walk skips above-band rows then hits the below-band tail.
    {
        let body = price_window_body(
            tiered_bid_book_below_band_tail_responder(),
            "side=bid&price_from=0.7&price_to=0.6&limit=10",
        )
        .await;
        assert!(order_ids(&body).is_empty(), "no in-band rows in this band");
        assert!(
            !body["has_more"].as_bool().unwrap(),
            "empty band past the tail -> has_more false"
        );
    }

    // 6) Ask cap-at-floor (mirror of 3 through the ask arm of the peek): limit == in-band depth,
    //    successor 14@2.0 is above-band/out-of-band -> has_more = false.
    {
        let body = price_window_body(
            tiered_ask_book_above_band_tail_responder(),
            "side=ask&price_from=1.2&price_to=1.5&limit=3",
        )
        .await;
        assert_eq!(order_ids(&body), vec![11, 12, 13]);
        assert!(
            !body["has_more"].as_bool().unwrap(),
            "ask cap at the band ceiling must not set has_more"
        );
        assert!(body["next_after_order_id"].is_null());
    }

    // 7) Ask cap<depth in-band continuation (mirror of 4 through the ask arm): page 1 peeks an
    //    in-band successor -> has_more=true + cursor; page 2 returns the rest then has_more=false.
    {
        let p1 = price_window_body(
            tiered_ask_book_above_band_tail_responder(),
            "side=ask&price_from=1.2&price_to=1.5&limit=2",
        )
        .await;
        assert_eq!(order_ids(&p1), vec![11, 12]);
        assert!(
            p1["has_more"].as_bool().unwrap(),
            "ask in-band depth beyond limit -> has_more true"
        );
        assert_eq!(p1["next_after_order_id"].as_u64().unwrap(), 12);

        let p2 = price_window_body(
            tiered_ask_book_above_band_tail_responder(),
            "side=ask&price_from=1.2&price_to=1.5&limit=2&after_order_id=12",
        )
        .await;
        assert_eq!(order_ids(&p2), vec![13]);
        assert!(
            !p2["has_more"].as_bool().unwrap(),
            "ask page 2 reaches the band ceiling -> has_more false"
        );
        assert!(p2["next_after_order_id"].is_null());
    }
}

/// GitLab #270 — when the per-page LCD budget is spent exactly as the page fills, the page-full peek
/// cannot run, so `has_more` falls back to `true` (a bounded over-report) with the last in-band order
/// as the cursor. A deep all-in-band book (price 1.0, window [1.0,1.0]) at `limit=100` consumes the
/// whole 101 budget on the walk (head + 100 rows); the peek's `bump_lcd()` would be query 102 and is
/// suppressed — no real 102nd LCD query is made.
#[tokio::test]
#[serial]
async fn price_window_has_more_budget_exhaustion_over_reports() {
    let (body, book_queries) = price_window_body_and_book_queries(
        deep_flat_bid_chain(120),
        "side=bid&price_from=1.0&price_to=1.0&limit=100",
    )
    .await;
    assert_eq!(order_ids(&body).len(), 100, "full page of in-band rows");
    assert_eq!(order_ids(&body)[0], 1);
    assert_eq!(order_ids(&body)[99], 100);
    assert!(
        body["has_more"].as_bool().unwrap(),
        "budget spent at page-full peek -> bounded over-report has_more=true"
    );
    assert_eq!(
        body["next_after_order_id"].as_u64().unwrap(),
        100,
        "over-report cursor is the last returned in-band order id"
    );
    assert_eq!(
        book_queries, 101,
        "budget ceiling holds: head + 100 rows, peek suppressed (no 102nd LCD query)"
    );
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
