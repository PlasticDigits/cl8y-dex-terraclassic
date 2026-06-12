//! Integration test: reorg halt webhook delivery (GitLab #362).

use cl8y_dex_indexer::indexer::reorg_alert::{emit_reorg_halt, ReorgHaltDetails};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn reorg_halt_webhook_delivered_when_url_set() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    std::env::set_var("REORG_ALERT_WEBHOOK_URL", server.uri());
    let details = ReorgHaltDetails::new(42, "old".into(), "new".into());
    emit_reorg_halt(&details);
    tokio::time::sleep(std::time::Duration::from_millis(300)).await;
    std::env::remove_var("REORG_ALERT_WEBHOOK_URL");

    let requests = server.received_requests().await.expect("requests");
    assert_eq!(requests.len(), 1);
    let body: serde_json::Value = serde_json::from_slice(&requests[0].body).expect("json body");
    assert_eq!(body["event"], "indexer_reorg_halt");
    assert_eq!(body["height"], 42);
    assert_eq!(body["recovery_fork_height"], 42);
    assert!(body["recovery_command"]
        .as_str()
        .unwrap_or("")
        .contains("--height 42"));
}
