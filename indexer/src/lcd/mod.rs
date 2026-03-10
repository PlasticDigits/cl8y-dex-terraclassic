pub mod types;

use std::sync::Arc;
use std::time::{Duration, Instant};

use base64::Engine;
use reqwest::Client;
use serde::de::DeserializeOwned;
use tokio::sync::RwLock;

pub use types::*;

#[derive(Debug, thiserror::Error)]
pub enum LcdError {
    #[error("All LCD endpoints failed: {0}")]
    AllEndpointsFailed(String),
    #[error("Request error: {0}")]
    Request(#[from] reqwest::Error),
    #[error("Deserialization error: {0}")]
    Deserialize(String),
    #[error("Base64 decode error: {0}")]
    Base64(String),
}

#[derive(Clone)]
struct Endpoint {
    url: String,
    cooldown_until: Option<Instant>,
}

#[derive(Clone)]
pub struct LcdClient {
    inner: Arc<LcdClientInner>,
}

struct LcdClientInner {
    endpoints: RwLock<Vec<Endpoint>>,
    client: Client,
    cooldown_duration: Duration,
}

impl LcdClient {
    pub fn new(urls: Vec<String>, timeout_ms: u64, cooldown_ms: u64) -> Self {
        let endpoints = urls
            .into_iter()
            .map(|url| Endpoint {
                url: url.trim_end_matches('/').to_string(),
                cooldown_until: None,
            })
            .collect();

        let client = Client::builder()
            .timeout(Duration::from_millis(timeout_ms))
            .build()
            .expect("failed to build reqwest client");

        Self {
            inner: Arc::new(LcdClientInner {
                endpoints: RwLock::new(endpoints),
                client,
                cooldown_duration: Duration::from_millis(cooldown_ms),
            }),
        }
    }

    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T, LcdError> {
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

            let full_url = format!("{}{}", url, path);
            match self.inner.client.get(&full_url).send().await {
                Ok(resp) => {
                    let status = resp.status();
                    if !status.is_success() {
                        let body = resp.text().await.unwrap_or_default();
                        let msg = format!("{} returned {}: {}", full_url, status, body);
                        tracing::warn!("{}", msg);
                        errors.push(msg);
                        self.mark_failed(idx).await;
                        continue;
                    }
                    let text = resp.text().await.map_err(LcdError::Request)?;
                    match serde_json::from_str::<T>(&text) {
                        Ok(val) => return Ok(val),
                        Err(e) => {
                            let msg = format!(
                                "Deserialize error from {}: {} (body: {})",
                                full_url,
                                e,
                                &text[..text.len().min(200)]
                            );
                            tracing::warn!("{}", msg);
                            errors.push(msg);
                            continue;
                        }
                    }
                }
                Err(e) => {
                    let msg = format!("{}: {}", full_url, e);
                    tracing::warn!("LCD request failed: {}", msg);
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
        }
    }

    pub async fn query_contract<T: DeserializeOwned>(
        &self,
        contract_addr: &str,
        query: &serde_json::Value,
    ) -> Result<T, LcdError> {
        let query_bytes = serde_json::to_vec(query)
            .map_err(|e| LcdError::Deserialize(format!("Failed to serialize query: {}", e)))?;
        let query_b64 = base64::engine::general_purpose::STANDARD.encode(&query_bytes);

        let path = format!(
            "/cosmwasm/wasm/v1/contract/{}/smart/{}",
            contract_addr, query_b64
        );

        let resp: SmartQueryResponse = self.get(&path).await?;

        serde_json::from_value(resp.data)
            .map_err(|e| LcdError::Deserialize(format!("Failed to parse contract response: {}", e)))
    }

    pub async fn get_latest_block_height(&self) -> Result<i64, LcdError> {
        let resp: BlockResponse = self
            .get("/cosmos/base/tendermint/v1beta1/blocks/latest")
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
        let event_queries: Vec<String> = events
            .iter()
            .map(|(k, v)| format!("{}='{}'", k, v))
            .collect();
        let events_param = event_queries.join("&events=");

        let path = format!(
            "/cosmos/tx/v1beta1/txs?events={}&pagination.offset={}&pagination.limit={}&order_by=ORDER_BY_ASC",
            events_param,
            (page.saturating_sub(1)) * limit,
            limit,
        );

        self.get(&path).await
    }

    pub async fn get_block_txs(&self, height: i64) -> Result<TxSearchResponse, LcdError> {
        self.search_txs(&[("tx.height", &height.to_string())], 1, 100)
            .await
    }
}
