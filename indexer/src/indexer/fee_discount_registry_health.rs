//! Background LCD `config` probe for the fee-discount registry (GitLab #373).
//!
//! Pairs fail-closed to full pair `fee_bps` when on-chain `GetDiscount` errors; this loop
//! exposes a narrow ops signal without extending generic `GET /health`.

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use crate::lcd::{LcdClient, LcdError};

const PROBE_INTERVAL: Duration = Duration::from_secs(60);
/// Log at `error` when LCD config probes fail at least this many times in a row.
const REPEATED_FAILURE_LOG_THRESHOLD: u32 = 2;

#[derive(Clone, Debug, Default, Serialize, PartialEq, Eq)]
pub struct FeeDiscountRegistryHealthSnapshot {
    pub configured: bool,
    /// `null` when `FEE_DISCOUNT_ADDRESS` is unset; otherwise latest probe result.
    pub fee_discount_registry_ok: Option<bool>,
    pub consecutive_lcd_failures: u32,
}

#[derive(Debug, Default)]
struct Inner {
    configured: bool,
    registry_ok: Option<bool>,
    consecutive_lcd_failures: u32,
    last_probe_at: Option<Instant>,
}

#[derive(Clone, Default)]
pub struct FeeDiscountRegistryHealth {
    inner: Arc<RwLock<Inner>>,
}

impl FeeDiscountRegistryHealth {
    pub fn from_config(fee_discount_address: Option<&str>) -> Self {
        match fee_discount_address.filter(|a| !a.is_empty()) {
            Some(_) => Self::configured(),
            None => Self::unconfigured(),
        }
    }

    pub fn new(configured: bool) -> Self {
        if configured {
            Self::configured()
        } else {
            Self::unconfigured()
        }
    }

    pub fn unconfigured() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner {
                configured: false,
                registry_ok: None,
                ..Default::default()
            })),
        }
    }

    pub fn configured() -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner {
                configured: true,
                registry_ok: None,
                ..Default::default()
            })),
        }
    }

    pub async fn snapshot(&self) -> FeeDiscountRegistryHealthSnapshot {
        let g = self.inner.read().await;
        FeeDiscountRegistryHealthSnapshot {
            configured: g.configured,
            fee_discount_registry_ok: g.registry_ok,
            consecutive_lcd_failures: g.consecutive_lcd_failures,
        }
    }

    async fn record_probe_success(&self) {
        let mut g = self.inner.write().await;
        let was_failing = g.consecutive_lcd_failures >= REPEATED_FAILURE_LOG_THRESHOLD;
        let had_failures = g.consecutive_lcd_failures > 0;
        g.registry_ok = Some(true);
        g.consecutive_lcd_failures = 0;
        g.last_probe_at = Some(Instant::now());
        if was_failing {
            tracing::info!(
                "Fee-discount registry LCD config probe recovered; on-chain pairs use registry discounts when GetDiscount succeeds"
            );
        } else if had_failures {
            tracing::info!("Fee-discount registry LCD config probe recovered");
        }
    }

    async fn record_probe_failure(&self, err: &LcdError) {
        let mut g = self.inner.write().await;
        g.consecutive_lcd_failures = g.consecutive_lcd_failures.saturating_add(1);
        g.registry_ok = Some(false);
        g.last_probe_at = Some(Instant::now());
        let failures = g.consecutive_lcd_failures;
        if failures >= REPEATED_FAILURE_LOG_THRESHOLD {
            tracing::error!(
                consecutive_failures = failures,
                "Fee-discount registry LCD config probe failing; on-chain pairs fail-closed to full pair fee when GetDiscount errors — poll GET /api/v1/health/fee-discount; upstream: {}",
                err
            );
        } else {
            tracing::warn!(
                consecutive_failures = failures,
                "Fee-discount registry LCD config probe failed; upstream: {}",
                err
            );
        }
    }
}

pub async fn run_fee_discount_registry_probe_loop(
    lcd: LcdClient,
    fee_discount_addr: String,
    health: FeeDiscountRegistryHealth,
    cancel: CancellationToken,
) {
    loop {
        if cancel.is_cancelled() {
            return;
        }

        probe_fee_discount_registry(&lcd, &fee_discount_addr, &health).await;

        tokio::select! {
            _ = cancel.cancelled() => return,
            _ = tokio::time::sleep(PROBE_INTERVAL) => {}
        }
    }
}

/// Single LCD `config` probe (used by background loop and integration tests).
pub async fn probe_fee_discount_registry(
    lcd: &LcdClient,
    fee_discount_addr: &str,
    health: &FeeDiscountRegistryHealth,
) {
    #[derive(serde::Deserialize)]
    struct ConfigResponse {
        governance: String,
        cl8y_token: String,
    }

    let query = serde_json::json!({ "config": {} });
    match lcd
        .query_contract::<ConfigResponse>(fee_discount_addr, &query)
        .await
    {
        Ok(_) => health.record_probe_success().await,
        Err(e) => health.record_probe_failure(&e).await,
    }
}

/// Alias for integration tests (GitLab #373).
pub async fn probe_fee_discount_registry_once(
    lcd: &LcdClient,
    fee_discount_addr: &str,
    health: &FeeDiscountRegistryHealth,
) {
    probe_fee_discount_registry(lcd, fee_discount_addr, health).await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path_regex};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn unconfigured_snapshot_is_null_ok_with_zero_failures() {
        let health = FeeDiscountRegistryHealth::unconfigured();
        let snap = health.snapshot().await;
        assert!(!snap.configured);
        assert_eq!(snap.fee_discount_registry_ok, None);
        assert_eq!(snap.consecutive_lcd_failures, 0);
    }

    #[tokio::test]
    async fn snapshot_reflects_configured_and_failures() {
        let health = FeeDiscountRegistryHealth::configured();
        let snap = health.snapshot().await;
        assert!(snap.configured);
        assert_eq!(snap.fee_discount_registry_ok, None);
        assert_eq!(snap.consecutive_lcd_failures, 0);

        health
            .record_probe_failure(&LcdError::AllEndpointsFailed("probe failed".into()))
            .await;
        let snap = health.snapshot().await;
        assert_eq!(snap.fee_discount_registry_ok, Some(false));
        assert_eq!(snap.consecutive_lcd_failures, 1);

        health.record_probe_success().await;
        let snap = health.snapshot().await;
        assert_eq!(snap.fee_discount_registry_ok, Some(true));
        assert_eq!(snap.consecutive_lcd_failures, 0);
    }

    #[tokio::test]
    async fn successful_probe_sets_ok_true() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "data": {
                    "governance": "terra1gov",
                    "cl8y_token": "terra1cl8y"
                }
            })))
            .mount(&server)
            .await;

        let lcd = LcdClient::new(vec![server.uri()], 5000, 30000);
        let health = FeeDiscountRegistryHealth::configured();
        probe_fee_discount_registry_once(&lcd, "terra1feediscount", &health).await;

        let snap = health.snapshot().await;
        assert!(snap.configured);
        assert_eq!(snap.fee_discount_registry_ok, Some(true));
        assert_eq!(snap.consecutive_lcd_failures, 0);
    }

    #[tokio::test]
    async fn repeated_lcd_failure_increments_counter() {
        let server = MockServer::start().await;
        Mock::given(method("GET"))
            .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/"))
            .respond_with(
                ResponseTemplate::new(500)
                    .set_body_string("All LCD endpoints failed; cosmwasm http://secret"),
            )
            .mount(&server)
            .await;

        let lcd = LcdClient::new(vec![server.uri()], 5000, 30000);
        let health = FeeDiscountRegistryHealth::configured();
        probe_fee_discount_registry_once(&lcd, "terra1feediscount", &health).await;
        probe_fee_discount_registry_once(&lcd, "terra1feediscount", &health).await;

        let snap = health.snapshot().await;
        assert_eq!(snap.fee_discount_registry_ok, Some(false));
        assert!(snap.consecutive_lcd_failures >= 2);
    }
}
