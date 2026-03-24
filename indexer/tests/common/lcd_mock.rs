//! Minimal LCD HTTP stub for cosmwasm `pool` smart queries (orderbook simulation).

use serde_json::json;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

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
