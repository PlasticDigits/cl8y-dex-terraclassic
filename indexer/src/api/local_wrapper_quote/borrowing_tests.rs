//! Bounded work-conserving native dispatch controls.
use super::tests::{override_smart_read, wrapper_fixture};
use super::*;
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};

#[tokio::test]
async fn issue1505_idle_peer_slot_cannot_strand_required_native_cap() {
    use base64::Engine;
    use std::sync::atomic::AtomicBool;
    use wiremock::{matchers::path, Mock, ResponseTemplate};
    let (server, lcd) =
        wrapper_fixture(Duration::from_millis(800), Duration::from_millis(50)).await;
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
            1000,
        ),
        (
            "terra1mapper",
            json!({"denom_mapping":{"denom":"uluna"}}),
            json!({"denom":"uluna","cw20_addr":"terra1wrapped"}),
            100,
        ),
        (
            "terra1mapper",
            json!({"rate_limit":{"denom":"uluna"}}),
            json!({"config":null,"amount_used":"0"}),
            100,
        ),
        (
            "terra1mapper",
            json!({"denom_mapping":{"denom":"uusd"}}),
            json!({"denom":"uusd","cw20_addr":"terra1wrapped"}),
            2200,
        ),
        (
            "terra1mapper",
            json!({"rate_limit":{"denom":"uusd"}}),
            json!({"config":null,"amount_used":"0"}),
            2000,
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
                .set_delay(Duration::from_millis(900)),
        )
        .with_priority(1)
        .mount(&server)
        .await;
    let started = tokio::time::Instant::now();
    let observed_at_ms = Utc::now().timestamp_millis();
    let deadline = started + Duration::from_millis(7900);
    let lcd = lcd.with_observation_deadline(deadline);
    let dispatch = ObservationDispatch::new(2);
    let reads = ObservationReads {
        lcd: &lcd,
        dispatch: &dispatch,
        poll_started: started,
        deadline,
    };
    let phase = AtomicU8::new(0);
    let done = AtomicBool::new(false);
    let work = async {
        let result = tokio::time::timeout_at(
            started + Duration::from_secs(8),
            observe_inner(
                &reads,
                "terra1mapper",
                Some("terra1router"),
                &phase,
                observed_at_ms,
            ),
        )
        .await;
        let elapsed = started.elapsed();
        done.store(true, Ordering::SeqCst);
        (result, elapsed)
    };
    let idle_probe = async {
        let mut since = None;
        let mut longest = Duration::ZERO;
        loop {
            let requests = server.received_requests().await.unwrap();
            let uluna_reads = requests
                .iter()
                .filter(|request| {
                    let path = request.url.path();
                    if path.ends_with("/tax_caps/uluna") {
                        return true;
                    }
                    path.split("/smart/")
                        .nth(1)
                        .and_then(|encoded| {
                            base64::engine::general_purpose::STANDARD
                                .decode(encoded)
                                .ok()
                        })
                        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
                        .is_some_and(|query| {
                            query["denom_mapping"]["denom"] == "uluna"
                                || query["rate_limit"]["denom"] == "uluna"
                        })
                })
                .count();
            let cap_started = requests
                .iter()
                .any(|request| request.url.path().ends_with("/tax_caps/uusd"));
            let idle = uluna_reads == 3
                && !cap_started
                && dispatch.denoms[0].available_permits() == 1
                && dispatch.denoms[1].available_permits() == 0
                && dispatch.global.available_permits() == 1;
            if idle {
                let beginning = *since.get_or_insert_with(tokio::time::Instant::now);
                longest = longest.max(beginning.elapsed());
            } else {
                since = None;
            }
            if done.load(Ordering::SeqCst) {
                return longest;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    };
    let ((result, elapsed), idle) = tokio::join!(work, idle_probe);
    let snapshot = result
        .unwrap()
        .expect_available("valid prerequisites publish a same-poll native view");
    let requests = server.received_requests().await.unwrap();
    let data_reads = requests
        .iter()
        .filter(|request| request.url.path() != "/cosmos/base/tendermint/v1beta1/node_info")
        .count();
    let counters = lcd.provider_counters().await;
    let useful = snapshot
        .unwrap_output("uusd", 10000, observed_at_ms + 8000)
        .is_some();
    println!("idle_peer_control elapsed_ms={} idle_global_ms={} mappings={} uusd_cap={} useful={} data_reads={} failures={} cancellations={}",
            elapsed.as_millis(),idle.as_millis(),snapshot.mappings.len(),snapshot.mappings.get("uusd").is_some_and(|mapping|mapping.tax_cap.is_some()),useful,data_reads,counters[0].failures,counters[0].caller_cancellations);
    assert_eq!(snapshot.observed_at_ms, observed_at_ms);
    assert_eq!(snapshot.mappings.len(), 2);
    assert_eq!(counters[0].failures, 0);
    assert_eq!(counters[0].in_flight, 0);
    assert!(elapsed < Duration::from_secs(8));
    assert!(data_reads <= 13);
    if !useful {
        assert!(
            idle >= Duration::from_millis(250),
            "the negative control requires a genuinely idle global slot after the peer finished"
        );
    }
    assert!(useful,"required cap cannot wait behind its own predecessors while the completed peer's global slot is idle");
    assert_eq!(data_reads, 13);
    assert_eq!(snapshot.direct_floor_protocol(true), Some(1));
    assert_eq!(snapshot.direct_floor_protocol(false), Some(1));
    assert!(idle < Duration::from_millis(250));
    let activity = dispatch.activity();
    println!("idle_peer_dispatch_stats global_peak={} uluna_peak={} uusd_peak={} early_parallel={} borrowed={}",
            activity.peak,activity.native_peak[0],activity.native_peak[1],activity.parallel_before_peer_done,dispatch.borrowed_reads());
    assert_eq!(activity.peak, 2);
    assert!(activity.native_peak[1] > 1);
    assert!(!activity.parallel_before_peer_done);
    assert!((1..=6).contains(&dispatch.borrowed_reads()));
}

#[tokio::test(start_paused = true)]
async fn issue1505_borrow_is_symmetric_and_global_two_survives_cancellation() {
    let (server, lcd) = wrapper_fixture(Duration::ZERO, Duration::ZERO).await;
    for slow in [0, 1] {
        let dispatch = ObservationDispatch::new(2);
        assert!(dispatch.plan_native_reads([true, true]));
        let started = tokio::time::Instant::now();
        let reads = ObservationReads {
            lcd: &lcd,
            dispatch: &dispatch,
            poll_started: started,
            deadline: started + Duration::from_secs(1),
        };
        let slots = [
            [
                ReadSlot::UlunaMapping,
                ReadSlot::UlunaRate,
                ReadSlot::UlunaCap,
            ],
            [ReadSlot::UusdMapping, ReadSlot::UusdRate, ReadSlot::UusdCap],
        ];
        let polls = AtomicUsize::new(0);
        let blocked = || async {
            polls.fetch_add(1, Ordering::SeqCst);
            std::future::pending::<Result<u8, LcdError>>().await
        };
        let fast = || async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            Ok::<_, LcdError>(1u8)
        };
        let slow_reads = async {
            tokio::join!(
                reads.read(slots[slow][0], blocked()),
                reads.read(slots[slow][1], blocked()),
                reads.read(slots[slow][2], blocked())
            )
        };
        let fast_reads = async {
            let result = tokio::join!(
                reads.read(slots[1 - slow][0], fast()),
                reads.read(slots[1 - slow][1], fast()),
                reads.read(slots[1 - slow][2], fast())
            );
            assert_eq!(started.elapsed(), Duration::from_millis(30));
            result
        };
        let (slow_result, fast_result) = tokio::join!(slow_reads, fast_reads);
        assert_eq!(slow_result, (None, None, None));
        assert_eq!(fast_result, (Some(1), Some(1), Some(1)));
        assert_eq!(
            polls.load(Ordering::SeqCst),
            2,
            "borrow can fill only one additional global slot"
        );
        assert_eq!(dispatch.global.available_permits(), 2);
        assert!(dispatch
            .denoms
            .iter()
            .all(|gate| gate.available_permits() == 1));
        let activity = dispatch.activity();
        assert_eq!(activity.peak, 2);
        assert_eq!(activity.native_peak[slow], 2);
        assert_eq!(activity.native_peak[1 - slow], 1);
        assert!(!activity.parallel_before_peer_done);
        assert_eq!(
            dispatch.borrowed_reads(),
            1,
            "expired queued work cannot record admission"
        );
    }
    assert!(server.received_requests().await.unwrap().is_empty());
    assert_eq!(lcd.provider_counters().await[0].attempts, 0);
}

#[tokio::test(start_paused = true)]
async fn issue1505_borrow_both_hot_never_borrows_before_original_deadline() {
    let (server, lcd) = wrapper_fixture(Duration::ZERO, Duration::ZERO).await;
    let dispatch = ObservationDispatch::new(2);
    assert!(dispatch.plan_native_reads([true, true]));
    let started = tokio::time::Instant::now();
    let reads = ObservationReads {
        lcd: &lcd,
        dispatch: &dispatch,
        poll_started: started,
        deadline: started + Duration::from_secs(1),
    };
    let polls = AtomicUsize::new(0);
    let blocked = || async {
        polls.fetch_add(1, Ordering::SeqCst);
        std::future::pending::<Result<u8, LcdError>>().await
    };
    let results = tokio::join!(
        reads.read(ReadSlot::UlunaMapping, blocked()),
        reads.read(ReadSlot::UlunaRate, blocked()),
        reads.read(ReadSlot::UlunaCap, blocked()),
        reads.read(ReadSlot::UusdMapping, blocked()),
        reads.read(ReadSlot::UusdRate, blocked()),
        reads.read(ReadSlot::UusdCap, blocked()),
    );
    assert_eq!(results, (None, None, None, None, None, None));
    assert_eq!(polls.load(Ordering::SeqCst), 2);
    assert_eq!(dispatch.borrowed_reads(), 0);
    assert_eq!(dispatch.activity().native_peak, [1, 1]);
    assert_eq!(dispatch.activity().peak, 2);
    assert!(!dispatch.activity().parallel_before_peer_done);
    assert_eq!(dispatch.global.available_permits(), 2);
    assert!(dispatch
        .denoms
        .iter()
        .all(|gate| gate.available_permits() == 1));
    assert!(server.received_requests().await.unwrap().is_empty());
}

#[tokio::test(start_paused = true)]
async fn issue1505_borrow_peer_error_finishes_work_without_authorizing_its_result() {
    let (server, lcd) = wrapper_fixture(Duration::ZERO, Duration::ZERO).await;
    let dispatch = ObservationDispatch::new(2);
    assert!(dispatch.plan_native_reads([true, true]));
    let started = tokio::time::Instant::now();
    let reads = ObservationReads {
        lcd: &lcd,
        dispatch: &dispatch,
        poll_started: started,
        deadline: started + Duration::from_secs(1),
    };
    let own = async {
        tokio::join!(
            reads.read(
                ReadSlot::UlunaMapping,
                std::future::pending::<Result<u8, LcdError>>()
            ),
            reads.read(ReadSlot::UlunaRate, async { Ok::<_, LcdError>(2u8) }),
            reads.read(ReadSlot::UlunaCap, async { Ok::<_, LcdError>(3u8) }),
        )
    };
    let peer = async {
        tokio::join!(
            reads.read(ReadSlot::UusdMapping, async {
                Err::<u8, _>(LcdError::ContractQueryRejected("known negative".into()))
            }),
            reads.read(ReadSlot::UusdRate, async { Ok::<_, LcdError>(2u8) }),
            reads.read(ReadSlot::UusdCap, async { Ok::<_, LcdError>(3u8) }),
        )
    };
    let (own_result, peer_result) = tokio::join!(own, peer);
    assert_eq!(
        peer_result,
        (None, Some(2), Some(3)),
        "borrowing cannot turn a negative into an observation"
    );
    assert_eq!(own_result, (None, Some(2), Some(3)));
    assert!(dispatch.borrowed_reads() > 0);
    assert_eq!(dispatch.activity().peak, 2);
    assert!(!dispatch.activity().parallel_before_peer_done);
    assert_eq!(dispatch.global.available_permits(), 2);
    assert!(server.received_requests().await.unwrap().is_empty());
    assert_eq!(lcd.provider_counters().await[0].attempts, 0);
}

#[tokio::test(start_paused = true)]
async fn issue1505_borrow_unbound_peer_and_unpolled_drop_retire_only_owned_work() {
    let (server, lcd) = wrapper_fixture(Duration::ZERO, Duration::ZERO).await;
    let dispatch = ObservationDispatch::new(2);
    assert!(dispatch.plan_native_reads([true, false]));
    let started = tokio::time::Instant::now();
    let reads = ObservationReads {
        lcd: &lcd,
        dispatch: &dispatch,
        poll_started: started,
        deadline: started + Duration::from_secs(1),
    };
    let polls = AtomicUsize::new(0);
    let forbidden = || async {
        polls.fetch_add(1, Ordering::SeqCst);
        Ok::<_, LcdError>(1u8)
    };
    assert_eq!(reads.read(ReadSlot::UusdMapping, forbidden()).await, None);
    let original = reads.read(ReadSlot::UlunaMapping, forbidden());
    assert_eq!(reads.read(ReadSlot::UlunaMapping, forbidden()).await, None);
    assert_eq!(polls.load(Ordering::SeqCst), 0);
    drop(original); // Construction-time completion survives an unpolled future.
    let available = || async {
        tokio::time::sleep(Duration::from_millis(10)).await;
        Ok::<_, LcdError>(1u8)
    };
    let result = tokio::join!(
        reads.read(ReadSlot::UlunaRate, available()),
        reads.read(ReadSlot::UlunaCap, available())
    );
    assert_eq!(result, (Some(1), Some(1)));
    assert_eq!(started.elapsed(), Duration::from_millis(10));
    assert_eq!(dispatch.activity().peak, 2);
    assert_eq!(dispatch.activity().native_peak, [2, 0]);
    assert!(!dispatch.activity().parallel_before_peer_done);
    assert_eq!(dispatch.borrowed_reads(), 1);
    assert!(!dispatch.plan_native_reads([true, true]));
    assert!(server.received_requests().await.unwrap().is_empty());
    assert_eq!(dispatch.global.available_permits(), 2);
}
