//! Operator alerting when the indexer halts on chain reorg (GitLab #362).
//!
//! Emits a structured `indexer_reorg_halt` event for log collectors and optionally POSTs
//! to `REORG_ALERT_WEBHOOK_URL` (PagerDuty, Slack, custom ops webhook).

use serde::Serialize;
use tracing::error;

const RUNBOOK: &str = "docs/runbooks/indexer-reorg-replay-dedup.md";
const RECOVERY_SCRIPT: &str = "scripts/indexer-reorg-recover.sh";

#[derive(Debug, Clone, Serialize)]
pub struct ReorgHaltPayload {
    pub event: &'static str,
    pub height: i64,
    pub stored_hash: String,
    pub canonical_hash: String,
    pub recovery_runbook: &'static str,
    pub recovery_script: &'static str,
    pub operator_action: String,
}

/// Structured halt signal: always logs; optionally fires webhook (best-effort, non-blocking).
pub fn emit_reorg_halt(height: i64, stored_hash: &str, canonical_hash: &str) {
    let operator_action = format!(
        "Stop indexer; identify fork height H; run `{RECOVERY_SCRIPT} --height H` (dry-run), \
         then `--apply` (+ `--cleanup-derived` if needed); restart indexer. See {RUNBOOK}."
    );

    error!(
        target: "indexer::reorg_alert",
        event = "indexer_reorg_halt",
        height,
        stored_hash = %stored_hash,
        canonical_hash = %canonical_hash,
        recovery_runbook = RUNBOOK,
        recovery_script = RECOVERY_SCRIPT,
        operator_action = %operator_action,
        "INDEXER_REORG_HALT: chain reorg detected — indexer halted"
    );

    let webhook_url = std::env::var("REORG_ALERT_WEBHOOK_URL")
        .ok()
        .filter(|u| !u.trim().is_empty());
    let Some(url) = webhook_url else {
        return;
    };

    let payload = ReorgHaltPayload {
        event: "indexer_reorg_halt",
        height,
        stored_hash: stored_hash.to_string(),
        canonical_hash: canonical_hash.to_string(),
        recovery_runbook: RUNBOOK,
        recovery_script: RECOVERY_SCRIPT,
        operator_action,
    };

    tokio::spawn(async move {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(10))
            .build();
        let Ok(client) = client else {
            tracing::warn!(target: "indexer::reorg_alert", "reorg webhook client build failed");
            return;
        };
        match client.post(&url).json(&payload).send().await {
            Ok(resp) if resp.status().is_success() => {
                tracing::info!(
                    target: "indexer::reorg_alert",
                    status = %resp.status(),
                    "reorg alert webhook delivered"
                );
            }
            Ok(resp) => {
                tracing::warn!(
                    target: "indexer::reorg_alert",
                    status = %resp.status(),
                    "reorg alert webhook returned non-success status"
                );
            }
            Err(e) => {
                tracing::warn!(
                    target: "indexer::reorg_alert",
                    error = %e,
                    "reorg alert webhook delivery failed"
                );
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn payload_serializes_operator_fields() {
        let p = ReorgHaltPayload {
            event: "indexer_reorg_halt",
            height: 42,
            stored_hash: "abc".into(),
            canonical_hash: "def".into(),
            recovery_runbook: RUNBOOK,
            recovery_script: RECOVERY_SCRIPT,
            operator_action: "test".into(),
        };
        let json = serde_json::to_value(&p).unwrap();
        assert_eq!(json["event"], "indexer_reorg_halt");
        assert_eq!(json["height"], 42);
        assert!(json["recovery_script"]
            .as_str()
            .unwrap()
            .contains("indexer-reorg-recover"));
    }
}
