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
