//! Router-first is a legal old select order, forced without a timing race.
use super::tests::{override_smart_read, wrapper_fixture, wrapper_fixture_with_smart_delay};
use super::*;
use std::{future::Future, task::Poll, time::Duration};

#[tokio::test]
async fn issue1505_router_first_cannot_delay_common_prerequisites_past_native_cap() {
    router_first_control(true, 1200, true).await;
}

#[tokio::test]
async fn issue1505_legacy_router_first_control_exposes_missing_native_output() {
    // Explicit no-plan counterfactual retains the old router-first competition.
    router_first_control(false, 1200, false).await;
}

async fn router_first_control(planned: bool, cap_ms: u64, expect_useful: bool) {
    use wiremock::{matchers::path, Mock, ResponseTemplate};
    let (server, lcd) =
        wrapper_fixture(Duration::from_millis(2200), Duration::from_millis(100)).await;
    for (address, query, response, millis) in [
        (
            "terra1mapper",
            json!({"config":{}}),
            json!({"paused":false,"treasury":"terra1treasury","fee_bps":100}),
            1000,
        ),
        (
            "terra1treasury",
            json!({"config":{}}),
            json!({"wrapping_paused":false}),
            1600,
        ),
        (
            "terra1treasury",
            json!({"denom_wrappers":{}}),
            json!({"wrappers":[{"denom":"uluna","wrapper":"terra1mapper"},{"denom":"uusd","wrapper":"terra1mapper"}]}),
            1600,
        ),
        (
            "terra1mapper",
            json!({"denom_mapping":{"denom":"uluna"}}),
            json!({"denom":"uluna","cw20_addr":"terra1wrapped"}),
            200,
        ),
        (
            "terra1mapper",
            json!({"rate_limit":{"denom":"uluna"}}),
            json!({"config":null,"amount_used":"0"}),
            200,
        ),
        (
            "terra1mapper",
            json!({"denom_mapping":{"denom":"uusd"}}),
            json!({"denom":"uusd","cw20_addr":"terra1wrapped"}),
            1900,
        ),
        (
            "terra1mapper",
            json!({"rate_limit":{"denom":"uusd"}}),
            json!({"config":null,"amount_used":"0"}),
            1400,
        ),
        (
            "terra1mapper",
            json!({"execution_capabilities":{}}),
            json!({"direct_wrapper_floor_version":1}),
            100,
        ),
        (
            "terra1treasury",
            json!({"execution_capabilities":{}}),
            json!({"direct_wrapper_floor_version":1}),
            100,
        ),
    ] {
        override_smart_read(
            &server,
            address,
            query,
            response,
            Duration::from_millis(millis),
        )
        .await;
    }
    Mock::given(path("/terra/treasury/v1beta1/tax_caps/uusd"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"tax_cap":"1000"}))
                .set_delay(Duration::from_millis(cap_ms)),
        )
        .with_priority(1)
        .mount(&server)
        .await;
    let started = tokio::time::Instant::now();
    let observed = Utc::now().timestamp_millis();
    let deadline = started + Duration::from_millis(7900);
    let lcd = lcd.with_observation_deadline(deadline);
    let dispatch = ObservationDispatch::new(2);
    let reads = ObservationReads {
        lcd: &lcd,
        dispatch: &dispatch,
        poll_started: started,
        deadline,
    };
    assert!(reads
        .read(ReadSlot::Chain, lcd.require_verified_chain())
        .await
        .is_some());
    let query = json!({"config":{}});
    let config: Value = reads
        .read(
            ReadSlot::MapperConfig,
            lcd.query_contract_bounded("terra1mapper", &query, 8192),
        )
        .await
        .unwrap();
    assert_eq!(config["paused"], false);
    let treasury = config["treasury"].as_str().unwrap().to_owned();
    let fees = mapper_fees(&config).unwrap();
    if planned {
        assert!(dispatch.plan_router_prerequisites());
    }
    let mut router = Box::pin(observe_router_bounded_with_reads(
        &reads,
        Some("terra1router"),
        "terra1mapper",
        deadline,
    ));
    // This is one allowed production select order. New admission policy must
    // remain correct even if this router future is first-polled by its owner.
    std::future::poll_fn(|cx| {
        assert!(router.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
    let phase = AtomicU8::new(2);
    let work = async {
        tokio::join!(
            router,
            observe_settlement(
                &reads,
                "terra1mapper",
                treasury,
                fees,
                observed,
                deadline,
                &phase
            )
        )
    };
    let (router, result) = tokio::time::timeout_at(started + Duration::from_secs(8), work)
        .await
        .unwrap();
    let mut snapshot = result
        .expect_available("current validated common prerequisites must publish a native view");
    snapshot.router_unwrap_supported = router;
    let useful = snapshot
        .unwrap_output("uusd", 10000, observed + 8000)
        .is_some();
    let activity = dispatch.activity();
    let admitted = |slot_name: &str| {
        activity
            .admitted
            .iter()
            .flatten()
            .enumerate()
            .find(|(_, (slot, _))| slot.name() == slot_name)
            .map(|(index, (_, at))| {
                (
                    index,
                    at.saturating_duration_since(started).as_millis() as u64,
                )
            })
            .unwrap()
    };
    let router_admit = admitted("router_config");
    let treasury_admit = admitted("treasury_config");
    let bindings_admit = admitted("denom_bindings");
    let counters = lcd.provider_counters().await;
    let data_wire = server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|request| request.url.path() != "/cosmos/base/tendermint/v1beta1/node_info")
        .count();
    println!("router_priority_control elapsed_ms={} router_index={} router_at_ms={} treasury_index={} treasury_at_ms={} bindings_index={} bindings_at_ms={} useful={} mappings={} uusd_cap={} data_wire={} provider_attempts={} failures={} cancellations={} logical_peak={} borrowed={}",
        started.elapsed().as_millis(),router_admit.0,router_admit.1,treasury_admit.0,treasury_admit.1,bindings_admit.0,bindings_admit.1,useful,snapshot.mappings.len(),snapshot.mappings.get("uusd").is_some_and(|m|m.tax_cap.is_some()),data_wire,counters[0].attempts,counters[0].failures,counters[0].caller_cancellations,activity.peak,dispatch.borrowed_reads());
    println!(
        "router_priority_native_logical_admission mapping_ms={} rate_ms={} cap_ms={}",
        admitted("uusd_mapping").1,
        admitted("uusd_rate").1,
        admitted("uusd_cap").1
    );
    assert_eq!(counters[0].failures, 0);
    assert_eq!(counters[0].in_flight, 0);
    assert_eq!(snapshot.observed_at_ms, observed);
    assert!(started.elapsed() < Duration::from_secs(8));
    assert!(data_wire <= 13);
    assert_eq!(
        useful, expect_useful,
        "router-first admission must preserve required native output work"
    );
    assert_eq!(activity.peak, 2);
    if planned {
        assert!(treasury_admit.0 < router_admit.0 && bindings_admit.0 < router_admit.0);
        assert_eq!(data_wire, 13);
        assert_eq!(snapshot.direct_floor_protocol(true), Some(1));
        assert_eq!(snapshot.direct_floor_protocol(false), Some(1));
    } else {
        assert!(router_admit.0 < treasury_admit.0 && router_admit.0 < bindings_admit.0);
    }
}

#[tokio::test]
async fn issue1505_router_barrier_requires_admission_after_cooperative_yield() {
    tokio::spawn(async {
        let dispatch = ObservationDispatch::new(2);
        assert!(dispatch.plan_router_prerequisites());
        let (_server, lcd) = wrapper_fixture(Duration::ZERO, Duration::ZERO).await;
        let started = tokio::time::Instant::now();
        let deadline = started + Duration::from_secs(1);
        let reads = ObservationReads {
            lcd: &lcd,
            dispatch: &dispatch,
            poll_started: started,
            deadline,
        };
        let treasury = reads.read(
            ReadSlot::TreasuryConfig,
            std::future::pending::<Result<(), LcdError>>(),
        );
        let bindings = reads.read(
            ReadSlot::DenomBindings,
            std::future::pending::<Result<(), LcdError>>(),
        );
        let router = dispatch.wait_router_prerequisites();
        tokio::pin!(treasury, bindings, router);
        // Setup may have consumed part of this task's 128-unit budget.
        // Exhaust only the remainder without yielding into a renewed budget.
        for _ in 0..128 {
            if !tokio::task::coop::has_budget_remaining() {
                break;
            }
            tokio::task::consume_budget().await;
        }
        assert!(!tokio::task::coop::has_budget_remaining());
        let (treasury_pending, bindings_pending, permits, router_pending) =
            std::future::poll_fn(|cx| {
                Poll::Ready((
                    treasury.as_mut().poll(cx).is_pending(),
                    bindings.as_mut().poll(cx).is_pending(),
                    dispatch.global.available_permits(),
                    router.as_mut().poll(cx).is_pending(),
                ))
            })
            .await;
        // Restore task budget before assertions can unwind the mock server.
        tokio::task::yield_now().await;
        assert!(treasury_pending && bindings_pending);
        assert_eq!(permits, 2);
        assert!(
            router_pending,
            "cooperative yield before semaphore admission cannot release router work"
        );
    })
    .await
    .unwrap();
}

async fn assert_pending<T>(future: &mut std::pin::Pin<Box<impl Future<Output = T>>>) {
    std::future::poll_fn(|cx| {
        assert!(future.as_mut().poll(cx).is_pending());
        Poll::Ready(())
    })
    .await;
}

#[tokio::test]
async fn issue1505_router_barrier_admits_both_common_reads_with_one_permit() {
    let (_server, lcd) = wrapper_fixture(Duration::ZERO, Duration::ZERO).await;
    let dispatch = ObservationDispatch::new(1);
    // Unplanned standalone callers never depend on a production-only barrier.
    dispatch.wait_router_prerequisites().await;
    assert!(dispatch.plan_router_prerequisites());
    assert!(!dispatch.plan_router_prerequisites());
    let started = tokio::time::Instant::now();
    let reads = ObservationReads {
        lcd: &lcd,
        dispatch: &dispatch,
        poll_started: started,
        deadline: started + Duration::from_secs(1),
    };
    let mut router = Box::pin(dispatch.wait_router_prerequisites());
    assert_pending(&mut router).await;
    assert_eq!(
        reads
            .read(ReadSlot::TreasuryConfig, async { Ok::<_, LcdError>(1) })
            .await,
        Some(1)
    );
    assert_pending(&mut router).await;
    // Duplicate admissions cannot substitute for the other prerequisite.
    assert_eq!(
        reads
            .read(ReadSlot::TreasuryConfig, async { Ok::<_, LcdError>(1) })
            .await,
        Some(1)
    );
    assert_pending(&mut router).await;
    assert_eq!(
        reads
            .read(ReadSlot::DenomBindings, async { Ok::<_, LcdError>(1) })
            .await,
        Some(1)
    );
    router.await;
    assert!(
        !dispatch.plan_router_prerequisites(),
        "completed plan cannot renew"
    );
    assert_eq!(dispatch.global.available_permits(), 1);
}

#[tokio::test(start_paused = true)]
async fn issue1505_router_barrier_expiry_cancellation_and_absence_start_no_transport() {
    let (server, lcd) = wrapper_fixture(Duration::ZERO, Duration::ZERO).await;
    for cancel in [false, true] {
        let dispatch = ObservationDispatch::new(2);
        assert!(dispatch.plan_router_prerequisites());
        let started = tokio::time::Instant::now();
        let deadline = started + Duration::from_secs(1);
        let scoped = lcd.with_observation_deadline(deadline);
        let reads = ObservationReads {
            lcd: &scoped,
            dispatch: &dispatch,
            poll_started: started,
            deadline,
        };
        let mut router = Box::pin(observe_router_bounded_with_reads(
            &reads,
            Some("terra1router"),
            "terra1mapper",
            deadline,
        ));
        assert_pending(&mut router).await;
        assert_eq!(dispatch.global.available_permits(), 2);
        assert_eq!(lcd.provider_counters().await[0].attempts, 0);
        if cancel {
            drop(router);
        } else {
            // Resume after the parent elapsed with a now-ready barrier. The
            // timeout's inner-first polling must still refuse physical work.
            tokio::time::advance(Duration::from_secs(1)).await;
            dispatch.common_admitted(ReadSlot::TreasuryConfig);
            dispatch.common_admitted(ReadSlot::DenomBindings);
            assert_eq!(router.await, None);
        }
        // No detached waiter may begin work after cancellation/expiration.
        dispatch.common_admitted(ReadSlot::TreasuryConfig);
        dispatch.common_admitted(ReadSlot::DenomBindings);
        tokio::task::yield_now().await;
        assert_eq!(
            observe_router_bounded_with_reads(&reads, None, "terra1mapper", started).await,
            Some(false)
        );
        assert_eq!(dispatch.global.available_permits(), 2);
        assert_eq!(lcd.provider_counters().await[0].attempts, 0);
        assert_eq!(lcd.data_request_poll_count(), 0);
    }
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test(start_paused = true)]
async fn issue1505_router_barrier_expired_queued_common_read_cannot_signal_admission() {
    let (_server, lcd) = wrapper_fixture(Duration::ZERO, Duration::ZERO).await;
    let dispatch = ObservationDispatch::new(0);
    assert!(dispatch.plan_router_prerequisites());
    let started = tokio::time::Instant::now();
    let reads = ObservationReads {
        lcd: &lcd,
        dispatch: &dispatch,
        poll_started: started,
        deadline: started + Duration::from_secs(1),
    };
    let polls = AtomicU8::new(0);
    let mut common = Box::pin(reads.read(ReadSlot::TreasuryConfig, async {
        polls.fetch_add(1, Ordering::Relaxed);
        Ok::<_, LcdError>(1)
    }));
    assert_pending(&mut common).await;
    tokio::time::advance(Duration::from_secs(1)).await;
    dispatch.global.add_permits(1);
    assert_eq!(common.await, None);
    dispatch.common_admitted(ReadSlot::DenomBindings);
    let mut router = Box::pin(dispatch.wait_router_prerequisites());
    assert_pending(&mut router).await;
    assert_eq!(polls.load(Ordering::Relaxed), 0);
    assert_eq!(dispatch.global.available_permits(), 1);
}

#[tokio::test]
async fn issue1505_router_barrier_production_observe_inner_initializes_priority() {
    let (_server, lcd) =
        wrapper_fixture_with_smart_delay(Duration::ZERO, Duration::ZERO, Duration::ZERO).await;
    let started = tokio::time::Instant::now();
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
    let observed = Utc::now().timestamp_millis();
    let snapshot = observe_inner(
        &reads,
        "terra1mapper",
        Some("terra1router"),
        &phase,
        observed,
    )
    .await
    .expect_available("production initializes the common priority barrier");
    assert!(
        !dispatch.plan_router_prerequisites(),
        "production must initialize before constructing router work"
    );
    let admitted = dispatch.activity().admitted;
    let ordinal = |name| {
        admitted
            .iter()
            .flatten()
            .position(|(slot, _)| slot.name() == name)
            .unwrap()
    };
    assert!(ordinal("treasury_config") < ordinal("router_config"));
    assert!(ordinal("denom_bindings") < ordinal("router_config"));
    assert_eq!(snapshot.router_unwrap_supported, Some(true));
    assert_eq!(snapshot.mappings.len(), 2);
    assert_eq!(snapshot.observed_at_ms, observed);
    assert_eq!(lcd.data_request_poll_count(), 13);
    assert_eq!(dispatch.global.available_permits(), 2);
    assert_eq!(lcd.provider_counters().await[0].failures, 0);
}

#[tokio::test]
async fn issue1505_router_barrier_production_pause_and_invalid_bindings_cancel_owned_work() {
    for pause in [false, true] {
        let (_server, lcd) =
            wrapper_fixture_with_smart_delay(Duration::ZERO, Duration::ZERO, Duration::ZERO).await;
        if pause {
            override_smart_read(
                &_server,
                "terra1treasury",
                json!({"config":{}}),
                json!({"wrapping_paused":true}),
                Duration::ZERO,
            )
            .await;
            override_smart_read(
                &_server,
                "terra1treasury",
                json!({"denom_wrappers":{}}),
                json!({"wrappers":[]}),
                Duration::from_millis(2200),
            )
            .await;
        } else {
            override_smart_read(
                &_server,
                "terra1treasury",
                json!({"denom_wrappers":{}}),
                json!({"wrappers":false}),
                Duration::ZERO,
            )
            .await;
        }
        let started = tokio::time::Instant::now();
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
        let outcome = observe_inner(
            &reads,
            "terra1mapper",
            Some("terra1router"),
            &phase,
            Utc::now().timestamp_millis(),
        )
        .await;
        if pause {
            assert!(matches!(
                outcome,
                WrapperObservation::Revoked(WrapperRevocation::TreasuryWrappingPaused)
            ));
        } else {
            assert!(matches!(outcome, WrapperObservation::Unavailable));
        }
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "negative settlement cannot await the unrelated delayed work"
        );
        assert_eq!(dispatch.global.available_permits(), 2);
        assert!(dispatch
            .activity()
            .admitted
            .iter()
            .flatten()
            .all(|(slot, _)| slot.denom().is_none()));
        let counters = lcd.provider_counters().await;
        assert_eq!(counters[0].failures, 0);
        assert_eq!(counters[0].in_flight, 0);
    }
}
