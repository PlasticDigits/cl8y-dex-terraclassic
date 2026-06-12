//! Operator-actionable reorg halt signal (GitLab #362).
//!
//! Emits a machine-parseable stderr line (`INDEXER_REORG_HALT`) for log routers and an optional
//! webhook POST when `REORG_ALERT_WEBHOOK_URL` is set.

use serde::Serialize;
use tracing::error;

const RUNBOOK: &str = "docs/runbooks/indexer-reorg-replay-dedup.md";

#[derive(Debug, Serialize)]
pub struct ReorgHaltEvent {
    pub event: &'static str,
    pub height: i64,
    pub stored_hash: String,
    pub canonical_hash: String,
    pub recovery_height: i64,
    pub recovery_cmd: String,
    pub runbook: &'static str,
}

/// Structured halt signal: tracing target + stderr JSON + optional webhook.
pub async fn emit_reorg_halt(
    height: i64,
    stored_hash: &str,
    canonical_hash: &str,
    webhook_url: Option<&str>,
) {
    let recovery_height = height;
    let recovery_cmd = format!(
        "./scripts/indexer-reorg-recover.sh --height {}",
        recovery_height
    );
    let event = ReorgHaltEvent {
        event: "indexer_reorg_halt",
        height,
        stored_hash: stored_hash.to_string(),
        canonical_hash: canonical_hash.to_string(),
        recovery_height,
        recovery_cmd: recovery_cmd.clone(),
        runbook: RUNBOOK,
    };

    error!(
        target: "indexer_reorg_halt",
        event = event.event,
        height = event.height,
        stored_hash = %event.stored_hash,
        canonical_hash = %event.canonical_hash,
        recovery_height = event.recovery_height,
        recovery_cmd = %event.recovery_cmd,
        runbook = event.runbook,
        "Indexer halted: chain reorg detected — run recovery script (dry-run first)"
    );

    match serde_json::to_string(&event) {
        Ok(json) => eprintln!("INDEXER_REORG_HALT {json}"),
        Err(e) => tracing::warn!(error = %e, "Failed to serialize reorg halt event"),
    }

    if let Some(url) = webhook_url.filter(|u| !u.is_empty()) {
        post_webhook(url, &event).await;
    }
}

async fn post_webhook(url: &str, event: &ReorgHaltEvent) {
    let client = match reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(5))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            tracing::warn!(error = %e, "Reorg webhook client build failed");
            return;
        }
    };

    match client.post(url).json(event).send().await {
        Ok(resp) if resp.status().is_success() => {
            tracing::info!(status = %resp.status(), "Reorg alert webhook delivered");
        }
        Ok(resp) => {
            tracing::warn!(
                status = %resp.status(),
                "Reorg alert webhook returned non-success status"
            );
        }
        Err(e) => {
            tracing::warn!(error = %e, "Reorg alert webhook request failed");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reorg_halt_event_serializes_required_fields() {
        let event = ReorgHaltEvent {
            event: "indexer_reorg_halt",
            height: 42,
            stored_hash: "OLD".into(),
            canonical_hash: "NEW".into(),
            recovery_height: 42,
            recovery_cmd: "./scripts/indexer-reorg-recover.sh --height 42".into(),
            runbook: RUNBOOK,
        };
        let json = serde_json::to_string(&event).expect("serialize");
        assert!(json.contains("\"event\":\"indexer_reorg_halt\""));
        assert!(json.contains("\"height\":42"));
        assert!(json.contains("indexer-reorg-recover.sh"));
    }
}
