//! #1376 bounded pool-only net receive sizing: Postgres discovery and authoritative forward LCD.
mod common;

use axum_test::TestServer;
use bigdecimal::{num_bigint::BigUint, ToPrimitive};
use common::lcd_mock;
use serde_json::{json, Value};
use serial_test::serial;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// Pool fixtures are deliberately independent of the solver inverse. Every forward request
/// asserts executable operations and fee subject rather than returning a constant success.
async fn forward_mock(
    pools: Vec<(String, String, u128, u128, u16)>,
    discounted: bool,
) -> MockServer {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"/cosmwasm/wasm/v1/contract/.*/smart/.*"))
        .respond_with(move |request: &Request| {
            let q = lcd_mock::smart_query_from_request(request);
            if q.get("is_paused").is_some() {
                return ResponseTemplate::new(200).set_body_json(json!({"data":{"paused":false}}));
            }
            if q.get("blacklist_check").is_some() {
                return ResponseTemplate::new(200).set_body_json(json!({"data":{"blocked":false}}));
            }
            if q.get("get_discount").is_some() {
                return ResponseTemplate::new(200)
                    .set_body_json(json!({"data":{"discount_bps":if discounted {5000} else {0}}}));
            }
            let sim = &q["simulate_swap_operations"];
            if sim.is_null() {
                return ResponseTemplate::new(500);
            }
            if discounted {
                assert_eq!(sim["trader"], "terra1000000000000000000000000000000000001");
                assert_eq!(sim["sender"], "terra1000000000000000000000000000000000002");
            }
            let mut offer = sim["offer_amount"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap();
            for op in sim["operations"].as_array().unwrap() {
                let op = &op["terra_swap"];
                assert!(op["hybrid"].is_null());
                let a = op["offer_asset_info"]["token"]["contract_addr"]
                    .as_str()
                    .unwrap();
                let b = op["ask_asset_info"]["token"]["contract_addr"]
                    .as_str()
                    .unwrap();
                let (_, _, r0, r1, fee) = pools
                    .iter()
                    .find(|(x, y, _, _, _)| (x == a && y == b) || (x == b && y == a))
                    .unwrap();
                let (rin, rout) = if pools.iter().any(|(x, y, _, _, _)| x == a && y == b) {
                    (*r0, *r1)
                } else {
                    (*r1, *r0)
                };
                let k = BigUint::from(rin) * BigUint::from(rout);
                let den = BigUint::from(rin.checked_add(offer).unwrap());
                let remain = ((k + &den - BigUint::from(1u32)) / den).to_u128().unwrap();
                let gross = rout - remain;
                let eff = if discounted { *fee / 2 } else { *fee };
                offer = gross
                    - (BigUint::from(gross) * BigUint::from(eff) / BigUint::from(10000u32))
                        .to_u128()
                        .unwrap();
            }
            ResponseTemplate::new(200).set_body_json(json!({"data":{"amount":offer.to_string()}}))
        })
        .mount(&mock)
        .await;
    mock
}

fn config(mock: &MockServer) -> cl8y_dex_indexer::config::Config {
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![lcd_mock::lcd_base_url(mock)];
    cfg.router_address = Some("terra1routertest".into());
    cfg.fee_discount_address = Some("terra1fees".into());
    cfg
}

async fn reserves(pool: &sqlx::PgPool, pair: &str, r0: u128, r1: u128, fee: u16) {
    sqlx::query("UPDATE pair_reserves SET reserve_0=$1::numeric,reserve_1=$2::numeric,fee_bps=$3,snapshot_at=NOW() WHERE pair_id=(SELECT id FROM pairs WHERE contract_address=$4)")
        .bind(r0.to_string()).bind(r1.to_string()).bind(i16::try_from(fee).unwrap()).bind(pair).execute(pool).await.unwrap();
}

#[serial]
#[tokio::test]
async fn route_exact_output_real_10_100_cold_repeat_and_exact_target_identity() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_with_mirror(&pool).await;
    let (r0, r1) = (297665983808u128, 2498115414904458263864u128);
    reserves(&pool, "terra1pairrouteabc", r0, r1, 180).await;
    sqlx::query("UPDATE assets SET decimals=18 WHERE contract_address=$1")
        .bind(&seed.token_b)
        .execute(&pool)
        .await
        .unwrap();
    let mock = forward_mock(
        vec![(seed.token_a.clone(), seed.token_b.clone(), r0, r1, 180)],
        false,
    )
    .await;
    let server = TestServer::new(
        common::build_test_app_with_price_and_config(pool, None, config(&mock)).await,
    );
    for (target, expected) in [
        ("10000000000000000000", "1218369992"),
        ("100000000000000000000", "12649685215"),
        ("10000000000000000000", "1218369992"),
    ] {
        let resp = server
            .get(&format!(
                "/api/v1/route/solve?token_in={}&token_out={}&amount_out={target}",
                seed.token_a, seed.token_b
            ))
            .await;
        resp.assert_status_ok();
        let body: Value = resp.json();
        assert_eq!(body["required_amount_in"], expected);
        assert_eq!(body["target_amount_out"], target);
        assert_eq!(body["sizing_mode"], "pool_only_exact_output");
        assert_eq!(body["fidelity_check"], "passed");
        assert!(body["spot_amount_out"].as_str().unwrap().parse::<u128>().unwrap() > 0);
        assert!(body["slippage_percent"].as_str().is_some());
        assert_eq!(body["snapshot_block_height"], 100);
        assert!(body["snapshot_age_ms"].as_u64().unwrap() < 30000);
        assert!(
            body["estimated_amount_out_net"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
                >= target.parse::<u128>().unwrap()
        );
        assert!(body["router_operations"][0]["terra_swap"]["hybrid"].is_null());
    }
    let queries = mock.received_requests().await.unwrap();
    assert_eq!(
        queries
            .iter()
            .filter(|q| lcd_mock::smart_query_from_request(q)
                .get("simulate_swap_operations")
                .is_some())
            .count(),
        3,
        "repeat targets still validate live; no executable cache"
    );
}

#[serial]
#[tokio::test]
async fn route_exact_output_competing_path_beats_direct_and_unusable_first_path() {
    for direct_reserve in [5000000000000u128, 0] {
        let pool = common::setup_pool().await;
        let seed = common::seed_route_solve_zero_reserve_poison(&pool).await;
        let deep = 10000000000000u128;
        reserves(&pool, "terra1pairroutempac", deep, direct_reserve, 30).await;
        reserves(&pool, "terra1pairroutempbc", deep, deep, 30).await;
        let mock = forward_mock(
            vec![
                (
                    seed.token_a.clone(),
                    seed.token_c.clone(),
                    deep,
                    direct_reserve,
                    30,
                ),
                (seed.token_a.clone(), seed.token_b.clone(), deep, deep, 30),
                (seed.token_b.clone(), seed.token_c.clone(), deep, deep, 30),
            ],
            false,
        )
        .await;
        let server = TestServer::new(
            common::build_test_app_with_price_and_config(pool, None, config(&mock)).await,
        );
        let resp = server
            .get(&format!(
                "/api/v1/route/solve?token_in={}&token_out={}&amount_out=1000000",
                seed.token_a, seed.token_c
            ))
            .await;
        resp.assert_status_ok();
        let body: Value = resp.json();
        assert_eq!(body["hops"].as_array().unwrap().len(), 2);
        assert_eq!(
            body["intermediate_tokens"],
            json!([seed.token_a, seed.token_b, seed.token_c])
        );
        assert!(
            body["required_amount_in"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
                < 1100000
        );
        assert!(body["optimality_scope"]
            .as_str()
            .unwrap()
            .contains("pool-only"));
    }
}

#[serial]
#[tokio::test]
async fn route_exact_output_fee_tier_and_sender_match_forward() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_with_mirror(&pool).await;
    let deep = 10000000000000u128;
    let mock = forward_mock(
        vec![(seed.token_a.clone(), seed.token_b.clone(), deep, deep, 30)],
        true,
    )
    .await;
    let server = TestServer::new(
        common::build_test_app_with_price_and_config(pool, None, config(&mock)).await,
    );
    let resp=server.get(&format!("/api/v1/route/solve?token_in={}&token_out={}&amount_out=1000000&trader=terra1000000000000000000000000000000000001&sender=terra1000000000000000000000000000000000002",seed.token_a,seed.token_b)).await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert!(
        body["required_amount_in"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            < 1003010
    );
}

#[serial]
#[tokio::test]
async fn route_exact_output_validation_fails_closed_and_rejects_modes() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_with_mirror(&pool).await;
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"/cosmwasm/wasm/v1/contract/.*/smart/.*"))
        .respond_with(|request: &Request| {
            let q = lcd_mock::smart_query_from_request(request);
            let data = if q.get("is_paused").is_some() {
                json!({"paused":false})
            } else if q.get("blacklist_check").is_some() {
                json!({"blocked":false})
            } else {
                json!({"amount":"1"})
            };
            ResponseTemplate::new(200).set_body_json(json!({"data":data}))
        })
        .mount(&mock)
        .await;
    let server = TestServer::new(
        common::build_test_app_with_price_and_config(pool, None, config(&mock)).await,
    );
    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_out=1000000",
        seed.token_a, seed.token_b
    );
    server.get(&url).await.assert_status_service_unavailable();
    server
        .get(&format!("{url}&amount_in=100"))
        .await
        .assert_status_bad_request();
    server
        .get(&format!("{url}&hybrid_optimize=true"))
        .await
        .assert_status_bad_request();
    server
        .post("/api/v1/route/solve")
        .json(&json!({"token_in":seed.token_a,"token_out":seed.token_b,"amount_out":"100"}))
        .await
        .assert_status_bad_request();
}

#[serial]
#[tokio::test]
async fn route_exact_output_stale_empty_and_infeasible_are_finite() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_with_mirror(&pool).await;
    sqlx::query("UPDATE pair_reserves SET snapshot_at=NOW()-INTERVAL '1 hour'")
        .execute(&pool)
        .await
        .unwrap();
    let mock = lcd_mock::start_smart_query_data_mock(json!({"amount":"1000000"})).await;
    let server = TestServer::new(
        common::build_test_app_with_price_and_config(pool, None, config(&mock)).await,
    );
    let resp = server
        .get(&format!(
            "/api/v1/route/solve?token_in={}&token_out={}&amount_out=1000000",
            seed.token_a, seed.token_b
        ))
        .await;
    resp.assert_status_unprocessable_entity();
    assert!(resp.text().contains("fresh reserves"));
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[serial]
#[tokio::test]
async fn route_exact_output_buy_tax_net_and_sell_extra_debit_are_not_double_counted() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_with_mirror(&pool).await;
    for (addr, buy, sell) in [(&seed.token_a, 0i16, 200i16), (&seed.token_b, 1000, 0)] {
        sqlx::query("INSERT INTO community_tokens (contract_address,code_id,buy_bps,sell_bps) VALUES ($1,11611,$2,$3)")
            .bind(addr).bind(buy).bind(sell).execute(&pool).await.unwrap();
    }
    let deep = 10000000000000u128;
    let mock = forward_mock(
        vec![(seed.token_a.clone(), seed.token_b.clone(), deep, deep, 30)],
        false,
    )
    .await;
    let server = TestServer::new(
        common::build_test_app_with_price_and_config(pool.clone(), None, config(&mock)).await,
    );
    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_out=1000000",
        seed.token_a, seed.token_b
    );
    let resp = server.get(&url).await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(body["buy_tax_bps"], 1000);
    assert_eq!(body["sell_tax_bps"], 200);
    assert!(
        body["estimated_amount_out_net"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            >= 1000000
    );
    assert!(
        body["required_amount_in"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            > 1100000
    );
    assert!(
        body["required_amount_in"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            < 1120000,
        "pool funding excludes extra debit; client checks debit separately"
    );
    sqlx::query("UPDATE community_tokens SET buy_bps=10000 WHERE contract_address=$1")
        .bind(&seed.token_b)
        .execute(&pool)
        .await
        .unwrap();
    server.get(&url).await.assert_status_unprocessable_entity();
}

#[serial]
#[tokio::test]
async fn route_exact_output_bad_intermediary_metadata_does_not_hide_direct_alternate() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_zero_reserve_poison(&pool).await;
    sqlx::query("UPDATE assets SET decimals=99 WHERE contract_address=$1")
        .bind(&seed.token_b)
        .execute(&pool)
        .await
        .unwrap();
    let deep = 10000000000000u128;
    let mock = forward_mock(
        vec![(seed.token_a.clone(), seed.token_c.clone(), deep, deep, 30)],
        false,
    )
    .await;
    let server = TestServer::new(
        common::build_test_app_with_price_and_config(pool, None, config(&mock)).await,
    );
    let resp = server
        .get(&format!(
            "/api/v1/route/solve?token_in={}&token_out={}&amount_out=1000000",
            seed.token_a, seed.token_c
        ))
        .await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(body["hops"].as_array().unwrap().len(), 1);
}

#[serial]
#[tokio::test]
async fn route_exact_output_paused_and_blacklisted_paths_cannot_quote() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_with_mirror(&pool).await;
    let mock = lcd_mock::start_smart_query_data_mock(
        json!({"paused":true,"blocked":true,"amount":"1000000"}),
    )
    .await;
    let server = TestServer::new(
        common::build_test_app_with_price_and_config(pool, None, config(&mock)).await,
    );
    let resp = server
        .get(&format!(
            "/api/v1/route/solve?token_in={}&token_out={}&amount_out=1000000",
            seed.token_a, seed.token_b
        ))
        .await;
    resp.assert_status_service_unavailable();
    let requests = mock.received_requests().await.unwrap();
    assert_eq!(
        requests
            .iter()
            .filter(|r| lcd_mock::smart_query_from_request(r)
                .get("simulate_swap_operations")
                .is_some())
            .count(),
        0
    );
}

#[serial]
#[tokio::test]
async fn route_exact_output_sender_only_exemption_matches_execution_context() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_with_mirror(&pool).await;
    sqlx::query("INSERT INTO community_tokens (contract_address,code_id,buy_bps,sell_bps,features) VALUES ($1,11611,1000,0,'{\"exemption_directory\":true}'::jsonb)")
        .bind(&seed.token_b).execute(&pool).await.unwrap();
    let deep = 10000000000000u128;
    let mock = forward_mock(
        vec![(seed.token_a.clone(), seed.token_b.clone(), deep, deep, 30)],
        false,
    )
    .await;
    Mock::given(method("GET"))
        .and(|r: &Request| {
            lcd_mock::smart_query_from_request(r)
                .get("is_protocol_exempt")
                .is_some()
        })
        .respond_with(|r: &Request| {
            let q = lcd_mock::smart_query_from_request(r);
            assert_eq!(
                q["is_protocol_exempt"]["address"],
                "terra1000000000000000000000000000000000002"
            );
            ResponseTemplate::new(200)
                .set_body_json(json!({"data":{"manager":true,"protocol":false}}))
        })
        .with_priority(1)
        .mount(&mock)
        .await;
    let server = TestServer::new(
        common::build_test_app_with_price_and_config(pool, None, config(&mock)).await,
    );
    let resp = server.get(&format!("/api/v1/route/solve?token_in={}&token_out={}&amount_out=1000000&sender=terra1000000000000000000000000000000000002",seed.token_a,seed.token_b)).await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(body["tax_kind"], "exempt");
    assert_eq!(body["buy_tax_bps"], 0);
    assert!(
        body["required_amount_in"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            < 1100000
    );
}

#[serial]
#[tokio::test]
async fn route_exact_output_fair_crossrate_rejects_suspiciously_good_route_for_safe_alternate() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_zero_reserve_poison(&pool).await;
    let deep = 1000000000000u128;
    reserves(&pool, "terra1pairroutempab", 1000, deep, 30).await;
    reserves(&pool, "terra1pairroutempac", deep, deep, 30).await;
    reserves(&pool, "terra1pairroutempbc", deep, deep, 30).await;
    sqlx::query("UPDATE assets SET symbol='cUSTC' WHERE contract_address=$1")
        .bind(&seed.token_c)
        .execute(&pool)
        .await
        .unwrap();
    let mock = forward_mock(
        vec![
            (seed.token_a.clone(), seed.token_b.clone(), 1000, deep, 30),
            (seed.token_a.clone(), seed.token_c.clone(), deep, deep, 30),
            (seed.token_b.clone(), seed.token_c.clone(), deep, deep, 30),
        ],
        false,
    )
    .await;
    let server = TestServer::new(
        common::build_test_app_with_price_and_config(pool, None, config(&mock)).await,
    );
    let resp = server
        .get(&format!(
            "/api/v1/route/solve?token_in={}&token_out={}&amount_out=1000000",
            seed.token_a, seed.token_b
        ))
        .await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(
        body["hops"].as_array().unwrap().len(),
        2,
        "one-raw-unit offer on thin direct pool looks spectacular; fair reference rejects it"
    );
    assert_eq!(
        body["intermediate_tokens"],
        json!([seed.token_a, seed.token_c, seed.token_b])
    );
    assert!(
        body["required_amount_in"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            > 1000000
    );
    assert!(
        body["spot_amount_out"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            > 0
    );
    assert_eq!(body["token_in_price_quote"], body["token_out_price_quote"]);
    assert!(
        body["slippage_percent"]
            .as_str()
            .unwrap()
            .parse::<f64>()
            .unwrap()
            < 5.0
    );
}

#[serial]
#[tokio::test]
async fn route_exact_output_large_impact_retains_shared_5_30_warning_fields() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_with_mirror(&pool).await;
    let deep = 10000000000000u128;
    let mock = forward_mock(
        vec![(seed.token_a.clone(), seed.token_b.clone(), deep, deep, 30)],
        false,
    )
    .await;
    let server = TestServer::new(
        common::build_test_app_with_price_and_config(pool, None, config(&mock)).await,
    );
    let resp = server
        .get(&format!(
            "/api/v1/route/solve?token_in={}&token_out={}&amount_out={}",
            seed.token_a,
            seed.token_b,
            deep / 2
        ))
        .await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    let impact = body["slippage_percent"]
        .as_str()
        .unwrap()
        .parse::<f64>()
        .unwrap();
    assert!(
        impact > 30.0 && impact < 99.0,
        "the caller keeps consent/Expert gates using these shared fields"
    );
    assert!(
        body["spot_amount_out"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            > deep / 2
    );
}

#[serial]
#[tokio::test]
async fn route_exact_output_present_standard_reference_unavailable_fails_closed() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_with_mirror(&pool).await;
    sqlx::query("UPDATE assets SET symbol='cUSTC' WHERE contract_address=$1")
        .bind(&seed.token_c)
        .execute(&pool)
        .await
        .unwrap();
    let mock = lcd_mock::start_smart_query_data_mock(json!({"amount":"1000000"})).await;
    let server = TestServer::new(
        common::build_test_app_with_price_and_config(pool, None, config(&mock)).await,
    );
    let resp = server
        .get(&format!(
            "/api/v1/route/solve?token_in={}&token_out={}&amount_out=1000000",
            seed.token_a, seed.token_b
        ))
        .await;
    resp.assert_status_service_unavailable();
    assert!(resp.text().contains("fair-route reference"));
    assert!(mock.received_requests().await.unwrap().is_empty());
}
