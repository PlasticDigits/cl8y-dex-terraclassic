//! #1392: each upstream attests the configured network before returning chain data.
//! No cached handshake is trusted: URL rotation and recovery require a fresh read.
use super::{bounded_response_text, LcdClient, LcdError};
use serde::Serialize;
use utoipa::ToSchema;

pub const NODE_INFO_PATH: &str = "/cosmos/base/tendermint/v1beta1/node_info";

pub fn valid_chain_id(id: &str) -> bool {
    !id.is_empty()
        && id.len() <= 128
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Every block payload must independently agree with the configured network.
pub(super) fn validate_block_chain(body: &str, expected: &str) -> Result<(), LcdError> {
    let value: serde_json::Value = serde_json::from_str(body)
        .map_err(|_| LcdError::ChainIdentity("malformed block response".into()))?;
    let observed = value
        .pointer("/block/header/chain_id")
        .and_then(|v| v.as_str())
        .filter(|id| valid_chain_id(id))
        .ok_or_else(|| {
            LcdError::ChainIdentity("block header missing or malformed chain_id".into())
        })?;
    if observed != expected {
        return Err(LcdError::ChainIdentity(format!(
            "block expected {expected}, observed {observed}"
        )));
    }
    Ok(())
}

/// Verify explicit historical block and transaction filters in the body. Live
/// response headers do not establish the height of a historical block/tx page.
pub(super) fn validate_requested_height(body: &str, path: &str) -> Result<bool, LcdError> {
    const BLOCKS: &str = "/cosmos/base/tendermint/v1beta1/blocks/";
    let block_height = path
        .strip_prefix(BLOCKS)
        .filter(|height| *height != "latest");
    let tx_height = if path.starts_with("/cosmos/tx/v1beta1/txs?") {
        reqwest::Url::parse(&format!("http://lcd.invalid{path}"))
            .ok()
            .and_then(|url| {
                url.query_pairs()
                    .find(|(key, _)| key == "query")
                    .and_then(|(_, query)| {
                        query
                            .split(" AND ")
                            .find_map(|term| term.strip_prefix("tx.height=").map(str::to_owned))
                    })
            })
    } else {
        None
    };
    let Some(raw_height) = block_height.or(tx_height.as_deref()) else {
        return Ok(false);
    };
    let wanted = raw_height
        .parse::<i64>()
        .ok()
        .filter(|height| *height > 0)
        .ok_or(LcdError::HistoricalHeightUnavailable)?;
    let value: serde_json::Value =
        serde_json::from_str(body).map_err(|_| LcdError::HistoricalHeightUnavailable)?;
    let height =
        |value: &serde_json::Value| value.as_str().and_then(|height| height.parse::<i64>().ok());
    if block_height.is_some() {
        if value.pointer("/block/header/height").and_then(height) != Some(wanted) {
            return Err(LcdError::HistoricalHeightUnavailable);
        }
    } else if let Some(txs) = value.get("tx_responses").filter(|value| !value.is_null()) {
        let txs = txs
            .as_array()
            .ok_or(LcdError::HistoricalHeightUnavailable)?;
        if txs.iter().any(|tx| height(&tx["height"]) != Some(wanted)) {
            return Err(LcdError::HistoricalHeightUnavailable);
        }
    } else {
        // SDK 0.50 may return null for an empty result. Missing body/count is
        // not proof of an empty economic block; accept only explicit zero.
        let total = value
            .get("total")
            .filter(|value| !value.is_null())
            .or_else(|| value.pointer("/pagination/total"))
            .and_then(serde_json::Value::as_str)
            .and_then(|value| value.parse::<u64>().ok());
        if total != Some(0) {
            return Err(LcdError::HistoricalHeightUnavailable);
        }
    }
    Ok(true)
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct EndpointIdentity {
    pub endpoint_index: usize,
    pub observed_chain_id: Option<String>,
    pub verified: bool,
    pub last_error: Option<String>,
    pub checked_at: Option<String>,
    pub provider: super::ProviderCounters,
}

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct ChainIdentityHealth {
    pub expected_chain_id: String,
    pub cache_namespace: String,
    pub endpoints: Vec<EndpointIdentity>,
}

impl LcdClient {
    /// Explicit chain identity is included alongside per-client isolation in every shared cache key.
    pub fn chain_cache_namespace(&self) -> String {
        format!(
            "{}:client:{}",
            self.inner.expected_chain_id, self.inner.discovery_namespace
        )
    }

    pub fn expected_chain_id(&self) -> &str {
        &self.inner.expected_chain_id
    }

    pub async fn chain_identity_health(&self) -> ChainIdentityHealth {
        let endpoints = self
            .inner
            .endpoints
            .read()
            .await
            .iter()
            .enumerate()
            .map(|(idx, ep)| EndpointIdentity {
                endpoint_index: idx,
                observed_chain_id: ep.observed_chain_id.clone(),
                verified: ep.verified,
                last_error: ep.identity_error.clone(),
                checked_at: ep.identity_checked_at.clone(),
                provider: ep.provider.counters(),
            })
            .collect();
        ChainIdentityHealth {
            expected_chain_id: self.inner.expected_chain_id.clone(),
            cache_namespace: self.chain_cache_namespace(),
            endpoints,
        }
    }

    /// Probe configured endpoints under the same admission and deadline bounds as data reads.
    pub async fn attest_endpoints(&self) -> Result<(), LcdError> {
        self.probe_identities(false).await
    }

    /// Used before serving previously computed quote/discovery caches as well as live reads.
    pub async fn require_verified_chain(&self) -> Result<(), LcdError> {
        self.probe_identities(true).await
    }

    async fn probe_identities(&self, stop_on_success: bool) -> Result<(), LcdError> {
        use std::time::Duration;
        let deadline = tokio::time::Instant::now()
            + Duration::from_millis(self.inner.provider_policy.deadline_ms);
        loop {
            let (result, attempts) = self.probe_identity_scan(stop_on_success, deadline).await;
            if self.ingestion_budget.is_some()
                && attempts == 0
                && matches!(&result, Err(LcdError::AdmissionDeferred))
                && tokio::time::Instant::now() < deadline
            {
                tokio::time::sleep(
                    Duration::from_millis(50)
                        .min(deadline.saturating_duration_since(tokio::time::Instant::now())),
                )
                .await;
                if tokio::time::Instant::now() >= deadline {
                    return result;
                }
                continue;
            }
            return result;
        }
    }

    async fn probe_identity_scan(
        &self,
        stop_on_success: bool,
        deadline: tokio::time::Instant,
    ) -> (Result<(), LcdError>, usize) {
        use std::time::Duration;
        let count = self.inner.endpoints.read().await.len();
        let mut verified = false;
        let mut errors = Vec::new();
        let mut integrity_error = None;
        let mut attempts = 0;
        let mut capacity_deferred = false;
        for idx in 0..count {
            let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());
            if remaining.is_zero() {
                break;
            }
            let ep = self.inner.endpoints.read().await[idx].clone();
            let mut permit = match ep.provider.admit() {
                Ok(permit) => permit,
                Err(reason) => {
                    capacity_deferred |= reason.is_capacity();
                    continue;
                }
            };
            attempts += 1;
            let timeout = remaining.min(Duration::from_millis(
                self.inner.provider_policy.attempt_timeout_ms,
            ));
            let outcome = tokio::time::timeout(timeout, self.attest_endpoint(idx, &ep.url)).await;
            permit.completed();
            match outcome {
                Ok(Ok(())) => {
                    permit.succeeded();
                    verified = true;
                    if stop_on_success {
                        return (Ok(()), attempts);
                    }
                }
                Ok(Err(
                    e @ (LcdError::ChainIdentity(_)
                    | LcdError::Deserialize(_)
                    | LcdError::HttpRejected(_)),
                )) => {
                    errors.push(format!("endpoint[{idx}] identity integrity rejected"));
                    integrity_error = Some(e);
                }
                Ok(Err(e)) => errors.push(e.to_string()),
                Err(_) => {
                    self.mark_failed(idx).await;
                    errors.push(format!("endpoint[{idx}] identity deadline exceeded"));
                }
            }
            if attempts >= self.inner.provider_policy.max_attempts {
                break;
            }
        }
        let result = if verified {
            Ok(())
        } else if let Some(error) = integrity_error {
            Err(error)
        } else if count > 0 && attempts == 0 && capacity_deferred {
            Err(LcdError::AdmissionDeferred)
        } else {
            Err(LcdError::AllEndpointsFailed(if errors.is_empty() {
                "no verified endpoint".into()
            } else {
                errors.join("; ")
            }))
        };
        (result, attempts)
    }

    pub(super) async fn attest_endpoint(&self, idx: usize, url: &str) -> Result<(), LcdError> {
        let mut observed = None;
        let mut retry_after = None;
        let result = async {
            let resp = self
                .inner
                .client
                .get(format!("{url}{NODE_INFO_PATH}"))
                .send()
                .await?;
            retry_after = super::provider_pool::retry_after(
                resp.headers(),
                std::time::Duration::from_millis(self.inner.provider_policy.max_backoff_ms),
            );
            if !resp.status().is_success() {
                if !super::should_cooldown_endpoint(resp.status()) {
                    return Err(LcdError::HttpRejected(resp.status().as_u16()));
                }
                return Err(LcdError::AllEndpointsFailed(format!(
                    "endpoint[{idx}] node_info unavailable ({})",
                    resp.status()
                )));
            }
            let body = bounded_response_text(resp, 64 * 1024).await?;
            let value: serde_json::Value = serde_json::from_str(&body).map_err(|_| {
                LcdError::ChainIdentity(format!("endpoint[{idx}] malformed node_info"))
            })?;
            observed = value
                .pointer("/default_node_info/network")
                .and_then(|v| v.as_str())
                .filter(|id| valid_chain_id(id))
                .map(str::to_string);
            let chain = observed
                .as_deref()
                .filter(|id| valid_chain_id(id))
                .ok_or_else(|| {
                    LcdError::ChainIdentity(format!("endpoint[{idx}] missing or malformed network"))
                })?;
            if chain != self.inner.expected_chain_id {
                return Err(LcdError::ChainIdentity(format!(
                    "endpoint[{idx}] expected {}, observed {chain}",
                    self.inner.expected_chain_id
                )));
            }
            Ok(())
        }
        .await;
        let result = result.map_err(|e| match e {
            LcdError::Request(_) => LcdError::AllEndpointsFailed(format!(
                "endpoint[{idx}] node_info transport unavailable"
            )),
            other => other,
        });
        let mut eps = self.inner.endpoints.write().await;
        let ep = &mut eps[idx];
        ep.identity_checked_at = Some(chrono::Utc::now().to_rfc3339());
        ep.observed_chain_id = observed;
        ep.verified = result.is_ok();
        ep.identity_error = result.as_ref().err().map(|e| match e {
            LcdError::Request(_) => format!("endpoint[{idx}] node_info transport unavailable"),
            _ => e.to_string(),
        });
        if result.is_err() {
            ep.provider
                .failed(self.inner.cooldown_duration, retry_after);
            tracing::warn!(endpoint_index = idx, expected_chain_id = %self.inner.expected_chain_id,
                observed_chain_id = ?ep.observed_chain_id, "LCD endpoint quarantined: chain identity failed");
        }
        result
    }
}

/// Strict test fixture: clients still perform the production HTTP attestation.
#[cfg(test)]
pub(crate) async fn mock_chain_identity(server: &wiremock::MockServer, chain: &str) {
    wiremock::Mock::given(wiremock::matchers::method("GET"))
        .and(wiremock::matchers::path(NODE_INFO_PATH))
        .respond_with(
            wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "default_node_info":{"network":chain}
            })),
        )
        .with_priority(1)
        .mount(server)
        .await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    async fn identity(server: &MockServer, body: serde_json::Value) {
        Mock::given(method("GET"))
            .and(path(NODE_INFO_PATH))
            .respond_with(ResponseTemplate::new(200).set_body_json(body))
            .mount(server)
            .await;
    }
    async fn data(server: &MockServer) {
        Mock::given(method("GET"))
            .and(path("/data"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"answer":42})))
            .mount(server)
            .await;
    }
    fn client(server: &MockServer, chain: &str) -> LcdClient {
        LcdClient::new(vec![server.uri()], 500, 0, chain)
    }
    async fn data_hits(server: &MockServer) -> usize {
        server
            .received_requests()
            .await
            .unwrap()
            .iter()
            .filter(|r| r.url.path() == "/data")
            .count()
    }

    #[test]
    fn recovery1464_missing_tx_body_cannot_become_an_empty_block() {
        let path = "/cosmos/tx/v1beta1/txs?query=tx.height=41&page=1&limit=100";
        for body in [
            json!({}),
            json!({"tx_responses":null}),
            json!({"total":"1"}),
            json!({"tx_responses":null,"total":"invalid"}),
        ] {
            assert!(matches!(
                validate_requested_height(&body.to_string(), path),
                Err(LcdError::HistoricalHeightUnavailable)
            ));
        }
        for body in [
            json!({"tx_responses":[]}),
            json!({"tx_responses":null,"total":"0"}),
            json!({"pagination":{"total":"0"}}),
        ] {
            assert_eq!(
                validate_requested_height(&body.to_string(), path).unwrap(),
                true
            );
        }
    }

    #[tokio::test]
    async fn correct_and_explicit_local_chain_can_read() {
        for chain in ["columbus-5", "localterra"] {
            let server = MockServer::start().await;
            identity(&server, json!({"default_node_info":{"network":chain}})).await;
            data(&server).await;
            let lcd = client(&server, chain);
            lcd.attest_endpoints().await.unwrap();
            assert_eq!(
                lcd.get::<serde_json::Value>("/data").await.unwrap()["answer"],
                42
            );
            let health = lcd.chain_identity_health().await;
            assert!(health.endpoints[0].verified);
            assert_eq!(
                health.endpoints[0].observed_chain_id.as_deref(),
                Some(chain)
            );
            assert!(health.cache_namespace.starts_with(chain));
        }
    }

    #[tokio::test]
    async fn wrong_missing_malformed_and_unavailable_identity_never_fetch_data() {
        for body in [
            json!({"default_node_info":{"network":"localterra"}}),
            json!({}),
            json!({"default_node_info":{"network":""}}),
            json!({"default_node_info":{"network":7}}),
        ] {
            let server = MockServer::start().await;
            identity(&server, body).await;
            data(&server).await;
            let lcd = client(&server, "columbus-5");
            assert!(lcd.attest_endpoints().await.is_err());
            assert!(lcd.get::<serde_json::Value>("/data").await.is_err());
            assert_eq!(data_hits(&server).await, 0);
            assert!(!lcd.chain_identity_health().await.endpoints[0].verified);
        }
        for response in [
            ResponseTemplate::new(503),
            ResponseTemplate::new(200).set_body_string("broken json"),
        ] {
            let server = MockServer::start().await;
            Mock::given(path(NODE_INFO_PATH))
                .respond_with(response)
                .mount(&server)
                .await;
            data(&server).await;
            let lcd = client(&server, "columbus-5");
            assert!(lcd.get::<serde_json::Value>("/data").await.is_err());
            assert_eq!(data_hits(&server).await, 0);
        }
        let lcd = LcdClient::new(vec!["http://127.0.0.1:9".into()], 50, 0, "columbus-5");
        assert!(lcd.attest_endpoints().await.is_err());
        assert!(!lcd.chain_identity_health().await.endpoints[0].verified);
    }

    #[tokio::test]
    async fn recovery1464_identity_outages_remain_retryable_and_integrity_stays_typed() {
        let server = MockServer::start().await;
        Mock::given(path(NODE_INFO_PATH))
            .respond_with(ResponseTemplate::new(503))
            .mount(&server)
            .await;
        let lcd = client(&server, "columbus-5");
        assert!(matches!(
            lcd.attest_endpoints().await,
            Err(LcdError::AllEndpointsFailed(_))
        ));
        assert!(matches!(
            lcd.get::<serde_json::Value>("/data").await,
            Err(LcdError::AllEndpointsFailed(_))
        ));
        server.reset().await;
        identity(
            &server,
            json!({"default_node_info":{"network":"localterra"}}),
        )
        .await;
        assert!(matches!(
            lcd.attest_endpoints().await,
            Err(LcdError::ChainIdentity(_))
        ));
        assert!(matches!(
            lcd.get::<serde_json::Value>("/data").await,
            Err(LcdError::ChainIdentity(_))
        ));
        assert_eq!(data_hits(&server).await, 0);
        server.reset().await;
        identity(
            &server,
            json!({"default_node_info":{"network":"columbus-5"}}),
        )
        .await;
        data(&server).await;
        assert_eq!(
            lcd.get::<serde_json::Value>("/data").await.unwrap()["answer"],
            42
        );
        let unavailable = LcdClient::new(vec!["http://127.0.0.1:9".into()], 50, 0, "columbus-5");
        assert!(matches!(
            unavailable.get::<serde_json::Value>("/data").await,
            Err(LcdError::AllEndpointsFailed(_))
        ));
    }

    #[tokio::test]
    async fn mixed_endpoints_failover_checks_each_and_quarantines_wrong_chain() {
        let wrong = MockServer::start().await;
        let good = MockServer::start().await;
        identity(
            &wrong,
            json!({"default_node_info":{"network":"localterra"}}),
        )
        .await;
        identity(&good, json!({"default_node_info":{"network":"columbus-5"}})).await;
        data(&wrong).await;
        data(&good).await;
        let lcd = LcdClient::new(vec![wrong.uri(), good.uri()], 500, 60_000, "columbus-5");
        lcd.attest_endpoints().await.unwrap();
        lcd.get::<serde_json::Value>("/data").await.unwrap();
        assert_eq!(data_hits(&wrong).await, 0);
        assert_eq!(data_hits(&good).await, 1);
        let health = lcd.chain_identity_health().await;
        assert!(!health.endpoints[0].verified);
        assert!(health.endpoints[1].verified);
        assert_eq!(
            health.endpoints[0].observed_chain_id.as_deref(),
            Some("localterra")
        );
    }

    #[tokio::test]
    async fn rotation_invalidates_previous_attestation_and_recovery_reattests() {
        let server = MockServer::start().await;
        identity(
            &server,
            json!({"default_node_info":{"network":"columbus-5"}}),
        )
        .await;
        data(&server).await;
        let lcd = client(&server, "columbus-5");
        lcd.get::<serde_json::Value>("/data").await.unwrap();
        server.reset().await;
        identity(
            &server,
            json!({"default_node_info":{"network":"localterra"}}),
        )
        .await;
        data(&server).await;
        assert!(lcd.require_verified_chain().await.is_err());
        assert!(lcd.get::<serde_json::Value>("/data").await.is_err());
        assert_eq!(data_hits(&server).await, 0);
        server.reset().await;
        identity(
            &server,
            json!({"default_node_info":{"network":"columbus-5"}}),
        )
        .await;
        data(&server).await;
        lcd.get::<serde_json::Value>("/data").await.unwrap();
        assert!(lcd.chain_identity_health().await.endpoints[0].verified);
        server.reset().await;
        assert!(lcd.require_verified_chain().await.is_err());
        assert!(!lcd.chain_identity_health().await.endpoints[0].verified);
    }

    #[tokio::test]
    async fn redirects_cannot_send_attested_reads_to_unverified_endpoints() {
        let source = MockServer::start().await;
        let destination = MockServer::start().await;
        identity(
            &source,
            json!({"default_node_info":{"network":"columbus-5"}}),
        )
        .await;
        data(&destination).await;
        Mock::given(path("/data"))
            .respond_with(
                ResponseTemplate::new(302)
                    .insert_header("Location", format!("{}/data", destination.uri())),
            )
            .mount(&source)
            .await;
        assert!(client(&source, "columbus-5")
            .get::<serde_json::Value>("/data")
            .await
            .is_err());
        assert_eq!(data_hits(&destination).await, 0);
    }

    #[test]
    fn cache_namespaces_separate_networks_clients_and_preserve_clones() {
        let first = LcdClient::new(vec![], 100, 0, "columbus-5");
        let local = LcdClient::new(vec![], 100, 0, "localterra");
        let second = LcdClient::new(vec![], 100, 0, "columbus-5");
        assert_ne!(first.chain_cache_namespace(), local.chain_cache_namespace());
        assert_ne!(
            first.chain_cache_namespace(),
            second.chain_cache_namespace()
        );
        assert_eq!(
            first.chain_cache_namespace(),
            first.clone().chain_cache_namespace()
        );
        for invalid in ["", " ", " columbus-5", "columbus-5\n", "x/y"] {
            assert!(!valid_chain_id(invalid));
        }
    }
}

#[cfg(test)]
mod contract_boundary_tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::{path, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn wrong_chain_plausible_contract_payload_is_never_queried() {
        let server = MockServer::start().await;
        Mock::given(path(NODE_INFO_PATH))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"default_node_info":{"network":"localterra"}})),
            )
            .mount(&server)
            .await;
        Mock::given(path_regex("^/cosmwasm/wasm/v1/contract/terra1same/smart/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({"data":{
                "owner":"terra1sameowner", "buy_bps":100, "sell_bps":100, "funding_version":1
            }})))
            .expect(0)
            .mount(&server)
            .await;
        let lcd = LcdClient::new(vec![server.uri()], 500, 0, "columbus-5");
        assert!(lcd
            .query_contract::<serde_json::Value>("terra1same", &json!({"get_config":{}}))
            .await
            .is_err());
    }

    #[tokio::test]
    async fn excessive_identity_payloads_and_network_names_are_rejected() {
        for response in [
            ResponseTemplate::new(200).set_body_string("x".repeat(70 * 1024)),
            ResponseTemplate::new(200)
                .set_body_json(json!({"default_node_info":{"network":"x".repeat(129)}})),
        ] {
            let server = MockServer::start().await;
            Mock::given(path(NODE_INFO_PATH))
                .respond_with(response)
                .mount(&server)
                .await;
            let lcd = LcdClient::new(vec![server.uri()], 500, 0, "columbus-5");
            assert!(lcd.attest_endpoints().await.is_err());
            let health = lcd.chain_identity_health().await;
            assert!(!health.endpoints[0].verified);
            assert!(health.endpoints[0].observed_chain_id.is_none());
        }
    }
}

#[cfg(test)]
mod block_identity_tests {
    use super::*;
    use serde_json::json;
    use wiremock::matchers::path;
    use wiremock::{Mock, MockServer, ResponseTemplate};
    const LATEST: &str = "/cosmos/base/tendermint/v1beta1/blocks/latest";

    #[tokio::test]
    async fn contradictory_or_missing_block_network_never_returns_height() {
        for chain in [
            json!("localterra"),
            json!(null),
            json!(7),
            json!(""),
            json!("columbus-5\n"),
        ] {
            let server = MockServer::start().await;
            mock_chain_identity(&server, "columbus-5").await;
            Mock::given(path(LATEST)).respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "block":{"header":{"chain_id":chain,"height":"123","time":"2026-10-05T00:00:00Z"}}
            }))).mount(&server).await;
            let lcd = LcdClient::new(vec![server.uri()], 500, 0, "columbus-5");
            assert!(lcd.get_latest_block_height().await.is_err());
            assert!(!lcd.chain_identity_health().await.endpoints[0].verified);
        }
    }

    #[tokio::test]
    async fn contradictory_block_endpoint_fails_over_and_historical_block_remains_readable() {
        let wrong = MockServer::start().await;
        let correct = MockServer::start().await;
        for server in [&wrong, &correct] {
            mock_chain_identity(server, "columbus-5").await;
        }
        for (server, chain) in [(&wrong, "localterra"), (&correct, "columbus-5")] {
            Mock::given(path(LATEST)).respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "block":{"header":{"chain_id":chain,"height":"123","time":"2020-01-01T00:00:00Z"}}
            }))).mount(server).await;
        }
        let lcd = LcdClient::new(vec![wrong.uri(), correct.uri()], 500, 0, "columbus-5");
        assert_eq!(lcd.get_latest_block_height().await.unwrap(), 123);
        let health = lcd.chain_identity_health().await;
        assert!(!health.endpoints[0].verified);
        assert!(health.endpoints[1].verified);
        assert!(validate_block_chain(
            r#"{"block":{"header":{"chain_id":"columbus-5","time":"2020-01-01T00:00:00Z"}}}"#,
            "columbus-5"
        )
        .is_ok());
    }
}
