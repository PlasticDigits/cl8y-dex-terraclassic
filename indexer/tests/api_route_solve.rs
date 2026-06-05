mod common;

use axum_test::TestServer;
use common::lcd_mock;
use serde_json::{json, Value};
use serial_test::serial;

#[serial]
#[tokio::test]
async fn route_solve_returns_hops_and_pool_only_hybrid() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}",
        seed.token_a, seed.token_b
    );
    let resp = server.get(&url).await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    let hops = body["hops"].as_array().expect("hops");
    assert_eq!(hops.len(), 1);
    assert_eq!(hops[0]["pair"], "terra1pairrouteabc");
    assert_eq!(hops[0]["offer_token"], seed.token_a);
    assert_eq!(hops[0]["ask_token"], seed.token_b);

    let ops = body["router_operations"]
        .as_array()
        .expect("router_operations");
    assert_eq!(ops.len(), 1);
    assert!(ops[0]["terra_swap"]["hybrid"].is_null());
    assert!(body["estimated_amount_out"].is_null());
    assert_eq!(
        body["intermediate_tokens"],
        json!([seed.token_a, seed.token_b])
    );
    assert_eq!(body["quote_kind"], "indexer_route_only");
}

#[serial]
#[tokio::test]
async fn route_solve_unknown_token_returns_400() {
    let pool = common::setup_pool().await;
    common::seed_route_solve(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get("/api/v1/route/solve?token_in=terra1notindexed&token_out=terra1routesolvebbb")
        .await;
    resp.assert_status_bad_request();
}

#[serial]
#[tokio::test]
async fn route_solve_no_path_returns_404() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}",
        seed.token_a, seed.token_c
    );
    let resp = server.get(&url).await;
    resp.assert_status_not_found();
    let text = resp.text();
    assert!(
        text.contains("no route") || text.contains("3 hops") || text.contains("4 hops"),
        "{}",
        text
    );
}

#[serial]
#[tokio::test]
async fn route_solve_post_hybrid_length_mismatch_returns_400() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let body = json!({
        "token_in": seed.token_a,
        "token_out": seed.token_b,
        "hybrid_by_hop": [
            { "pool_input": "1", "book_input": "1", "max_maker_fills": 8 },
            { "pool_input": "1", "book_input": "1", "max_maker_fills": 8 }
        ]
    });
    let resp = server.post("/api/v1/route/solve").json(&body).await;
    resp.assert_status_bad_request();
}

#[serial]
#[tokio::test]
async fn route_solve_post_merges_hybrid_and_simulates() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve(&pool).await;
    let mock = lcd_mock::start_smart_query_data_mock(json!({ "amount": "424242" })).await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![lcd_mock::lcd_base_url(&mock)];
    cfg.router_address = Some("terra1routertest".to_string());
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let body = json!({
        "token_in": seed.token_a,
        "token_out": seed.token_b,
        "amount_in": "1000000",
        "hybrid_by_hop": [{
            "pool_input": "700000",
            "book_input": "300000",
            "max_maker_fills": 8,
            "book_start_hint": null
        }]
    });
    let resp = server.post("/api/v1/route/solve").json(&body).await;
    resp.assert_status_ok();
    let j: Value = resp.json();
    let hybrid = &j["router_operations"][0]["terra_swap"]["hybrid"];
    assert!(!hybrid.is_null());
    assert_eq!(hybrid["pool_input"], "700000");
    assert_eq!(hybrid["book_input"], "300000");
    assert_eq!(j["estimated_amount_out"], "424242");
    assert_eq!(
        j["intermediate_tokens"],
        json!([seed.token_a, seed.token_b])
    );
    assert_eq!(j["quote_kind"], "indexer_hybrid_lcd");
}

#[serial]
#[tokio::test]
async fn route_solve_get_default_hybrid_two_hops() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_2hop(&pool).await;
    let mock = lcd_mock::start_hybrid_route_optimizer_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![lcd_mock::lcd_base_url(&mock)];
    cfg.router_address = Some("terra1routertest".to_string());
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in={}",
        seed.token_a, seed.token_c, "1000000"
    );
    let resp = server.get(&url).await;
    resp.assert_status_ok();
    let j: Value = resp.json();
    assert_eq!(j["hops"].as_array().unwrap().len(), 2);
    assert_eq!(
        j["intermediate_tokens"],
        json!([seed.token_a, seed.token_b, seed.token_c])
    );
    assert_eq!(j["quote_kind"], "indexer_hybrid_lcd");
    assert_eq!(j["estimated_amount_out"], "8888888");
    assert!(!j["hybrid_notes"].is_null());
    let ops = j["router_operations"].as_array().unwrap();
    assert!(!ops[0]["terra_swap"]["hybrid"].is_null());
    assert!(!ops[1]["terra_swap"]["hybrid"].is_null());
}

#[serial]
#[tokio::test]
async fn route_solve_pool_only_escape_hatch() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve(&pool).await;
    let mock = lcd_mock::start_smart_query_data_mock(json!({ "amount": "424242" })).await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![lcd_mock::lcd_base_url(&mock)];
    cfg.router_address = Some("terra1routertest".to_string());
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1000000&pool_only=true",
        seed.token_a, seed.token_b
    );
    let resp = server.get(&url).await;
    resp.assert_status_ok();
    let j: Value = resp.json();
    assert!(j["router_operations"][0]["terra_swap"]["hybrid"].is_null());
    assert_eq!(j["estimated_amount_out"], "424242");
    assert_eq!(j["quote_kind"], "indexer_pool_lcd");
    assert!(j["hybrid_notes"].is_null());
}

#[serial]
#[tokio::test]
async fn route_solve_hybrid_optimize_requires_amount_in() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&hybrid_optimize=true",
        seed.token_a, seed.token_b
    );
    let resp = server.get(&url).await;
    resp.assert_status_bad_request();
}

#[serial]
#[tokio::test]
async fn route_solve_get_hybrid_optimize_two_hops() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_2hop(&pool).await;
    let mock = lcd_mock::start_hybrid_route_optimizer_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![lcd_mock::lcd_base_url(&mock)];
    cfg.router_address = Some("terra1routertest".to_string());
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in={}&hybrid_optimize=true",
        seed.token_a, seed.token_c, "1000000"
    );
    let resp = server.get(&url).await;
    resp.assert_status_ok();
    let j: Value = resp.json();
    assert_eq!(j["hops"].as_array().unwrap().len(), 2);
    assert_eq!(
        j["intermediate_tokens"],
        json!([seed.token_a, seed.token_b, seed.token_c])
    );
    assert_eq!(j["quote_kind"], "indexer_hybrid_lcd");
    assert_eq!(j["estimated_amount_out"], "8888888");
    assert!(!j["hybrid_notes"].is_null());
    let ops = j["router_operations"].as_array().unwrap();
    assert!(!ops[0]["terra_swap"]["hybrid"].is_null());
    assert!(!ops[1]["terra_swap"]["hybrid"].is_null());
}

#[serial]
#[tokio::test]
async fn route_solve_post_three_hop_multi_leg_hybrid() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_3hop(&pool).await;
    let token_d = seed.token_d.as_ref().expect("3-hop seed includes token_d");
    let mock = lcd_mock::start_smart_query_data_mock(json!({ "amount": "7777777" })).await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![lcd_mock::lcd_base_url(&mock)];
    cfg.router_address = Some("terra1routertest".to_string());
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let body = json!({
        "token_in": seed.token_a,
        "token_out": token_d,
        "amount_in": "1000000",
        "hybrid_by_hop": [
            {
                "pool_input": "250000",
                "book_input": "750000",
                "max_maker_fills": 8,
                "book_start_hint": null
            },
            {
                "pool_input": "400000",
                "book_input": "600000",
                "max_maker_fills": 8,
                "book_start_hint": null
            },
            null
        ]
    });
    let resp = server.post("/api/v1/route/solve").json(&body).await;
    resp.assert_status_ok();
    let j: Value = resp.json();
    let hops = j["hops"].as_array().unwrap();
    assert_eq!(hops.len(), 3, "expected 3-hop path A→B→C→D");
    let ops = j["router_operations"].as_array().unwrap();
    assert_eq!(ops.len(), 3);
    assert!(!ops[0]["terra_swap"]["hybrid"].is_null());
    assert!(!ops[1]["terra_swap"]["hybrid"].is_null());
    assert!(ops[2]["terra_swap"]["hybrid"].is_null());
    assert_eq!(j["estimated_amount_out"], "7777777");
    assert_eq!(
        j["intermediate_tokens"],
        json!([seed.token_a, seed.token_b, seed.token_c, token_d])
    );
    assert_eq!(j["quote_kind"], "indexer_hybrid_lcd");
}

#[serial]
#[tokio::test]
async fn route_solve_get_hybrid_optimize_three_hops() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_3hop(&pool).await;
    let token_d = seed.token_d.as_ref().expect("3-hop seed includes token_d");
    let mock = lcd_mock::start_hybrid_route_optimizer_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![lcd_mock::lcd_base_url(&mock)];
    cfg.router_address = Some("terra1routertest".to_string());
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in={}&hybrid_optimize=true",
        seed.token_a, token_d, "1000000"
    );
    let resp = server.get(&url).await;
    resp.assert_status_ok();
    let j: Value = resp.json();
    assert_eq!(j["hops"].as_array().unwrap().len(), 3);
    assert_eq!(
        j["intermediate_tokens"],
        json!([seed.token_a, seed.token_b, seed.token_c, token_d])
    );
    assert_eq!(j["quote_kind"], "indexer_hybrid_lcd");
    assert_eq!(j["estimated_amount_out"], "8888888");
    let ops = j["router_operations"].as_array().unwrap();
    assert_eq!(ops.len(), 3);
    assert!(!ops[0]["terra_swap"]["hybrid"].is_null());
    assert!(!ops[1]["terra_swap"]["hybrid"].is_null());
    assert!(!ops[2]["terra_swap"]["hybrid"].is_null());
}

#[serial]
#[tokio::test]
async fn route_solve_get_default_hybrid_four_hops() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_4hop(&pool).await;
    let token_e = seed.token_e.as_ref().expect("4-hop seed includes token_e");
    let mock = lcd_mock::start_hybrid_route_optimizer_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![lcd_mock::lcd_base_url(&mock)];
    cfg.router_address = Some("terra1routertest".to_string());
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in={}",
        seed.token_a, token_e, "1000000"
    );
    let resp = server.get(&url).await;
    resp.assert_status_ok();
    let j: Value = resp.json();
    assert_eq!(j["hops"].as_array().unwrap().len(), 4);
    assert_eq!(
        j["intermediate_tokens"],
        json!([
            seed.token_a,
            seed.token_b,
            seed.token_c,
            seed.token_d.as_ref().unwrap(),
            token_e
        ])
    );
    assert_eq!(j["quote_kind"], "indexer_hybrid_lcd");
    assert_eq!(j["estimated_amount_out"], "8888888");
    let ops = j["router_operations"].as_array().unwrap();
    assert_eq!(ops.len(), 4);
    for op in ops {
        assert!(!op["terra_swap"]["hybrid"].is_null());
    }
}

#[serial]
#[tokio::test]
async fn route_solve_best_requires_amount_in() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve/best?token_in={}&token_out={}",
        seed.token_a, seed.token_b
    );
    let resp = server.get(&url).await;
    resp.assert_status_bad_request();
}

#[serial]
#[tokio::test]
async fn route_solve_best_matches_hybrid_optimize() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_2hop(&pool).await;
    let mock = lcd_mock::start_hybrid_route_optimizer_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![lcd_mock::lcd_base_url(&mock)];
    cfg.router_address = Some("terra1routertest".to_string());
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let best_url = format!(
        "/api/v1/route/solve/best?token_in={}&token_out={}&amount_in={}",
        seed.token_a, seed.token_c, "1000000"
    );
    let default_url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in={}",
        seed.token_a, seed.token_c, "1000000"
    );

    let best: Value = server.get(&best_url).await.json();
    let default_get: Value = server.get(&default_url).await.json();

    assert_eq!(best["quote_kind"], "indexer_hybrid_lcd");
    assert_eq!(
        best["estimated_amount_out"],
        default_get["estimated_amount_out"]
    );
    assert_eq!(best["router_operations"], default_get["router_operations"]);
}

#[serial]
#[tokio::test]
async fn route_solve_hybrid_optimize_degraded_falls_back_to_pool_only() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve(&pool).await;
    let mock = lcd_mock::start_hybrid_degraded_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![lcd_mock::lcd_base_url(&mock)];
    cfg.router_address = Some("terra1routertest".to_string());
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve/best?token_in={}&token_out={}&amount_in={}",
        seed.token_a, seed.token_b, "1000000"
    );
    let resp = server.get(&url).await;
    resp.assert_status_ok();
    let j: Value = resp.json();
    assert_eq!(j["quote_kind"], "indexer_hybrid_lcd_degraded");
    assert!(j["router_operations"][0]["terra_swap"]["hybrid"].is_null());
}

#[serial]
#[tokio::test]
async fn route_solve_global_picks_best_path_not_shortest() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_multi_path(&pool).await;
    let mock = lcd_mock::start_multi_path_router_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![lcd_mock::lcd_base_url(&mock)];
    cfg.router_address = Some("terra1routertest".to_string());
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1000000",
        seed.token_a, seed.token_c
    );
    let resp = server.get(&url).await;
    resp.assert_status_ok();
    let j: Value = resp.json();
    assert_eq!(j["solver_version"], "global_v1");
    assert_eq!(j["paths_considered"], 2);
    assert_eq!(j["hops"].as_array().unwrap().len(), 2, "2-hop path beats 1-hop");
    assert_eq!(j["estimated_amount_out"], "9000000");
    assert!(
        j["optimality_scope"]
            .as_str()
            .unwrap_or("")
            .contains("top-5"),
        "optimality_scope must describe search bounds"
    );
}

#[serial]
#[tokio::test]
async fn route_solve_global_response_metadata_contract() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_2hop(&pool).await;
    let mock = lcd_mock::start_hybrid_route_optimizer_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![lcd_mock::lcd_base_url(&mock)];
    cfg.router_address = Some("terra1routertest".to_string());
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve/best?token_in={}&token_out={}&amount_in=1000000",
        seed.token_a, seed.token_c
    );
    let j: Value = server.get(&url).await.json();
    assert_eq!(j["solver_version"], "global_v1");
    assert!(j["paths_considered"].as_u64().unwrap_or(0) >= 1);
    assert!(!j["hybrid_notes"].as_str().unwrap_or("").is_empty());
    assert!(j["lcd_hybrid_queries"].as_u64().unwrap_or(0) > 0);
}

#[serial]
#[tokio::test]
async fn route_solve_get_with_trader_returns_higher_estimate() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_2hop(&pool).await;
    let mock = lcd_mock::start_hybrid_route_optimizer_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![lcd_mock::lcd_base_url(&mock)];
    cfg.router_address = Some("terra1routertest".to_string());
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let base_url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1000000",
        seed.token_a, seed.token_c
    );
    let base: Value = server.get(&base_url).await.json();
    let trader = "terra1discountwallet000000000000000000000";
    let discounted_url = format!("{base_url}&trader={trader}");
    let discounted: Value = server.get(&discounted_url).await.json();

    assert_eq!(base["estimated_amount_out"], "8888888");
    assert_eq!(discounted["estimated_amount_out"], "9777776");
}

#[serial]
#[tokio::test]
async fn route_solve_invalid_trader_returns_400() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1000000&trader=not-a-wallet",
        seed.token_a, seed.token_b
    );
    server.get(&url).await.assert_status_bad_request();
}

#[serial]
#[tokio::test]
async fn route_solve_post_forwards_trader_to_router_sim() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve(&pool).await;
    let mock = lcd_mock::start_hybrid_route_optimizer_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![lcd_mock::lcd_base_url(&mock)];
    cfg.router_address = Some("terra1routertest".to_string());
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let trader = "terra1discountwallet000000000000000000000";
    let body = json!({
        "token_in": seed.token_a,
        "token_out": seed.token_b,
        "amount_in": "1000000",
        "trader": trader,
        "hybrid_by_hop": [{
            "pool_input": "700000",
            "book_input": "300000",
            "max_maker_fills": 8,
            "book_start_hint": null
        }]
    });
    let j: Value = server.post("/api/v1/route/solve").json(&body).await.json();
    assert_eq!(j["estimated_amount_out"], "9777776");
}

/// Tier-0 router/hybrid LCD mock output (no discount subject or tier-0 sender).
const TIER0_ROUTE_AMOUNT: &str = "8888888";
/// Tier-5 router/hybrid LCD mock output (sender/trader in tier5_addrs list).
const TIER5_ROUTE_AMOUNT: &str = "9777776";

const TIER0_SENDER: &str = "terra1tier0sender000000000000000000000000";
const TIER5_SENDER_A: &str = "terra1tier5sendera00000000000000000000000";
const TIER5_SENDER_B: &str = "terra1tier5senderb00000000000000000000000";
const UNKNOWN_SENDER: &str = "terra1unknownsender0000000000000000000000";

#[serial]
#[tokio::test]
async fn route_solve_get_cache_tier_isolation() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_2hop(&pool).await;
    common::seed_traders_with_tiers(
        &pool,
        &[(TIER0_SENDER, 0), (TIER5_SENDER_A, 5)],
    )
    .await;

    let mock =
        lcd_mock::start_tier_aware_route_optimizer_mock(&[TIER5_SENDER_A]).await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![lcd_mock::lcd_base_url(&mock)];
    cfg.router_address = Some("terra1routertest".to_string());
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let base_url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1000000",
        seed.token_a, seed.token_c
    );

    let tier0: Value = server
        .get(&format!("{base_url}&sender={TIER0_SENDER}"))
        .await
        .json();
    assert_eq!(tier0["estimated_amount_out"], TIER0_ROUTE_AMOUNT);

    let tier5: Value = server
        .get(&format!("{base_url}&sender={TIER5_SENDER_A}"))
        .await
        .json();
    assert_eq!(tier5["estimated_amount_out"], TIER5_ROUTE_AMOUNT);
    assert_ne!(
        tier5["estimated_amount_out"], tier0["estimated_amount_out"],
        "tier-5 sender must not receive tier-0 cached quote"
    );

    let unknown: Value = server
        .get(&format!("{base_url}&sender={UNKNOWN_SENDER}"))
        .await
        .json();
    assert_eq!(
        unknown["estimated_amount_out"], TIER0_ROUTE_AMOUNT,
        "unknown sender resolves to tier 0"
    );
}

#[serial]
#[tokio::test]
async fn route_solve_get_cache_same_tier_reuses_lcd() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_2hop(&pool).await;
    common::seed_traders_with_tiers(
        &pool,
        &[(TIER5_SENDER_A, 5), (TIER5_SENDER_B, 5)],
    )
    .await;

    let mock = lcd_mock::start_tier_aware_route_optimizer_mock(&[
        TIER5_SENDER_A,
        TIER5_SENDER_B,
    ])
    .await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![lcd_mock::lcd_base_url(&mock)];
    cfg.router_address = Some("terra1routertest".to_string());
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let base_url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1000000",
        seed.token_a, seed.token_c
    );

    server
        .get(&format!("{base_url}&sender={TIER5_SENDER_A}"))
        .await
        .assert_status_ok();
    let lcd_calls_after_first = mock
        .received_requests()
        .await
        .expect("mock server should expose request log")
        .len();

    let tier5_b: Value = server
        .get(&format!("{base_url}&sender={TIER5_SENDER_B}"))
        .await
        .json();
    assert_eq!(tier5_b["estimated_amount_out"], TIER5_ROUTE_AMOUNT);

    let lcd_calls_after_second = mock
        .received_requests()
        .await
        .expect("request log")
        .len();
    assert_eq!(
        lcd_calls_after_first, lcd_calls_after_second,
        "same-tier senders should share hybrid GET cache (no extra LCD sim calls)"
    );
}

#[serial]
#[tokio::test]
async fn route_solve_invalid_sender_returns_400() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1000000&sender=not-a-wallet",
        seed.token_a, seed.token_b
    );
    server.get(&url).await.assert_status_bad_request();
}
