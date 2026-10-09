mod common;

use axum_test::TestServer;
use common::lcd_mock;
use serde_json::{json, Value};
use serial_test::serial;

/// Synthetic generation prepared once; requests cannot fetch or repair observations.
async fn local_app(
    pool: sqlx::PgPool,
    config: cl8y_dex_indexer::config::Config,
    books: bool,
) -> axum::Router {
    use bigdecimal::BigDecimal;
    use cl8y_dex_indexer::{
        db::queries::{pair_reserves, pairs, resting_orders},
        lcd::LcdClient,
    };
    use std::{
        collections::{HashMap, HashSet},
        str::FromStr,
    };
    let rows = pairs::get_all_pairs(&pool).await.unwrap();
    for pair in &rows {
        let depth = if pair.contract_address == "terra1pairroutempac" {
            "10000000"
        } else {
            "10000000000000"
        };
        pair_reserves::upsert_pair_reserves(
            &pool,
            pair.id,
            &BigDecimal::from_str(depth).unwrap(),
            &BigDecimal::from_str(depth).unwrap(),
            30,
            Some(100),
        )
        .await
        .unwrap();
        let orders = if books && pair.contract_address != "terra1pairroutempac" {
            vec![resting_orders::RestingOrderInput {
                order_id: 1,
                side: "bid".into(),
                price: BigDecimal::from_str("1.01").unwrap(),
                remaining: BigDecimal::from_str("100000000000000").unwrap(),
                owner: Some("terra1syntheticmaker".into()),
                expires_at: None,
            }]
        } else {
            vec![]
        };
        resting_orders::replace_pair_resting_orders(&pool, pair.id, Some(100), &orders)
            .await
            .unwrap();
    }
    let tier_rows: Vec<(String, Option<i16>)> =
        sqlx::query_as("SELECT address,tier_id FROM traders")
            .fetch_all(&pool)
            .await
            .unwrap();
    let actors: HashMap<_, _> = tier_rows
        .into_iter()
        .map(|(address, tier)| (address, (if tier == Some(5) { 5000 } else { 0 }, false)))
        .collect();
    let discounted: HashSet<_> = rows.iter().map(|p| p.contract_address.clone()).collect();
    let lcd = LcdClient::new_with_policy(
        config.lcd_urls.clone(),
        config.lcd_timeout_ms,
        config.lcd_cooldown_ms,
        config.expected_chain_id.clone(),
        config.lcd_provider_policy.clone(),
    );
    let namespace = lcd.chain_cache_namespace();
    let assets = cl8y_dex_indexer::db::queries::assets::get_all_assets(&pool)
        .await
        .unwrap();
    let addresses: HashMap<_, _> = assets
        .iter()
        .filter_map(|a| {
            a.contract_address
                .as_ref()
                .or(a.denom.as_ref())
                .map(|address| (a.id, address.clone()))
        })
        .collect();
    let pair_addresses = rows
        .iter()
        .map(|p| p.contract_address.clone())
        .collect::<Vec<_>>();
    let mirrors = cl8y_dex_indexer::api::db_orderbook_sim::preload_mirrors_for_pairs(
        &pool,
        &pair_addresses,
        &addresses,
        60_000,
    )
    .await
    .unwrap();
    cl8y_dex_indexer::api::local_quote::initialize_for_test(
        namespace.clone(),
        rows,
        addresses,
        mirrors,
        60_000,
    );
    cl8y_dex_indexer::api::local_quote::initialize_fixture_contexts(&namespace, actors, discounted);
    cl8y_dex_indexer::api::build_router(common::local_quotes::state(pool, &config, lcd), &config)
}
fn amount(body: &Value) -> u128 {
    body["estimated_amount_out"]
        .as_str()
        .expect("positive useful output")
        .parse()
        .unwrap()
}
/// Independent arithmetic for these 1:1 reserves / 1.01 escrow-book fixtures.
fn assert_local_economics(body: &Value) {
    assert_eq!(body["solver_version"], "local_v1");
    assert_eq!(body["lcd_hybrid_queries"], 0);
    let metadata = &body["execution_metadata"];
    assert!(!metadata["generation"].as_str().unwrap().is_empty());
    let actor = metadata["trader"]
        .as_str()
        .or_else(|| metadata["sender"].as_str());
    let different = metadata["trader"].as_str().is_some()
        && metadata["sender"].as_str().is_some()
        && metadata["trader"] != metadata["sender"];
    let fee = if !different
        && matches!(
            actor,
            Some(TIER5_SENDER_A | TIER5_SENDER_B | "terra1discountwallet000000000000000000000")
        ) {
        15u128
    } else {
        30u128
    };
    let mut current: u128 = metadata["input_amount"].as_str().unwrap().parse().unwrap();
    for (index, hop) in metadata["hops"].as_array().unwrap().iter().enumerate() {
        assert_eq!(
            hop["offer_amount"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap(),
            current
        );
        let pool_input: u128 = hop["pool_input_actual"].as_str().unwrap().parse().unwrap();
        let consumed: u128 = hop["book_offer_consumed"]
            .as_str()
            .unwrap()
            .parse()
            .unwrap();
        assert_eq!(pool_input + consumed, current);
        let reserve = if hop["pair"] == "terra1pairroutempac" {
            10_000_000u128
        } else {
            10_000_000_000_000u128
        };
        let pool_gross = reserve * pool_input / (reserve + pool_input);
        let pool_net = pool_gross - pool_gross * fee / 10_000;
        let book_gross = consumed * 101 / 100;
        let book_net = book_gross - book_gross * (fee - fee / 2) / 10_000;
        assert_eq!(
            hop["pool_return_amount"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap(),
            pool_net
        );
        assert_eq!(
            hop["book_return_net"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap(),
            book_net
        );
        current = pool_net + book_net;
        assert_eq!(
            hop["return_amount"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap(),
            current
        );
        let floor: u128 = hop["min_return"].as_str().unwrap().parse().unwrap();
        assert!(floor > 0 && floor >= current * 95 / 100 && floor <= current);
        assert_eq!(
            body["router_operations"][index]["terra_swap"]["min_return"],
            hop["min_return"]
        );
    }
    assert_eq!(amount(body), current);
    assert_eq!(
        metadata["minimum_receive"],
        metadata["hops"].as_array().unwrap().last().unwrap()["min_return"]
    );
}

/// Retail GET Policy A (#1280): hop 0 may carry a declared split; hops 1+ are pool-only.
fn assert_retail_get_hybrid_hop0_only(ops: &[Value], amount_in: &str) {
    assert!(!ops.is_empty());
    let h0 = &ops[0]["terra_swap"]["hybrid"];
    if !h0.is_null() {
        let pool: u128 = h0["pool_input"]
            .as_str()
            .expect("pool_input")
            .parse()
            .expect("pool_input u128");
        let book: u128 = h0["book_input"]
            .as_str()
            .expect("book_input")
            .parse()
            .expect("book_input u128");
        let offer: u128 = amount_in.parse().expect("amount_in");
        assert_eq!(
            pool.checked_add(book),
            Some(offer),
            "hop 0 pool+book must equal amount_in"
        );
    }
    for (i, op) in ops.iter().enumerate().skip(1) {
        assert!(
            op["terra_swap"]["hybrid"].is_null(),
            "retail GET hop {i} must be pool-only (#1280 Policy A)"
        );
    }
}

#[serial]
#[tokio::test]
async fn route_solve_returns_hops_and_pool_only_hybrid() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve(&pool).await;
    let app = local_app(pool, common::test_config(), true).await;
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
    let app = local_app(pool, common::test_config(), true).await;
    let server = TestServer::new(app);

    let resp = server
        .get("/api/v1/route/solve?token_in=terra1notindexed&token_out=terra1routesolvebbb")
        .await;
    resp.assert_status_bad_request();
}

/// Missing cached wrapper observations cannot manufacture a native graph identity.
#[serial]
#[tokio::test]
async fn route_solve_native_uluna_returns_400() {
    let pool = common::setup_pool().await;
    common::seed_route_solve(&pool).await;
    let app = local_app(pool, common::test_config(), true).await;
    let server = TestServer::new(app);

    let resp = server
        .get("/api/v1/route/solve?token_in=uluna&token_out=terra1routesolvebbb")
        .await;
    resp.assert_status_bad_request();
}

#[serial]
#[tokio::test]
async fn route_solve_no_path_returns_404() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve(&pool).await;
    let app = local_app(pool, common::test_config(), true).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}",
        seed.token_a, seed.token_c
    );
    let resp = server.get(&url).await;
    resp.assert_status_unprocessable_entity();
    let text = resp.text();
    assert!(text.contains("No eligible route"), "{}", text);
}

#[serial]
#[tokio::test]
async fn route_solve_post_hybrid_length_mismatch_returns_400() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve(&pool).await;
    let app = local_app(pool, common::test_config(), true).await;
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
    let app = local_app(pool, cfg, true).await;
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
    assert_local_economics(&j);
    assert_eq!(
        j["intermediate_tokens"],
        json!([seed.token_a, seed.token_b])
    );
    assert_eq!(j["quote_kind"], "indexer_hybrid_db");
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
    let app = local_app(pool, cfg, true).await;
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
    assert_eq!(j["quote_kind"], "indexer_hybrid_db");
    assert_local_economics(&j);
    assert!(!j["execution_metadata"]["generation"]
        .as_str()
        .unwrap()
        .is_empty());
    let ops = j["router_operations"].as_array().unwrap();
    assert!(!ops[0]["terra_swap"]["hybrid"].is_null());
    assert_retail_get_hybrid_hop0_only(ops, "1000000");
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
    let app = local_app(pool, cfg, true).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1000000&pool_only=true",
        seed.token_a, seed.token_b
    );
    let resp = server.get(&url).await;
    resp.assert_status_ok();
    let j: Value = resp.json();
    assert!(j["router_operations"][0]["terra_swap"]["hybrid"].is_null());
    assert_local_economics(&j);
    assert_eq!(j["quote_kind"], "indexer_pool_db");
    assert!(j["hybrid_notes"].is_null());
}

#[serial]
#[tokio::test]
async fn route_solve_slippage_percent_enrichment_293() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_slippage_293(&pool).await;
    let mock = lcd_mock::start_hybrid_route_optimizer_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![lcd_mock::lcd_base_url(&mock)];
    cfg.router_address = Some("terra1routertest".to_string());
    let app = local_app(pool, cfg, true).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1000000&pool_only=true",
        seed.token_a, seed.token_b
    );
    let resp = server.get(&url).await;
    resp.assert_status_ok();
    let j: Value = resp.json();

    let est = j["estimated_amount_out"]
        .as_str()
        .expect("estimated_amount_out")
        .parse::<u128>()
        .expect("parse estimated_amount_out");
    let spot = j["spot_amount_out"]
        .as_str()
        .expect("spot_amount_out")
        .parse::<u128>()
        .expect("parse spot_amount_out");
    let slip: f64 = j["slippage_percent"]
        .as_str()
        .expect("slippage_percent")
        .parse()
        .expect("parse slippage_percent");

    assert!(est > 0 && spot > 0, "expected non-zero quote outputs");
    assert_eq!(spot, 1_000_000, "independent same-unit fixture reference");
    assert_local_economics(&j);
    let lo = est.min(spot) as f64;
    let hi = est.max(spot) as f64;
    let recomputed = (1.0 - lo / hi) * 100.0;
    assert!(
        (recomputed - slip).abs() < 1.0,
        "symmetric slippage: reported={slip} recomputed={recomputed}"
    );
}

#[serial]
#[tokio::test]
async fn route_solve_hybrid_optimize_requires_amount_in() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve(&pool).await;
    let app = local_app(pool, common::test_config(), true).await;
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
    let app = local_app(pool, cfg, true).await;
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
    assert_eq!(j["quote_kind"], "indexer_hybrid_db");
    assert_local_economics(&j);
    assert!(!j["execution_metadata"]["generation"]
        .as_str()
        .unwrap()
        .is_empty());
    let ops = j["router_operations"].as_array().unwrap();
    assert!(!ops[0]["terra_swap"]["hybrid"].is_null());
    assert_retail_get_hybrid_hop0_only(ops, "1000000");
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
    let app = local_app(pool, cfg, true).await;
    let server = TestServer::new(app);

    let first: Value = server.post("/api/v1/route/solve").json(&json!({"token_in":seed.token_a,"token_out":seed.token_b,"amount_in":"1000000","hybrid_by_hop":[{"pool_input":"250000","book_input":"750000","max_maker_fills":8}]})).await.json();
    let second_offer = amount(&first);
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
                "pool_input": (second_offer - 600000).to_string(),
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
    assert_local_economics(&j);
    assert_eq!(
        j["intermediate_tokens"],
        json!([seed.token_a, seed.token_b, seed.token_c, token_d])
    );
    assert_eq!(j["quote_kind"], "indexer_hybrid_db");
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
    let app = local_app(pool, cfg, true).await;
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
    assert_eq!(j["quote_kind"], "indexer_hybrid_db");
    assert_local_economics(&j);
    let ops = j["router_operations"].as_array().unwrap();
    assert_eq!(ops.len(), 3);
    assert!(!ops[0]["terra_swap"]["hybrid"].is_null());
    assert_retail_get_hybrid_hop0_only(ops, "1000000");
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
    let app = local_app(pool, cfg, true).await;
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
    assert_eq!(j["quote_kind"], "indexer_hybrid_db");
    assert_local_economics(&j);
    let ops = j["router_operations"].as_array().unwrap();
    assert_eq!(ops.len(), 4);
    assert!(!ops[0]["terra_swap"]["hybrid"].is_null());
    assert_retail_get_hybrid_hop0_only(ops, "1000000");
}

#[serial]
#[tokio::test]
async fn route_solve_best_requires_amount_in() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve(&pool).await;
    let app = local_app(pool, common::test_config(), true).await;
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
    let app = local_app(pool, cfg, true).await;
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

    assert_eq!(best["quote_kind"], "indexer_hybrid_db");
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
    let app = local_app(pool, cfg, false).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve/best?token_in={}&token_out={}&amount_in={}",
        seed.token_a, seed.token_b, "1000000"
    );
    let resp = server.get(&url).await;
    resp.assert_status_ok();
    let j: Value = resp.json();
    assert_eq!(j["quote_kind"], "indexer_pool_db");
    assert!(j["router_operations"][0]["terra_swap"]["hybrid"].is_null());
    assert_local_economics(&j);
    assert!(mock.received_requests().await.unwrap().is_empty());
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
    let app = local_app(pool, cfg, true).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1000000",
        seed.token_a, seed.token_c
    );
    let resp = server.get(&url).await;
    resp.assert_status_ok();
    let j: Value = resp.json();
    assert_eq!(j["solver_version"], "local_v1");
    assert_eq!(j["paths_considered"], 2);
    assert_eq!(
        j["hops"].as_array().unwrap().len(),
        2,
        "2-hop path beats 1-hop"
    );
    assert_local_economics(&j);
    assert!(
        j["optimality_scope"]
            .as_str()
            .unwrap_or("")
            .contains("bounded"),
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
    let app = local_app(pool, cfg, true).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve/best?token_in={}&token_out={}&amount_in=1000000",
        seed.token_a, seed.token_c
    );
    let j: Value = server.get(&url).await.json();
    assert_eq!(j["solver_version"], "local_v1");
    assert!(j["paths_considered"].as_u64().unwrap_or(0) >= 1);
    assert!(!j["execution_metadata"]["generation"]
        .as_str()
        .unwrap_or("")
        .is_empty());
    assert_eq!(j["lcd_hybrid_queries"], 0);
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[serial]
#[tokio::test]
async fn route_solve_get_with_trader_returns_higher_estimate() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_2hop(&pool).await;
    let trader = "terra1discountwallet000000000000000000000";
    // Cached actor fee context is prepared explicitly before requests.
    common::seed_traders_with_tiers(&pool, &[(trader, 5)]).await;
    let mock = lcd_mock::start_hybrid_route_optimizer_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![lcd_mock::lcd_base_url(&mock)];
    cfg.router_address = Some("terra1routertest".to_string());
    let app = local_app(pool, cfg, true).await;
    let server = TestServer::new(app);

    let base_url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1000000",
        seed.token_a, seed.token_c
    );
    let base: Value = server.get(&base_url).await.json();
    let discounted_url = format!("{base_url}&trader={trader}");
    let discounted: Value = server.get(&discounted_url).await.json();

    assert_local_economics(&base);
    assert_local_economics(&discounted);
    assert!(amount(&discounted) > amount(&base));
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[serial]
#[tokio::test]
async fn route_solve_invalid_trader_returns_400() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve(&pool).await;
    let app = local_app(pool, common::test_config(), true).await;
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
    common::seed_traders_with_tiers(&pool, &[("terra1discountwallet000000000000000000000", 5)])
        .await;
    let mock = lcd_mock::start_hybrid_route_optimizer_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![lcd_mock::lcd_base_url(&mock)];
    cfg.router_address = Some("terra1routertest".to_string());
    let app = local_app(pool, cfg, true).await;
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
    assert_local_economics(&j);
}

const TIER0_SENDER: &str = "terra1tier0sender000000000000000000000000";
const TIER5_SENDER_A: &str = "terra1tier5sendera00000000000000000000000";
const TIER5_SENDER_B: &str = "terra1tier5senderb00000000000000000000000";
const UNKNOWN_SENDER: &str = "terra1unknownsender0000000000000000000000";

#[serial]
#[tokio::test]
async fn route_solve_get_cache_tier_isolation() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_2hop(&pool).await;
    common::seed_traders_with_tiers(&pool, &[(TIER0_SENDER, 0), (TIER5_SENDER_A, 5)]).await;

    let mock = lcd_mock::start_tier_aware_route_optimizer_mock(&[TIER5_SENDER_A]).await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![lcd_mock::lcd_base_url(&mock)];
    cfg.router_address = Some("terra1routertest".to_string());
    let app = local_app(pool, cfg, true).await;
    let server = TestServer::new(app);

    let base_url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1000000",
        seed.token_a, seed.token_c
    );

    let tier0: Value = server
        .get(&format!("{base_url}&sender={TIER0_SENDER}"))
        .await
        .json();
    assert_local_economics(&tier0);

    let tier5: Value = server
        .get(&format!("{base_url}&sender={TIER5_SENDER_A}"))
        .await
        .json();
    assert_local_economics(&tier5);
    assert_ne!(
        tier5["estimated_amount_out"], tier0["estimated_amount_out"],
        "tier-5 sender must not receive tier-0 cached quote"
    );

    let unknown: Value = server
        .get(&format!("{base_url}&sender={UNKNOWN_SENDER}"))
        .await
        .json();
    assert_eq!(
        unknown["estimated_amount_out"], tier0["estimated_amount_out"],
        "unknown sender resolves to tier 0"
    );
}

#[serial]
#[tokio::test]
async fn route_solve_get_cache_same_tier_reuses_lcd() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_2hop(&pool).await;
    common::seed_traders_with_tiers(&pool, &[(TIER5_SENDER_A, 5), (TIER5_SENDER_B, 5)]).await;

    let mock =
        lcd_mock::start_tier_aware_route_optimizer_mock(&[TIER5_SENDER_A, TIER5_SENDER_B]).await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![lcd_mock::lcd_base_url(&mock)];
    cfg.router_address = Some("terra1routertest".to_string());
    let app = local_app(pool, cfg, true).await;
    let server = TestServer::new(app);

    let base_url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1000000",
        seed.token_a, seed.token_c
    );

    let first_response = server
        .get(&format!("{base_url}&sender={TIER5_SENDER_A}"))
        .await;
    first_response.assert_status_ok();
    let tier5_a: Value = first_response.json();
    assert_local_economics(&tier5_a);
    let requests_after_first = mock
        .received_requests()
        .await
        .expect("mock server should expose request log");
    assert!(
        requests_after_first.is_empty(),
        "initial useful quote performs zero provider calls"
    );
    let second_response = server
        .get(&format!("{base_url}&sender={TIER5_SENDER_B}"))
        .await;
    second_response.assert_status_ok();
    let tier5_b: Value = second_response.json();
    assert_local_economics(&tier5_b);
    assert_eq!(tier5_b["router_operations"], tier5_a["router_operations"]);
    assert_eq!(tier5_b["hops"], tier5_a["hops"]);

    let requests_after_second = mock.received_requests().await.expect("request log");
    assert!(
        requests_after_second.is_empty(),
        "same-tier and distinct actor cache identity perform zero provider calls"
    );
    assert_eq!(tier5_a["execution_metadata"]["sender"], TIER5_SENDER_A);
    assert_eq!(tier5_b["execution_metadata"]["sender"], TIER5_SENDER_B);
    let counters = cl8y_dex_indexer::api::local_quote::cache_counters();
    let repeated: Value = server
        .get(&format!("{base_url}&sender={TIER5_SENDER_B}"))
        .await
        .json();
    assert_eq!(repeated["router_operations"], tier5_b["router_operations"]);
    assert_eq!(
        repeated["execution_metadata"],
        tier5_b["execution_metadata"]
    );
    assert_eq!(
        cl8y_dex_indexer::api::local_quote::cache_counters().0,
        counters.0 + 1,
        "identical actual actor request hits its exact local cache key"
    );
}

#[serial]
#[tokio::test]
async fn route_solve_invalid_sender_returns_400() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve(&pool).await;
    let app = local_app(pool, common::test_config(), true).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1000000&sender=not-a-wallet",
        seed.token_a, seed.token_b
    );
    server.get(&url).await.assert_status_bad_request();
}

#[serial]
#[tokio::test]
async fn route_solve_progress_idle_then_terminal_after_solve() {
    // GitLab #485: progress poll is advisory; idle before solve, terminal after.
    // Unique amount bucket keeps prior route tests from warming this advisory cache.
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve_2hop(&pool).await;
    let mock = lcd_mock::start_hybrid_route_optimizer_mock().await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![lcd_mock::lcd_base_url(&mock)];
    cfg.router_address = Some("terra1routertest".to_string());
    let app = local_app(pool, cfg, true).await;
    let server = TestServer::new(app);

    let progress_url = format!(
        "/api/v1/route/solve/progress?token_in={}&token_out={}&amount_in=13760000001",
        seed.token_a, seed.token_c
    );
    let idle: Value = server.get(&progress_url).await.json();
    assert_eq!(idle["stage"], "idle");
    assert!(idle["estimated_amount_out"].is_null());

    let solve_url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in=13760000001",
        seed.token_a, seed.token_c
    );
    let useful = server.get(&solve_url).await;
    useful.assert_status_ok();
    assert_local_economics(&useful.json());
    let mut after: Value = server.get(&progress_url).await.json();
    for _ in 0..100 {
        if after["stage"] != "evaluating" {
            break;
        }
        assert_eq!(after["done"], 0);
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        after = server.get(&progress_url).await.json();
    }
    assert!(mock.received_requests().await.unwrap().is_empty());
    let stage = after["stage"].as_str().unwrap_or("");
    assert!(
        stage == "done" || stage == "cached",
        "expected terminal stage after solve, got {stage}"
    );
    assert!(after.get("label").and_then(|v| v.as_str()).is_some());
    assert!(after["estimated_amount_out"].is_null());
}

#[serial]
#[tokio::test]
async fn route_solve_progress_requires_amount_in() {
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve(&pool).await;
    let app = local_app(pool, common::test_config(), true).await;
    let server = TestServer::new(app);

    let url = format!(
        "/api/v1/route/solve/progress?token_in={}&token_out={}",
        seed.token_a, seed.token_b
    );
    server.get(&url).await.assert_status_bad_request();
}

#[serial]
#[tokio::test]
async fn route_solve_progress_trader_reuses_discount_bps_cache() {
    // GitLab #694 / RE-02: two progress polls with trader set → at most one GetDiscount.
    cl8y_dex_indexer::api::reset_discount_bps_cache();
    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve(&pool).await;
    let mock = lcd_mock::start_smart_query_data_mock(json!({ "discount_bps": 5000 })).await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![lcd_mock::lcd_base_url(&mock)];
    cfg.fee_discount_address = Some("terra1feediscount69400000000000000000000".to_string());
    let app = local_app(pool, cfg, true).await;
    let server = TestServer::new(app);

    let trader = "terra1discountwallet000000000000000000000";
    let url = format!(
        "/api/v1/route/solve/progress?token_in={}&token_out={}&amount_in=1000000&trader={trader}",
        seed.token_a, seed.token_b
    );
    server.get(&url).await.assert_status_ok();
    server.get(&url).await.assert_status_ok();
    server.get(&url).await.assert_status_ok();

    let reqs = mock.received_requests().await.expect("request log");
    let get_discount = reqs
        .iter()
        .filter(|r| {
            lcd_mock::smart_query_from_request(r)
                .get("get_discount")
                .is_some()
        })
        .count();
    assert_eq!(
        get_discount, 0,
        "progress polls only read initialized context (got {get_discount} provider calls)"
    );
}

#[serial]
#[tokio::test]
async fn route_solve_excludes_code_id_frozen_pair() {
    use cl8y_dex_indexer::indexer::asset_code_id_freeze::{
        replace_frozen_pair_addresses, snapshot_frozen_pair_addresses,
    };
    use std::collections::HashSet;

    let pool = common::setup_pool().await;
    let seed = common::seed_route_solve(&pool).await;
    struct RestoreFrozenPairs(HashSet<String>);
    impl Drop for RestoreFrozenPairs {
        fn drop(&mut self) {
            replace_frozen_pair_addresses(std::mem::take(&mut self.0));
        }
    }
    let _restore = RestoreFrozenPairs(snapshot_frozen_pair_addresses());
    replace_frozen_pair_addresses(HashSet::from(["terra1pairrouteabc".to_string()]));

    let app = local_app(pool, common::test_config(), true).await;
    let server = TestServer::new(app);
    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}",
        seed.token_a, seed.token_b
    );
    let resp = server.get(&url).await;
    resp.assert_status_unprocessable_entity();
}
