//! Owned candidate/HTTP controls for bounded missing-prerequisite diagnostics.
use super::tests::{fixture, request};
use super::*;

#[tokio::test]
#[tracing_test::traced_test]
async fn issue1505_http_missing_candidate_is_bounded_private_and_final503_only() {
    use axum::{
        body::{to_bytes, Body},
        http::Request,
        response::IntoResponse,
        routing::get,
        Router,
    };
    use tower::ServiceExt;
    let app = Router::new()
        .route(
            "/api/v1/route/solve",
            get(|| async {
                let mut snapshot = fixture();
                // A real candidate reaches fair-reference evaluation after
                // successful mirror/economic checks, then loses one mark.
                snapshot.references.prices.remove(&1);
                let path = vec![("pair7a".into(), 1, 10), ("pair7b".into(), 10, 2)];
                let paths = vec![path; 5];
                let input = 10u128.pow(18);
                for _ in 0..64 {
                    let selected =
                        select_detailed(&snapshot, &paths, input, SCALE / 20, false, 8, None, None);
                    assert!(selected.executable.is_none());
                    assert_eq!(selected.rejected, Some(diagnostics::Rejection::Missing));
                }
                public_error(diagnostics::amount_error(
                    &snapshot.namespace,
                    &snapshot,
                    &paths,
                    1,
                    2,
                    None,
                    &request(input),
                    input,
                    SCALE / 20,
                    false,
                    8,
                    None,
                ))
            }),
        )
        .route(
            "/api/v1/route/solve/best",
            get(|| async {
                let mut snapshot = fixture();
                let path = vec![("pair7a".into(), 1, 10), ("pair7b".into(), 10, 2)];
                snapshot.references.prices.remove(&1);
                assert_eq!(
                    evaluate_with_plan(&snapshot, &path, 10u128.pow(18), SCALE / 20, 0, 8, None,)
                        .err(),
                    Some(diagnostics::Rejection::Missing)
                );
                snapshot.references =
                    super::super::local_reference_quote::ReferencePrices::synthetic(
                        snapshot.addresses.keys().copied(),
                    );
                assert!(evaluate_with_plan(
                    &snapshot,
                    &path,
                    10u128.pow(18),
                    SCALE / 20,
                    0,
                    8,
                    None,
                )
                .is_ok());
                Json(json!({"useful":true})).into_response()
            }),
        )
        .route(
            "/api/v1/route/solve/progress",
            get(|| async {
                let mut snapshot = fixture();
                snapshot.references.prices.remove(&1);
                let path = vec![("pair7a".into(), 1, 10), ("pair7b".into(), 10, 2)];
                assert_eq!(
                    evaluate_with_plan(&snapshot, &path, 10u128.pow(18), SCALE / 20, 0, 8, None,)
                        .err(),
                    Some(diagnostics::Rejection::Missing)
                );
                public_error(diagnostics::Rejection::Protection.error())
            }),
        )
        .layer(axum::middleware::from_fn(
            super::super::quote_trace::correlate,
        ));
    let mut request_ids = Vec::new();
    for (path, status) in [
        ("/api/v1/route/solve", StatusCode::SERVICE_UNAVAILABLE),
        ("/api/v1/route/solve/best", StatusCode::OK),
        (
            "/api/v1/route/solve/progress",
            StatusCode::UNPROCESSABLE_ENTITY,
        ),
    ] {
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "{path}?trader=private-missing-actor&amount_in=private-amount"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), status);
        request_ids.push(
            response.headers()["x-quote-request-id"]
                .to_str()
                .unwrap()
                .to_owned(),
        );
        let body: Value =
            serde_json::from_slice(&to_bytes(response.into_body(), 2048).await.unwrap()).unwrap();
        if status == StatusCode::SERVICE_UNAVAILABLE {
            assert_eq!(body["error"], "missing_observations");
            assert!(body.get("stage").is_none());
        }
    }
    assert_eq!(request_ids.iter().collect::<HashSet<_>>().len(), 3);
    logs_assert(|lines: &[&str]| {
        let missing = lines
            .iter()
            .filter(|line| line.contains("Local quote prerequisite missing"))
            .collect::<Vec<_>>();
        if missing.len() != 1 {
            return Err(format!(
                "expected one deduplicated final503 missing witness, got {}",
                missing.len()
            ));
        }
        let line = missing[0];
        for expected in [
            "component=\"reference\"",
            "stage=\"reference_price\"",
            "age_known=false",
            "expiry_known=false",
            "age_ms=0",
            "expires_in_ms=0",
            &request_ids[0],
        ] {
            if !line.contains(expected) {
                return Err(format!("missing fixed witness field {expected}"));
            }
        }
        if lines
            .iter()
            .any(|line| line.contains("Local quote prerequisite stale"))
        {
            return Err("missing observations must not be mislabeled stale".into());
        }
        Ok(())
    });
    for private in [
        "private-missing-actor",
        "private-amount",
        "pair7a",
        "pair7b",
    ] {
        assert!(!logs_contain(private));
    }
}

#[tokio::test]
#[tracing_test::traced_test]
async fn issue1505_missing_stage_sources_preserve_existing_rejections() {
    use super::super::local_wrapper_quote::{NativeMapping, WrapperSnapshot};
    let input = 10u128.pow(18);
    let path = vec![("pair7a".into(), 1, 10), ("pair7b".into(), 10, 2)];
    // Missing identity is still rejected by the existing protocol guard before
    // these candidate lookups; tracing must not relax that classification.
    let mut missing_identity = fixture();
    missing_identity.addresses.remove(&1);
    assert_eq!(
        evaluate_with_plan(&missing_identity, &path, input, SCALE / 20, 0, 8, None).err(),
        Some(diagnostics::Rejection::TradingBlocked)
    );
    for stage in [
        "mirror_admission",
        "tax_pair",
        "tax_actor",
        "tax_balance",
        "simulation_mirror",
        "simulation_fee",
        "native_mapping",
        "native_router",
        "native_tax_rate",
        "native_tax_cap",
    ] {
        super::super::prerequisite_trace::scope(async {
            let mut snapshot = fixture();
            let mut tax = tax::TokenObservation {
                policy_identity: "private-policy".into(),
                code_id: 1,
                buy_bps: 100,
                sell_bps: 100,
                observed_at_ms: now_ms(),
                max_wallet: Some(100),
                pairs: HashSet::from(["private-pair".into()]),
                actors: Default::default(),
            };
            let rejected = match stage {
                "mirror_admission" => {
                    snapshot.mirrors.remove("pair7a");
                    evaluate_with_plan(&snapshot, &path, input, SCALE / 20, 0, 8, None)
                        .err()
                        .unwrap()
                }
                "tax_pair" => diagnostics::tax_rejection(&tax, None, "private-other-pair"),
                "tax_actor" => {
                    assert_eq!(
                        diagnostics::tax_rejection(&tax, None, "private-pair"),
                        diagnostics::Rejection::Missing
                    );
                    diagnostics::tax_rejection(&tax, Some("private-actor"), "private-pair")
                }
                "tax_balance" => {
                    tax.actors.insert(
                        "private-actor".into(),
                        tax::ActorObservation {
                            observed_at_ms: now_ms(),
                            protocol: false,
                            manager: false,
                            balance: None,
                        },
                    );
                    diagnostics::tax_rejection(&tax, Some("private-actor"), "private-pair")
                }
                "simulation_mirror" => {
                    diagnostics::simulation_rejection(sim::DbSimError::MissingMirror)
                }
                "simulation_fee" => {
                    assert_eq!(
                        diagnostics::simulation_rejection(
                            sim::DbSimError::IncompleteFeeGeneration {
                                height: Some(123),
                                observed_at: Utc::now(),
                                reserve_0: 100,
                                reserve_1: 200,
                            }
                        ),
                        diagnostics::Rejection::Missing
                    );
                    diagnostics::simulation_rejection(sim::DbSimError::FeeGenerationRejected)
                }
                _ => {
                    let candidate = score(&snapshot, &path, input, SCALE / 20).unwrap();
                    let mut original = request(input);
                    original.token_out = "uusd".into();
                    let mut native = NativeContext {
                        request: original,
                        wrap: false,
                        unwrap: true,
                        wrapper: WrapperSnapshot {
                            mapper: "private-mapper".into(),
                            treasury: "private-treasury".into(),
                            observed_at_ms: now_ms(),
                            mappings: HashMap::from([(
                                "uusd".into(),
                                NativeMapping {
                                    cw20: "out18".into(),
                                    maximum: None,
                                    used: 0,
                                    tax_cap: Some(0),
                                },
                            )]),
                            fee_wrap_bps: 0,
                            fee_unwrap_bps: 0,
                            tax_atomics: Some(0),
                            router_unwrap_supported: Some(true),
                            mapper_floor_version: Some(1),
                            treasury_floor_version: Some(1),
                        },
                    };
                    assert_eq!(native_candidate_rejection(&native, &candidate, input), None);
                    match stage {
                        "native_mapping" => native.wrapper.mappings.clear(),
                        "native_router" => native.wrapper.router_unwrap_supported = None,
                        "native_tax_rate" => native.wrapper.tax_atomics = None,
                        "native_tax_cap" => {
                            native.wrapper.mappings.get_mut("uusd").unwrap().tax_cap = None
                        }
                        _ => unreachable!(),
                    }
                    native_candidate_rejection(&native, &candidate, input).unwrap()
                }
            };
            assert_eq!(rejected, diagnostics::Rejection::Missing);
            assert_eq!(
                public_error(rejected.error()).0,
                StatusCode::SERVICE_UNAVAILABLE
            );
        })
        .await;
        assert!(logs_contain(&format!("stage=\"{stage}\"")));
    }
    for private in [
        "private-policy",
        "private-pair",
        "private-other-pair",
        "private-actor",
        "private-mapper",
        "private-treasury",
        "pair7a",
    ] {
        assert!(!logs_contain(private));
    }
}
