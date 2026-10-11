//! Scheduled LCD capability admission + PostgreSQL economics, followed by disabled
//! providers. Contract multitests separately prove execution and conservation.
mod common;
#[path = "common/local_tax_quotes.rs"]
mod local_tax_quotes;

use axum_test::TestServer;
use common::lcd_mock::smart_query_from_request;
use local_tax_quotes::{
    app, assert_no_request_rpc, config, mock_tax_lcd, pad_seed, pool_net, seed_reserves,
    taxed_two_hop,
};
use serde_json::{json, Value};
use serial_test::serial;
use wiremock::{
    matchers::{method, path_regex},
    Mock, Request, ResponseTemplate,
};

/// A useful baseline is already cached before optional deeper work completes.
/// Wait for the bounded job rather than racing its explicitly supported evaluating stage.
async fn ready_progress(server: &TestServer, url: &str) -> Value {
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        let response = server.get(url).await;
        response.assert_status_ok();
        let progress: Value = response.json();
        assert_eq!(
            progress["cache_hit"], true,
            "baseline must remain available: {progress}"
        );
        if matches!(progress["stage"].as_str(), Some("done" | "cached")) {
            return progress;
        }
        assert_eq!(
            progress["stage"], "evaluating",
            "unexpected progress: {progress}"
        );
        assert!(
            tokio::time::Instant::now() < deadline,
            "deeper local job failed to finish: {progress}"
        );
        tokio::time::sleep(std::time::Duration::from_millis(10)).await;
    }
}

#[serial]
#[tokio::test]
async fn upgraded_taxed_intermediate_absent_catalog_get_post_are_net_and_eligible() {
    let pool = common::setup_pool().await;
    let seed = pad_seed(&pool, common::seed_route_solve_2hop(&pool).await).await;
    seed_reserves(&pool).await;
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM community_tokens")
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(count, 0);
    let mock = mock_tax_lcd(&seed.token_b, Some(1), false).await;
    let server = TestServer::new(app(pool, config(&mock, true), &mock).await);
    let response = server
        .get(&format!(
            "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1010000&pool_only=true",
            seed.token_a, seed.token_c
        ))
        .await;
    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(body["hops"].as_array().unwrap().len(), 2);
    assert_eq!(body["tax_kind"], "route_local_v1");
    assert_eq!(
        body["estimated_amount_out"],
        taxed_two_hop(1_010_000).to_string()
    );
    assert_eq!(
        body["estimated_amount_out_net"],
        body["estimated_amount_out"]
    );
    let first = &body["execution_metadata"]["hops"][0];
    let second = &body["execution_metadata"]["hops"][1];
    assert_eq!(second["offer_amount"], first["return_amount"]);
    let gross = second["offer_amount"].as_str().unwrap();
    let response = server
        .post("/api/v1/route/solve")
        .json(
            &json!({"token_in":seed.token_a,"token_out":seed.token_c,"amount_in":"1010000",
        "hybrid_by_hop":[null,{"pool_input":gross,"book_input":"0","max_maker_fills":1}]}),
        )
        .await;
    response.assert_status_ok();
    let posted: Value = response.json();
    assert_eq!(posted["tax_kind"], "route_local_v1");
    assert_eq!(posted["estimated_amount_out"], body["estimated_amount_out"]);
    assert_eq!(
        posted["estimated_amount_out_net"],
        posted["estimated_amount_out"]
    );
    assert_eq!(
        posted["router_operations"][1]["terra_swap"]["hybrid"]["pool_input"],
        second["economic_offer_amount"]
    );
    assert_no_request_rpc(&mock).await;
}
#[serial]
#[tokio::test]
async fn failed_discovery_and_untrusted_spoofed_capability_never_waive_tax() {
    let pool = common::setup_pool().await;
    let seed = pad_seed(&pool, common::seed_route_solve_2hop(&pool).await).await;
    seed_reserves(&pool).await;
    for (trusted, fail) in [(true, true), (false, false)] {
        let mock = mock_tax_lcd(&seed.token_b, Some(1), fail).await;
        let server = TestServer::new(app(pool.clone(), config(&mock, trusted), &mock).await);
        let response = server
            .get(&format!(
                "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1010000&pool_only=true",
                seed.token_a, seed.token_c
            ))
            .await;
        let status = if fail {
            axum::http::StatusCode::SERVICE_UNAVAILABLE
        } else {
            axum::http::StatusCode::UNPROCESSABLE_ENTITY
        };
        let reason = if fail {
            "missing_observations"
        } else {
            "unsupported_semantics"
        };
        response.assert_status(status);
        assert_eq!(response.json::<Value>()["error"], reason);
        assert!(!response.text().contains("estimated_amount_out"));
        let response = server
            .post("/api/v1/route/solve")
            .json(&json!({"token_in":seed.token_a,"token_out":seed.token_c,"amount_in":"1010000"}))
            .await;
        // Missing producer evidence and completed unsupported capability are
        // distinct; neither can waive tax or trigger request-time discovery.
        response.assert_status(status);
        assert_eq!(response.json::<Value>()["error"], reason);
        assert_no_request_rpc(&mock).await;
    }
}
#[serial]
#[tokio::test]
async fn legacy_middle_tax_and_incompatible_funding_versions_are_rejected() {
    let pool = common::setup_pool().await;
    let seed = pad_seed(&pool, common::seed_route_solve_2hop(&pool).await).await;
    seed_reserves(&pool).await;
    for version in [None, Some(2)] {
        let mock = mock_tax_lcd(&seed.token_b, version, false).await;
        let server =
            TestServer::new(app(pool.clone(), config(&mock, version.is_some()), &mock).await);
        let response = server
            .get(&format!(
                "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1010000&pool_only=true",
                seed.token_a, seed.token_c
            ))
            .await;
        response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(response.json::<Value>()["error"], "unsupported_semantics");
        assert_no_request_rpc(&mock).await;
    }
}
#[serial]
#[tokio::test]
async fn best_execution_keeps_supported_taxed_intermediate() {
    let pool = common::setup_pool().await;
    let seed = pad_seed(&pool, common::seed_route_solve_2hop(&pool).await).await;
    seed_reserves(&pool).await;
    let mock = mock_tax_lcd(&seed.token_b, Some(1), false).await;
    let server = TestServer::new(app(pool, config(&mock, true), &mock).await);
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
    assert_eq!(
        body["estimated_amount_out_net"],
        taxed_two_hop(1_010_000).to_string()
    );
    assert_no_request_rpc(&mock).await;
}
#[serial]
#[tokio::test]
async fn best_execution_compares_supported_taxed_route_by_executable_net() {
    let pool = common::setup_pool().await;
    let seed = pad_seed(&pool, common::seed_route_solve_multi_path(&pool).await).await;
    seed_reserves(&pool).await;
    // The ordinary direct corridor loses against untaxed 2-hop output, but wins
    // against actual funded intermediate net. Reserves, not LCD return constants, establish it.
    let pair_id: i32 =
        sqlx::query_scalar("SELECT id FROM pairs WHERE contract_address='terra1pairroutempac'")
            .fetch_one(&pool)
            .await
            .unwrap();
    cl8y_dex_indexer::db::queries::pair_reserves::upsert_pair_reserves(
        &pool,
        pair_id,
        &"10000000000000".parse().unwrap(),
        &"9600000000000".parse().unwrap(),
        30,
        Some(1381),
    )
    .await
    .unwrap();
    let mock = mock_tax_lcd(&seed.token_b, Some(1), false).await;
    let server = TestServer::new(app(pool, config(&mock, true), &mock).await);
    let response = server
        .get(&format!(
            "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1010000",
            seed.token_a, seed.token_c
        ))
        .await;
    response.assert_status_ok();
    let body: Value = response.json();
    let direct = pool_net(1_010_000, 10_000_000_000_000, 9_600_000_000_000);
    let untaxed = pool_net(
        pool_net(1_010_000, 10_000_000_000_000, 10_000_000_000_000),
        10_000_000_000_000,
        10_000_000_000_000,
    );
    assert!(untaxed > direct && direct > taxed_two_hop(1_010_000));
    assert_eq!(body["hops"].as_array().unwrap().len(), 1);
    assert_eq!(body["estimated_amount_out_net"], direct.to_string());
    assert_eq!(body["paths_considered"], 2);
    assert_no_request_rpc(&mock).await;
}
#[serial]
#[tokio::test]
async fn funded_quote_cache_and_progress_keep_actual_gross_amount_identity() {
    let pool = common::setup_pool().await;
    let seed = pad_seed(&pool, common::seed_route_solve_2hop(&pool).await).await;
    seed_reserves(&pool).await;
    let mock = mock_tax_lcd(&seed.token_b, Some(1), false).await;
    let server = TestServer::new(app(pool, config(&mock, true), &mock).await);
    let mut prior = None;
    for amount in [1_010_000u128, 1_010_000, 1_020_000] {
        let response = server
            .get(&format!(
                "/api/v1/route/solve?token_in={}&token_out={}&amount_in={amount}",
                seed.token_a, seed.token_c
            ))
            .await;
        response.assert_status_ok();
        let body: Value = response.json();
        assert_eq!(
            body["estimated_amount_out_net"],
            taxed_two_hop(amount).to_string()
        );
        assert_eq!(
            body["execution_metadata"]["input_amount"],
            amount.to_string()
        );
        assert_eq!(body["tax_kind"], "route_local_v1");
        if amount == 1_020_000 {
            assert_ne!(prior.as_ref().unwrap(), &body["estimated_amount_out_net"]);
        }
        prior = Some(body["estimated_amount_out_net"].clone());
        ready_progress(
            &server,
            &format!(
                "/api/v1/route/solve/progress?token_in={}&token_out={}&amount_in={amount}",
                seed.token_a, seed.token_c
            ),
        )
        .await;
    }
    tokio::time::sleep(std::time::Duration::from_secs(6)).await;
    ready_progress(
        &server,
        &format!(
            "/api/v1/route/solve/progress?token_in={}&token_out={}&amount_in=1010000",
            seed.token_a, seed.token_c
        ),
    )
    .await;
    assert_no_request_rpc(&mock).await;
}
#[serial]
#[tokio::test]
async fn protocol_exempt_trader_guard_rejection_is_observable_on_all_forward_apis() {
    let pool = common::setup_pool().await;
    let seed = pad_seed(&pool, common::seed_route_solve_2hop(&pool).await).await;
    seed_reserves(&pool).await;
    let trader = "terra1synthetictrader1390xxxxxxxxxxxxxxxxx";
    common::seed_traders_with_tiers(&pool, &[(trader, 0)]).await;
    let mock = mock_tax_lcd(&seed.token_b, Some(1), false).await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/.+$"))
        .and(move |req: &Request| {
            smart_query_from_request(req)["is_protocol_exempt"]["address"] == trader
        })
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"data":{"address":trader,"manager":false,"protocol":true}})),
        )
        .with_priority(1)
        .mount(&mock)
        .await;
    let server = TestServer::new(app(pool, config(&mock, true), &mock).await);
    for endpoint in ["solve", "solve/best"] {
        for pool_only in ["true", "false"] {
            let response=server.get(&format!("/api/v1/route/{endpoint}?token_in={}&token_out={}&amount_in=1010000&pool_only={pool_only}&trader={trader}",seed.token_a,seed.token_c)).await;
            response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
            assert_eq!(
                response.json::<serde_json::Value>()["error"],
                "trading_blocked"
            );
            assert!(!response.text().contains("estimated_amount_out"));
        }
    }
    let response=server.post("/api/v1/route/solve").json(&json!({"token_in":seed.token_a,"token_out":seed.token_c,"amount_in":"1010000","trader":trader})).await;
    response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    assert!(!response.text().contains("estimated_amount_out"));
    assert_no_request_rpc(&mock).await;
}
#[serial]
#[tokio::test]
async fn best_execution_keeps_valid_alternative_after_tax_preview_rejection() {
    let pool = common::setup_pool().await;
    let seed = pad_seed(&pool, common::seed_route_solve_multi_path(&pool).await).await;
    seed_reserves(&pool).await;
    let mock = mock_tax_lcd(&seed.token_b, Some(1), false).await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/.+$"))
        .and(|req: &Request| {
            smart_query_from_request(req)
                .get("route_tax_preview")
                .is_some()
        })
        .respond_with(
            ResponseTemplate::new(500)
                .set_body_string("query wasm contract failed: route pair is not registered"),
        )
        .with_priority(1)
        .mount(&mock)
        .await;
    let server = TestServer::new(app(pool, config(&mock, true), &mock).await);
    let response = server
        .get(&format!(
            "/api/v1/route/solve/best?token_in={}&token_out={}&amount_in=1010000",
            seed.token_a, seed.token_c
        ))
        .await;
    response.assert_status_ok();
    let body: Value = response.json();
    assert_eq!(body["hops"].as_array().unwrap().len(), 1);
    assert_eq!(
        body["estimated_amount_out"],
        pool_net(1_010_000, 10_000_000_000_000, 10_000_000_000_000).to_string()
    );
    assert_no_request_rpc(&mock).await;
}
#[serial]
#[tokio::test]
async fn amountless_get_post_cannot_inject_a_repeated_capable_asset_path() {
    let pool = common::setup_pool().await;
    let seed = pad_seed(&pool, common::seed_route_solve_2hop(&pool).await).await;
    seed_reserves(&pool).await;
    let mock = mock_tax_lcd(&seed.token_b, Some(1), false).await;
    let server = TestServer::new(app(pool, config(&mock, true), &mock).await);
    let cycle = [
        seed.token_a.clone(),
        seed.token_b.clone(),
        seed.token_a.clone(),
        seed.token_b.clone(),
        seed.token_c.clone(),
    ];
    let attempted_ops:Vec<_>=cycle.windows(2).map(|assets|json!({"terra_swap":{"offer_asset_info":{"token":{"contract_addr":assets[0]}},"ask_asset_info":{"token":{"contract_addr":assets[1]}},"hybrid":null}})).collect();
    let get = server
        .get(&format!(
            "/api/v1/route/solve?token_in={}&token_out={}&path={}&pairs=untrusted-cycle",
            seed.token_a,
            seed.token_c,
            cycle.join(",")
        ))
        .await;
    get.assert_status_ok();
    let post=server.post("/api/v1/route/solve").json(&json!({"token_in":seed.token_a,"token_out":seed.token_c,"path":cycle,"pairs":["untrusted-cycle"],"router_operations":attempted_ops,"hybrid_by_hop":[null,null]})).await;
    post.assert_status(axum::http::StatusCode::BAD_REQUEST);
    assert!(!post.text().contains("estimated_amount_out"));
    let priced=server.post("/api/v1/route/solve").json(&json!({"token_in":seed.token_a,"token_out":seed.token_c,"amount_in":"1010000","path":cycle,"pairs":["untrusted-cycle"],"router_operations":attempted_ops,"hybrid_by_hop":[null,null]})).await;
    priced.assert_status_ok();
    assert_eq!(get.json::<Value>()["quote_kind"], "indexer_route_only");
    assert!(get.json::<Value>()["estimated_amount_out"].is_null());
    assert_eq!(
        priced.json::<Value>()["estimated_amount_out"],
        taxed_two_hop(1_010_000).to_string()
    );
    for body in [get.json::<Value>(), priced.json::<Value>()] {
        let hops = body["hops"].as_array().unwrap();
        assert_eq!(hops.len(), 2);
        assert_eq!(hops[0]["offer_token"], seed.token_a);
        assert_eq!(hops[0]["ask_token"], seed.token_b);
        assert_eq!(hops[1]["offer_token"], seed.token_b);
        assert_eq!(hops[1]["ask_token"], seed.token_c);
        assert_eq!(body["router_operations"].as_array().unwrap().len(), 2);
    }
    assert_no_request_rpc(&mock).await;
}
