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

/// LCD stub for hybrid route optimization: `HybridSimulation`, pool `Simulation`, router `simulate_swap_operations`.
pub async fn start_hybrid_route_optimizer_mock() -> MockServer {
    let responder = |req: &Request| {
        let q = smart_query_from_request(req);
        let data = if q.get("simulate_swap_operations").is_some() {
            json!({ "amount": "8888888" })
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
        } else if q.get("simulation").is_some() {
            let offer = q["simulation"]["offer_asset"]["amount"]
                .as_str()
                .unwrap_or("0")
                .parse::<u128>()
                .unwrap_or(0);
            json!({
                "return_amount": (offer / 2).to_string(),
                "spread_amount": "0",
                "commission_amount": "0",
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

/// Starts a mock server that answers any `GET .../cosmwasm/wasm/v1/contract/*/smart/*` with a valid [`PoolResponse`](cl8y_dex_indexer::lcd::PoolResponse)-shaped JSON.
pub async fn start_pool_query_mock() -> MockServer {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "data": {
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
            }
        })))
        .mount(&server)
        .await;
    server
}

/// Base URL for [`LcdClient::new`](cl8y_dex_indexer::lcd::LcdClient::new) (no trailing slash).
pub fn lcd_base_url(server: &MockServer) -> String {
    server.uri().trim_end_matches('/').to_string()
}
