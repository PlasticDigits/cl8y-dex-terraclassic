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

#[derive(Clone, Debug, Serialize, ToSchema)]
pub struct EndpointIdentity {
    pub endpoint_index: usize,
    pub observed_chain_id: Option<String>,
    pub verified: bool,
    pub last_error: Option<String>,
    pub checked_at: Option<String>,
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
            })
            .collect();
        ChainIdentityHealth {
            expected_chain_id: self.inner.expected_chain_id.clone(),
            cache_namespace: self.chain_cache_namespace(),
            endpoints,
        }
    }

    /// Probe every endpoint at startup; proceed only when at least one independently verifies.
    pub async fn attest_endpoints(&self) -> Result<(), LcdError> {
        let count = self.inner.endpoints.read().await.len();
        let mut verified = false;
        let mut errors = Vec::new();
        for idx in 0..count {
            let url = self.inner.endpoints.read().await[idx].url.clone();
            match self.attest_endpoint(idx, &url).await {
                Ok(()) => verified = true,
                Err(e) => errors.push(e.to_string()),
            }
        }
        if verified {
            Ok(())
        } else {
            Err(LcdError::AllEndpointsFailed(errors.join("; ")))
        }
    }

    /// Used before serving previously computed quote/discovery caches as well as live reads.
    pub async fn require_verified_chain(&self) -> Result<(), LcdError> {
        let count = self.inner.endpoints.read().await.len();
        let mut errors = Vec::new();
        for idx in 0..count {
            let (url, cooled) = {
                let eps = self.inner.endpoints.read().await;
                let ep = &eps[idx];
                (
                    ep.url.clone(),
                    ep.cooldown_until
                        .is_some_and(|at| at > std::time::Instant::now()),
                )
            };
            if cooled {
                continue;
            }
            match self.attest_endpoint(idx, &url).await {
                Ok(()) => return Ok(()),
                Err(e) => errors.push(e.to_string()),
            }
        }
        Err(LcdError::AllEndpointsFailed(errors.join("; ")))
    }

    pub(super) async fn attest_endpoint(&self, idx: usize, url: &str) -> Result<(), LcdError> {
        let mut observed = None;
        let result = async {
            let resp = self
                .inner
                .client
                .get(format!("{url}{NODE_INFO_PATH}"))
                .send()
                .await?;
            if !resp.status().is_success() {
                return Err(LcdError::ChainIdentity(format!(
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
            LcdError::Request(_) => {
                LcdError::ChainIdentity(format!("endpoint[{idx}] node_info transport unavailable"))
            }
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
            ep.cooldown_until = Some(std::time::Instant::now() + self.inner.cooldown_duration);
            tracing::warn!(endpoint_index = idx, expected_chain_id = %self.inner.expected_chain_id,
                observed_chain_id = ?ep.observed_chain_id, "LCD endpoint quarantined: chain identity failed");
        } else {
            ep.cooldown_until = None;
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
