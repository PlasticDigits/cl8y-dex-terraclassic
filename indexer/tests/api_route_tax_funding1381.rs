//! Synthetic LCD capability fixtures + actual PostgreSQL route API. These prove discovery,
//! eligibility and quote semantics; contract multitests prove execution and tax conservation.
mod common;

use axum_test::TestServer;
use common::lcd_mock::smart_query_from_request;
use serde_json::{Value, json};
use serial_test::serial;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

async fn mock_tax_lcd(taxed: &str, version: Option<u16>, fail: bool) -> MockServer {
    let mock = MockServer::start().await;
    let tax = taxed.to_owned();
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+$"))
        .respond_with(move |req: &Request| {
            let addr = req.url.path().rsplit('/').next().unwrap();
            ResponseTemplate::new(200).set_body_json(json!({"contract_info":{
                "code_id": if addr == tax {"11667"} else {"10184"}, "admin":"terra1governance"
            }}))
        })
        .mount(&mock)
        .await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/.+$"))
        .respond_with(move |req: &Request| {
            let q = smart_query_from_request(req);
            let data = if q.get("route_tax_capability").is_some() {
                if fail { return ResponseTemplate::new(503); }
                if version.is_none() { return ResponseTemplate::new(500).set_body_string("query wasm contract failed: unknown variant"); }
                json!({"funding_version":version,"router":"terra1router1381","buy_bps":450,"sell_bps":100})
            } else if q.get("get_config").is_some() {
                json!({"buy_bps":450,"sell_bps":100})
            } else if q.get("config").is_some() {
                json!({"funding_version":1})
            } else if q.get("is_protocol_exempt").is_some() {
                json!({"manager":false,"protocol":false})
            } else if let Some(sim) = q.get("simulate_swap_operations") {
                // Explicit already-net response, distinct from the gross final buy amount.
                let gross: u128 = sim["offer_amount"].as_str().unwrap().parse().unwrap();
                json!({"amount":(gross*955/1000).to_string()})
            } else if let Some(sim) = q.get("hybrid_simulation") {
                // Pair LCD fixture is economic-input output, before token buy tax.
                let gross: u128 = sim["offer_asset"]["amount"].as_str().unwrap().parse().unwrap();
                json!({"return_amount":gross.to_string(),"spread_amount":"0","commission_amount":"0"})
            } else { json!({}) };
            ResponseTemplate::new(200).set_body_json(json!({"data":data}))
        }).mount(&mock).await;
    mock
}

fn config(mock: &MockServer, trusted: bool) -> cl8y_dex_indexer::config::Config {
    let mut cfg = common::test_config();
    cfg.router_address = Some("terra1router1381".into());
    cfg.lcd_urls = vec![mock.uri()];
    cfg.trading_tax_free_code_ids.insert(10184);
    if trusted {
        cfg.trading_route_tax_code_ids.insert(11667);
    }
    // Deliberately different narrow Create catalog pin; tax discovery must be independent.
    cfg.community_tax_code_id = Some(11630);
    cfg
}

#[serial]
#[tokio::test]
async fn upgraded_taxed_intermediate_absent_catalog_get_post_are_net_and_eligible() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_2hop(&pool).await;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM community_tokens")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    let mock = mock_tax_lcd(&seed.token_b, Some(1), false).await;
    let app = common::build_test_app_with_price_and_config(pool, None, config(&mock, true)).await;
    let server = TestServer::new(app);
    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1010000&pool_only=true",
        seed.token_a, seed.token_c
    );
    let response = server.get(&url).await;
    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(body["hops"].as_array().unwrap().len(), 2);
    assert_eq!(body["tax_kind"], "route_local_v1");
    assert_eq!(body["estimated_amount_out"], "964550");
    assert_eq!(
        body["estimated_amount_out_net"],
        body["estimated_amount_out"]
    );
    let response = server
        .post("/api/v1/route/solve")
        .json(&json!({
            "token_in":seed.token_a,"token_out":seed.token_c,"amount_in":"1010000",
            "hybrid_by_hop":[null,{"pool_input":"900000","book_input":"0","max_maker_fills":1}]
        }))
        .await;
    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(body["tax_kind"], "route_local_v1");
    assert_eq!(
        body["estimated_amount_out_net"],
        body["estimated_amount_out"]
    );
}

#[serial]
#[tokio::test]
async fn failed_discovery_and_untrusted_spoofed_capability_never_waive_tax() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_2hop(&pool).await;
    for (trusted, fail) in [(true, true), (false, false)] {
        let mock = mock_tax_lcd(&seed.token_b, Some(1), fail).await;
        let app = common::build_test_app_with_price_and_config(
            pool.clone(),
            None,
            config(&mock, trusted),
        )
        .await;
        let server = TestServer::new(app);
        let response = server
            .get(&format!(
                "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1010000&pool_only=true",
                seed.token_a, seed.token_c
            ))
            .await;
        response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        let response = server
            .post("/api/v1/route/solve")
            .json(&json!({"token_in":seed.token_a,"token_out":seed.token_c,"amount_in":"1010000"}))
            .await;
        response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    }
}

#[serial]
#[tokio::test]
async fn legacy_middle_tax_and_incompatible_funding_versions_are_rejected() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_2hop(&pool).await;
    for version in [None, Some(2)] {
        let mock = mock_tax_lcd(&seed.token_b, version, false).await;
        let app = common::build_test_app_with_price_and_config(
            pool.clone(),
            None,
            config(&mock, version.is_some()),
        )
        .await;
        let server = TestServer::new(app);
        let response = server
            .get(&format!(
                "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1010000&pool_only=true",
                seed.token_a, seed.token_c
            ))
            .await;
        response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    }
}

#[serial]
#[tokio::test]
async fn best_execution_keeps_supported_taxed_intermediate() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_2hop(&pool).await;
    let mock = mock_tax_lcd(&seed.token_b, Some(1), false).await;
    let app = common::build_test_app_with_price_and_config(pool, None, config(&mock, true)).await;
    let server = TestServer::new(app);
    let response = server
        .get(&format!(
            "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1010000",
            seed.token_a, seed.token_c
        ))
        .await;
    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(body["hops"].as_array().unwrap().len(), 2);
    assert_eq!(body["tax_kind"], "route_local_v1");
    assert_eq!(body["estimated_amount_out_net"], "964550");
}

#[serial]
#[tokio::test]
async fn best_execution_compares_supported_taxed_route_by_executable_net() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_multi_path(&pool).await;
    let mock = mock_tax_lcd(&seed.token_b, Some(1), false).await;
    Mock::given(method("GET"))
        .and(path_regex(
            r"^/cosmwasm/wasm/v1/contract/terra1router1381/smart/.+$",
        ))
        .respond_with(move |req: &Request| {
            let q = smart_query_from_request(req);
            let data = if let Some(sim) = q.get("simulate_swap_operations") {
                let hops = sim["operations"].as_array().unwrap().len();
                json!({"amount": if hops==1 {"900000"} else {"850000"}})
            } else {
                json!({"funding_version":1})
            };
            ResponseTemplate::new(200).set_body_json(json!({"data":data}))
        })
        .with_priority(1)
        .mount(&mock)
        .await;
    let app = common::build_test_app_with_price_and_config(pool, None, config(&mock, true)).await;
    let server = TestServer::new(app);
    let response = server
        .get(&format!(
            "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1010000",
            seed.token_a, seed.token_c
        ))
        .await;
    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(body["hops"].as_array().unwrap().len(), 1);
    assert_eq!(body["estimated_amount_out_net"], "900000");
    assert_eq!(body["paths_considered"], 2);
}

#[serial]
#[tokio::test]
async fn funded_quote_cache_and_progress_keep_actual_gross_amount_identity() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_2hop(&pool).await;
    let mock = mock_tax_lcd(&seed.token_b, Some(1), false).await;
    let app = common::build_test_app_with_price_and_config(pool, None, config(&mock, true)).await;
    let server = TestServer::new(app);
    for amount in [1_010_000u128, 1_010_000, 1_020_000] {
        let url = format!(
            "/api/v1/route/solve?token_in={}&token_out={}&amount_in={}",
            seed.token_a, seed.token_c, amount
        );
        let response = server.get(&url).await;
        response.assert_status_ok();
        let body: Value = response.json();
        assert_eq!(
            body["estimated_amount_out_net"],
            (amount * 955 / 1000).to_string()
        );
        assert_eq!(body["tax_kind"], "route_local_v1");
        let response = server
            .get(&format!(
                "/api/v1/route/solve/progress?token_in={}&token_out={}&amount_in={}",
                seed.token_a, seed.token_c, amount
            ))
            .await;
        response.assert_status_ok();
        let progress: Value = response.json();
        assert!(matches!(
            progress["stage"].as_str(),
            Some("done" | "cached")
        ));
    }
    // Discovery refresh alone must not hide an active/completed progress record. The old
    // wall-clock epoch cache identity lost it every five seconds, even without a migration.
    tokio::time::sleep(std::time::Duration::from_secs(6)).await;
    let response = server
        .get(&format!(
            "/api/v1/route/solve/progress?token_in={}&token_out={}&amount_in=1010000",
            seed.token_a, seed.token_c
        ))
        .await;
    response.assert_status_ok();
    let progress: Value = response.json();
    assert!(matches!(
        progress["stage"].as_str(),
        Some("done" | "cached")
    ));
    // Different actual amounts share the old coarse bucket; v1 now keys the exact gross
    // amount, so it cannot reuse the prior amount's funded splits. Both distinct gross budgets reach router simulation.
    let requests = mock.received_requests().await.unwrap();
    for amount in ["1010000", "1020000"] {
        assert!(requests.iter().any(
            |request| smart_query_from_request(request)["simulate_swap_operations"]["offer_amount"]
                == amount
        ));
    }
}

/// #1390: protocol infrastructure cannot impersonate a route trader. The indexer
/// must expose the preview identity rejection without a route estimate.
#[serial]
#[tokio::test]
async fn protocol_exempt_trader_guard_rejection_is_observable_on_all_forward_apis() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_2hop(&pool).await;
    let mock = mock_tax_lcd(&seed.token_b, Some(1), false).await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/.+$"))
        .and(|req: &Request| {
            let q = smart_query_from_request(req);
            q.get("simulate_swap_operations").is_some() || q.get("is_protocol_exempt").is_some()
        })
        .respond_with(|req: &Request| {
            let q = smart_query_from_request(req);
            if q.get("is_protocol_exempt").is_some() {
                ResponseTemplate::new(200)
                    .set_body_json(json!({"data":{"manager":false,"protocol":true}}))
            } else {
                ResponseTemplate::new(500).set_body_string(
                    "query wasm contract failed: router swap requires a non-exempt trader",
                )
            }
        })
        .with_priority(1)
        .mount(&mock)
        .await;
    let app = common::build_test_app_with_price_and_config(pool, None, config(&mock, true)).await;
    let server = TestServer::new(app);
    let trader = "terra1synthetictrader1390xxxxxxxxxxxxxxxxx";
    for endpoint in ["solve", "solve/best"] {
        for pool_only in ["true", "false"] {
            let response = server.get(&format!(
                "/api/v1/route/{endpoint}?token_in={}&token_out={}&amount_in=1010000&pool_only={pool_only}&trader={trader}",
                seed.token_a, seed.token_c
            )).await;
            response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
            assert!(response.text().contains("router_preview_rejected"));
            assert!(!response.text().contains("estimated_amount_out"));
        }
    }
    let response = server
        .post("/api/v1/route/solve")
        .json(&json!({
            "token_in":seed.token_a, "token_out":seed.token_c,
            "amount_in":"1010000", "trader":trader
        }))
        .await;
    response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.text().contains("router_preview_rejected"));
}

#[serial]
#[tokio::test]
async fn best_execution_keeps_valid_alternative_after_tax_preview_rejection() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_multi_path(&pool).await;
    let mock = mock_tax_lcd(&seed.token_b, Some(1), false).await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/terra1router1381/smart/.+$"))
        .respond_with(|request: &Request| {
            let q = smart_query_from_request(request);
            if let Some(sim) = q.get("simulate_swap_operations") {
                if sim["operations"].as_array().unwrap().len() > 1 {
                    return ResponseTemplate::new(500).set_body_string(
                        "query wasm contract failed: Official router hop requires a trusted non-exempt trader",
                    );
                }
                ResponseTemplate::new(200).set_body_json(json!({"data":{"amount":"900000"}}))
            } else {
                ResponseTemplate::new(200).set_body_json(json!({"data":{"funding_version":1}}))
            }
        })
        .with_priority(1)
        .mount(&mock).await;
    let app = common::build_test_app_with_price_and_config(pool, None, config(&mock, true)).await;
    let server = TestServer::new(app);
    let response = server
        .get(&format!(
            "/api/v1/route/solve/best?token_in={}&token_out={}&amount_in=1010000",
            seed.token_a, seed.token_c,
        ))
        .await;
    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(body["hops"].as_array().unwrap().len(), 1);
    assert_eq!(body["estimated_amount_out"], "900000");
}

/// Structural GET/POST accept endpoints, not a caller-selected asset path. POST
/// overrides hybrid quantities on the discovered hops; it cannot replace their assets.
#[serial]
#[tokio::test]
async fn amountless_get_post_cannot_inject_a_repeated_capable_asset_path() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_2hop(&pool).await;
    let mock = mock_tax_lcd(&seed.token_b, Some(1), false).await;
    let app = common::build_test_app_with_price_and_config(pool, None, config(&mock, true)).await;
    let server = TestServer::new(app);
    let cycle = [
        seed.token_a.clone(),
        seed.token_b.clone(),
        seed.token_a.clone(),
        seed.token_b.clone(),
        seed.token_c.clone(),
    ];
    let attempted_ops: Vec<_> = cycle
        .windows(2)
        .map(|assets| {
            json!({"terra_swap":{
                "offer_asset_info":{"token":{"contract_addr":assets[0]}},
                "ask_asset_info":{"token":{"contract_addr":assets[1]}},"hybrid":null
            }})
        })
        .collect();
    let get = server
        .get(&format!(
            "/api/v1/route/solve?token_in={}&token_out={}&path={}&pairs=untrusted-cycle",
            seed.token_a,
            seed.token_c,
            cycle.join(",")
        ))
        .await;
    get.assert_status_ok();
    let post = server
        .post("/api/v1/route/solve")
        .json(&json!({
            "token_in":seed.token_a,"token_out":seed.token_c,
            "path":cycle,"pairs":["untrusted-cycle"],"router_operations":attempted_ops,
            "hybrid_by_hop":[null,null]
        }))
        .await;
    post.assert_status_ok();
    for body in [get.json::<Value>(), post.json::<Value>()] {
        assert_eq!(body["quote_kind"], "indexer_route_only");
        assert!(body["estimated_amount_out"].is_null());
        let hops = body["hops"].as_array().unwrap();
        assert_eq!(hops.len(), 2);
        assert_eq!(hops[0]["offer_token"], seed.token_a);
        assert_eq!(hops[0]["ask_token"], seed.token_b);
        assert_eq!(hops[1]["offer_token"], seed.token_b);
        assert_eq!(hops[1]["ask_token"], seed.token_c);
        assert_eq!(body["router_operations"].as_array().unwrap().len(), 2);
    }
}
