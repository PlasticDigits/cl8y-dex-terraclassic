//! Operator alerting when the indexer halts on chain reorg (GitLab #362).
//!
//! Emits structured `tracing` events, a machine-parseable stderr line (`INDEXER_REORG_HALT`),
//! and optionally POSTs JSON to `REORG_ALERT_WEBHOOK_URL` (PagerDuty, Slack, custom ops hook).

use serde::Serialize;

/// Details captured when the reorg guard halts indexing.
#[derive(Debug, Clone, Serialize)]
pub struct ReorgHaltDetails {
    /// Height where stored vs canonical block hash diverged.
    pub height: i64,
    pub stored_hash: String,
    pub canonical_hash: String,
    /// First height to replay after cursor rewind (`height` when mismatch is at committed tip).
    pub recovery_fork_height: i64,
    pub recovery_command: String,
    pub runbook_path: &'static str,
}

impl ReorgHaltDetails {
    pub fn new(height: i64, stored_hash: String, canonical_hash: String) -> Self {
        let recovery_fork_height = height;
        Self {
            height,
            stored_hash,
            canonical_hash,
            recovery_fork_height,
            recovery_command: format!(
                "./scripts/indexer-reorg-recover.sh --height {}",
                recovery_fork_height
            ),
            runbook_path: "docs/runbooks/indexer-reorg-replay-dedup.md",
        }
    }
}

/// Structured log + stderr JSON + optional webhook. Webhook delivery is best-effort but awaited
/// so the POST can finish before the indexer task exits on reorg halt.
pub async fn emit_reorg_halt(details: &ReorgHaltDetails) {
    tracing::error!(
        target: "indexer_reorg_halt",
        event = "indexer_reorg_halt",
        height = details.height,
        stored_hash = %details.stored_hash,
        canonical_hash = %details.canonical_hash,
        recovery_fork_height = details.recovery_fork_height,
        recovery_command = %details.recovery_command,
        runbook = details.runbook_path,
        "Indexer halted: chain reorg detected — operator recovery required"
    );

    let payload = WebhookPayload::from_details(details);
    match serde_json::to_string(&payload) {
        Ok(json) => eprintln!("INDEXER_REORG_HALT {json}"),
        Err(e) => tracing::warn!(error = %e, "Failed to serialize reorg halt event"),
    }

    let webhook_url = match std::env::var("REORG_ALERT_WEBHOOK_URL") {
        Ok(url) if !url.trim().is_empty() => url,
        _ => return,
    };

    if let Err(e) = post_webhook(&webhook_url, &payload).await {
        tracing::warn!(
            target: "indexer_reorg_halt",
            error = %e,
            "Failed to deliver reorg alert webhook"
        );
    }
}

#[derive(Debug, Serialize)]
struct WebhookPayload {
    event: &'static str,
    height: i64,
    stored_hash: String,
    canonical_hash: String,
    recovery_fork_height: i64,
    recovery_command: String,
    runbook: &'static str,
}

impl WebhookPayload {
    fn from_details(details: &ReorgHaltDetails) -> Self {
        Self {
            event: "indexer_reorg_halt",
            height: details.height,
            stored_hash: details.stored_hash.clone(),
            canonical_hash: details.canonical_hash.clone(),
            recovery_fork_height: details.recovery_fork_height,
            recovery_command: details.recovery_command.clone(),
            runbook: details.runbook_path,
        }
    }
}

async fn post_webhook(url: &str, payload: &WebhookPayload) -> Result<(), String> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()
        .map_err(|e| e.to_string())?;

    let resp = client
        .post(url)
        .json(payload)
        .send()
        .await
        .map_err(|e| e.to_string())?;

    if resp.status().is_success() {
        tracing::info!(
            target: "indexer_reorg_halt",
            status = %resp.status(),
            "Reorg alert webhook delivered"
        );
        Ok(())
    } else {
        Err(format!("webhook returned {}", resp.status()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn recovery_command_matches_fork_height() {
        let d = ReorgHaltDetails::new(1_234_567, "OLD".into(), "NEW".into());
        assert_eq!(d.recovery_fork_height, 1_234_567);
        assert!(d.recovery_command.contains("--height 1234567"));
    }

    #[test]
    fn webhook_payload_serializes_event_type() {
        let d = ReorgHaltDetails::new(100, "a".into(), "b".into());
        let p = WebhookPayload::from_details(&d);
        let json = serde_json::to_value(&p).expect("serialize");
        assert_eq!(json["event"], "indexer_reorg_halt");
        assert_eq!(json["height"], 100);
        assert_eq!(json["stored_hash"], "a");
        assert_eq!(json["canonical_hash"], "b");
    }
}
