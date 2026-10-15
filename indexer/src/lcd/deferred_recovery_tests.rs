//! Zero-attempt recovery owns local waiting, never a forced probe or fresh budget.
use super::*;
use serde_json::{json, Value};
use wiremock::{matchers::path, Mock, MockServer, ResponseTemplate};

#[derive(Clone, Copy, Debug)]
enum Operation {
    Identity,
    Data,
}
async fn observe(lcd: &LcdClient, operation: Operation) -> Result<Value, LcdError> {
    match operation {
        Operation::Identity => lcd.require_verified_chain().await.map(|()| Value::Null),
        Operation::Data => lcd.get("/observation").await,
    }
}
async fn fixture(cooldown_ms: u64, deadline_ms: u64) -> (MockServer, LcdClient) {
    let server = MockServer::start().await;
    mock_chain_identity(&server, "columbus-5").await;
    Mock::given(path("/observation"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"height":1})))
        .mount(&server)
        .await;
    let lcd = LcdClient::new_with_policy(
        vec![server.uri()],
        500,
        cooldown_ms,
        "columbus-5",
        ProviderPolicy {
            deadline_ms,
            attempt_timeout_ms: 250,
            max_backoff_ms: cooldown_ms,
            ..Default::default()
        },
    );
    (server, lcd)
}
async fn open(lcd: &LcdClient, cooldown_ms: u64) {
    for endpoint in lcd.inner.endpoints.read().await.iter() {
        endpoint
            .provider
            .failed(Duration::from_millis(cooldown_ms), None);
    }
}
async fn short_circuit(operation: Operation) {
    let (server, lcd) = fixture(100, 1000).await;
    open(&lcd, 100).await;
    let scheduled = lcd.with_ingestion_budget();
    let result = observe(&scheduled, operation).await;
    assert!(
        result.is_ok(),
        "{operation:?}: a cooldown expiring inside the original deadline must recover: {result:?}"
    );
    let counters = lcd.provider_counters().await;
    assert_eq!(counters[0].attempts, 1);
    assert_eq!(
        counters[0].failures, 1,
        "waiting does not add failures or reset backoff"
    );
    assert!(counters[0].circuit_deferrals > 0);
    assert_eq!(counters[0].in_flight, 0);
    assert!(!counters[0].circuit_open);
    assert_eq!(
        server.received_requests().await.unwrap().len(),
        if matches!(operation, Operation::Identity) {
            1
        } else {
            2
        }
    );
    assert_eq!(
        scheduled
            .ingestion_budget
            .as_ref()
            .unwrap()
            .permits
            .available_permits(),
        2
    );
}
#[tokio::test]
async fn issue1505_scheduled_short_circuit_identity_recovers() {
    short_circuit(Operation::Identity).await;
}
#[tokio::test]
async fn issue1505_scheduled_short_circuit_data_recovers() {
    short_circuit(Operation::Data).await;
}

#[tokio::test]
async fn issue1505_long_circuit_keeps_original_deadline_and_zero_reads() {
    for operation in [Operation::Identity, Operation::Data] {
        let (server, lcd) = fixture(5000, 150).await;
        open(&lcd, 5000).await;
        let scheduled = lcd.with_ingestion_budget();
        let result = tokio::time::timeout(Duration::from_secs(1), observe(&scheduled, operation))
            .await
            .unwrap();
        assert!(
            matches!(&result, Err(LcdError::AllEndpointsFailed(message)) if message == if matches!(operation, Operation::Identity) { "no verified endpoint" } else { "all provider circuits open" }),
            "{operation:?}: preserve public terminal error: {result:?}"
        );
        let counters = lcd.provider_counters().await;
        assert_eq!(
            (
                counters[0].attempts,
                counters[0].failures,
                counters[0].in_flight
            ),
            (0, 1, 0)
        );
        assert!(
            counters[0].circuit_deferrals >= 2,
            "bounded local waiting really occurred"
        );
        assert!(server.received_requests().await.unwrap().is_empty());
        assert_eq!(
            scheduled
                .ingestion_budget
                .as_ref()
                .unwrap()
                .permits
                .available_permits(),
            2
        );
    }
}
#[tokio::test]
async fn issue1505_non_ingestion_circuit_still_fails_without_waiting() {
    for operation in [Operation::Identity, Operation::Data] {
        for scheduled_without_lane in [false, true] {
            let (server, lcd) = fixture(100, 1000).await;
            open(&lcd, 100).await;
            let client = if scheduled_without_lane {
                lcd.with_scheduled_observation()
            } else {
                lcd.clone()
            };
            assert!(observe(&client, operation).await.is_err());
            let counters = lcd.provider_counters().await;
            assert_eq!(
                (counters[0].attempts, counters[0].circuit_deferrals),
                (0, 1)
            );
            assert!(server.received_requests().await.unwrap().is_empty());
        }
    }
}
#[tokio::test]
async fn issue1505_elapsed_total_deadline_cannot_rescan_or_start_reads() {
    for operation in [Operation::Identity, Operation::Data] {
        let (server, lcd) = fixture(100, 60).await;
        open(&lcd, 100).await;
        let scheduled = lcd.with_ingestion_budget();
        let mut pending = Box::pin(observe(&scheduled, operation));
        tokio::select! {
            result = &mut pending => panic!("{operation:?}: must initially wait: {result:?}"),
            _ = tokio::time::sleep(Duration::from_millis(10)) => {},
        }
        // Delay polling until both cooldown and total deadline have elapsed.
        // Timeout polls its inner future first; its presence alone is no guard.
        std::thread::sleep(Duration::from_millis(150));
        assert!(pending.await.is_err());
        assert_eq!(lcd.provider_counters().await[0].attempts, 0);
        assert!(server.received_requests().await.unwrap().is_empty());
    }
}
#[tokio::test]
async fn issue1505_queued_data_admission_cannot_start_after_total_deadline() {
    let (server, lcd) = fixture(5000, 60).await;
    let scheduled = lcd.with_ingestion_budget();
    *scheduled
        .ingestion_budget
        .as_ref()
        .unwrap()
        .next_request
        .lock()
        .await = tokio::time::Instant::now() + Duration::from_millis(100);
    let mut pending = Box::pin(observe(&scheduled, Operation::Data));
    tokio::select! {
        result = &mut pending => panic!("queued admission must remain pending: {result:?}"),
        _ = tokio::time::sleep(Duration::from_millis(10)) => {},
    }
    std::thread::sleep(Duration::from_millis(150));
    assert!(pending.await.is_err());
    let counters = lcd.provider_counters().await;
    assert_eq!(
        (
            counters[0].attempts,
            counters[0].in_flight,
            counters[0].caller_cancellations,
            counters[0].failures
        ),
        (0, 0, 0, 0)
    );
    assert!(server.received_requests().await.unwrap().is_empty());
    assert_eq!(
        scheduled
            .ingestion_budget
            .as_ref()
            .unwrap()
            .permits
            .available_permits(),
        2
    );
}
#[tokio::test]
async fn issue1505_cancelled_circuit_wait_owns_no_provider_or_lane_permit() {
    for operation in [Operation::Identity, Operation::Data] {
        let (server, lcd) = fixture(5000, 1000).await;
        open(&lcd, 5000).await;
        let scheduled = lcd.with_ingestion_budget();
        let mut pending = Box::pin(observe(&scheduled, operation));
        tokio::select! {
            result = &mut pending => panic!("{operation:?}: must wait until cancelled: {result:?}"),
            _ = tokio::time::sleep(Duration::from_millis(75)) => {},
        }
        let counters = lcd.provider_counters().await;
        assert_eq!(
            (
                counters[0].attempts,
                counters[0].in_flight,
                counters[0].caller_cancellations
            ),
            (0, 0, 0)
        );
        assert_eq!(
            scheduled
                .ingestion_budget
                .as_ref()
                .unwrap()
                .permits
                .available_permits(),
            2
        );
        drop(pending);
        tokio::time::sleep(Duration::from_millis(75)).await;
        let after = lcd.provider_counters().await;
        assert_eq!(
            after[0].circuit_deferrals, counters[0].circuit_deferrals,
            "dropping the caller leaves no detached retry"
        );
        assert!(server.received_requests().await.unwrap().is_empty());
    }
}
#[tokio::test]
async fn issue1505_mixed_capacity_keeps_precedence_over_circuit_deferral() {
    let (server, _) = fixture(5000, 1000).await;
    let (second, _) = fixture(5000, 1000).await;
    let lcd = LcdClient::new_with_policy(
        vec![server.uri(), second.uri()],
        500,
        5000,
        "columbus-5",
        ProviderPolicy {
            concurrency: 1,
            ..Default::default()
        },
    )
    .with_ingestion_budget();
    let endpoints = lcd.inner.endpoints.read().await;
    endpoints[0].provider.failed(Duration::from_secs(5), None);
    let held = endpoints[1].provider.admit().unwrap();
    drop(endpoints);
    let deadline = tokio::time::Instant::now() + Duration::from_secs(1);
    let identity = lcd.probe_identity_scan(true, deadline).await;
    assert_eq!(
        (identity.attempts, identity.deferral),
        (0, Some(ScanDeferral::Capacity))
    );
    assert!(matches!(identity.result, Err(LcdError::AdmissionDeferred)));
    let attempts = AtomicUsize::new(0);
    let integrity = std::sync::Mutex::new(None);
    let data = lcd
        .get_bounded_attempts::<Value>(
            "/observation",
            RequestConstraints::default(),
            &attempts,
            &integrity,
            deadline,
        )
        .await;
    assert_eq!(
        (data.attempts, data.deferral),
        (0, Some(ScanDeferral::Capacity))
    );
    assert!(matches!(data.result, Err(LcdError::AdmissionDeferred)));
    assert!(server.received_requests().await.unwrap().is_empty());
    assert!(second.received_requests().await.unwrap().is_empty());
    drop(held);
}

#[tokio::test]
async fn issue1505_real_attempts_never_restart_the_scan_or_attempt_budget() {
    for operation in [Operation::Identity, Operation::Data] {
        let mut servers = Vec::new();
        for _ in 0..3 {
            let server = MockServer::start().await;
            match operation {
                Operation::Identity => {
                    Mock::given(path(chain_identity::NODE_INFO_PATH))
                        .respond_with(ResponseTemplate::new(503))
                        .mount(&server)
                        .await;
                }
                Operation::Data => {
                    mock_chain_identity(&server, "columbus-5").await;
                    Mock::given(path("/observation"))
                        .respond_with(ResponseTemplate::new(503))
                        .mount(&server)
                        .await;
                }
            }
            servers.push(server);
        }
        let lcd = LcdClient::new_with_policy(
            servers.iter().map(MockServer::uri).collect(),
            500,
            100,
            "columbus-5",
            ProviderPolicy {
                max_attempts: 2,
                deadline_ms: 2000,
                max_backoff_ms: 100,
                ..Default::default()
            },
        )
        .with_ingestion_budget();
        assert!(matches!(
            observe(&lcd, operation).await,
            Err(LcdError::AllEndpointsFailed(_))
        ));
        let counters = lcd.provider_counters().await;
        assert_eq!(
            counters.iter().map(|c| c.attempts).sum::<u64>(),
            2,
            "{operation:?}: actual failures cannot restart the outer scan"
        );
        assert_eq!(
            counters[2].attempts, 0,
            "the configured maximum remains authoritative"
        );
        assert_eq!(counters.iter().map(|c| c.failures).sum::<u64>(), 2);
        assert!(counters.iter().all(|c| c.in_flight == 0));
    }
}

#[tokio::test]
async fn issue1505_integrity_evidence_precedes_later_circuit_deferral() {
    for operation in [Operation::Identity, Operation::Data] {
        let wrong = MockServer::start().await;
        mock_chain_identity(&wrong, "wrong-chain").await;
        let (closed, _) = fixture(5000, 1000).await;
        let lcd = LcdClient::new_with_policy(
            vec![wrong.uri(), closed.uri()],
            500,
            5000,
            "columbus-5",
            ProviderPolicy::default(),
        )
        .with_ingestion_budget();
        lcd.inner.endpoints.read().await[1]
            .provider
            .failed(Duration::from_secs(5), None);
        let result = observe(&lcd, operation).await;
        assert!(
            matches!(result, Err(LcdError::ChainIdentity(_))),
            "{operation:?}: retain the actual integrity rejection"
        );
        let counters = lcd.provider_counters().await;
        assert_eq!(counters.iter().map(|c| c.attempts).sum::<u64>(), 1);
        assert_eq!(counters[1].attempts, 0);
        assert!(closed.received_requests().await.unwrap().is_empty());
    }
}

#[tokio::test]
async fn issue1505_empty_endpoint_inventory_is_not_a_circuit_deferral() {
    let lcd = LcdClient::new(vec![], 500, 100, "columbus-5").with_ingestion_budget();
    for operation in [Operation::Identity, Operation::Data] {
        assert!(matches!(
            observe(&lcd, operation).await,
            Err(LcdError::AllEndpointsFailed(_))
        ));
    }
    assert_eq!(
        lcd.ingestion_budget
            .as_ref()
            .unwrap()
            .permits
            .available_permits(),
        2
    );
}

#[tokio::test]
async fn issue1505_identity_metadata_wait_consumes_the_original_total_budget() {
    let (server, lcd) = fixture(5000, 100).await;
    let scheduled = lcd.with_ingestion_budget();
    let held = lcd.inner.endpoints.write().await;
    // The absolute budget must expire even while metadata is unavailable;
    // releasing this lock cannot grant a fresh identity or attempt deadline.
    assert!(
        tokio::time::timeout(Duration::from_secs(1), scheduled.require_verified_chain())
            .await
            .unwrap()
            .is_err()
    );
    drop(held);
    assert_eq!(lcd.provider_counters().await[0].attempts, 0);
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test]
async fn issue1505_near_deadline_identity_attempt_is_clipped_and_neutral() {
    let (server, lcd) = fixture(5000, 150).await;
    server.reset().await;
    let delayed_started = Arc::new(tokio::sync::Notify::new());
    let notify_started = delayed_started.clone();
    Mock::given(path(chain_identity::NODE_INFO_PATH))
        .respond_with(move |_: &wiremock::Request| {
            notify_started.notify_one();
            ResponseTemplate::new(200)
                .set_body_json(json!({"default_node_info":{"network":"columbus-5"}}))
                .set_delay(Duration::from_secs(1))
        })
        .with_priority(1)
        .mount(&server)
        .await;
    let scheduled = lcd.with_ingestion_budget();
    let held = lcd.inner.endpoints.write().await;
    let mut pending = Box::pin(scheduled.require_verified_chain());
    tokio::select! {
        result=&mut pending=>panic!("identity must wait for metadata: {result:?}"),
        _=tokio::time::sleep(Duration::from_millis(75))=>{},
    }
    drop(held);
    tokio::time::timeout(Duration::from_secs(1), async {
        tokio::select! {
            result=&mut pending=>panic!("identity completed before the actual delayed responder was reached: {result:?}"),
            _=delayed_started.notified()=>{},
        }
    }).await.expect("the actual delayed identity responder is observed");
    assert!(
        tokio::time::timeout(Duration::from_millis(200), pending)
            .await
            .unwrap()
            .is_err(),
        "the remaining total budget, not a fresh 250ms attempt, is authoritative"
    );
    let counters = lcd.provider_counters().await;
    assert_eq!(
        (
            counters[0].attempts,
            counters[0].caller_cancellations,
            counters[0].failures,
            counters[0].in_flight
        ),
        (1, 1, 0, 0)
    );
    assert!(
        !counters[0].circuit_open,
        "total-budget clipping does not condemn a healthy closed provider"
    );
    assert_eq!(server.received_requests().await.unwrap().len(), 1);
}

async fn identity_timeout_cleanup(half_open: bool, cancel: bool) {
    let server = MockServer::start().await;
    mock_chain_identity(&server, "columbus-5").await;
    let lcd = LcdClient::new_with_policy(
        vec![server.uri()],
        1000,
        1000,
        "columbus-5",
        ProviderPolicy {
            attempt_timeout_ms: 100,
            deadline_ms: 400,
            max_backoff_ms: 1000,
            ..Default::default()
        },
    );
    lcd.require_verified_chain().await.unwrap();
    let provider = lcd.inner.endpoints.read().await[0].provider.clone();
    if half_open {
        provider.failed(Duration::ZERO, None);
    }
    let before = provider.counters();
    server.reset().await;
    let started = Arc::new(tokio::sync::Notify::new());
    let notify = started.clone();
    Mock::given(path(chain_identity::NODE_INFO_PATH))
        .respond_with(move |_: &wiremock::Request| {
            notify.notify_one();
            ResponseTemplate::new(200)
                .set_body_json(json!({"default_node_info":{"network":"columbus-5"}}))
                .set_delay(Duration::from_secs(2))
        })
        .mount(&server)
        .await;
    let scheduled = lcd.with_ingestion_budget();
    let call_started = tokio::time::Instant::now();
    let mut pending = Box::pin(scheduled.require_verified_chain());
    tokio::time::timeout(Duration::from_secs(1),async {
        tokio::select! {
            result=&mut pending=>panic!("identity completed before the admitted-request barrier: {result:?}"),
            _=started.notified()=>{},
        }
    }).await.expect("the real delayed identity request started");
    let metadata = lcd.inner.endpoints.write().await;
    assert!(
        metadata[0].verified,
        "the previous positive metadata remains visible to this lock owner"
    );
    let early = tokio::select! {
        result=&mut pending=>Some(result),
        _=tokio::time::sleep(Duration::from_millis(150))=>None,
    };
    let while_blocked = provider.counters();
    let completion = if cancel {
        None
    } else {
        Some(match early {
            Some(result) => Ok(result),
            None => {
                tokio::time::timeout_at(call_started + Duration::from_millis(500), &mut pending)
                    .await
            }
        })
    };
    drop(pending);
    let final_counters = provider.counters();
    drop(metadata);
    let wire_before = server.received_requests().await.unwrap().len();
    let retry = lcd.require_verified_chain().await;
    let after = provider.counters();
    println!("identity_timeout_cleanup half_open={half_open} cancelled={cancel} bounded={} early_failures={} final_failures={} attempts={} wire_reads={wire_before}",
        completion.as_ref().is_none_or(|outcome|outcome.is_ok()),while_blocked.failures-before.failures,
        final_counters.failures-before.failures,final_counters.attempts-before.attempts);
    if let Some(completion) = completion {
        let result = completion
            .expect("identity timeout cleanup must finish inside its original total budget");
        assert!(
            matches!(result, Err(LcdError::AllEndpointsFailed(_))),
            "failed identity cleanup cannot report verified success"
        );
    }
    assert_eq!(
        while_blocked.failures,
        before.failures + 1,
        "a real attempt timeout immediately quarantines even while metadata is blocked"
    );
    assert_eq!(
        (
            final_counters.failures,
            final_counters.caller_cancellations,
            final_counters.in_flight
        ),
        (before.failures + 1, before.caller_cancellations, 0),
        "completion or cancellation counts the real timeout exactly once"
    );
    assert!(final_counters.circuit_open && !final_counters.half_open);
    assert!(
        retry.is_err(),
        "cached verified metadata alone cannot authorize chain identity"
    );
    assert_eq!(after.attempts, final_counters.attempts);
    assert_eq!(wire_before, 1);
    assert_eq!(server.received_requests().await.unwrap().len(), wire_before);
}

#[tokio::test]
async fn issue1505_identity_timeout_cleanup_retains_original_total_deadline() {
    for half_open in [false, true] {
        identity_timeout_cleanup(half_open, false).await;
    }
}

#[tokio::test]
async fn issue1505_identity_timeout_cleanup_cancellation_counts_failure_once() {
    for half_open in [false, true] {
        identity_timeout_cleanup(half_open, true).await;
    }
}
