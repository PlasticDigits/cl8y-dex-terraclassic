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

use serde_json::{json, Value};
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

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
