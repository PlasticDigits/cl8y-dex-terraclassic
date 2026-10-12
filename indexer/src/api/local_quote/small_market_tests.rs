//! #1479 economic-floor, public rejection and original readiness regressions.
use super::tests::{fixture, request};
use super::*;
use bigdecimal::BigDecimal;

#[test]
fn issue1479_structural_reference_exclusion_is_precise_in_both_directions_and_never_signs() {
    let mut s = fixture();
    let now = Utc::now();
    s.references.prices.remove(&3);
    s.references.policy_exclusions.insert(3, now);
    for path in [vec![("pair0a".into(), 1, 3)], vec![("pair0a".into(), 3, 1)]] {
        let rejection = diagnostics::reference_rejection(&s, &path, 1_000);
        assert_eq!(rejection, diagnostics::Rejection::ReferencePolicyExcluded);
        let (status, Json(body)) = public_error(rejection.error());
        assert_eq!(status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(body["error"], "reference_policy_excluded");
        for field in [
            "indicative_amount_out",
            "router_operations",
            "execution_metadata",
            "minimum_receive",
        ] {
            assert!(body[field].is_null());
        }
    }
    s.references
        .policy_exclusions
        .insert(3, now - chrono::Duration::seconds(21));
    assert_eq!(
        diagnostics::reference_rejection(&s, &[("pair0a".into(), 1, 3)], 1_000),
        diagnostics::Rejection::Missing
    );
}

#[test]
fn issue1479_small_market_ten_dollar_policy_quotes_exact_bidirectional_net_and_ui_floors() {
    use super::super::local_reference_quote::{ReferencePrice, ReferencePrices};
    use crate::indexer::{
        economic_usd::resolve_economic_marks,
        hub_usd::{AssetRef, HubMark, HubTicker, HubUsdConfig, HubUsdSnapshot, ReservePair},
    };
    let now = Utc::now();
    let cfg = HubUsdConfig {
        custc_address: "terra1custc".into(),
        clunc_address: "terra1clunc".into(),
        ust1_address: "terra1ust1".into(),
        ustr_address: "terra1ustr".into(),
        cl8y_address: "terra1cl8y".into(),
        usdt_address: "terra1usdt".into(),
        tvl_floor: 100.into(),
        economic_tvl_floor: 10.into(),
        max_staleness: std::time::Duration::from_secs(20),
    };
    let token = AssetRef {
        id: 1,
        symbol: "SMALL".into(),
        denom: None,
        contract_address: Some("terra1small".into()),
        is_cw20: true,
        decimals: 6,
    };
    let anchor = AssetRef {
        id: 3,
        symbol: "cLUNC".into(),
        denom: None,
        contract_address: Some(cfg.clunc_address.clone()),
        is_cw20: true,
        decimals: 6,
    };
    // KENA-like raw reserves, no identity whitelist. At $0.00005 per LUNC,
    // total TVL=$25.2903132784; external anchor alone supplies $12.6451566392.
    let reserves = ReservePair {
        pair_id: 0,
        pair_address: "pair0a".into(),
        asset_0: token,
        asset_1: anchor,
        reserve_0: 110_044_622_670_234u128.into(),
        reserve_1: 252_903_132_784u128.into(),
        snapshot_at: now,
    };
    let anchor_usd: BigDecimal = "0.00005".parse().unwrap();
    let hubs = HubUsdSnapshot {
        lunc: Some(HubMark {
            ticker: HubTicker::Lunc,
            asset_id: Some(3),
            price_usd: anchor_usd.clone(),
            source_pair_id: None,
            source_pair_address: None,
            tvl_usd: None,
            priced_leg_depth_usd: None,
            as_of: Some(now),
        }),
        ..Default::default()
    };
    let marks = resolve_economic_marks(now, &cfg, &[reserves.clone()], &hubs);
    assert_eq!(marks.len(), 1);
    assert_eq!(
        marks[0].tvl_usd.round(18),
        "25.2903132784".parse::<BigDecimal>().unwrap()
    );
    let mut s = fixture();
    s.quote_relative_protection = true;
    s.addresses.insert(1, "terra1small".into());
    s.addresses.insert(3, cfg.clunc_address.clone());
    let mirror = Arc::make_mut(s.mirrors.get_mut("pair0a").unwrap());
    mirror.asset_0_addr = s.addresses[&1].clone();
    mirror.asset_1_addr = s.addresses[&3].clone();
    mirror.reserve_0 = 110_044_622_670_234;
    mirror.reserve_1 = 252_903_132_784;
    mirror.fee_bps = 180;
    s.references = ReferencePrices {
        prices: HashMap::from([
            (
                1,
                ReferencePrice {
                    price: marks[0].price_usd.clone(),
                    decimals: 6,
                    as_of: marks[0].as_of,
                },
            ),
            (
                3,
                ReferencePrice {
                    price: anchor_usd,
                    decimals: 6,
                    as_of: now,
                },
            ),
        ]),
        policy_identity: ReferencePrices::policy_identity(&cfg),
        ..Default::default()
    };
    for (offer, ask, rin, rout) in [
        (1, 3, 110_044_622_670_234u128, 252_903_132_784u128),
        (3, 1, 252_903_132_784, 110_044_622_670_234),
    ] {
        let path = vec![("pair0a".into(), offer, ask)];
        for input in [
            1u128,
            1_000,
            10_000,
            100_000,
            1_000_000,
            10_000_000,
            100_000_000,
            1_000_000_000,
            10_000_000_000,
        ] {
            let gross = rout * input / (rin + input);
            let expected = gross - gross * 180 / 10_000;
            for tolerance in [SCALE / 100, SCALE / 20, SCALE * 30 / 100] {
                let result = evaluate_with_plan(&s, &path, input, tolerance, 0, 8, None);
                if expected == 0 {
                    assert!(matches!(result, Err(diagnostics::Rejection::Dust)));
                    continue;
                }
                let candidate = result.unwrap();
                assert_eq!(candidate.output, expected);
                assert!(!candidate.protection_blocked);
                let mut q = request(input);
                q.token_in = s.addresses[&offer].clone();
                q.token_out = s.addresses[&ask].clone();
                let body = response(&s, &q, &candidate, input, tolerance, 1);
                assert_eq!(body["estimated_amount_out_net"], expected.to_string());
                let floor = expected - expected * tolerance / SCALE;
                assert!(floor > 0);
                assert_eq!(
                    body["execution_metadata"]["minimum_receive"],
                    floor.to_string()
                );
                assert_eq!(
                    body["router_operations"][0]["terra_swap"]["min_return"],
                    floor.to_string()
                );
            }
        }
    }
    let mut old = cfg.clone();
    old.economic_tvl_floor = 100.into();
    assert!(resolve_economic_marks(now, &old, &[reserves], &hubs).is_empty());
}

#[test]
fn issue1479_idle_search_is_not_quote_readiness_and_expired_cache_is_not_ready() {
    let namespace = "issue1479-progress-distinct-readiness";
    let s = fixture();
    initialize_for_test(
        namespace.into(),
        s.pairs,
        s.addresses,
        s.mirrors
            .into_iter()
            .map(|(address, mirror)| (address, (*mirror).clone()))
            .collect(),
        s.max_age_ms,
    );
    let q = request(10u128.pow(18));
    let idle = progress_local(namespace, &q).unwrap();
    assert_eq!(idle.stage, "idle");
    assert_eq!(idle.done, 0);
    assert_eq!(idle.quote_eligibility.as_deref(), Some("unverified"));
    let mut unknown = q.clone();
    unknown.token_out = "unindexed-identity".into();
    let missing_identity = progress_local(namespace, &unknown).unwrap();
    assert_eq!(
        missing_identity.quote_eligibility.as_deref(),
        Some("waiting_for_observations")
    );
    assert_eq!(
        missing_identity.quote_blocker.as_deref(),
        Some("missing_observations")
    );
    {
        let mut g = store().lock().unwrap();
        let mut s = (**g.snapshots.get(namespace).unwrap()).clone();
        s.references.prices.remove(&2);
        s.references.policy_exclusions.insert(2, Utc::now());
        g.snapshots.insert(namespace.into(), Arc::new(s));
    }
    let excluded = progress_local(namespace, &q).unwrap();
    assert_eq!(excluded.done, 0);
    assert_eq!(
        excluded.quote_eligibility.as_deref(),
        Some("policy_excluded")
    );
    assert_eq!(
        excluded.quote_blocker.as_deref(),
        Some("reference_policy_excluded")
    );
    {
        let mut g = store().lock().unwrap();
        let snapshot = g.snapshots[namespace].clone();
        let ck = key(
            namespace,
            &snapshot,
            &q,
            spread_atomics(q.max_spread.as_deref()).unwrap(),
        );
        g.results.insert(
            ck,
            json!({"execution_metadata":{"expires_at_ms":now_ms()-1}}),
        );
    }
    let expired = progress_local(namespace, &q).unwrap();
    assert!(!expired.cache_hit);
    assert_eq!(expired.done, 0);
}
