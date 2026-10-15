//! Real HTTP at an owned producer deadline, shorter than the LCD policy clocks.
use super::*;
use serde_json::{json, Value};
use wiremock::{
    matchers::{path, path_regex},
    Mock, MockServer, ResponseTemplate,
};

#[derive(Clone, Copy, Debug)]
enum Operation {
    Data,
    Identity,
    BelowMinimum,
}

async fn run(lcd: &LcdClient, operation: Operation) -> Result<(), LcdError> {
    match operation {
        Operation::Data => lcd
            .query_contract_at_height_bounded::<Value>("terra1pair", &json!({"pool":{}}), 900, 8192)
            .await
            .map(|_| ()),
        Operation::Identity => lcd.require_verified_chain().await,
        Operation::BelowMinimum => lcd
            .get_latest_block_height_bounded_at_least(8192, 900)
            .await
            .map(|_| ()),
    }
}

async fn late_metadata(
    operation: Operation,
    scheduled: bool,
    half_open: bool,
    own_attempt: bool,
    wrong_chain: bool,
) {
    let server = MockServer::start().await;
    Mock::given(path("/cosmos/base/tendermint/v1beta1/node_info"))
        .respond_with(ResponseTemplate::new(200)
            .set_body_json(json!({"default_node_info":{"network":if wrong_chain {"wrong-chain"} else {"columbus-5"}}}))
            .set_delay(Duration::from_millis(80)))
        .mount(&server).await;
    Mock::given(path_regex(r"^/cosmwasm/wasm/v1/contract/terra1pair/smart/"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-cosmos-block-height", "900")
                .set_body_json(json!({"data":{"height":900}})),
        )
        .mount(&server)
        .await;
    let base = LcdClient::new_with_policy(
        vec![server.uri()],
        2500,
        5000,
        "columbus-5",
        ProviderPolicy {
            attempt_timeout_ms: 2500,
            deadline_ms: 4000,
            ..Default::default()
        },
    );
    if half_open {
        base.inner.endpoints.read().await[0]
            .provider
            .failed(Duration::from_millis(10), None);
        tokio::time::sleep(Duration::from_millis(20)).await;
    }
    let callbacks = Arc::new(AtomicUsize::new(0));
    let signal = callbacks.clone();
    let client = if scheduled {
        base.with_scheduled_observation()
    } else {
        base.clone()
    }
    .with_fee_safety_callback(Arc::new(move || {
        signal.fetch_add(1, Ordering::SeqCst);
    }));
    let started = tokio::time::Instant::now();
    let parent_deadline = started + Duration::from_millis(if own_attempt { 4000 } else { 300 });
    let client = client.with_observation_deadline(parent_deadline);
    assert_eq!(
        client.scheduled_observation, scheduled,
        "a scope grants no cancellation privilege"
    );
    let mut work = Box::pin(tokio::time::timeout_at(
        parent_deadline,
        run(&client, operation),
    ));
    let received = async {
        while server.received_requests().await.unwrap().is_empty() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    };
    tokio::select! {
        result = &mut work => panic!("identity transport must start before its metadata is held: {result:?}"),
        () = received => {},
    }
    let metadata = base.inner.endpoints.write().await;
    assert_eq!(base.inner.endpoint_metadata_waits.load(Ordering::SeqCst), 0);
    let parked = async {
        while base.inner.endpoint_metadata_waits.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    };
    tokio::select! {
        result = &mut work => panic!("successful/negative identity must await held metadata: {result:?}"),
        () = parked => {},
    }
    // Deliberately stop polling this caller-owned future until its parent or
    // intrinsic attempt deadline expires. No detached query keeps progressing.
    let release_at = if own_attempt {
        started + Duration::from_millis(2550)
    } else {
        parent_deadline + Duration::from_millis(50)
    };
    tokio::time::sleep_until(release_at).await;
    drop(metadata);
    let result = work.await;
    let counts = base.provider_counters().await;
    let selected = &counts[0];
    println!("parent_deadline_control operation={operation:?} scheduled={scheduled} half_open={half_open} own_attempt={own_attempt} wrong_chain={wrong_chain} elapsed_ms={} data_future_polls={} accepted={} failures={} cancellations={} callbacks={}",
        started.elapsed().as_millis(),base.data_request_poll_count(),matches!(result,Ok(Ok(()))),selected.failures,selected.caller_cancellations,callbacks.load(Ordering::SeqCst));
    assert_eq!(
        base.data_request_poll_count(),
        0,
        "no data future may start after the enclosing scope"
    );
    assert!(
        !matches!(result, Ok(Ok(()))),
        "identity-only success must not cross an original deadline"
    );
    if wrong_chain {
        assert!(
            matches!(result, Ok(Err(LcdError::ChainIdentity(_)))),
            "known integrity must win: {result:?}"
        );
    }
    let expected_failure =
        u64::from(half_open) + u64::from(wrong_chain || own_attempt || !scheduled || half_open);
    assert_eq!(selected.failures, expected_failure);
    assert_eq!(
        selected.caller_cancellations,
        u64::from(!wrong_chain && !own_attempt && scheduled && !half_open)
    );
    assert_eq!(
        callbacks.load(Ordering::SeqCst),
        usize::from(wrong_chain && matches!(operation, Operation::Data))
    );
    assert_eq!(selected.in_flight, 0);
    assert_eq!(selected.attempts, 1);
    assert_eq!(selected.recovery_probes, u64::from(half_open));
}

#[tokio::test]
async fn issue1505_parent_scope_stops_late_data_transport() {
    late_metadata(Operation::Data, true, false, false, false).await;
}
#[tokio::test]
async fn issue1505_parent_scope_identity_cannot_accept_late_positive() {
    late_metadata(Operation::Identity, true, false, false, false).await;
}
#[tokio::test]
async fn issue1505_parent_scope_direct_identity_keeps_quarantine() {
    late_metadata(Operation::Identity, false, false, false, false).await;
}
#[tokio::test]
async fn issue1505_parent_scope_half_open_identity_keeps_quarantine() {
    late_metadata(Operation::Identity, true, true, false, false).await;
}
#[tokio::test]
async fn issue1505_identity_own_deadline_cannot_accept_late_positive() {
    late_metadata(Operation::Identity, true, false, true, false).await;
}
#[tokio::test]
async fn issue1505_parent_scope_identity_retains_late_known_negative() {
    late_metadata(Operation::Identity, true, false, false, true).await;
}
#[tokio::test]
async fn issue1505_parent_scope_data_retains_late_known_negative() {
    late_metadata(Operation::Data, true, false, false, true).await;
}

#[tokio::test]
async fn issue1505_scope_view_min_merges_without_new_state_or_flags() {
    for paced in [false, true] {
        let base = LcdClient::new(vec![], 2500, 5000, "columbus-5");
        let base = if paced {
            base.with_ingestion_budget()
        } else {
            base
        };
        let now = tokio::time::Instant::now();
        let first = base.with_observation_deadline(now + Duration::from_secs(1));
        let later = first.with_observation_deadline(now + Duration::from_secs(2));
        let earlier = later.with_observation_deadline(now + Duration::from_millis(500));
        assert_eq!(base.observation_deadline(), None);
        assert_eq!(later.observation_deadline(), first.observation_deadline());
        assert_eq!(
            earlier.observation_deadline(),
            Some(now + Duration::from_millis(500))
        );
        assert!(Arc::ptr_eq(&base.inner, &earlier.inner));
        assert_eq!(base.scheduled_observation, earlier.scheduled_observation);
        assert_eq!(
            base.chain_cache_namespace(),
            earlier.chain_cache_namespace()
        );
        if paced {
            assert!(Arc::ptr_eq(
                base.ingestion_budget.as_ref().unwrap(),
                earlier.ingestion_budget.as_ref().unwrap()
            ));
        } else {
            assert!(earlier.ingestion_budget.is_none());
        }
    }
}

async fn buffered_body(operation: Operation, own_attempt: bool) {
    let server = MockServer::start().await;
    Mock::given(path("/cosmos/base/tendermint/v1beta1/node_info"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"default_node_info":{"network":"columbus-5"}}))
                // Distinguish the whole attempt clock from reqwest's later
                // per-data-wire clock in the intrinsic-attempt control.
                .set_delay(Duration::from_millis(if own_attempt { 200 } else { 0 })),
        )
        .mount(&server)
        .await;
    let target = if matches!(operation, Operation::BelowMinimum) {
        LATEST_BLOCK_PATH.to_owned()
    } else {
        format!(
            "/cosmwasm/wasm/v1/contract/terra1pair/smart/{}",
            base64::engine::general_purpose::STANDARD
                .encode(serde_json::to_vec(&json!({"pool":{}})).unwrap())
        )
    };
    let body = if matches!(operation, Operation::BelowMinimum) {
        json!({"block":{"header":{"chain_id":"columbus-5","height":"800","time":"2026-01-01T00:00:00Z"}}})
    } else {
        json!({"data":{"height":900}})
    };
    Mock::given(path(target.clone()))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-cosmos-block-height", "900")
                .set_body_json(body)
                .set_delay(Duration::from_millis(80)),
        )
        .mount(&server)
        .await;
    let base = LcdClient::new_with_policy(
        vec![server.uri()],
        2500,
        5000,
        "columbus-5",
        ProviderPolicy {
            attempt_timeout_ms: 2500,
            deadline_ms: 4000,
            ..Default::default()
        },
    )
    .with_scheduled_observation();
    let started = tokio::time::Instant::now();
    let deadline = started + Duration::from_millis(if own_attempt { 4000 } else { 300 });
    let scoped = base.with_observation_deadline(deadline);
    let mut work = Box::pin(tokio::time::timeout_at(deadline, run(&scoped, operation)));
    let received = async {
        loop {
            if server
                .received_requests()
                .await
                .unwrap()
                .iter()
                .any(|request| request.url.path() == target)
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    };
    tokio::select! {
        result = &mut work => panic!("transport must be observed before buffered reply: {result:?}"),
        () = received => {},
    }
    let resume_at = if own_attempt {
        started + Duration::from_millis(2550)
    } else {
        deadline + Duration::from_millis(50)
    };
    tokio::time::sleep_until(resume_at).await;
    let result = work.await;
    let counts = base.provider_counters().await;
    let latest = base.inner.latest_height.load(Ordering::SeqCst);
    println!("buffered_body_control operation={operation:?} own_attempt={own_attempt} elapsed_ms={} accepted={} successes={} failures={} cancellations={} shared_latest_height={latest}",
        started.elapsed().as_millis(),matches!(result,Ok(Ok(()))),counts[0].successes,counts[0].failures,counts[0].caller_cancellations);
    assert!(
        !matches!(result, Ok(Ok(()))),
        "buffered positive cannot be accepted after its original deadline"
    );
    assert_eq!(
        counts[0].successes, 0,
        "below-minimum positive cannot reset health after expiry"
    );
    assert_eq!(
        latest, 0,
        "expired positive cannot advance the shared live-height watermark"
    );
    assert_eq!(
        base.data_request_poll_count(),
        1,
        "the original transport began before expiry"
    );
    assert_eq!(counts[0].failures, u64::from(own_attempt));
    assert_eq!(counts[0].caller_cancellations, u64::from(!own_attempt));
    assert_eq!(counts[0].in_flight, 0);
}

#[tokio::test]
async fn issue1505_buffered_body_parent_cannot_accept_late_positive() {
    buffered_body(Operation::Data, false).await;
}
#[tokio::test]
async fn issue1505_buffered_body_own_attempt_cannot_accept_late_positive() {
    buffered_body(Operation::Data, true).await;
}
#[tokio::test]
async fn issue1505_buffered_body_parent_cannot_publish_below_minimum_height() {
    buffered_body(Operation::BelowMinimum, false).await;
}

#[tokio::test]
async fn issue1505_ineligible_head_keeps_actual_observation_without_inventing_floor() {
    let server = MockServer::start().await;
    mock_chain_identity(&server, "columbus-5").await;
    Mock::given(path(LATEST_BLOCK_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "block":{"header":{"chain_id":"columbus-5","height":"900","time":"2026-01-01T00:00:00Z"}}
        }))).mount(&server).await;
    let lcd = LcdClient::new_with_policy(
        vec![server.uri()],
        2500,
        5000,
        "columbus-5",
        ProviderPolicy {
            attempt_timeout_ms: 2500,
            deadline_ms: 4000,
            ..Default::default()
        },
    )
    .with_scheduled_observation();
    let result = lcd
        .get_latest_block_height_bounded_at_least(8192, 1000)
        .await;
    let counts = lcd.provider_counters().await;
    let latest = lcd.inner.latest_height.load(Ordering::SeqCst);
    println!("ineligible_head_control accepted={} shared_latest_height={latest} attempts={} successes={} failures={}",
        result.is_ok(),counts[0].attempts,counts[0].successes,counts[0].failures);
    assert!(result.is_err());
    assert_eq!(counts[0].attempts, 1);
    assert_eq!(
        counts[0].successes, 1,
        "valid ineligible head still proves provider health"
    );
    assert_eq!(counts[0].failures, 0);
    assert_eq!(
        latest, 900,
        "actual timely attestation remains shared lag evidence"
    );
    assert_ne!(
        latest, 1000,
        "the query minimum is never an observed height"
    );
    Mock::given(path(LATEST_BLOCK_PATH))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "block":{"header":{"chain_id":"columbus-5","height":"860","time":"2026-01-01T00:00:00Z"}}
        })))
        .with_priority(1).mount(&server).await;
    assert!(
        lcd.get_latest_block_height_bounded(8192).await.is_err(),
        "a later860 response still violates the unchanged20-block lag policy"
    );
    let after = lcd.provider_counters().await;
    assert_eq!(after[0].attempts, 2);
    assert_eq!(after[0].successes, 1);
    assert_eq!(after[0].failures, 1);
    assert_eq!(lcd.inner.latest_height.load(Ordering::SeqCst), 900);
}
