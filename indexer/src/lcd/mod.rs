mod chain_identity;
pub mod pair_discovery_state;
pub mod types;
#[cfg(test)]
pub(crate) use chain_identity::mock_chain_identity;
pub use chain_identity::{valid_chain_id, ChainIdentityHealth, EndpointIdentity};

use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::Engine;
use reqwest::Client;
use serde::de::DeserializeOwned;
use tokio::sync::RwLock;

use pair_discovery_state::PairDiscoveryState;
pub use types::*;

/// Limit the decoded HTTP body before allocating/deserializing an untrusted snapshot.
/// Enforce the limit while streaming as Content-Length can be absent or dishonest.
async fn bounded_response_text(
    mut response: reqwest::Response,
    max_bytes: usize,
) -> Result<String, LcdError> {
    if response
        .content_length()
        .is_some_and(|n| n > max_bytes as u64)
    {
        return Err(LcdError::Deserialize(format!(
            "LCD response exceeds {max_bytes} byte limit"
        )));
    }
    let mut body = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        if chunk.len() > max_bytes.saturating_sub(body.len()) {
            return Err(LcdError::Deserialize(format!(
                "LCD response exceeds {max_bytes} byte limit"
            )));
        }
        body.extend_from_slice(&chunk);
    }
    String::from_utf8(body).map_err(|_| LcdError::Deserialize("LCD response is not UTF-8".into()))
}

/// Redact LCD path for WARN logs (GitLab #379 L-04).
fn lcd_log_path(path: &str) -> &'static str {
    if path.starts_with("/cosmwasm/") {
        "/cosmwasm/..."
    } else if path.starts_with("/cosmos/tx/") {
        "/cosmos/tx/..."
    } else if path.starts_with("/cosmos/base/tendermint/") {
        "/cosmos/base/tendermint/..."
    } else {
        "<lcd-path>"
    }
}

#[derive(Debug, thiserror::Error)]
pub enum LcdError {
    #[error("All LCD endpoints failed: {0}")]
    AllEndpointsFailed(String),
    #[error("Chain identity rejected: {0}")]
    ChainIdentity(String),
    /// Deterministic CosmWasm smart-query reject (e.g. factory `pair not found`,
    /// or foreign contract `unknown variant \`pair\``). Not an LCD outage — do not
    /// fan out to other endpoints or WARN.
    #[error("LCD contract query rejected: {0}")]
    ContractQueryRejected(String),
    /// A provider's finite smart-query gas ceiling was exhausted. Do not repeat
    /// the same query on another endpoint or mark the provider unhealthy.
    #[error("LCD smart-query gas limit exceeded: {0}")]
    QueryGasLimitExceeded(String),
    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Deserialization error: {0}")]
    Deserialize(String),
    #[error("Base64 decode error: {0}")]
    Base64(String),
}

/// CosmWasm LCD returns HTTP 500 for smart-query contract errors. Same answer on
/// every LCD — not an upstream outage (GitLab #493).
///
/// Classic factory/foreign-pair phrases always classify. Broader wasmd envelope
/// markers (`query wasm contract failed`, `codespace wasm`) apply only on
/// `/cosmwasm/wasm/v1/contract/...` so plain infra 500s and non-wasm REST paths
/// still fan out / WARN.
fn is_deterministic_contract_query_reject(
    status: reqwest::StatusCode,
    path: &str,
    body: &str,
) -> bool {
    if status.as_u16() != 500 {
        return false;
    }
    let lower = body.to_ascii_lowercase();
    let classic = lower.contains("pair not found")
        || lower.contains("unknown variant")
        || lower.contains("error parsing into type");
    if classic {
        return true;
    }
    if !path.starts_with("/cosmwasm/wasm/v1/contract/") {
        return false;
    }
    lower.contains("query wasm contract failed") || lower.contains("codespace wasm")
}

fn should_cooldown_endpoint(status: reqwest::StatusCode) -> bool {
    let code = status.as_u16();
    // Rate limits and gateway failures — back off this endpoint and try others.
    code == 429 || code == 408 || code >= 502
}

fn is_query_gas_limit_exceeded(status: reqwest::StatusCode, path: &str, body: &str) -> bool {
    if status.as_u16() != 500
        || !path.starts_with("/cosmwasm/wasm/v1/contract/")
        || !path.contains("/smart/")
    {
        return false;
    }
    let lower = body.to_ascii_lowercase();
    lower.contains("out of gas") && lower.contains("gaswanted") && lower.contains("gasused")
}

#[derive(Clone)]
struct Endpoint {
    url: String,
    cooldown_until: Option<Instant>,
    observed_chain_id: Option<String>,
    verified: bool,
    identity_error: Option<String>,
    identity_checked_at: Option<String>,
}

#[derive(Clone)]
pub struct LcdClient {
    inner: Arc<LcdClientInner>,
}

struct LcdClientInner {
    endpoints: RwLock<Vec<Endpoint>>,
    client: Client,
    cooldown_duration: Duration,
    discovery_namespace: usize,
    expected_chain_id: String,
    pair_discovery_state: Arc<PairDiscoveryState>,
}

impl LcdClient {
    pub fn new(
        urls: Vec<String>,
        timeout_ms: u64,
        cooldown_ms: u64,
        expected_chain_id: impl Into<String>,
    ) -> Self {
        let expected_chain_id = expected_chain_id.into();
        assert!(
            valid_chain_id(&expected_chain_id),
            "invalid expected chain identifier"
        );
        static NEXT_NAMESPACE: std::sync::atomic::AtomicUsize =
            std::sync::atomic::AtomicUsize::new(1);
        let endpoints = urls
            .into_iter()
            .map(|url| Endpoint {
                url: url.trim_end_matches('/').to_string(),
                cooldown_until: None,
                observed_chain_id: None,
                verified: false,
                identity_error: None,
                identity_checked_at: None,
            })
            .collect();

        let client = Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .expect("failed to build reqwest client");

        Self {
            inner: Arc::new(LcdClientInner {
                endpoints: RwLock::new(endpoints),
                expected_chain_id,
                client,
                cooldown_duration: Duration::from_millis(cooldown_ms),
                discovery_namespace: NEXT_NAMESPACE
                    .fetch_add(1, std::sync::atomic::Ordering::Relaxed),
                pair_discovery_state: Arc::new(PairDiscoveryState::default()),
            }),
        }
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, LcdError> {
        self.get_with_body_limit(path, None).await
    }

    async fn get_with_body_limit<T: DeserializeOwned>(
        &self,
        path: &str,
        max_bytes: Option<usize>,
    ) -> Result<T, LcdError> {
        let mut errors = Vec::new();

        let endpoint_count = {
            let eps = self.inner.endpoints.read().await;
            eps.len()
        };

        for idx in 0..endpoint_count {
            let (url, is_cooled_down) = {
                let eps = self.inner.endpoints.read().await;
                let ep = &eps[idx];
                let cooled = ep
                    .cooldown_until
                    .map(|t| Instant::now() < t)
                    .unwrap_or(false);
                (ep.url.clone(), cooled)
            };

            if is_cooled_down {
                continue;
            }

            if let Err(e) = self.attest_endpoint(idx, &url).await {
                errors.push(e.to_string());
                continue;
            }

            let full_url = format!("{}{}", url, path);
            let log_path = lcd_log_path(path);
            match self.inner.client.get(&full_url).send().await {
                Ok(resp) => {
                    let status = resp.status();
                    let text = match max_bytes {
                        Some(limit) => match bounded_response_text(resp, limit).await {
                            Ok(body) => body,
                            Err(e) => {
                                errors.push(format!("endpoint[{idx}] {log_path}: {e}"));
                                // A hostile contract's size/encoding rejection is local
                                // to that query. It must not cool down healthy pair reads.
                                if matches!(&e, LcdError::Request(_)) {
                                    self.mark_failed(idx).await;
                                }
                                continue;
                            }
                        },
                        None if status.is_success() => {
                            resp.text().await.map_err(LcdError::Request)?
                        }
                        None => resp.text().await.unwrap_or_default(),
                    };
                    if !status.is_success() {
                        let body = text;
                        let body_snippet = body.chars().take(200).collect::<String>();
                        if is_query_gas_limit_exceeded(status, path, &body) {
                            tracing::debug!(
                                endpoint_idx = idx,
                                path = log_path,
                                "LCD smart-query gas limit exceeded"
                            );
                            return Err(LcdError::QueryGasLimitExceeded(body_snippet));
                        }
                        if is_deterministic_contract_query_reject(status, path, &body) {
                            // Expected on mainnet when foreign contracts emit compatible swap
                            // events but reject our pair/factory queries. Do not WARN or retry
                            // other LCD endpoints (same deterministic contract answer).
                            tracing::debug!(
                                endpoint_idx = idx,
                                status = %status,
                                path = log_path,
                                body_snippet = %body_snippet,
                                "LCD contract query rejected"
                            );
                            return Err(LcdError::ContractQueryRejected(body_snippet));
                        }
                        let msg =
                            format!("endpoint[{idx}] {log_path} returned {status}: {body_snippet}");
                        errors.push(msg);
                        if status.as_u16() == 429 {
                            // Public LCD mirrors rate-limit aggressively during catch-up.
                            // Cool down and try the next endpoint; keep logs at debug.
                            tracing::debug!(
                                endpoint_idx = idx,
                                status = %status,
                                path = log_path,
                                "LCD rate limited; cooling down endpoint"
                            );
                            self.mark_failed(idx).await;
                            continue;
                        }
                        tracing::warn!(
                            endpoint_idx = idx,
                            status = %status,
                            path = log_path,
                            "LCD upstream returned non-success status"
                        );
                        tracing::debug!(
                            endpoint_idx = idx,
                            path = log_path,
                            body_snippet = %body_snippet,
                            "LCD upstream error detail"
                        );
                        // Transient gateway/timeouts — cool down. Other 4xx/500 contract
                        // rejects keep the endpoint available for subsequent queries.
                        if should_cooldown_endpoint(status) {
                            self.mark_failed(idx).await;
                        }
                        continue;
                    }
                    // Node identity and block payload must agree independently (#1392).
                    // Historical blocks retain their original timestamps; identity has no age exemption.
                    if path.starts_with("/cosmos/base/tendermint/v1beta1/blocks/") {
                        if let Err(e) =
                            chain_identity::validate_block_chain(&text, self.expected_chain_id())
                        {
                            errors.push(format!("endpoint[{idx}]: {e}"));
                            self.mark_failed(idx).await;
                            let mut eps = self.inner.endpoints.write().await;
                            eps[idx].identity_error = Some(e.to_string());
                            continue;
                        }
                    }
                    match serde_json::from_str::<T>(&text) {
                        Ok(val) => return Ok(val),
                        Err(e) => {
                            let body_snippet = text.chars().take(200).collect::<String>();
                            tracing::warn!(
                                endpoint_idx = idx,
                                path = log_path,
                                "LCD response deserialization failed"
                            );
                            tracing::debug!(
                                endpoint_idx = idx,
                                path = log_path,
                                error = %e,
                                body_snippet = %body_snippet,
                                "LCD deserialize error detail"
                            );
                            let msg = format!("endpoint[{idx}] {log_path} deserialize error: {e}");
                            errors.push(msg);
                            continue;
                        }
                    }
                }
                Err(e) => {
                    tracing::warn!(
                        endpoint_idx = idx,
                        path = log_path,
                        error = %e,
                        "LCD request failed"
                    );
                    tracing::debug!(
                        endpoint_idx = idx,
                        path = log_path,
                        error = %e,
                        "LCD request failure detail"
                    );
                    let msg = format!("endpoint[{idx}] {log_path}: {e}");
                    errors.push(msg);
                    self.mark_failed(idx).await;
                }
            }
        }

        Err(LcdError::AllEndpointsFailed(errors.join("; ")))
    }

    async fn mark_failed(&self, idx: usize) {
        let mut eps = self.inner.endpoints.write().await;
        if let Some(ep) = eps.get_mut(idx) {
            ep.cooldown_until = Some(Instant::now() + self.inner.cooldown_duration);
            ep.verified = false;
        }
    }

    /// Process-local identity: isolates bounded discovery caches by LCD client/chain.
    pub(crate) fn cache_namespace(&self) -> usize {
        self.inner.discovery_namespace
    }

    /// Clones share discovery evidence; a newly configured client starts fresh.
    pub(crate) fn pair_discovery_state(&self) -> Arc<PairDiscoveryState> {
        Arc::clone(&self.inner.pair_discovery_state)
    }

    /// Fixed discovery outcomes; no event address or factory becomes a metric label.
    pub fn pair_discovery_counters(&self) -> pair_discovery_state::DiscoveryCounters {
        self.inner.pair_discovery_state.counters()
    }

    pub async fn get_code_data_hash(&self, code_id: u64) -> Result<String, LcdError> {
        let value: serde_json::Value = self
            .get(&format!("/cosmwasm/wasm/v1/code/{}", code_id))
            .await?;
        value
            .get("code_info")
            .and_then(|c| c.get("data_hash"))
            .and_then(|v| v.as_str())
            .map(str::to_owned)
            .ok_or_else(|| LcdError::Deserialize("missing code data hash".into()))
    }

    pub async fn query_contract<T: DeserializeOwned>(
        &self,
        contract_addr: &str,
        query: &serde_json::Value,
    ) -> Result<T, LcdError> {
        self.query_contract_with_body_limit(contract_addr, query, None)
            .await
    }

    /// Bounded response variant for pool/book mirrors. The cap covers success and error bodies.
    pub async fn query_contract_bounded<T: DeserializeOwned>(
        &self,
        contract_addr: &str,
        query: &serde_json::Value,
        max_bytes: usize,
    ) -> Result<T, LcdError> {
        self.query_contract_with_body_limit(contract_addr, query, Some(max_bytes))
            .await
    }

    async fn query_contract_with_body_limit<T: DeserializeOwned>(
        &self,
        contract_addr: &str,
        query: &serde_json::Value,
        max_bytes: Option<usize>,
    ) -> Result<T, LcdError> {
        let query_bytes = serde_json::to_vec(query)
            .map_err(|e| LcdError::Deserialize(format!("Failed to serialize query: {}", e)))?;
        let query_b64 = base64::engine::general_purpose::STANDARD.encode(&query_bytes);

        let path = format!(
            "/cosmwasm/wasm/v1/contract/{}/smart/{}",
            contract_addr, query_b64
        );

        let resp: SmartQueryResponse = self.get_with_body_limit(&path, max_bytes).await?;

        serde_json::from_value(resp.data)
            .map_err(|e| LcdError::Deserialize(format!("Failed to parse contract response: {}", e)))
    }

    /// Live wasm `ContractInfo` (code_id + admin) for community-tax attestation (#594).
    pub async fn get_contract_info(
        &self,
        contract_addr: &str,
    ) -> Result<crate::lcd::types::WasmContractInfo, LcdError> {
        let path = format!("/cosmwasm/wasm/v1/contract/{}", contract_addr);
        let env: WasmContractInfoEnvelope = self.get(&path).await?;
        Ok(env.contract_info)
    }

    /// Live wasm `code_id` for a contract instance (GitLab #585 F6 freeze probe).
    pub async fn get_contract_code_id(&self, contract_addr: &str) -> Result<u64, LcdError> {
        self.get_contract_info(contract_addr)
            .await?
            .code_id_u64()
            .ok_or_else(|| {
                LcdError::Deserialize("contract_info.code_id missing or not a u64".to_string())
            })
    }

    pub async fn get_latest_block_height(&self) -> Result<i64, LcdError> {
        self.get_latest_block_height_with_body_limit(None).await
    }

    pub async fn get_latest_block_height_bounded(&self, max_bytes: usize) -> Result<i64, LcdError> {
        self.get_latest_block_height_with_body_limit(Some(max_bytes))
            .await
    }

    async fn get_latest_block_height_with_body_limit(
        &self,
        max_bytes: Option<usize>,
    ) -> Result<i64, LcdError> {
        let resp: BlockResponse = self
            .get_with_body_limit("/cosmos/base/tendermint/v1beta1/blocks/latest", max_bytes)
            .await?;
        resp.block
            .header
            .height
            .parse::<i64>()
            .map_err(|e| LcdError::Deserialize(format!("Invalid block height: {}", e)))
    }

    pub async fn search_txs(
        &self,
        events: &[(&str, &str)],
        page: u32,
        limit: u32,
    ) -> Result<TxSearchResponse, LcdError> {
        // Cosmos SDK 0.50+ (terrad v4) GetTxsEvent: a single `query` param with conditions
        // joined by " AND ", a 1-based `page`, and a plain `limit`. The legacy `events=` and
        // `pagination.offset|limit` params were removed — on v4 `events=` returns HTTP 500
        // ("query cannot be empty") and the pagination.* params are silently ignored (every
        // page refetches the first). Encode spaces; the raw `=` inside a condition is fine.
        let query = events
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(" AND ")
            .replace(' ', "%20");

        let path = format!(
            "/cosmos/tx/v1beta1/txs?query={}&page={}&limit={}&order_by=1",
            query,
            page.max(1),
            limit,
        );

        self.get(&path).await
    }

    pub async fn get_block_at_height(&self, height: i64) -> Result<BlockResponse, LcdError> {
        let path = format!("/cosmos/base/tendermint/v1beta1/blocks/{}", height);
        self.get(&path).await
    }

    pub async fn get_block_hash(&self, height: i64) -> Result<String, LcdError> {
        let resp = self.get_block_at_height(height).await?;
        resp.block_id
            .map(|id| id.hash)
            .ok_or_else(|| LcdError::Deserialize(format!("block {} missing block_id.hash", height)))
    }

    /// Fetch all txs for a block, paginating until `pagination.total` is satisfied or pages are empty.
    pub async fn get_block_txs(
        &self,
        height: i64,
        page_limit: u32,
        max_pages: u32,
    ) -> Result<BlockTxsResult, LcdError> {
        let mut all_txs = Vec::new();
        let mut page = 1u32;
        let mut expected_total: Option<u64> = None;

        loop {
            if page > max_pages {
                return Err(LcdError::Deserialize(format!(
                    "block {} tx pagination exceeded max_pages={} (collected {} txs)",
                    height,
                    max_pages,
                    all_txs.len()
                )));
            }

            let resp = self
                .search_txs(&[("tx.height", &height.to_string())], page, page_limit)
                .await?;

            if expected_total.is_none() {
                // SDK 0.50+ returns the count at top-level `total` with `pagination: null`;
                // older LCDs only set `pagination.total`. Prefer the former, fall back to the latter.
                expected_total = resp
                    .total
                    .as_ref()
                    .or_else(|| resp.pagination.as_ref().and_then(|p| p.total.as_ref()))
                    .and_then(|t| t.parse::<u64>().ok());
            }

            let page_txs = resp.tx_responses.unwrap_or_default();
            let page_len = page_txs.len();
            all_txs.extend(page_txs);

            let done = page_len == 0
                || page_len < page_limit as usize
                || expected_total.is_some_and(|total| all_txs.len() as u64 >= total);

            if done {
                if let Some(total) = expected_total {
                    if all_txs.len() as u64 != total {
                        return Err(LcdError::Deserialize(format!(
                            "block {} incomplete tx fetch: LCD total={} collected={} pages={}",
                            height,
                            total,
                            all_txs.len(),
                            page
                        )));
                    }
                }
                return Ok(BlockTxsResult {
                    txs: all_txs,
                    page_count: page,
                });
            }

            page += 1;
        }
    }
}

#[derive(Debug, Clone)]
pub struct BlockTxsResult {
    pub txs: Vec<TxResponse>,
    pub page_count: u32,
}

#[cfg(test)]
mod tests {
    #[tokio::test]
    async fn bounded_body_caps_chunked_response_without_content_length() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut request = [0; 1024];
            socket.read(&mut request).await.unwrap();
            socket.write_all(
                b"HTTP/1.1 200 OK\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n14\r\nxxxxxxxxxxxxxxxxxxxx\r\n14\r\nxxxxxxxxxxxxxxxxxxxx\r\n0\r\n\r\n",
            ).await.unwrap();
        });
        let response = reqwest::Client::new()
            .get(format!("http://{address}"))
            .send()
            .await
            .unwrap();
        assert_eq!(response.content_length(), None);
        let error = super::bounded_response_text(response, 32)
            .await
            .unwrap_err();
        assert!(error.to_string().contains("exceeds 32 byte limit"));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn bounded_contract_response_rejects_huge_success_and_error_bodies() {
        use wiremock::matchers::method;
        use wiremock::{Mock, MockServer, ResponseTemplate};
        for status in [200, 500] {
            let server = MockServer::start().await;
            crate::lcd::mock_chain_identity(&server, "columbus-5").await;
            Mock::given(method("GET"))
                .respond_with(ResponseTemplate::new(status).set_body_string("x".repeat(65_537)))
                .mount(&server)
                .await;
            let lcd = super::LcdClient::new(vec![server.uri()], 1000, 1000, "columbus-5");
            let result = lcd
                .query_contract_bounded::<serde_json::Value>(
                    "terra1pair",
                    &serde_json::json!({ "pool": {} }),
                    65_536,
                )
                .await;
            let error = result.unwrap_err().to_string();
            assert!(error.contains("exceeds 65536 byte limit"));
            assert!(
                !error.contains(&"x".repeat(20)),
                "oversized bodies never enter errors/logs"
            );
            server.reset().await;
            crate::lcd::mock_chain_identity(&server, "columbus-5").await;
            Mock::given(method("GET"))
                .respond_with(
                    ResponseTemplate::new(200)
                        .set_body_json(serde_json::json!({"data": {"healthy": true}})),
                )
                .mount(&server)
                .await;
            let healthy = lcd
                .query_contract_bounded::<serde_json::Value>(
                    "terra1healthy",
                    &serde_json::json!({"pool": {}}),
                    65_536,
                )
                .await
                .expect("one oversized pair cannot cool down healthy pair queries");
            assert_eq!(healthy["healthy"], true);
        }
    }

    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path, query_param};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    fn tx_at(height: i64, idx: u32) -> serde_json::Value {
        json!({
            "height": height.to_string(),
            "txhash": format!("hash{}_{}", height, idx),
            "timestamp": "2024-01-01T00:00:00Z",
            "logs": [],
            "events": []
        })
    }

    #[tokio::test]
    async fn get_block_txs_single_page() {
        let server = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server, "columbus-5").await;
        let height = 42i64;
        Mock::given(method("GET"))
            .and(path("/cosmos/tx/v1beta1/txs"))
            .and(query_param("query", format!("tx.height={}", height)))
            .and(query_param("page", "1"))
            .and(query_param("limit", "100"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "tx_responses": [tx_at(height, 1), tx_at(height, 2)],
                "total": "2"
            })))
            .mount(&server)
            .await;

        let lcd = LcdClient::new(vec![server.uri()], 5000, 30000, "columbus-5");
        let result = lcd.get_block_txs(height, 100, 10).await.unwrap();
        assert_eq!(result.txs.len(), 2);
        assert_eq!(result.page_count, 1);
    }

    #[tokio::test]
    async fn get_block_txs_multi_page() {
        let server = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server, "columbus-5").await;
        let height = 99i64;
        let page1: Vec<_> = (0..100).map(|i| tx_at(height, i)).collect();
        let page2: Vec<_> = (100..150).map(|i| tx_at(height, i)).collect();

        Mock::given(method("GET"))
            .and(path("/cosmos/tx/v1beta1/txs"))
            .and(query_param("page", "1"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "tx_responses": page1,
                "total": "150"
            })))
            .mount(&server)
            .await;

        Mock::given(method("GET"))
            .and(path("/cosmos/tx/v1beta1/txs"))
            .and(query_param("page", "2"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "tx_responses": page2,
                "total": "150"
            })))
            .mount(&server)
            .await;

        let lcd = LcdClient::new(vec![server.uri()], 5000, 30000, "columbus-5");
        let result = lcd.get_block_txs(height, 100, 10).await.unwrap();
        assert_eq!(result.txs.len(), 150);
        assert_eq!(result.page_count, 2);
    }

    #[tokio::test]
    async fn get_block_txs_rejects_incomplete_total() {
        let server = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server, "columbus-5").await;
        Mock::given(method("GET"))
            .and(path("/cosmos/tx/v1beta1/txs"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "tx_responses": [tx_at(1, 0)],
                "total": "150"
            })))
            .mount(&server)
            .await;

        let lcd = LcdClient::new(vec![server.uri()], 5000, 30000, "columbus-5");
        let err = lcd.get_block_txs(1, 100, 1).await.unwrap_err();
        assert!(err.to_string().contains("incomplete tx fetch"));
    }

    #[tokio::test]
    async fn rate_limit_429_cools_down_and_fails_over() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use wiremock::matchers::path_regex;

        let server_a = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server_a, "columbus-5").await;
        let server_b = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server_b, "columbus-5").await;
        let hits_b = Arc::new(AtomicUsize::new(0));
        let b = hits_b.clone();

        Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
            .respond_with(ResponseTemplate::new(429).set_body_string("Too Many Requests"))
            .mount(&server_a)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
            .respond_with(move |_req: &wiremock::Request| {
                b.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(200).set_body_json(json!({ "data": { "ok": true } }))
            })
            .mount(&server_b)
            .await;

        // Short cooldown so a follow-up request still skips endpoint A.
        let lcd = LcdClient::new(
            vec![server_a.uri(), server_b.uri()],
            5000,
            60_000,
            "columbus-5",
        );
        let val: serde_json::Value = lcd
            .query_contract("terra1pair", &json!({"pair": {}}))
            .await
            .expect("should fail over to endpoint B");
        assert_eq!(val, json!({ "ok": true }));
        assert_eq!(hits_b.load(Ordering::SeqCst), 1);

        // Second call should not re-hit cooled-down A; B serves again.
        let _val: serde_json::Value = lcd
            .query_contract("terra1pair", &json!({"pair": {}}))
            .await
            .unwrap();
        assert_eq!(hits_b.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn classify_deterministic_cosmwasm_rejects() {
        let ok = reqwest::StatusCode::OK;
        let s500 = reqwest::StatusCode::INTERNAL_SERVER_ERROR;
        let s502 = reqwest::StatusCode::BAD_GATEWAY;
        let cw = "/cosmwasm/wasm/v1/contract/terra1x/smart/abc";
        let tx = "/cosmos/tx/v1beta1/txs?query=tx.height=1";

        assert!(!is_deterministic_contract_query_reject(
            ok,
            cw,
            r#"{"message":"query wasm contract failed"}"#
        ));
        assert!(!is_deterministic_contract_query_reject(
            s502,
            cw,
            r#"{"message":"query wasm contract failed"}"#
        ));
        assert!(is_deterministic_contract_query_reject(
            s500,
            cw,
            r#"{"message":"Generic error: pair not found: query wasm contract failed"}"#
        ));
        assert!(is_deterministic_contract_query_reject(
            s500,
            cw,
            r#"{"message":"Error parsing into type: unknown variant `pair`"}"#
        ));
        assert!(is_deterministic_contract_query_reject(
            s500,
            cw,
            r#"{"code":2,"message":"codespace wasm code 9: query wasm contract failed: key not found"}"#
        ));
        assert!(is_deterministic_contract_query_reject(
            s500,
            cw,
            r#"{"message":"query wasm contract failed"}"#
        ));
        // Infra-looking body on cosmwasm path without wasm markers → not classified.
        assert!(!is_deterministic_contract_query_reject(
            s500,
            cw,
            "internal LCD failure"
        ));
        // Non-cosmwasm path: classic phrases still classify; bare wasmd phrase does not.
        assert!(is_deterministic_contract_query_reject(
            s500,
            tx,
            r#"{"message":"pair not found"}"#
        ));
        assert!(!is_deterministic_contract_query_reject(
            s500,
            tx,
            r#"{"message":"query cannot be empty"}"#
        ));
    }

    #[tokio::test]
    async fn wasm_query_reject_does_not_fan_out() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use wiremock::matchers::path_regex;

        let server_a = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server_a, "columbus-5").await;
        let server_b = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server_b, "columbus-5").await;
        let hits_b = Arc::new(AtomicUsize::new(0));
        let b = hits_b.clone();

        Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
            .respond_with(ResponseTemplate::new(500).set_body_json(json!({
                "code": 2,
                "message": "codespace wasm code 9: query wasm contract failed: key not found",
                "details": []
            })))
            .mount(&server_a)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
            .respond_with(move |_req: &wiremock::Request| {
                b.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(200).set_body_json(json!({ "data": { "ok": true } }))
            })
            .mount(&server_b)
            .await;

        let lcd = LcdClient::new(
            vec![server_a.uri(), server_b.uri()],
            5000,
            30_000,
            "columbus-5",
        );
        let err = lcd
            .query_contract::<serde_json::Value>(
                "terra1pair",
                &json!({"limit_order": {"order_id": 1}}),
            )
            .await
            .unwrap_err();
        assert!(
            matches!(err, LcdError::ContractQueryRejected(_)),
            "unexpected: {err}"
        );
        assert_eq!(hits_b.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn wasm_query_gas_limit_does_not_fan_out_or_cool_down() {
        use wiremock::matchers::path_regex;

        let server_a = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server_a, "columbus-5").await;
        let server_b = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server_b, "columbus-5").await;
        let gas_rejection = Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
            .respond_with(ResponseTemplate::new(500).set_body_json(json!({
                "code": 2,
                "message": "out of gas in location: wasm contract; gasWanted: 3000000, gasUsed: 3043317",
            })))
            .expect(1)
            .mount_as_scoped(&server_a)
            .await;
        Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({ "data": { "ok": true } })),
            )
            .expect(0)
            .mount(&server_b)
            .await;
        let lcd = LcdClient::new(
            vec![server_a.uri(), server_b.uri()],
            5000,
            30_000,
            "columbus-5",
        );
        let error = lcd
            .query_contract::<serde_json::Value>(
                "terra1router",
                &json!({"simulate_swap_operations": {}}),
            )
            .await
            .unwrap_err();
        assert!(matches!(error, LcdError::QueryGasLimitExceeded(_)));
        drop(gas_rejection);
        Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
            .respond_with(
                ResponseTemplate::new(200).set_body_json(json!({ "data": { "ok": true } })),
            )
            .expect(1)
            .mount(&server_a)
            .await;
        let quote: serde_json::Value = lcd
            .query_contract("terra1router", &json!({"config": {}}))
            .await
            .unwrap();
        assert_eq!(quote["ok"], true);
    }

    #[test]
    fn wasm_query_gas_limit_classification_is_scoped() {
        let path = "/cosmwasm/wasm/v1/contract/terra1router/smart/abc";
        let body = "out of gas in location: wasm contract; gasWanted: 3000000, gasUsed: 3043317";
        let status = reqwest::StatusCode::INTERNAL_SERVER_ERROR;
        assert!(is_query_gas_limit_exceeded(status, path, body));
        assert!(!is_query_gas_limit_exceeded(
            reqwest::StatusCode::BAD_GATEWAY,
            path,
            body
        ));
        assert!(!is_query_gas_limit_exceeded(
            status,
            "/cosmos/tx/v1beta1/txs",
            body
        ));
        assert!(!is_query_gas_limit_exceeded(
            status,
            "/cosmwasm/wasm/v1/contract/terra1router/raw/abc",
            body
        ));
        assert!(!is_query_gas_limit_exceeded(
            status,
            path,
            "out of gas infrastructure failure"
        ));
        assert!(!is_query_gas_limit_exceeded(
            status,
            path,
            "gasWanted: 3000000, gasUsed: 2000000"
        ));
    }

    #[tokio::test]
    async fn infra_500_still_fans_out() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use wiremock::matchers::path_regex;

        let server_a = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server_a, "columbus-5").await;
        let server_b = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server_b, "columbus-5").await;
        let hits_b = Arc::new(AtomicUsize::new(0));
        let b = hits_b.clone();

        Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
            .respond_with(ResponseTemplate::new(500).set_body_string("internal LCD failure"))
            .mount(&server_a)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
            .respond_with(move |_req: &wiremock::Request| {
                b.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(200).set_body_json(json!({ "data": { "ok": true } }))
            })
            .mount(&server_b)
            .await;

        let lcd = LcdClient::new(
            vec![server_a.uri(), server_b.uri()],
            5000,
            30_000,
            "columbus-5",
        );
        let val: serde_json::Value = lcd
            .query_contract("terra1pair", &json!({"pair": {}}))
            .await
            .expect("infra 500 should fail over");
        assert_eq!(val, json!({ "ok": true }));
        assert_eq!(hits_b.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn gateway_502_cools_down_and_fails_over() {
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::Arc;
        use wiremock::matchers::path_regex;

        let server_a = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server_a, "columbus-5").await;
        let server_b = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server_b, "columbus-5").await;
        let hits_b = Arc::new(AtomicUsize::new(0));
        let b = hits_b.clone();

        Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
            .respond_with(ResponseTemplate::new(502).set_body_string("bad gateway"))
            .mount(&server_a)
            .await;

        Mock::given(method("GET"))
            .and(path_regex(r"/cosmwasm/wasm/v1/contract/.+/smart/.+"))
            .respond_with(move |_req: &wiremock::Request| {
                b.fetch_add(1, Ordering::SeqCst);
                ResponseTemplate::new(200).set_body_json(json!({ "data": { "ok": true } }))
            })
            .mount(&server_b)
            .await;

        let lcd = LcdClient::new(
            vec![server_a.uri(), server_b.uri()],
            5000,
            60_000,
            "columbus-5",
        );
        let val: serde_json::Value = lcd
            .query_contract("terra1pair", &json!({"pair": {}}))
            .await
            .expect("502 should fail over");
        assert_eq!(val, json!({ "ok": true }));
        assert_eq!(hits_b.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn get_contract_code_id_parses_string_and_number() {
        let server = MockServer::start().await;
        crate::lcd::mock_chain_identity(&server, "columbus-5").await;
        Mock::given(method("GET"))
            .and(path("/cosmwasm/wasm/v1/contract/terra1token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "contract_info": { "code_id": "10184", "creator": "terra1x", "admin": "", "label": "t" }
            })))
            .mount(&server)
            .await;
        let lcd = LcdClient::new(vec![server.uri()], 5000, 30000, "columbus-5");
        assert_eq!(
            lcd.get_contract_code_id("terra1token").await.unwrap(),
            10184
        );
    }
}
