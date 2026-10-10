use super::*;
fn complete(at: DateTime<Utc>) -> Arc<HopMirror> {
    Arc::new(HopMirror {
        pair_id: 1,
        asset_0_addr: "asset0".into(),
        asset_1_addr: "asset1".into(),
        reserve_0: 10_000,
        reserve_1: 10_000,
        fee_bps: 30,
        block_height: Some(100),
        snapshot_at: at,
        freshness: MirrorFreshness::Fresh,
        bids: Vec::new(),
        asks: Vec::new(),
    })
}
fn qualified(namespace: &str, mirror: Arc<HopMirror>, expiry: i64) {
    assert!(qualify(
        namespace,
        vec![Qualification {
            pair_id: 1,
            assets: (1, 2),
            context: "reviewed".into(),
            expiry_ms: expiry,
            max_age_ms: 20_000,
            mirror: Some(mirror)
        }]
    ));
}
fn old(namespace: &str, at: DateTime<Utc>) -> Option<(Arc<HopMirror>, i64)> {
    retained(namespace, 1, "reviewed", Some(101), at, (20_000, 20_000))
}
#[test]
#[serial_test::serial]
fn recovery1464_fee_phase_requires_exact_raw_context_and_keeps_original_deadline() {
    let ns = "fee-phase-exact";
    let original = Utc::now() - chrono::Duration::seconds(6);
    let mirror = complete(original);
    let expiry = Utc::now().timestamp_millis() + 10_000;
    qualified(ns, mirror.clone(), expiry);
    let at = Utc::now();
    let owner = begin(ns, 1, (1, 2), 101, at, (20_000, 20_000)).unwrap();
    let (saved, deadline) = old(ns, at).unwrap();
    assert!(Arc::ptr_eq(&saved, &old(ns, at).unwrap().0));
    assert_eq!(saved.snapshot_at, mirror.snapshot_at);
    assert_eq!(deadline, expiry);
    assert!(retained(ns, 1, "changed", Some(101), at, (20_000, 20_000)).is_none());
    assert!(retained(ns, 1, "reviewed", Some(101), at, (20_000, 19_999)).is_none());
    assert!(retained(ns, 1, "reviewed", Some(102), at, (20_000, 20_000)).is_none());
    assert!(begin(ns, 1, (1, 2), 99, original, (20_000, 20_000)).is_none());
    owner.finish_transient();
    assert!(old(ns, at).is_some());
    let reg = registry().lock().unwrap();
    let until = reg.namespaces[ns].entries[&1].phase.unwrap().until;
    drop(reg);
    assert!(
        until <= Instant::now() + PHASE_BOUND,
        "typed availability must not renew the original bound"
    );
    reject(ns, 1, Some(101), at);
    assert!(!permits(ns, 1, Some(100), original));
}
#[test]
#[serial_test::serial]
fn recovery1464_fee_phase_expiry_unknown_zero_and_cancellation_fail_closed() {
    let ns = "fee-phase-expiry";
    let at = Utc::now();
    qualified(
        ns,
        complete(at - chrono::Duration::seconds(19)),
        at.timestamp_millis() + 10_000,
    );
    let guard = begin(ns, 1, (1, 2), 101, at, (20_000, 20_000)).unwrap();
    assert!(old(ns, at).is_some());
    registry()
        .lock()
        .unwrap()
        .namespaces
        .get_mut(ns)
        .unwrap()
        .entries
        .get_mut(&1)
        .unwrap()
        .complete
        .as_mut()
        .unwrap()
        .mirror = complete(Utc::now() - chrono::Duration::seconds(21));
    assert!(
        old(ns, at).is_none(),
        "original completed snapshot expiry is decisive while fee is pending"
    );
    drop(guard);
    assert!(!permits(ns, 1, Some(101), at));
    assert!(begin("unqualified", 1, (1, 2), 101, at, (20_000, 20_000)).is_none());
    let ns = "fee-phase-zero";
    qualified(ns, complete(at), at.timestamp_millis() + 10_000);
    assert!(begin(ns, 1, (1, 2), 101, at, (0, 20_000)).is_none());
    assert!(!permits(ns, 1, Some(100), at));
    let ns = "fee-phase-bound";
    qualified(ns, complete(at), at.timestamp_millis() + 10_000);
    let guard = begin(ns, 1, (1, 2), 101, at, (20_000, 20_000)).unwrap();
    guard.finish_transient();
    registry()
        .lock()
        .unwrap()
        .namespaces
        .get_mut(ns)
        .unwrap()
        .entries
        .get_mut(&1)
        .unwrap()
        .phase
        .as_mut()
        .unwrap()
        .until = Instant::now();
    assert!(old(ns, at).is_none());
    assert!(!permits(ns, 1, Some(100), at));
}
#[test]
#[serial_test::serial]
fn recovery1464_fee_phase_matching_alternative_recovers_but_obsolete_positive_never_does() {
    let ns = "fee-phase-alternative";
    let at = Utc::now();
    qualified(
        ns,
        complete(at - chrono::Duration::seconds(1)),
        at.timestamp_millis() + 10_000,
    );
    let guard = begin(ns, 1, (1, 2), 101, at, (20_000, 20_000)).unwrap();
    let callback = guard.safety_callback();
    callback();
    assert!(!permits(
        ns,
        1,
        Some(100),
        at - chrono::Duration::seconds(1)
    ));
    guard.finish_validated();
    assert!(permits(ns, 1, Some(101), at));
    assert!(!permits(
        ns,
        1,
        Some(100),
        at - chrono::Duration::seconds(1)
    ));
    let first = begin(ns, 1, (1, 2), 102, Utc::now(), (20_000, 20_000)).unwrap();
    let second = begin(ns, 1, (1, 2), 103, Utc::now(), (20_000, 20_000)).unwrap();
    second.finish_negative();
    first.finish_validated();
    assert!(!permits(ns, 1, Some(103), Utc::now()));
}
#[test]
#[serial_test::serial]
fn recovery1464_fee_phase_capacity_and_context_changes_cannot_enable_fallback() {
    let ns = "fee-phase-context";
    let at = Utc::now();
    qualified(
        ns,
        complete(at - chrono::Duration::seconds(1)),
        at.timestamp_millis() + 10_000,
    );
    let guard = begin(ns, 1, (1, 2), 101, at, (20_000, 20_000)).unwrap();
    assert!(qualify(
        ns,
        vec![Qualification {
            pair_id: 1,
            assets: (1, 2),
            context: "new-policy".into(),
            expiry_ms: at.timestamp_millis() + 10_000,
            max_age_ms: 20_000,
            mirror: None
        }]
    ));
    guard.finish_validated();
    assert!(old(ns, at).is_none());
    let oversized = (0..MAX_MARKERS + 1)
        .map(|id| Qualification {
            pair_id: id as i32,
            assets: (1, 2),
            context: "reviewed".into(),
            expiry_ms: at.timestamp_millis() + 10_000,
            max_age_ms: 20_000,
            mirror: None,
        })
        .collect();
    assert!(!qualify("fee-phase-capacity", oversized));
    assert!(begin("fee-phase-capacity", 1, (1, 2), 101, at, (20_000, 20_000)).is_none());
    let mut invalid = (*complete(at)).clone();
    invalid.reserve_0 = 0;
    assert!(mirror_cost(&invalid).is_none());
    invalid.reserve_0 = 1;
    invalid.fee_bps = 10_001;
    assert!(mirror_cost(&invalid).is_none());
}

#[test]
#[serial_test::serial]
fn recovery1464_fee_negative_survives_expiry_empty_and_removed_qualification() {
    for (suffix, removal) in [("expired", 0), ("empty", 1), ("removed", 2)] {
        let ns = format!("fee-tombstone-{suffix}");
        let at = Utc::now();
        let mirror = complete(at - chrono::Duration::seconds(1));
        qualified(&ns, mirror.clone(), at.timestamp_millis() + 10_000);
        let guard = begin(&ns, 1, (1, 2), 101, at, (20_000, 20_000)).unwrap();
        guard.finish_negative();
        match removal {
            0 => {
                registry()
                    .lock()
                    .unwrap()
                    .namespaces
                    .get_mut(&ns)
                    .unwrap()
                    .entries
                    .get_mut(&1)
                    .unwrap()
                    .expiry_ms = Utc::now().timestamp_millis();
                let _ = revision(&ns);
            }
            1 => {
                assert!(qualify(&ns, Vec::new()));
            }
            _ => {
                assert!(qualify(
                    &ns,
                    vec![Qualification {
                        pair_id: 2,
                        assets: (3, 4),
                        context: "different".into(),
                        expiry_ms: at.timestamp_millis() + 10_000,
                        max_age_ms: 20_000,
                        mirror: None
                    }]
                ));
            }
        }
        assert!(!permits(&ns, 1, Some(100), mirror.snapshot_at));
        qualified(&ns, mirror.clone(), Utc::now().timestamp_millis() + 10_000);
        assert!(!permits(&ns, 1, Some(100), mirror.snapshot_at));
        assert!(registry().lock().unwrap().namespaces[&ns].entries[&1]
            .complete
            .is_none());
        let recovered = begin(&ns, 1, (1, 2), 102, Utc::now(), (20_000, 20_000)).unwrap();
        recovered.finish_validated_fee(30);
        assert!(permits(&ns, 1, Some(102), Utc::now()));
    }
}
#[test]
#[serial_test::serial]
fn recovery1464_fee_completed_change_and_delayed_publication_never_restore_old_economics() {
    let ns = "fee-authoritative-change";
    let at = Utc::now();
    let mirror = complete(at - chrono::Duration::seconds(1));
    qualified(ns, mirror.clone(), at.timestamp_millis() + 10_000);
    begin(ns, 1, (1, 2), 101, at, (20_000, 20_000))
        .unwrap()
        .finish_validated_fee(10_000);
    assert!(!permits(ns, 1, mirror.block_height, mirror.snapshot_at));
    qualified(ns, mirror.clone(), at.timestamp_millis() + 10_000);
    assert!(registry().lock().unwrap().namespaces[ns].entries[&1]
        .complete
        .is_none());
    let mut newer = (*mirror).clone();
    newer.block_height = Some(101);
    newer.snapshot_at = at;
    newer.fee_bps = 10_000;
    qualified(ns, Arc::new(newer), at.timestamp_millis() + 10_000);
    qualified(ns, mirror, at.timestamp_millis() + 10_000);
    let reg = registry().lock().unwrap();
    let entry = &reg.namespaces[ns].entries[&1];
    assert_eq!(entry.validated.unwrap().height, Some(101));
    assert_eq!(entry.complete.as_ref().unwrap().mirror.fee_bps, 10_000);
}

#[test]
#[serial_test::serial]
fn recovery1464_active_fee_negative_authority_survives_temporary_disqualification() {
    for (suffix, removal) in [("expired", 0), ("empty", 1), ("removed", 2)] {
        let ns = format!("fee-active-{suffix}");
        let at = Utc::now();
        let mirror = complete(at - chrono::Duration::seconds(1));
        qualified(&ns, mirror.clone(), at.timestamp_millis() + 10_000);
        let guard = begin(&ns, 1, (1, 2), 101, at, (20_000, 20_000)).unwrap();
        let callback = guard.safety_callback();
        match removal {
            0 => {
                registry()
                    .lock()
                    .unwrap()
                    .namespaces
                    .get_mut(&ns)
                    .unwrap()
                    .entries
                    .get_mut(&1)
                    .unwrap()
                    .expiry_ms = Utc::now().timestamp_millis();
                let _ = revision(&ns);
            }
            1 => {
                assert!(qualify(&ns, Vec::new()));
            }
            _ => {
                assert!(qualify(
                    &ns,
                    vec![Qualification {
                        pair_id: 2,
                        assets: (3, 4),
                        context: "other".into(),
                        expiry_ms: at.timestamp_millis() + 10_000,
                        max_age_ms: 20_000,
                        mirror: None
                    }]
                ));
            }
        }
        assert!(begin(&ns, 1, (1, 2), 102, Utc::now(), (20_000, 20_000)).is_none());
        qualified(&ns, mirror.clone(), Utc::now().timestamp_millis() + 10_000);
        callback();
        assert!(!permits(&ns, 1, Some(100), mirror.snapshot_at));
        guard.finish_negative();
    }
}
#[test]
#[serial_test::serial]
fn recovery1464_changed_fee_revokes_even_when_saved_book_allocation_is_unavailable() {
    let ns = "fee-no-saved-book";
    let at = Utc::now();
    let mirror = complete(at - chrono::Duration::seconds(1));
    let mut oversized = (*mirror).clone();
    oversized.bids = Vec::with_capacity(
        MAX_COMPLETE_BYTES
            / std::mem::size_of::<crate::db::queries::resting_orders::RestingOrderRow>()
            + 1,
    );
    assert!(mirror_cost(&oversized).unwrap().0 > MAX_COMPLETE_BYTES);
    qualified(ns, Arc::new(oversized), at.timestamp_millis() + 10_000);
    assert!(registry().lock().unwrap().namespaces[ns].entries[&1]
        .complete
        .is_none());
    begin(ns, 1, (1, 2), 101, at, (20_000, 20_000))
        .unwrap()
        .finish_validated_fee(100);
    assert!(!permits(ns, 1, Some(100), mirror.snapshot_at));
}

#[test]
#[serial_test::serial]
fn recovery1464_fee_phase_rejects_same_millisecond_different_raw_timestamp() {
    let ns = "fee-submillisecond-provenance";
    let at = DateTime::from_timestamp_micros(Utc::now().timestamp_micros().div_euclid(1000) * 1000)
        .unwrap();
    qualified(
        ns,
        complete(at - chrono::Duration::seconds(1)),
        at.timestamp_millis() + 10_000,
    );
    let owner = begin(ns, 1, (1, 2), 101, at, (20_000, 20_000)).unwrap();
    let different = at + chrono::Duration::microseconds(1);
    assert_eq!(at.timestamp_millis(), different.timestamp_millis());
    assert!(old(ns, at).is_some());
    assert!(old(ns, different).is_none());
    owner.finish_negative();
}

#[test]
#[serial_test::serial]
fn recovery1464_untracked_guard_latches_late_qualification_and_cancellation_without_retention() {
    let ns = "fee-untracked-late";
    let at = Utc::now();
    assert!(begin(ns, 1, (1, 2), 101, at, (20_000, 20_000)).is_none());
    let guard = UntrackedFeeGuard::new(ns, 1, (1, 2), 101, at);
    let callback = guard.safety_callback();
    qualified(
        ns,
        complete(at - chrono::Duration::seconds(1)),
        at.timestamp_millis() + 10_000,
    );
    callback();
    assert!(!permits(ns, 1, Some(100), at));
    guard.finish_negative();
    assert!(old(ns, at).is_none());

    let ns = "fee-untracked-cancel";
    let guard = UntrackedFeeGuard::new(ns, 1, (1, 2), 101, at);
    qualified(
        ns,
        complete(at - chrono::Duration::seconds(1)),
        at.timestamp_millis() + 10_000,
    );
    drop(guard);
    assert!(!permits(ns, 1, Some(100), at));
}

#[test]
#[serial_test::serial]
fn recovery1464_untracked_guard_never_erases_newer_validated_generation() {
    let ns = "fee-untracked-obsolete";
    let old_at = Utc::now() - chrono::Duration::seconds(1);
    let guard = UntrackedFeeGuard::new(ns, 1, (1, 2), 101, old_at);
    let mut newer = (*complete(Utc::now())).clone();
    newer.block_height = Some(102);
    qualified(
        ns,
        Arc::new(newer.clone()),
        Utc::now().timestamp_millis() + 10_000,
    );
    guard.finish_negative();
    assert!(permits(ns, 1, newer.block_height, newer.snapshot_at));
}
