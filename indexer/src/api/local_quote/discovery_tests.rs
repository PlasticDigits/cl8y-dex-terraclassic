//! Retained issue #1489 counterexample: native exact input has a useful but
//! materially inferior two-hop route and an eligible three-hop corridor.
use super::*;
use crate::indexer::fee_phase::{self, Qualification};

fn fixture(namespace: &str) -> Snapshot {
    let mut s = tests::fixture();
    s.namespace = namespace.into();
    s.pairs.clear();
    s.mirrors.clear();
    s.addresses = [
        (1, "clunc"),
        (2, "ustr"),
        (3, "short"),
        (4, "custc"),
        (5, "ust1"),
        (6, "unrelated-a"),
        (7, "unrelated-b"),
    ]
    .into_iter()
    .map(|(id, address)| (id, address.into()))
    .collect();
    s.ids = s
        .addresses
        .iter()
        .map(|(id, address)| (address.clone(), *id))
        .collect();
    s.references = super::super::local_reference_quote::ReferencePrices::synthetic(
        s.addresses.keys().copied(),
    );
    for (id, price) in &mut s.references.prices {
        price.decimals = if *id == 2 { 18 } else { 6 };
    }
    let now = Utc::now();
    for (id, a, b, depth) in [
        (101, 1, 3, 5_000u128),
        (102, 3, 2, 5_000),
        (103, 1, 4, 1_000_000),
        (104, 4, 5, 1_000_000),
        (105, 5, 2, 1_000_000),
        (106, 6, 7, 1_000_000),
    ] {
        let address = format!("discovery1489-{id}");
        let unit = |id| if id == 2 { 10u128.pow(18) } else { 1_000_000 };
        s.pairs.push(pairs::PairRow {
            id,
            contract_address: address.clone(),
            asset_0_id: a,
            asset_1_id: b,
            lp_token: None,
            fee_bps: Some(30),
            hooks: vec![],
            created_at_block: Some(1),
            created_at: now,
            updated_at: now,
        });
        s.mirrors.insert(
            address,
            Arc::new(sim::HopMirror {
                pair_id: id,
                asset_0_addr: s.addresses[&a].clone(),
                asset_1_addr: s.addresses[&b].clone(),
                reserve_0: depth * unit(a),
                reserve_1: depth * unit(b),
                fee_bps: 30,
                block_height: Some(10),
                snapshot_at: if id == 101 || id == 102 {
                    now - chrono::Duration::milliseconds(500)
                } else {
                    now
                },
                freshness: sim::MirrorFreshness::Fresh,
                bids: vec![],
                asks: vec![],
            }),
        );
    }
    assert!(fee_phase::qualify(
        namespace,
        s.pairs
            .iter()
            .map(|p| Qualification {
                pair_id: p.id,
                assets: (p.asset_0_id, p.asset_1_id),
                context: format!("1489-{}", p.id),
                expiry_ms: s.config_expiry_ms,
                max_age_ms: 20_000,
                mirror: Some(s.mirrors[&p.contract_address].clone()),
                pool_mirror: None
            })
            .collect()
    ));
    s
}
fn native(namespace: &str) -> super::super::local_wrapper_quote::WrapperSnapshot {
    super::super::local_wrapper_quote::WrapperSnapshot {
        mapper: format!("terra1mapper{namespace}"),
        treasury: "terra1treasury1489".into(),
        observed_at_ms: now_ms(),
        mappings: HashMap::from([(
            "uluna".into(),
            super::super::local_wrapper_quote::NativeMapping {
                cw20: "clunc".into(),
                maximum: None,
                used: 0,
                tax_cap: None,
            },
        )]),
        fee_wrap_bps: 200,
        fee_unwrap_bps: 200,
        tax_atomics: None,
        router_unwrap_supported: Some(false),
        mapper_floor_version: Some(1),
        treasury_floor_version: Some(1),
    }
}
fn independent(mut amount: u128, s: &Snapshot, ids: &[i32]) -> u128 {
    for id in ids {
        let m = &s.mirrors[&format!("discovery1489-{id}")];
        let gross = wide_floor(m.reserve_1, amount, m.reserve_0 + amount).unwrap();
        amount = gross - gross * 30 / 10_000;
    }
    amount
}
fn request() -> route_solver::SolveRouteParams {
    let mut q = tests::request(10_000_000_000);
    q.token_in = "uluna".into();
    q.token_out = "ustr".into();
    q.pool_only = Some(true);
    q.protection_policy = Some("quote_relative_v1".into());
    q
}

#[tokio::test]
#[serial_test::serial]
async fn issue1489_native_three_hop_promotes_after_saturation_and_unchanged_producers() {
    let namespace = "discovery1489-native";
    let snapshot = fixture(namespace);
    let q = request();
    let credit = 9_800_000_000;
    let short = independent(credit, &snapshot, &[101, 102]);
    let best = independent(credit, &snapshot, &[103, 104, 105]);
    assert!(best > short);
    let slots = (0..MAX_JOBS)
        .map(|i| format!("1489-occupied-{i}"))
        .collect::<Vec<_>>();
    {
        let mut g = store().lock().unwrap();
        g.snapshots
            .insert(namespace.into(), Arc::new(snapshot.clone()));
        g.wrappers.insert(namespace.into(), native(namespace));
        g.jobs.extend(slots.iter().cloned());
    }
    let initial = solve_local(namespace, &q).await.unwrap().0;
    assert_eq!(initial["estimated_amount_out_net"], short.to_string());
    assert_eq!(initial["hops"].as_array().unwrap().len(), 2);
    assert_eq!(initial["search_complete"], false);
    let generation = initial["execution_metadata"]["generation"].clone();
    // The original faulty cache hit bypassed scheduling forever. Repeated hits
    // coalesce one FIFO entry while all four existing workers remain occupied.
    for _ in 0..8 {
        assert_eq!(
            solve_local(namespace, &q).await.unwrap().0["estimated_amount_out_net"],
            short.to_string()
        );
    }
    {
        let g = store().lock().unwrap();
        assert_eq!(
            g.deferred_jobs
                .iter()
                .filter(|w| w.key.starts_with(namespace))
                .count(),
            1
        );
    }
    let progress = progress_local(namespace, &q).unwrap();
    assert_eq!(progress.done, 0);
    assert_eq!(progress.best_amount_out, Some(short.to_string()));
    let unrelated = &snapshot.mirrors["discovery1489-106"];
    let before = fee_phase::revision(namespace);
    for _ in 0..8 {
        fee_phase::begin(
            namespace,
            106,
            (6, 7),
            10,
            unrelated.snapshot_at,
            (unrelated.reserve_0, unrelated.reserve_1),
        )
        .unwrap()
        .finish_validated_fee(30);
    }
    assert_eq!(
        fee_phase::revision(namespace),
        before,
        "unchanged unrelated validation is not economic invalidation"
    );
    // An independently scheduled identical configuration refresh advances only
    // ingestion ownership. Preserve original economics/time/expiry in flight.
    {
        let mut g = store().lock().unwrap();
        let mut refreshed = (*g.snapshots[namespace]).clone();
        refreshed.config_epoch += 1;
        g.snapshots.insert(namespace.into(), Arc::new(refreshed));
    }
    // Release one worker: automatic drain must promote without a new solve.
    drop(JobGuard(slots[0].clone()));
    let started = std::time::Instant::now();
    loop {
        let p = progress_local(namespace, &q).unwrap();
        if p.best_amount_out == Some(best.to_string()) {
            assert_eq!(p.done, 1);
            assert_eq!(p.search_truncated, Some(false));
            break;
        }
        assert!(
            started.elapsed() < Duration::from_secs(2),
            "native improvement deadline exceeded"
        );
        tokio::time::sleep(Duration::from_millis(2)).await;
    }
    let improved = solve_local(namespace, &q).await.unwrap().0;
    assert_eq!(improved["hops"].as_array().unwrap().len(), 3);
    assert_eq!(improved["execution_metadata"]["generation"], generation);
    for field in ["observed_at_ms", "expires_at_ms"] {
        assert_eq!(
            improved["execution_metadata"][field], initial["execution_metadata"][field],
            "promotion changed original {field}"
        );
        assert_eq!(
            improved["economic_attribution"][field],
            initial["execution_metadata"][field]
        );
    }
    assert_eq!(
        improved["execution_metadata"]["wrapper"]["route_input_amount"],
        credit.to_string()
    );
    assert!(
        improved["execution_metadata"]["minimum_receive"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            > 0
    );
    for slot in slots.into_iter().skip(1) {
        drop(JobGuard(slot));
    }
    // Relevant authoritative changed fee immediately revokes cached and
    // in-flight approved economics, even before the next snapshot publication.
    let relevant = &snapshot.mirrors["discovery1489-103"];
    fee_phase::begin(
        namespace,
        103,
        (1, 4),
        10,
        relevant.snapshot_at,
        (relevant.reserve_0, relevant.reserve_1),
    )
    .unwrap()
    .finish_validated_fee(31);
    assert_ne!(
        progress_local(namespace, &q)
            .unwrap()
            .quote_generation
            .unwrap(),
        generation.as_str().unwrap()
    );
    assert!(!store()
        .lock()
        .unwrap()
        .results
        .keys()
        .any(|key| key.starts_with(namespace)));
}

#[test]
fn issue1489_discovery_reports_dense_path_cap_without_cap_escalation() {
    let mut s = fixture("discovery1489-dense");
    let template = s.pairs[0].clone();
    s.pairs.clear();
    for index in 0..400 {
        let mut p = template.clone();
        p.id = index;
        p.contract_address = format!("parallel1489-{index:03}");
        p.asset_0_id = if index < 200 { 1 } else { 3 };
        p.asset_1_id = if index < 200 { 3 } else { 2 };
        s.pairs.push(p);
    }
    let discovery = skeletons::discovery(&s, 1, 2, 4);
    assert!(discovery.truncated);
    assert_eq!(discovery.paths.len(), 1024);
    let mut body = json!({});
    discovery.annotate(&mut body, true);
    assert_eq!(body["search_truncated"], true);
    assert_eq!(body["search_complete"], true);
    assert_eq!(body["search_max_pool_hops"], 4);
}

#[test]
fn issue1489_ineligible_native_higher_net_cannot_suppress_eligible_lower_alternative() {
    let mut s = fixture("discovery1489-native-cap");
    s.quote_relative_protection = true;
    let amount = 9_800_000_000;
    let short = independent(amount, &s, &[101, 102]);
    let mut wrapper = native("cap");
    wrapper.router_unwrap_supported = Some(true);
    wrapper.tax_atomics = Some(0);
    wrapper.mappings.insert(
        "uusd".into(),
        super::super::local_wrapper_quote::NativeMapping {
            cw20: "ustr".into(),
            maximum: Some(short + 1),
            used: 0,
            tax_cap: Some(0),
        },
    );
    let mut request = request();
    request.token_in = "clunc".into();
    request.token_out = "uusd".into();
    request.amount_in = Some(amount.to_string());
    let native = NativeContext {
        request,
        wrapper,
        wrap: false,
        unwrap: true,
    };
    let selected = select_detailed(
        &s,
        &paths(&s, 1, 2, 4),
        amount,
        SCALE / 20,
        false,
        8,
        None,
        Some(&native),
    );
    assert_eq!(
        selected.executable.unwrap().output,
        short,
        "known native output-cap rejection must apply before winner selection"
    );
}

#[test]
#[serial_test::serial]
fn issue1489_tighter_windows_invalidate_without_refresh_epoch_churn() {
    let namespace = "validity1489";
    let mut source = tests::fixture();
    source.namespace = namespace.into();
    let mut g = Store::default();
    g.snapshots
        .insert(namespace.into(), Arc::new(source.clone()));
    let reviewed = protocol_guards::derive(&g, namespace, &source);
    let mut current = source.clone();
    current.config_epoch += 1;
    current.config_expiry_ms += 1000;
    assert!(!validity_tightened(&current, &source));
    g.snapshots
        .insert(namespace.into(), Arc::new(current.clone()));
    assert!(reviewed_snapshot_current(&g, namespace, &reviewed, None));
    current.max_age_ms = 10_000;
    assert!(validity_tightened(&current, &source));
    g.snapshots.insert(namespace.into(), Arc::new(current));
    assert!(!reviewed_snapshot_current(&g, namespace, &reviewed, None));
    let mut current = source.clone();
    current.config_expiries.insert(
        source.pairs[0].contract_address.clone(),
        source.config_expiry_ms - 1000,
    );
    assert!(validity_tightened(&current, &source));
    g.snapshots
        .insert(namespace.into(), Arc::new(current.clone()));
    assert!(!reviewed_snapshot_current(&g, namespace, &reviewed, None));
    // Exercise actual publication: identical economics with a shortened pair
    // window create a different admission key, and an unchanged refresh cannot
    // restore the earlier key or approve its in-flight work.
    let publish_fixture = |s: Snapshot| {
        publish(
            s.namespace,
            s.pairs,
            s.addresses,
            s.mirrors,
            s.pool_mirrors,
            s.max_age_ms,
            s.hooks,
            s.config_expiry_ms,
            s.config_expiries,
            s.pair_contexts,
            s.references,
            s.actors,
            s.discounted_pairs,
            s.policy_identity,
            s.taxes,
            None,
        );
    };
    publish_fixture(source.clone());
    let (first, old_reviewed) = {
        let g = store().lock().unwrap();
        let first = g.snapshots[namespace].clone();
        let reviewed = protocol_guards::derive(&g, namespace, &first);
        (first, reviewed)
    };
    let mut tightened = (*first).clone();
    tightened.config_expiries.insert(
        first.pairs[0].contract_address.clone(),
        first.config_expiry_ms - 1000,
    );
    publish_fixture(tightened);
    let (second, next_generation) = {
        let g = store().lock().unwrap();
        let second = g.snapshots[namespace].clone();
        assert!(!reviewed_snapshot_current(
            &g,
            namespace,
            &old_reviewed,
            None
        ));
        let generation = protocol_guards::derive(&g, namespace, &second).generation;
        assert_ne!(generation, old_reviewed.generation);
        (second, generation)
    };
    publish_fixture((*second).clone());
    let g = store().lock().unwrap();
    assert_eq!(
        protocol_guards::derive(&g, namespace, &g.snapshots[namespace]).generation,
        next_generation
    );
    assert!(!reviewed_snapshot_current(
        &g,
        namespace,
        &old_reviewed,
        None
    ));
}

#[tokio::test]
#[serial_test::serial]
async fn issue1489_native_without_mode_flags_remains_pool_only_when_wrapped_books_win() {
    let namespace = "native-book1489";
    let mut s = tests::fixture();
    s.namespace = namespace.into();
    s.quote_relative_protection = true;
    let mirror = Arc::make_mut(s.mirrors.get_mut("pair7a").unwrap());
    mirror
        .bids
        .push(crate::db::queries::resting_orders::RestingOrderRow {
            pair_id: mirror.pair_id,
            order_id: 1,
            side: "bid".into(),
            price: "0.000000000002".parse().unwrap(),
            remaining: 1_000_000u128.into(),
            owner: None,
            expires_at: None,
        });
    let mut wrapper = native(namespace);
    wrapper.mappings.get_mut("uluna").unwrap().cw20 = "in18".into();
    {
        let mut g = store().lock().unwrap();
        g.snapshots.insert(namespace.into(), Arc::new(s.clone()));
        g.wrappers.insert(namespace.into(), wrapper);
    }
    let amount = 10u128.pow(16);
    let credit = amount - amount * 200 / 10_000;
    let pool = select(&s, &paths(&s, 1, 2, 4), credit, SCALE / 20, false, 8).unwrap();
    let hybrid = select(&s, &paths(&s, 1, 2, 4), credit, SCALE / 20, true, 8).unwrap();
    assert!(
        hybrid.output > pool.output,
        "book discriminator must actually improve wrapped economics"
    );
    let mut q = tests::request(amount);
    q.token_in = "uluna".into();
    q.protection_policy = Some("quote_relative_v1".into());
    let quote = solve_local(namespace, &q).await.unwrap().0;
    assert_eq!(quote["estimated_amount_out_net"], pool.output.to_string());
    assert!(quote["router_operations"]
        .as_array()
        .unwrap()
        .iter()
        .all(|operation| operation["terra_swap"]["hybrid"].is_null()));
    let plan = [
        Some(route_solver::HybridHopJson {
            pool_input: (credit / 2).to_string(),
            book_input: (credit - credit / 2).to_string(),
            max_maker_fills: 8,
            book_start_hint: None,
        }),
        None,
    ];
    let error = solve_local_with_plan(namespace, &q, Some(&plan))
        .await
        .unwrap_err();
    assert!(error.1.contains("unsupported_semantics"));
    q.token_in = "in18".into();
    q.amount_in = Some(credit.to_string());
    let wrapped = solve_local(namespace, &q).await.unwrap().0;
    assert_eq!(
        wrapped["estimated_amount_out_net"],
        hybrid.output.to_string()
    );
    assert!(!wrapped["router_operations"][0]["terra_swap"]["hybrid"].is_null());
}

#[test]
#[serial_test::serial]
fn issue1489_repeated_preserved_views_keep_identical_effective_generation() {
    let namespace = "preserved-generation1489";
    let mut source = fixture(namespace);
    source.pair_contexts = source
        .pairs
        .iter()
        .map(|p| (p.contract_address.clone(), format!("1489-{}", p.id)))
        .collect();
    source.config_expiries = source
        .pairs
        .iter()
        .map(|p| (p.contract_address.clone(), source.config_expiry_ms))
        .collect();
    let publish_fixture = |s: Snapshot| {
        publish(
            namespace.into(),
            s.pairs,
            s.addresses,
            s.mirrors,
            s.pool_mirrors,
            s.max_age_ms,
            s.hooks,
            s.config_expiry_ms,
            s.config_expiries,
            s.pair_contexts,
            s.references,
            s.actors,
            s.discounted_pairs,
            s.policy_identity,
            s.taxes,
            None,
        );
    };
    publish_fixture(source.clone());
    let original = store().lock().unwrap().snapshots[namespace].clone();
    let mut delayed = source.clone();
    delayed.config_expiry_ms += 10_000;
    for expiry in delayed.config_expiries.values_mut() {
        *expiry += 10_000;
    }

    for mirror in delayed.mirrors.values_mut() {
        let mirror = Arc::make_mut(mirror);
        mirror.snapshot_at -= chrono::Duration::milliseconds(1000);
        mirror.block_height = Some(9);
        mirror.reserve_0 /= 2;
    }
    publish_fixture(delayed.clone());
    let first = store().lock().unwrap().snapshots[namespace].clone();
    assert_eq!(
        first.mirrors["discovery1489-103"].reserve_0,
        original.mirrors["discovery1489-103"].reserve_0
    );
    assert_eq!(
        first.config_expiries, original.config_expiries,
        "preserving views also preserves their original qualification deadlines"
    );
    assert_eq!(
        first.generation, original.generation,
        "preserved effective economics must not acquire a new history-dependent generation"
    );
    publish_fixture(delayed);
    let repeated = store().lock().unwrap().snapshots[namespace].clone();
    assert_eq!(
        repeated.generation, original.generation,
        "identical delayed input cannot recursively churn generation"
    );
    store().lock().unwrap().snapshots.remove(namespace);
    fee_phase::retire_completed_test_namespace(namespace);
}
