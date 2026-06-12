//! Background LCD probe + in-memory state for fee-discount registry reachability (GitLab #365).
//!
//! The pair contract fail-closes to full fee when `GetDiscount` errors; this module gives ops and
//! integrators an off-chain signal when the registry LCD surface is unhealthy.

use std::sync::Arc;
use std::time::{Duration, Instant};

use serde::Serialize;
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;

use crate::lcd::LcdClient;

const PROBE_INTERVAL: Duration = Duration::from_secs(60);
/// Log at `error` when LCD config probes fail at least this many times in a row.
const REPEATED_FAILURE_LOG_THRESHOLD: u32 = 2;

#[derive(Debug, Clone, Serialize)]
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
    pub fn new(configured: bool) -> Self {
        Self {
            inner: Arc::new(RwLock::new(Inner {
                configured,
                registry_ok: if configured { Some(true) } else { None },
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

    pub(crate) async fn record_probe_success(&self) {
        let mut g = self.inner.write().await;
        let was_failing = g.consecutive_lcd_failures >= REPEATED_FAILURE_LOG_THRESHOLD;
        g.registry_ok = Some(true);
        g.consecutive_lcd_failures = 0;
        g.last_probe_at = Some(Instant::now());
        if was_failing {
            tracing::info!("Fee-discount registry LCD probe recovered");
        }
    }

    pub(crate) async fn record_probe_failure(&self) {
        let mut g = self.inner.write().await;
        g.consecutive_lcd_failures = g.consecutive_lcd_failures.saturating_add(1);
        g.registry_ok = Some(false);
        g.last_probe_at = Some(Instant::now());
        if g.consecutive_lcd_failures == REPEATED_FAILURE_LOG_THRESHOLD {
            tracing::error!(
                consecutive_failures = g.consecutive_lcd_failures,
                "Fee-discount registry LCD probe failing repeatedly; pairs fail-closed to full fee on-chain"
            );
        } else if g.consecutive_lcd_failures > REPEATED_FAILURE_LOG_THRESHOLD {
            tracing::warn!(
                consecutive_failures = g.consecutive_lcd_failures,
                "Fee-discount registry LCD probe still failing"
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
        Err(e) => {
            tracing::warn!(detail = %e, "Fee-discount registry LCD config probe failed");
            health.record_probe_failure().await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn snapshot_reflects_configured_and_failures() {
        let health = FeeDiscountRegistryHealth::new(true);
        let snap = health.snapshot().await;
        assert!(snap.configured);
        assert_eq!(snap.fee_discount_registry_ok, Some(true));
        assert_eq!(snap.consecutive_lcd_failures, 0);

        health.record_probe_failure().await;
        let snap = health.snapshot().await;
        assert_eq!(snap.fee_discount_registry_ok, Some(false));
        assert_eq!(snap.consecutive_lcd_failures, 1);

        health.record_probe_success().await;
        let snap = health.snapshot().await;
        assert_eq!(snap.fee_discount_registry_ok, Some(true));
        assert_eq!(snap.consecutive_lcd_failures, 0);
    }

    #[tokio::test]
    async fn unconfigured_snapshot_has_null_ok() {
        let health = FeeDiscountRegistryHealth::new(false);
        let snap = health.snapshot().await;
        assert!(!snap.configured);
        assert_eq!(snap.fee_discount_registry_ok, None);
    }
}
