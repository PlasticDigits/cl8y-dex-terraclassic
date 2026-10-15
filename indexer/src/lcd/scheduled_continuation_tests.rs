//! One scheduled query may finish its unspent alternatives, never restart work.
use super::*;
use serde_json::{json, Value};
use wiremock::{matchers::path_regex, Mock, MockServer, ResponseTemplate};

async fn historical_fixture(max_attempts: usize, deadline_ms: u64) -> ([MockServer; 2], LcdClient) {
    let servers = [MockServer::start().await, MockServer::start().await];
    for (index, server) in servers.iter().enumerate() {
        mock_chain_identity(server, "columbus-5").await;
        Mock::given(path_regex(r"^/cosmwasm/wasm/v1/contract/terra1pair/smart/"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header(
                        "x-cosmos-block-height",
                        if index == 0 { "899" } else { "900" },
                    )
                    .set_body_json(json!({"data":{"height":900}})),
            )
            .mount(server)
            .await;
    }
    let lcd = LcdClient::new_with_policy(
        servers.iter().map(MockServer::uri).collect(),
        2500,
        5000,
        "columbus-5",
        ProviderPolicy {
            concurrency: 1,
            max_attempts,
            attempt_timeout_ms: 2500,
            deadline_ms,
            ..Default::default()
        },
    );
    (servers, lcd)
}

async fn pinned(lcd: &LcdClient) -> Result<Value, LcdError> {
    lcd.query_contract_at_height_bounded("terra1pair", &json!({"pool":{}}), 900, 8192)
        .await
}

async fn data_hits(server: &MockServer) -> usize {
    server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|request| request.url.path().contains("/smart/"))
        .count()
}

async fn observed_capacity(
    lcd: &LcdClient,
    callbacks: &AtomicUsize,
    ordinals: &[usize],
    deadline: tokio::time::Instant,
) {
    tokio::time::timeout_at(deadline, async {
        loop {
            let counts = lcd.provider_counters().await;
            if callbacks.load(Ordering::SeqCst) > 0
                && ordinals
                    .iter()
                    .all(|&index| counts[index].concurrency_deferrals > 0)
            {
                return;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .expect("actual negative and alternate deferral inside the original query deadline");
}

#[tokio::test]
async fn issue1505_scheduled_unattempted_capacity_recovers_after_historical_rejection() {
    let (servers, lcd) = historical_fixture(3, 4000).await;
    let provider = lcd.inner.endpoints.read().await[1].provider.clone();
    let mut held = provider.admit().unwrap();
    let callbacks = Arc::new(AtomicUsize::new(0));
    let signal = callbacks.clone();
    let scheduled = lcd
        .with_scheduled_observation()
        .with_fee_safety_callback(Arc::new(move || {
            signal.fetch_add(1, Ordering::SeqCst);
        }));
    assert!(
        scheduled.ingestion_budget.is_none(),
        "production pool lane has no quote pacing view"
    );
    let started = tokio::time::Instant::now();
    let work = async {
        let result = pinned(&scheduled).await;
        (result, started.elapsed())
    };
    let release = async {
        tokio::join!(
            tokio::time::sleep(Duration::from_millis(300)),
            observed_capacity(&lcd, &callbacks, &[1], started + Duration::from_secs(4)),
        );
        assert_eq!(
            callbacks.load(Ordering::SeqCst),
            1,
            "historical negative must be latched before the alternate becomes available"
        );
        held.completed();
        drop(held);
    };
    let ((result, elapsed), ()) = tokio::join!(work, release);
    let counts = lcd.provider_counters().await;
    println!("scheduled_continuation_control elapsed_ms={} primary_attempts={} alternate_attempts={} primary_data_reads={} alternate_data_reads={} callbacks={}",
        elapsed.as_millis(), counts[0].attempts, counts[1].attempts - 1,
        data_hits(&servers[0]).await,data_hits(&servers[1]).await,callbacks.load(Ordering::SeqCst));
    assert!(result.is_ok(), "an unattempted qualified alternate becomes available inside the original deadline: {result:?}");
    assert_eq!(result.unwrap()["height"], 900);
    assert!(elapsed >= Duration::from_millis(300) && elapsed < Duration::from_secs(4));
    assert_eq!((counts[0].attempts, counts[1].attempts - 1), (1, 1));
    assert_eq!((counts[0].failures, counts[1].failures), (1, 0));
    assert!(counts[1].concurrency_deferrals > 0);
    assert_eq!(
        (data_hits(&servers[0]).await, data_hits(&servers[1]).await),
        (1, 1)
    );
    assert_eq!(callbacks.load(Ordering::SeqCst), 1);
    assert!(counts.iter().all(|provider| provider.in_flight == 0));
}

fn with_callback(lcd: LcdClient) -> (LcdClient, Arc<AtomicUsize>) {
    let callbacks = Arc::new(AtomicUsize::new(0));
    let signal = callbacks.clone();
    (
        lcd.with_fee_safety_callback(Arc::new(move || {
            signal.fetch_add(1, Ordering::SeqCst);
        })),
        callbacks,
    )
}

#[tokio::test]
#[tracing_test::traced_test]
async fn issue1505_scheduled_continuation_preserves_quote_pacing_and_one_wait_witness() {
    let (servers, lcd) = historical_fixture(3, 4000).await;
    let provider = lcd.inner.endpoints.read().await[1].provider.clone();
    let mut held = provider.admit().unwrap();
    let (scheduled, callbacks) = with_callback(lcd.with_ingestion_budget());
    let started = tokio::time::Instant::now();
    let work = async { pinned(&scheduled).await };
    let release = async {
        tokio::join!(
            tokio::time::sleep(Duration::from_millis(450)),
            observed_capacity(&lcd, &callbacks, &[1], started + Duration::from_secs(4)),
        );
        assert_eq!(callbacks.load(Ordering::SeqCst), 1);
        held.completed();
        drop(held);
    };
    let (result, ()) = tokio::join!(work, release);
    assert_eq!(result.unwrap()["height"], 900);
    let counts = lcd.provider_counters().await;
    assert_eq!((counts[0].attempts, counts[1].attempts - 1), (1, 1));
    assert_eq!(
        (data_hits(&servers[0]).await, data_hits(&servers[1]).await),
        (1, 1)
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
    logs_assert(|lines: &[&str]| {
        let events: Vec<_> = lines
            .iter()
            .filter(|line| line.contains("LCD scheduled alternatives deferred"))
            .collect();
        assert_eq!(events.len(), 1, "one event per query, never per50ms wait");
        let event = events[0];
        for field in [
            "phase=\"data\"",
            "reason=\"capacity\"",
            "attempt_count=1",
            "unattempted_capacity_count=1",
            "original_remaining_ms=",
        ] {
            assert!(event.contains(field));
        }
        assert!(
            !event.contains("http") && !event.contains("terra1pair") && !event.contains("/smart/")
        );
        Ok(())
    });
}

#[tokio::test]
async fn issue1505_scheduled_continuation_expiry_retains_historical_and_chain_integrity() {
    for wrong_chain in [false, true] {
        let (servers, lcd) = historical_fixture(3, 250).await;
        if wrong_chain {
            // The common attestation mock already has priority1. Replace the
            // owned server's routes rather than adding an equal-priority mock.
            servers[0].reset().await;
            mock_chain_identity(&servers[0], "wrong-chain").await;
        }
        let provider = lcd.inner.endpoints.read().await[1].provider.clone();
        let mut held = provider.admit().unwrap();
        let (scheduled, callbacks) = with_callback(lcd.with_scheduled_observation());
        let started = tokio::time::Instant::now();
        let result = tokio::time::timeout(Duration::from_secs(1), pinned(&scheduled))
            .await
            .unwrap();
        assert!(started.elapsed() >= Duration::from_millis(250));
        if wrong_chain {
            assert!(
                matches!(result, Err(LcdError::ChainIdentity(_))),
                "{result:?}"
            );
        } else {
            assert!(
                matches!(result, Err(LcdError::HistoricalHeightUnavailable)),
                "{result:?}"
            );
        }
        let counts = lcd.provider_counters().await;
        assert_eq!((counts[0].attempts, counts[1].attempts - 1), (1, 0));
        assert_eq!((counts[0].failures, counts[1].failures), (1, 0));
        assert!(counts[1].concurrency_deferrals >= 2);
        assert_eq!(callbacks.load(Ordering::SeqCst), 1);
        assert_eq!(data_hits(&servers[0]).await, usize::from(!wrong_chain));
        assert_eq!(data_hits(&servers[1]).await, 0);
        held.completed();
        drop(held);
        assert!(lcd
            .provider_counters()
            .await
            .iter()
            .all(|provider| provider.in_flight == 0));
    }
}

#[tokio::test]
async fn issue1505_scheduled_continuation_spends_one_global_attempt_budget() {
    let (servers, original) = historical_fixture(2, 4000).await;
    let third = MockServer::start().await;
    mock_chain_identity(&third, "columbus-5").await;
    for (server, height) in [(&servers[1], "899"), (&third, "900")] {
        Mock::given(path_regex(r"^/cosmwasm/wasm/v1/contract/terra1pair/smart/"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("x-cosmos-block-height", height)
                    .set_body_json(json!({"data":{"height":900}})),
            )
            .with_priority(1)
            .mount(server)
            .await;
    }
    let lcd = LcdClient::new_with_policy(
        vec![servers[0].uri(), servers[1].uri(), third.uri()],
        2500,
        10,
        "columbus-5",
        original.inner.provider_policy.clone(),
    );
    let mut held = Vec::new();
    for endpoint in lcd.inner.endpoints.read().await.iter().skip(1) {
        held.push(endpoint.provider.admit().unwrap());
    }
    let (scheduled, callbacks) = with_callback(lcd.with_scheduled_observation());
    let started = tokio::time::Instant::now();
    let work = async { pinned(&scheduled).await };
    let release = async {
        tokio::join!(
            tokio::time::sleep(Duration::from_millis(300)),
            observed_capacity(&lcd, &callbacks, &[1, 2], started + Duration::from_secs(4)),
        );
        assert_eq!(callbacks.load(Ordering::SeqCst), 1);
        for mut permit in held {
            permit.completed();
        }
    };
    let (result, ()) = tokio::join!(work, release);
    assert!(matches!(result, Err(LcdError::HistoricalHeightUnavailable)));
    assert!(
        started.elapsed() >= Duration::from_millis(300)
            && started.elapsed() < Duration::from_secs(4)
    );
    let counts = lcd.provider_counters().await;
    assert_eq!(
        (
            counts[0].attempts,
            counts[1].attempts - 1,
            counts[2].attempts - 1
        ),
        (1, 1, 0)
    );
    assert_eq!(
        (counts[0].failures, counts[1].failures, counts[2].failures),
        (1, 1, 0)
    );
    assert_eq!(
        (
            data_hits(&servers[0]).await,
            data_hits(&servers[1]).await,
            data_hits(&third).await
        ),
        (1, 1, 0)
    );
    assert_eq!(callbacks.load(Ordering::SeqCst), 2);
    assert!(counts.iter().all(|provider| provider.in_flight == 0));
    // Primary cooldown expired long before the alternate was released. Its
    // successful re-admission must still not become this query's second cursor.
    let provider = lcd.inner.endpoints.read().await[0].provider.clone();
    let mut ready = provider.admit().expect("primary cooldown has expired");
    ready.completed();
    drop(ready);
}

#[tokio::test]
async fn issue1505_scheduled_continuation_cancellation_keeps_negative_and_releases_owned_work() {
    let (servers, lcd) = historical_fixture(3, 4000).await;
    let provider = lcd.inner.endpoints.read().await[1].provider.clone();
    let mut held = provider.admit().unwrap();
    let (scheduled, callbacks) = with_callback(lcd.with_ingestion_budget());
    let mut work = Box::pin(pinned(&scheduled));
    let waiting = async {
        loop {
            if lcd.provider_counters().await[1].concurrency_deferrals > 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    };
    tokio::select! {
        result = &mut work => panic!("original query must own the capacity wait: {result:?}"),
        result = tokio::time::timeout(Duration::from_secs(1), waiting) => result.unwrap(),
    }
    assert_eq!(callbacks.load(Ordering::SeqCst), 1);
    assert_eq!(
        scheduled
            .ingestion_budget
            .as_ref()
            .unwrap()
            .permits
            .available_permits(),
        2
    );
    drop(work);
    let before = lcd.provider_counters().await;
    assert_eq!(
        (before[0].attempts, before[0].failures, before[0].in_flight),
        (1, 1, 0)
    );
    held.completed();
    drop(held);
    tokio::time::sleep(Duration::from_millis(75)).await;
    let after = lcd.provider_counters().await;
    assert_eq!(
        after[1].attempts, before[1].attempts,
        "no detached retry after cancellation"
    );
    assert_eq!(
        after[1].concurrency_deferrals,
        before[1].concurrency_deferrals
    );
    assert_eq!(callbacks.load(Ordering::SeqCst), 1);
    assert_eq!(
        (data_hits(&servers[0]).await, data_hits(&servers[1]).await),
        (1, 0)
    );
    assert!(after.iter().all(|provider| provider.in_flight == 0));
}

#[tokio::test]
async fn issue1505_scheduled_continuation_direct_and_spent_budget_remain_prompt() {
    for (scheduled, max_attempts) in [(false, 3), (true, 1)] {
        let (servers, lcd) = historical_fixture(max_attempts, 4000).await;
        let provider = lcd.inner.endpoints.read().await[1].provider.clone();
        let mut held = provider.admit().unwrap();
        let client = if scheduled {
            lcd.with_scheduled_observation()
        } else {
            lcd.clone()
        };
        let (client, callbacks) = with_callback(client);
        let result = tokio::time::timeout(Duration::from_millis(300), pinned(&client))
            .await
            .unwrap();
        assert!(matches!(result, Err(LcdError::HistoricalHeightUnavailable)));
        let counts = lcd.provider_counters().await;
        assert_eq!((counts[0].attempts, counts[1].attempts - 1), (1, 0));
        assert_eq!(
            counts[1].concurrency_deferrals,
            if scheduled { 0 } else { 1 }
        );
        assert_eq!(callbacks.load(Ordering::SeqCst), 1);
        assert_eq!(data_hits(&servers[1]).await, 0);
        held.completed();
        drop(held);
    }
}

async fn metadata_deadline_control(own_attempt: bool, scheduled: bool) {
    let primary = MockServer::start().await;
    let selected = MockServer::start().await;
    let mut urls = Vec::new();
    if !own_attempt {
        mock_chain_identity(&primary, "columbus-5").await;
        Mock::given(path_regex(r"^/cosmwasm/wasm/v1/contract/terra1pair/smart/"))
            .respond_with(
                ResponseTemplate::new(200)
                    .insert_header("x-cosmos-block-height", "899")
                    .set_body_json(json!({"data":{"height":900}}))
                    .set_delay(Duration::from_secs(2)),
            )
            .mount(&primary)
            .await;
        urls.push(primary.uri());
    }
    Mock::given(wiremock::matchers::path(
        "/cosmos/base/tendermint/v1beta1/node_info",
    ))
    .respond_with(
        ResponseTemplate::new(200)
            .set_body_json(json!({"default_node_info":{"network":"columbus-5"}}))
            .set_delay(Duration::from_millis(200)),
    )
    .mount(&selected)
    .await;
    Mock::given(path_regex(r"^/cosmwasm/wasm/v1/contract/terra1pair/smart/"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("x-cosmos-block-height", "900")
                .set_body_json(json!({"data":{"height":900}})),
        )
        .mount(&selected)
        .await;
    urls.push(selected.uri());
    let lcd = LcdClient::new_with_policy(
        urls,
        2500,
        5000,
        "columbus-5",
        ProviderPolicy {
            attempt_timeout_ms: 2500,
            deadline_ms: 4000,
            ..Default::default()
        },
    );
    let (client, callbacks) = with_callback(if scheduled {
        lcd.with_scheduled_observation()
    } else {
        lcd.clone()
    });
    let started = tokio::time::Instant::now();
    let mut work = Box::pin(pinned(&client));
    let selected_received = async {
        while selected.received_requests().await.unwrap().is_empty() {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    };
    tokio::select! {
        result = &mut work => panic!("selected identity transport must start before metadata is held: {result:?}"),
        () = selected_received => {},
    }
    let metadata = lcd.inner.endpoints.write().await;
    let expected_metadata = if own_attempt { 1 } else { 2 };
    assert!(lcd.inner.endpoint_metadata_waits.load(Ordering::SeqCst) < expected_metadata);
    let awaiting_metadata = async {
        while lcd.inner.endpoint_metadata_waits.load(Ordering::SeqCst) < expected_metadata {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    };
    tokio::select! {
        result = &mut work => panic!("successful identity must now wait for held metadata: {result:?}"),
        () = awaiting_metadata => {},
    }
    // The caller owns and deliberately does not poll work across the deadline.
    // The server and timers continue normally. This exercises Timeout's
    // inner-first poll after the successful metadata write becomes available.
    let boundary_ms = if own_attempt { 2500 } else { 4000 };
    tokio::time::sleep_until(started + Duration::from_millis(boundary_ms + 50)).await;
    drop(metadata);
    let result = work.await;
    let prefix_data_polls = usize::from(!own_attempt);
    let data_polls = lcd.inner.data_request_polls.load(Ordering::SeqCst);
    let counts = lcd.provider_counters().await;
    println!("metadata_deadline_control own_attempt={} scheduled={} elapsed_ms={} data_future_polls={} allowed_prefix_polls={} selected_failures={} selected_cancellations={} callbacks={}",
        own_attempt, scheduled, started.elapsed().as_millis(),data_polls,prefix_data_polls,
        counts.last().unwrap().failures,counts.last().unwrap().caller_cancellations,callbacks.load(Ordering::SeqCst));
    assert_eq!(
        data_polls, prefix_data_polls,
        "elapsed original boundary must not first-poll the data transport after metadata unlock"
    );
    assert_eq!(data_hits(&selected).await, 0);
    assert_eq!(callbacks.load(Ordering::SeqCst), usize::from(!own_attempt));
    if own_attempt {
        assert!(
            matches!(result, Err(LcdError::AllEndpointsFailed(_))),
            "{result:?}"
        );
    } else {
        assert!(
            matches!(result, Err(LcdError::HistoricalHeightUnavailable)),
            "original negative cannot be lost: {result:?}"
        );
    }
    let selected = counts.last().unwrap();
    assert_eq!(selected.failures, u64::from(own_attempt || !scheduled));
    assert_eq!(
        selected.caller_cancellations,
        u64::from(!own_attempt && scheduled)
    );
    assert!(counts.iter().all(|provider| provider.in_flight == 0));
}

#[tokio::test]
async fn issue1505_post_metadata_total_deadline_never_polls_data_transport() {
    metadata_deadline_control(false, true).await;
}

#[tokio::test]
async fn issue1505_post_metadata_attempt_deadline_never_polls_data_transport() {
    metadata_deadline_control(true, true).await;
}

#[tokio::test]
async fn issue1505_post_metadata_direct_total_deadline_keeps_generic_quarantine() {
    metadata_deadline_control(false, false).await;
}
