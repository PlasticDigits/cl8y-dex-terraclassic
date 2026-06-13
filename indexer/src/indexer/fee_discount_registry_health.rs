//! Background LCD `config` probe for the fee-discount registry (GitLab #373).
//!
//! Pairs fail-closed to full pair `fee_bps` when on-chain `GetDiscount` errors; this loop
//! exposes a narrow ops signal without extending generic `GET /health`.

use std::sync::Arc;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use crate::lcd::{LcdClient, LcdError};

/// Probe interval for registry `config` smart query.
pub const PROBE_INTERVAL_SECS: u64 = 60;

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct FeeDiscountRegistryHealthSnapshot {
    pub configured: bool,
    pub fee_discount_registry_ok: Option<bool>,
    pub consecutive_lcd_failures: u32,
}

#[derive(Clone)]
pub struct FeeDiscountRegistryHealth {
    inner: Arc<RwLock<FeeDiscountRegistryHealthSnapshot>>,
}

impl FeeDiscountRegistryHealth {
    pub fn from_config(fee_discount_address: Option<&str>) -> Self {
        match fee_discount_address.filter(|a| !a.is_empty()) {
            Some(_) => Self::configured(),
            None => Self::unconfigured(),
        }
    }

    pub fn unconfigured() -> Self {
        Self {
            inner: Arc::new(RwLock::new(FeeDiscountRegistryHealthSnapshot {
                configured: false,
                fee_discount_registry_ok: None,
                consecutive_lcd_failures: 0,
            })),
        }
    }

    pub fn configured() -> Self {
        Self {
            inner: Arc::new(RwLock::new(FeeDiscountRegistryHealthSnapshot {
                configured: true,
                fee_discount_registry_ok: None,
                consecutive_lcd_failures: 0,
            })),
        }
    }

    pub async fn snapshot(&self) -> FeeDiscountRegistryHealthSnapshot {
        self.inner.read().await.clone()
    }
}

pub async fn run_fee_discount_registry_health_probe(
    lcd: LcdClient,
    fee_discount_addr: String,
    health: FeeDiscountRegistryHealth,
) {
    loop {
        probe_fee_discount_registry_once(&lcd, &fee_discount_addr, &health).await;
        tokio::time::sleep(Duration::from_secs(PROBE_INTERVAL_SECS)).await;
    }
}

/// One LCD `config` probe; updates shared snapshot and tracing counters.
pub async fn probe_fee_discount_registry_once(
    lcd: &LcdClient,
    fee_discount_addr: &str,
    health: &FeeDiscountRegistryHealth,
) {
    let query = serde_json::json!({ "config": {} });
    let result: Result<serde_json::Value, LcdError> =
        lcd.query_contract(fee_discount_addr, &query).await;

    let mut snap = health.inner.write().await;
    match result {
        Ok(_) => {
            if snap.consecutive_lcd_failures >= 2 {
                tracing::info!(
                    "Fee-discount registry LCD config probe recovered; on-chain pairs use registry discounts when GetDiscount succeeds"
                );
            } else if snap.consecutive_lcd_failures > 0 {
                tracing::info!("Fee-discount registry LCD config probe recovered");
            }
            snap.fee_discount_registry_ok = Some(true);
            snap.consecutive_lcd_failures = 0;
        }
        Err(e) => {
            snap.fee_discount_registry_ok = Some(false);
            snap.consecutive_lcd_failures = snap.consecutive_lcd_failures.saturating_add(1);
            let failures = snap.consecutive_lcd_failures;
            if failures >= 2 {
                tracing::error!(
                    consecutive_failures = failures,
                    "Fee-discount registry LCD config probe failing; on-chain pairs fail-closed to full pair fee when GetDiscount errors — poll GET /api/v1/health/fee-discount; upstream: {}",
                    e
                );
            } else {
                tracing::warn!(
                    consecutive_failures = failures,
                    "Fee-discount registry LCD config probe failed; upstream: {}",
                    e
                );
            }
        }
    }
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
