//! The mapper's mandatory identity check must not have an extra serial preflight.
use super::tests::wrapper_fixture_with_smart_delay;
use super::*;
use std::time::Duration;

#[tokio::test]
async fn issue1505_redundant_identity_preflight_cannot_strand_required_native_fields() {
    use wiremock::{matchers::path, Mock, ResponseTemplate};
    let (server, lcd) = wrapper_fixture_with_smart_delay(
        Duration::from_millis(100),
        Duration::from_millis(100),
        Duration::from_millis(100),
    )
    .await;
    // Every identity HTTP response has the same permitted delay. No ordinal-
    // dependent fixture can move a first-call delay into the mapper after removal.
    Mock::given(path("/cosmos/base/tendermint/v1beta1/node_info"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"default_node_info":{"network":"columbus-5"}}))
                .set_delay(Duration::from_millis(1100)),
        )
        .with_priority(1)
        .mount(&server)
        .await;
    let started = tokio::time::Instant::now();
    let observed = Utc::now().timestamp_millis();
    let deadline = started + Duration::from_millis(7900);
    let scoped = lcd.with_observation_deadline(deadline);
    let dispatch = ObservationDispatch::new(2);
    let reads = ObservationReads {
        lcd: &scoped,
        dispatch: &dispatch,
        poll_started: started,
        deadline,
    };
    let phase = AtomicU8::new(0);
    let critical_completion = async {
        loop {
            if phase.load(Ordering::Relaxed) >= 5 {
                return Some(started.elapsed());
            }
            if started.elapsed() >= Duration::from_secs(8) {
                return None;
            }
            tokio::time::sleep(Duration::from_millis(1)).await;
        }
    };
    let work = tokio::time::timeout_at(
        started + Duration::from_secs(8),
        observe_inner(
            &reads,
            "terra1mapper",
            Some("terra1router"),
            &phase,
            observed,
        ),
    );
    let (result, critical_at) = tokio::join!(work, critical_completion);
    let snapshot = result
        .unwrap()
        .expect_available("independent scheduled fields may publish");
    let useful = snapshot
        .unwrap_output("uusd", 10000, observed + 8000)
        .is_some();
    let requests = server.received_requests().await.unwrap();
    let identities = requests
        .iter()
        .filter(|r| r.url.path() == "/cosmos/base/tendermint/v1beta1/node_info")
        .count();
    let data = requests.len() - identities;
    let counters = lcd.provider_counters().await;
    let critical_ms = critical_at.map(|d| d.as_millis() as u64);
    println!("identity_preflight_control elapsed_ms={} critical_ready_ms={:?} useful={} mappings={} uluna_cap={} uusd_cap={} router={} tax={} mapper_floor={} treasury_floor={} identity_wire={} data_wire={} provider_attempts={} failures={} cancellations={} logical_peak={}",
        started.elapsed().as_millis(),critical_ms,useful,snapshot.mappings.len(),snapshot.mappings.get("uluna").is_some_and(|m|m.tax_cap.is_some()),snapshot.mappings.get("uusd").is_some_and(|m|m.tax_cap.is_some()),snapshot.router_unwrap_supported==Some(true),snapshot.tax_atomics.is_some(),snapshot.mapper_floor_version.is_some(),snapshot.treasury_floor_version.is_some(),identities,data,counters[0].attempts,counters[0].failures,counters[0].caller_cancellations,dispatch.activity().peak);
    assert_eq!(snapshot.observed_at_ms, observed);
    assert!(snapshot.is_fresh(observed + 20000));
    assert!(!snapshot.is_fresh(observed + 20001));
    assert_eq!(counters[0].failures, 0);
    assert_eq!(counters[0].in_flight, 0);
    assert_eq!(dispatch.global.available_permits(), 2);
    assert!(data <= 13);
    assert!(
        useful,
        "redundant identity preflight cannot consume required output's original margin"
    );
    assert!(
        critical_ms.unwrap() < 7800,
        "critical fields complete before optional capability timeout"
    );
}

#[tokio::test]
async fn issue1505_healthy_wrapper_attests_each_data_attempt_without_extra_identity_request() {
    let (server, lcd) =
        wrapper_fixture_with_smart_delay(Duration::ZERO, Duration::ZERO, Duration::ZERO).await;
    let observed = Utc::now().timestamp_millis();
    let snapshot = observe_config(&lcd, "terra1mapper", Some("terra1router"))
        .await
        .expect_available("healthy scheduled observations");
    let requests = server.received_requests().await.unwrap();
    let identity = requests
        .iter()
        .filter(|r| r.url.path() == "/cosmos/base/tendermint/v1beta1/node_info")
        .count();
    let data = requests.len() - identity;
    let counters = lcd.provider_counters().await;
    println!("identity_preflight_healthy useful={} identity_wire={} data_wire={} provider_attempts={} failures={} cancellations={}",snapshot.unwrap_output("uusd",10000,observed+8000).is_some(),identity,data,counters[0].attempts,counters[0].failures,counters[0].caller_cancellations);
    assert_eq!(data, 13);
    assert!(snapshot
        .unwrap_output("uusd", 10000, observed + 8000)
        .is_some());
    assert_eq!(snapshot.direct_floor_protocol(true), Some(1));
    assert_eq!(snapshot.direct_floor_protocol(false), Some(1));
    assert_eq!(counters[0].failures, 0);
    assert_eq!(counters[0].caller_cancellations, 0);
    assert_eq!(
        identity, 13,
        "each data attempt is freshly attested without an extra standalone identity"
    );
    assert_eq!(counters[0].attempts, 13);
}

#[tokio::test]
async fn issue1505_mapper_rejects_fresh_wrong_malformed_or_unavailable_identity_after_old_positive()
{
    use wiremock::{matchers::path, Mock, ResponseTemplate};
    for kind in 0..3 {
        let (server, lcd) =
            wrapper_fixture_with_smart_delay(Duration::ZERO, Duration::ZERO, Duration::ZERO).await;
        let namespace = lcd.chain_cache_namespace();
        // Leave real old verified metadata, then change the remote response.
        let _: Value = lcd
            .query_contract_bounded("terra1mapper", &json!({"config":{}}), 8192)
            .await
            .unwrap();
        assert!(lcd.chain_identity_health().await.endpoints[0].verified);
        let before = lcd.data_request_poll_count();
        let response = match kind {
            0 => ResponseTemplate::new(200)
                .set_body_json(json!({"default_node_info":{"network":"wrong-chain"}})),
            1 => ResponseTemplate::new(200)
                .set_body_json(json!({"default_node_info":{"network":false}})),
            _ => ResponseTemplate::new(503),
        };
        Mock::given(path("/cosmos/base/tendermint/v1beta1/node_info"))
            .respond_with(response)
            .with_priority(1)
            .mount(&server)
            .await;
        let result = observe_config(&lcd, "terra1mapper", Some("terra1router")).await;
        assert!(matches!(result, WrapperObservation::Unavailable));
        assert_eq!(
            lcd.data_request_poll_count(),
            before,
            "historical verified health grants no data authority"
        );
        assert_eq!(lcd.chain_cache_namespace(), namespace);
        let health = lcd.chain_identity_health().await;
        assert!(!health.endpoints[0].verified);
        assert!(health.endpoints[0].last_error.is_some());
        assert!(health.endpoints[0].provider.failures > 0);
        assert_eq!(health.endpoints[0].provider.in_flight, 0);
    }
}

#[tokio::test]
async fn issue1505_mapper_alternative_requires_fresh_attestation_before_every_data_read() {
    use wiremock::{matchers::path, Mock, ResponseTemplate};
    let (wrong, _unused) =
        wrapper_fixture_with_smart_delay(Duration::ZERO, Duration::ZERO, Duration::ZERO).await;
    let (healthy, _unused) =
        wrapper_fixture_with_smart_delay(Duration::ZERO, Duration::ZERO, Duration::ZERO).await;
    Mock::given(path("/cosmos/base/tendermint/v1beta1/node_info"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"default_node_info":{"network":"wrong-chain"}})),
        )
        .with_priority(1)
        .mount(&wrong)
        .await;
    let lcd = LcdClient::new_with_policy(
        vec![wrong.uri(), healthy.uri()],
        2500,
        1,
        "columbus-5",
        crate::lcd::ProviderPolicy {
            attempt_timeout_ms: 2500,
            ..Default::default()
        },
    )
    .with_ingestion_budget();
    let namespace = lcd.chain_cache_namespace();
    let snapshot = observe_config(&lcd, "terra1mapper", Some("terra1router"))
        .await
        .expect_available("fresh qualified alternative");
    assert!(snapshot
        .unwrap_output("uusd", 10000, snapshot.observed_at_ms + 8000)
        .is_some());
    assert_eq!(lcd.chain_cache_namespace(), namespace);
    let rejected = wrong.received_requests().await.unwrap();
    assert!(!rejected.is_empty());
    assert!(rejected
        .iter()
        .all(|r| r.url.path() == "/cosmos/base/tendermint/v1beta1/node_info"));
    let accepted = healthy.received_requests().await.unwrap();
    let identities = accepted
        .iter()
        .filter(|r| r.url.path() == "/cosmos/base/tendermint/v1beta1/node_info")
        .count();
    assert_eq!(accepted.len() - identities, 13);
    assert_eq!(
        identities, 13,
        "every accepted data attempt attests its actual endpoint"
    );
    let health = lcd.chain_identity_health().await;
    assert!(!health.endpoints[0].verified);
    assert!(health.endpoints[0].provider.failures > 0);
    assert!(health.endpoints[1].verified);
    assert_eq!(health.endpoints[1].provider.failures, 0);
}

#[tokio::test]
async fn issue1505_mapper_identity_expiry_and_cancellation_start_no_data() {
    use std::{future::Future, task::Poll};
    use wiremock::{matchers::path, Mock, ResponseTemplate};
    for cancel in [false, true] {
        let (server, lcd) =
            wrapper_fixture_with_smart_delay(Duration::ZERO, Duration::ZERO, Duration::ZERO).await;
        Mock::given(path("/cosmos/base/tendermint/v1beta1/node_info"))
            .respond_with(
                ResponseTemplate::new(200)
                    .set_body_json(json!({"default_node_info":{"network":"columbus-5"}}))
                    .set_delay(Duration::from_millis(100)),
            )
            .with_priority(1)
            .mount(&server)
            .await;
        let started = tokio::time::Instant::now();
        let deadline = started + Duration::from_millis(150);
        let scoped = lcd.with_observation_deadline(deadline);
        let dispatch = ObservationDispatch::new(2);
        let reads = ObservationReads {
            lcd: &scoped,
            dispatch: &dispatch,
            poll_started: started,
            deadline,
        };
        let phase = AtomicU8::new(0);
        let mut work = Box::pin(observe_inner(
            &reads,
            "terra1mapper",
            Some("terra1router"),
            &phase,
            Utc::now().timestamp_millis(),
        ));
        let received = async {
            loop {
                if !server.received_requests().await.unwrap().is_empty() {
                    return;
                }
                assert!(
                    started.elapsed() < Duration::from_secs(1),
                    "finite identity arrival barrier"
                );
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        };
        tokio::select! {result=&mut work=>panic!("identity must be pending before suspension: {result:?}"),()=received=>{}}
        if cancel {
            drop(work);
        } else {
            // Do not poll the owned getter while the positive identity becomes
            // ready; after its original parent bound it must start no data.
            tokio::time::sleep_until(deadline + Duration::from_millis(50)).await;
            std::future::poll_fn(|cx| match work.as_mut().poll(cx) {
                Poll::Ready(result) => {
                    assert!(matches!(result, WrapperObservation::Unavailable));
                    Poll::Ready(())
                }
                Poll::Pending => Poll::Pending,
            })
            .await;
            drop(work);
        }
        assert_eq!(lcd.data_request_poll_count(), 0);
        assert_eq!(dispatch.global.available_permits(), 2);
        let counters = lcd.provider_counters().await;
        assert_eq!(counters[0].failures, 0);
        assert_eq!(counters[0].caller_cancellations, 1);
        assert_eq!(counters[0].in_flight, 0);
    }
}

#[tokio::test]
async fn issue1505_mapper_and_treasury_pauses_remain_authoritative_after_fresh_identity() {
    use super::tests::override_smart_read;
    for treasury_pause in [false, true] {
        let (server, lcd) =
            wrapper_fixture_with_smart_delay(Duration::ZERO, Duration::ZERO, Duration::ZERO).await;
        let (address, response, reason) = if treasury_pause {
            (
                "terra1treasury",
                json!({"wrapping_paused":true}),
                WrapperRevocation::TreasuryWrappingPaused,
            )
        } else {
            (
                "terra1mapper",
                json!({"paused":true,"treasury":"terra1treasury","fee_bps":100}),
                WrapperRevocation::MapperPaused,
            )
        };
        override_smart_read(
            &server,
            address,
            json!({"config":{}}),
            response,
            Duration::ZERO,
        )
        .await;
        assert!(
            matches!(observe_config(&lcd,"terra1mapper",Some("terra1router")).await,WrapperObservation::Revoked(actual) if actual==reason)
        );
        let counters = lcd.provider_counters().await;
        assert_eq!(counters[0].failures, 0);
        assert_eq!(counters[0].in_flight, 0);
    }
}
