//! Wiremock-backed **Terra LCD HTTP stub** for integration tests.
//!
//! This module only **fakes the LCD REST surface** (for example cosmwasm `.../smart/` responses).
//! It is **not** the on-chain **FIFO limit orderbook**; nothing here walks real resting orders.
//!
//! Production code that **synthesizes bid/ask depth from AMM reserves** (constant-product curve
//! walk for ticker-style APIs) lives in [`cl8y_dex_indexer::api::orderbook_sim`] — a separate
//! concept from both this stub and the pair contract’s orderbook state.
//!
//! See GitLab issue #105 for a repo-wide catalog of stubs and test stand-ins.
//! Production CG/CMC depth: hybrid merge [`hybrid_orderbook_sim`](cl8y_dex_indexer::api::hybrid_orderbook_sim) + pool walk [`orderbook_sim`](cl8y_dex_indexer::api::orderbook_sim) (**#220**, **#210**). Use [`start_hybrid_orderbook_mock`] for limit+pool tests.

use base64::Engine;
use serde_json::{json, Value};
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// Decode the JSON cosmwasm smart query from a `/smart/{base64}` LCD path.
pub fn smart_query_from_request(req: &Request) -> Value {
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

/// When `hybrid_simulation.trader` is set, bump output by 10% (fee-discount parity tests, GitLab #245).
fn hybrid_sim_return_amount(offer: u128, book: u128, has_trader: bool) -> u128 {
    let base = offer.saturating_add(book / 2);
    if has_trader {
        base.saturating_mul(110) / 100
    } else {
        base
    }
}

/// Discount subject on a pair `HybridSimulation` or router `SimulateSwapOperations` query.
fn discount_subject(sim: &Value) -> Option<&str> {
    sim.get("trader")
        .and_then(|v| v.as_str())
        .or_else(|| sim.get("sender").and_then(|v| v.as_str()))
}

fn subject_has_tier5_discount(subject: Option<&str>, tier5_addrs: &[String]) -> bool {
    subject
        .map(|a| tier5_addrs.iter().any(|t| t == a))
        .unwrap_or(false)
}

/// Like [`start_hybrid_route_optimizer_mock`], but router/hybrid output varies by discount
/// subject (`trader` if set, else `sender`) against `tier5_addrs` (GitLab #306).
pub async fn start_tier_aware_route_optimizer_mock(tier5_addrs: &[&str]) -> MockServer {
    let tier5: Vec<String> = tier5_addrs.iter().map(|s| (*s).to_string()).collect();
    let responder = move |req: &Request| {
        let q = smart_query_from_request(req);
        let data = if let Some(sim) = q.get("simulate_swap_operations") {
            let discounted = subject_has_tier5_discount(discount_subject(sim), &tier5);
            let amount = if discounted { "9777776" } else { "8888888" };
            json!({ "amount": amount })
        } else if let Some(sim) = q.get("hybrid_simulation") {
            let book = sim["hybrid"]["book_input"]
                .as_str()
                .unwrap_or("0")
                .parse::<u128>()
                .unwrap_or(0);
            let offer = sim["offer_asset"]["amount"]
                .as_str()
                .unwrap_or("0")
                .parse::<u128>()
                .unwrap_or(0);
            let discounted = subject_has_tier5_discount(discount_subject(sim), &tier5);
            let ret = hybrid_sim_return_amount(offer, book, discounted);
            json!({
                "return_amount": ret.to_string(),
                "spread_amount": "0",
                "commission_amount": "0",
                "book_return_amount": "0",
                "pool_return_amount": ret.to_string(),
            })
        } else {
            json!(null)
        };
        ResponseTemplate::new(200).set_body_json(json!({ "data": data }))
    };
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/.+$"))
        .respond_with(responder)
        .mount(&server)
        .await;
    server
}

/// LCD stub for hybrid route optimization: `HybridSimulation` (including pool-only `book_input: 0`), router `simulate_swap_operations`.
pub async fn start_hybrid_route_optimizer_mock() -> MockServer {
    let responder = |req: &Request| {
        let q = smart_query_from_request(req);
        let data = if q.get("simulate_swap_operations").is_some() {
            let has_trader = q["simulate_swap_operations"]
                .get("trader")
                .and_then(|v| v.as_str())
                .is_some();
            let amount = if has_trader { "9777776" } else { "8888888" };
            json!({ "amount": amount })
        } else if q.get("hybrid_simulation").is_some() {
            let book = q["hybrid_simulation"]["hybrid"]["book_input"]
                .as_str()
                .unwrap_or("0")
                .parse::<u128>()
                .unwrap_or(0);
            let offer = q["hybrid_simulation"]["offer_asset"]["amount"]
                .as_str()
                .unwrap_or("0")
                .parse::<u128>()
                .unwrap_or(0);
            let has_trader = q["hybrid_simulation"]
                .get("trader")
                .and_then(|v| v.as_str())
                .is_some();
            let ret = hybrid_sim_return_amount(offer, book, has_trader);
            json!({
                "return_amount": ret.to_string(),
                "spread_amount": "0",
                "commission_amount": "0",
                "book_return_amount": "0",
                "pool_return_amount": ret.to_string(),
            })
        } else {
            json!(null)
        };
        ResponseTemplate::new(200).set_body_json(json!({ "data": data }))
    };
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/.+$"))
        .respond_with(responder)
        .mount(&server)
        .await;
    server
}

/// Grid hybrid sim (`max_maker_fills` > 1) fails; pool-only fallback (`max_maker_fills` = 1, `book_input: 0`) succeeds (#190).
pub async fn start_hybrid_degraded_mock() -> MockServer {
    let responder = |req: &Request| {
        let q = smart_query_from_request(req);
        if q.get("hybrid_simulation").is_some() {
            let max_fills = q["hybrid_simulation"]["hybrid"]["max_maker_fills"]
                .as_u64()
                .unwrap_or(0);
            if max_fills > 1 {
                // HTTP 200 with unparsable output — grid treats as candidate failure without LCD cooldown.
                return ResponseTemplate::new(200).set_body_json(json!({
                    "data": {
                        "return_amount": "not-a-uint",
                        "spread_amount": "0",
                        "commission_amount": "0",
                    }
                }));
            }
            let offer = q["hybrid_simulation"]["offer_asset"]["amount"]
                .as_str()
                .unwrap_or("0")
                .parse::<u128>()
                .unwrap_or(0);
            return ResponseTemplate::new(200).set_body_json(json!({
                "data": {
                    "return_amount": (offer / 2).to_string(),
                    "spread_amount": "0",
                    "commission_amount": "0",
                    "book_return_amount": "0",
                    "pool_return_amount": (offer / 2).to_string(),
                }
            }));
        }
        if q.get("simulate_swap_operations").is_some() {
            return ResponseTemplate::new(200)
                .set_body_json(json!({ "data": { "amount": "424242" } }));
        }
        ResponseTemplate::new(200).set_body_json(json!({ "data": null }))
    };
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/.+$"))
        .respond_with(responder)
        .mount(&server)
        .await;
    server
}

/// Router sim returns lower out for 1-hop routes than 2-hop (multi-path global solver #209).
pub async fn start_multi_path_router_mock() -> MockServer {
    let responder = |req: &Request| {
        let q = smart_query_from_request(req);
        let data = if let Some(sim) = q.get("simulate_swap_operations") {
            let ops = sim["operations"].as_array().map(|a| a.len()).unwrap_or(1);
            let amount = if ops <= 1 { "1000000" } else { "9000000" };
            json!({ "amount": amount })
        } else if q.get("hybrid_simulation").is_some() {
            let book = q["hybrid_simulation"]["hybrid"]["book_input"]
                .as_str()
                .unwrap_or("0")
                .parse::<u128>()
                .unwrap_or(0);
            let offer = q["hybrid_simulation"]["offer_asset"]["amount"]
                .as_str()
                .unwrap_or("0")
                .parse::<u128>()
                .unwrap_or(0);
            let ret = offer.saturating_add(book / 2);
            json!({
                "return_amount": ret.to_string(),
                "spread_amount": "0",
                "commission_amount": "0",
                "book_return_amount": "0",
                "pool_return_amount": ret.to_string(),
            })
        } else {
            json!(null)
        };
        ResponseTemplate::new(200).set_body_json(json!({ "data": data }))
    };
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/.+$"))
        .respond_with(responder)
        .mount(&server)
        .await;
    server
}

/// Smart-query stub: returns `{"data": ...}` for any wasm contract smart GET (router simulate, pool query, etc.).
pub async fn start_smart_query_data_mock(data: Value) -> MockServer {
    let server = MockServer::start().await;
    let body = json!({ "data": data });
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(body))
        .mount(&server)
        .await;
    server
}

fn pool_only_orderbook_responder(req: &Request) -> ResponseTemplate {
    let q = smart_query_from_request(req);
    let data = if q.get("pool").is_some() {
        json!({
            "assets": [
                {
                    "info": { "native_token": { "denom": "uluna" } },
                    "amount": "10000000000"
                },
                {
                    "info": { "token": { "contract_addr": "terra1quote" } },
                    "amount": "5000000000000"
                }
            ],
            "total_share": "1"
        })
    } else if q.get("get_fee_config").is_some() {
        json!({ "fee_config": { "fee_bps": 30 } })
    } else if q.get("order_book_head").is_some() {
        Value::Null
    } else if q.get("limit_order").is_some() {
        Value::Null
    } else {
        Value::Null
    };
    ResponseTemplate::new(200).set_body_json(json!({ "data": data }))
}

/// CG/CMC orderbook LCD stub: pool + fee + **empty** limit book (hybrid merge = pool-only).
pub async fn start_pool_query_mock() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/"))
        .respond_with(pool_only_orderbook_responder)
        .mount(&server)
        .await;
    server
}

/// Hybrid orderbook mock: pool reserves + one resting bid and ask for merge tests (**#220**).
pub async fn start_hybrid_orderbook_mock() -> MockServer {
    let responder = |req: &Request| {
        let q = smart_query_from_request(req);
        let data = if q.get("pool").is_some() {
            json!({
                "assets": [
                    {
                        "info": { "native_token": { "denom": "uluna" } },
                        "amount": "10000000000"
                    },
                    {
                        "info": { "token": { "contract_addr": "terra1quote" } },
                        "amount": "5000000000000"
                    }
                ],
                "total_share": "1"
            })
        } else if q.get("order_book_head").is_some() {
            let side = q["order_book_head"]["side"].as_str().unwrap_or("");
            if side == "bid" {
                json!(101u64)
            } else {
                json!(201u64)
            }
        } else if q.get("limit_order").is_some() {
            let id = q["limit_order"]["order_id"].as_u64().unwrap();
            if id == 101 {
                json!({
                    "order_id": 101,
                    "owner": "terra1bid",
                    "side": "bid",
                    "price": "999999.5",
                    "remaining": "2000000000",
                    "expires_at": null,
                    "prev": null,
                    "next": null
                })
            } else if id == 201 {
                json!({
                    "order_id": 201,
                    "owner": "terra1ask",
                    "side": "ask",
                    "price": "0.0000001",
                    "remaining": "500000",
                    "expires_at": null,
                    "prev": null,
                    "next": null
                })
            } else {
                Value::Null
            }
        } else {
            Value::Null
        };
        ResponseTemplate::new(200).set_body_json(json!({ "data": data }))
    };
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/"))
        .respond_with(responder)
        .mount(&server)
        .await;
    server
}

/// Book snapshot loop mock: pool + fee + small two-sided resting book; optional latest block height.
pub async fn start_book_snapshot_mock(block_height: Option<&str>) -> MockServer {
    let height = block_height.map(str::to_string);
    let book_responder = |req: &Request| {
        let q = smart_query_from_request(req);
        let data = if q.get("pool").is_some() {
            json!({
                "assets": [
                    { "info": { "native_token": { "denom": "uluna" } }, "amount": "1000000" },
                    { "info": { "token": { "contract_addr": "terra1ustctoken" } }, "amount": "2000000" }
                ],
                "total_share": "1"
            })
        } else if q.get("get_fee_config").is_some() {
            json!({ "fee_config": { "fee_bps": 25, "treasury": "terra1treasury" } })
        } else if q.get("order_book_head").is_some() {
            let side = q["order_book_head"]["side"].as_str().unwrap_or("");
            if side == "bid" {
                json!(102u64)
            } else {
                json!(201u64)
            }
        } else if q.get("limit_order").is_some() {
            let id = q["limit_order"]["order_id"].as_u64().unwrap();
            match id {
                102 => json!({
                    "order_id": 102,
                    "owner": "terra1bid2",
                    "side": "bid",
                    "price": "1.05",
                    "remaining": "100",
                    "expires_at": 999,
                    "prev": null,
                    "next": 101
                }),
                101 => json!({
                    "order_id": 101,
                    "owner": "terra1bid1",
                    "side": "bid",
                    "price": "1.05",
                    "remaining": "200",
                    "expires_at": null,
                    "prev": 102,
                    "next": 100
                }),
                100 => json!({
                    "order_id": 100,
                    "owner": "terra1bid0",
                    "side": "bid",
                    "price": "1.00",
                    "remaining": "300",
                    "expires_at": null,
                    "prev": 101,
                    "next": null
                }),
                201 => json!({
                    "order_id": 201,
                    "owner": "terra1ask1",
                    "side": "ask",
                    "price": "1.10",
                    "remaining": "400",
                    "expires_at": null,
                    "prev": null,
                    "next": 202
                }),
                202 => json!({
                    "order_id": 202,
                    "owner": "terra1ask2",
                    "side": "ask",
                    "price": "1.20",
                    "remaining": "500",
                    "expires_at": null,
                    "prev": 201,
                    "next": null
                }),
                _ => Value::Null,
            }
        } else {
            Value::Null
        };
        ResponseTemplate::new(200).set_body_json(json!({ "data": data }))
    };

    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/"))
        .respond_with(book_responder)
        .mount(&server)
        .await;
    if let Some(height) = height {
        Mock::given(method("GET"))
            .and(path_regex(r"^/cosmos/base/tendermint/v1beta1/blocks/latest$"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "block": { "header": { "height": height, "time": "2026-06-05T00:00:00Z" } }
            })))
            .mount(&server)
            .await;
    }
    server
}

/// Base URL for [`LcdClient::new`](cl8y_dex_indexer::lcd::LcdClient::new) (no trailing slash).
pub fn lcd_base_url(server: &MockServer) -> String {
    server.uri().trim_end_matches('/').to_string()
}
