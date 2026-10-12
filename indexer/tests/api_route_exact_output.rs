//! #1376 bounded local net receive sizing: independently initialized observations and zero request RPC.
mod common;

use axum_test::TestServer;
use bigdecimal::{num_bigint::BigUint, ToPrimitive};
use common::lcd_mock;
use serde_json::{json, Value};
use serial_test::serial;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

/// The provider is enabled only for independently scheduled admission. Quotes,
/// inverse sizing, floors and cache reuse operate on the resulting observations.
async fn forward_mock(
    pools: Vec<(String, String, u128, u128, u16)>,
    discounted: bool,
) -> MockServer {
    forward_tax_mock(pools, discounted, Default::default(), false).await
}
async fn forward_tax_mock(
    pools: Vec<(String, String, u128, u128, u16)>,
    discounted: bool,
    taxes: std::collections::HashMap<String, (u16, u16)>,
    exempt: bool,
) -> MockServer {
    let mock = lcd_mock::start_ordinary_code_mock().await;
    let registered = taxes.clone();
    Mock::given(method("GET")).and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+$"))
        .respond_with(move |r:&Request| {
            let addr=r.url.path().rsplit('/').next().unwrap();
            ResponseTemplate::new(200).set_body_json(json!({"contract_info":{"code_id":if registered.contains_key(addr){"11667"}else{"10184"},"admin":""}}))
        }).with_priority(1).mount(&mock).await;
    Mock::given(method("GET")).and(path_regex(r"/cosmwasm/wasm/v1/contract/.*/smart/.*"))
        .respond_with(move |request:&Request| {
            let q=lcd_mock::smart_query_from_request(request);
            let addr=request.url.path().split("/contract/").nth(1).unwrap().split('/').next().unwrap();
            let (buy,sell)=taxes.get(addr).copied().unwrap_or((0,0));
            let data=if q.get("config").is_some() {
                json!({"funding_version":1,"factory":"terra1factory"})
            } else if q.get("route_tax_capability").is_some() {
                json!({"funding_version":1,"router":"terra1routertest","buy_bps":buy,"sell_bps":sell})
            } else if q.get("get_config").is_some() {
                json!({"factory":"terra1factory","router":"terra1routertest","buy_bps":buy,"sell_bps":sell,"treasury":"terra1treasury","sinks":[],"autolp":null,"launch_guards":null})
            } else if q.get("get_features").is_some() {
                json!({"launch_guards":false,"auto_v2_lp":false,"split_router":false,"exemption_directory":true})
            } else if q.get("route_tax_preview").is_some() {
                json!({"funding_version":1,"gross":"0","economic":"0","net":"0","tax":"0"})
            } else if let Some(flags)=q.get("is_protocol_exempt") {
                json!({"address":flags["address"],"manager":exempt,"protocol":false})
            } else if q.get("all_accounts").is_some() { json!({"accounts":[]})
            } else if q.get("balance").is_some() { json!({"balance":"0"})
            } else if q.get("is_paused").is_some() { json!({"paused":false})
            } else if q.get("get_hooks").is_some() { json!({"hooks":[]})
            } else if q.get("get_discount_registry").is_some() { json!({"registry":"terra1fees"})
            } else if q.get("is_code_id_whitelisted").is_some() { json!({"whitelisted":true})
            } else if q.get("get_asset_code_ids").is_some() {
                let suffix=if addr.contains("mpac"){("aaa","ccc")}else if addr.contains("mpbc"){("bbb","ccc")}else{("aaa","bbb")};
                let code=|suffix:&str| {
                    let token=pools.iter().flat_map(|p|[&p.0,&p.1]).find(|a|a.contains(suffix));
                    if token.is_some_and(|a|taxes.contains_key(a)){11667}else{10184}
                };
                json!({"code_ids":[code(suffix.0),code(suffix.1)]})
            } else if q.get("blacklist_check").is_some() {
                json!({"wallet_blacklisted":false,"pair_blacklisted":false,"blacklisted_tokens":[],"blacklisted_pairs":[],"blocked":false})
            } else if q.get("get_discount").is_some() {
                json!({"discount_bps":if discounted {5000}else{0}})
            } else { return ResponseTemplate::new(500); };
            ResponseTemplate::new(200).set_body_json(json!({"data":data}))
        }).mount(&mock).await;
    mock
}
async fn observed_app(
    pool: sqlx::PgPool,
    cfg: cl8y_dex_indexer::config::Config,
    mock: &MockServer,
) -> axum::Router {
    observed_app_with_references(pool, cfg, mock, true, None).await
}
async fn observed_app_with_references(
    pool: sqlx::PgPool,
    cfg: cl8y_dex_indexer::config::Config,
    mock: &MockServer,
    with_references: bool,
    expected_actor: Option<(&str, &str, &str)>,
) -> axum::Router {
    // Independent address-bound fixture references: unit prices for unit corridors.
    // Mixed 6/18 example assigns a known economic crossrate below in its fixture.
    if with_references {
        common::reference_marks::seed_reference_marks(
            &pool,
            &cl8y_dex_indexer::indexer::hub_usd::HubUsdConfig::from_indexer_config(&cfg),
            &[],
        )
        .await;
    }
    for actor in [
        "terra1000000000000000000000000000000000001",
        "terra1000000000000000000000000000000000002",
        "terra1synthetictrader1390xxxxxxxxxxxxxxxxx",
    ] {
        sqlx::query("INSERT INTO traders(address,tier_id,registered) VALUES($1,255,false) ON CONFLICT(address) DO NOTHING").bind(actor).execute(&pool).await.unwrap();
    }
    // Cold scheduled admission is not an initialized quote latency sample.
    // Keep production-shaped bounded I/O while preserving each admission deadline.
    let policy = cl8y_dex_indexer::lcd::ProviderPolicy {
        attempt_timeout_ms: 5_000,
        deadline_ms: 5_000,
        ..cfg.lcd_provider_policy.clone()
    };
    let lcd = cl8y_dex_indexer::lcd::LcdClient::new_with_policy(
        cfg.lcd_urls.clone(),
        5_000,
        1,
        cfg.expected_chain_id.clone(),
        policy,
    );
    let state = common::local_quotes::state(pool, &cfg, lcd);
    cl8y_dex_indexer::api::local_quote::ingest_fixture(&state)
        .await
        .unwrap();
    let provider_counters = state.lcd.provider_counters().await;
    assert!(provider_counters.iter().all(|counter| counter.rate_deferrals == 0
        && counter.concurrency_deferrals == 0 && counter.circuit_deferrals == 0 && !counter.circuit_open),
        "synthetic scheduled admission must finish without provider deferral/circuit poisoning: {provider_counters:?}");
    if let Some((input, output, actor)) = expected_actor {
        // A different receive target keeps the measured/API target cold while
        // proving the required actor observation before providers are disabled.
        let probe = serde_json::from_value(json!({"token_in":input,"token_out":output,
            "amount_out":"1000001","max_spread":"0.05","sender":actor,"pool_only":true}))
        .unwrap();
        let body = cl8y_dex_indexer::api::local_quote::solve(&state, &probe)
            .await
            .expect("cold fixture must initialize the manager actor before provider isolation")
            .0;
        assert_eq!(body["tax_kind"], "route_local_v1");
        assert_eq!(body["buy_tax_bps"], 0);
        assert_eq!(body["execution_metadata"]["sender"], actor);
    }
    mock.reset().await;
    cl8y_dex_indexer::api::build_router(state, &cfg)
}

async fn seed(pool: &sqlx::PgPool, multiple: bool) -> common::RouteSolveSeed {
    let mut seeded = if multiple {
        common::seed_route_solve_zero_reserve_poison(pool).await
    } else {
        common::seed_route_solve_with_mirror(pool).await
    };
    for token in [
        &mut seeded.token_a,
        &mut seeded.token_b,
        &mut seeded.token_c,
    ] {
        let valid = format!("{}0000000000000000000000", token);
        sqlx::query("UPDATE assets SET contract_address=$1 WHERE contract_address=$2")
            .bind(&valid)
            .bind(&*token)
            .execute(pool)
            .await
            .unwrap();
        *token = valid;
    }
    seeded
}
fn delivered(input: u128, rin: u128, rout: u128, fee: u16, sell: u16, buy: u16) -> u128 {
    let economic = ((BigUint::from(input) * BigUint::from(10000u32) + BigUint::from(9999u32))
        / BigUint::from(10000u32 + u32::from(sell)))
    .to_u128()
    .unwrap();
    let gross = (BigUint::from(economic) * BigUint::from(rout) / BigUint::from(rin + economic))
        .to_u128()
        .unwrap();
    let after_fee = gross
        - (BigUint::from(gross) * BigUint::from(fee) / BigUint::from(10000u32))
            .to_u128()
            .unwrap();
    after_fee
        - (BigUint::from(after_fee) * BigUint::from(buy) / BigUint::from(10000u32))
            .to_u128()
            .unwrap()
}
fn config(mock: &MockServer) -> cl8y_dex_indexer::config::Config {
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![lcd_mock::lcd_base_url(mock)];
    cfg.router_address = Some("terra1routertest".into());
    cfg.fee_discount_address = Some("terra1fees".into());
    cfg.trading_route_tax_code_ids.insert(11667);
    cfg
}

async fn reserves(pool: &sqlx::PgPool, pair: &str, r0: u128, r1: u128, fee: u16) {
    let id: i32 = sqlx::query_scalar("SELECT id FROM pairs WHERE contract_address=$1")
        .bind(pair)
        .fetch_one(pool)
        .await
        .unwrap();
    cl8y_dex_indexer::db::queries::pair_reserves::upsert_pair_reserves(
        pool,
        id,
        &r0.to_string().parse().unwrap(),
        &r1.to_string().parse().unwrap(),
        fee as i16,
        Some(100),
    )
    .await
    .unwrap();
}

#[serial]
#[tokio::test]
async fn route_exact_output_real_10_100_cold_repeat_and_exact_target_identity() {
    let pool = common::setup_pool().await;
    let seed = seed(&pool, false).await;
    let (r0, r1) = (297665983808u128, 2498115414904458263864u128);
    reserves(&pool, "terra1pairrouteabc", r0, r1, 180).await;
    sqlx::query("UPDATE assets SET decimals=18 WHERE contract_address=$1")
        .bind(&seed.token_b)
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("INSERT INTO economic_token_marks(contract_address,price_usd,as_of,anchor_ticker) VALUES($1,0.0084,NOW(),'custc')")
        .bind(&seed.token_a).execute(&pool).await.unwrap();
    let mock = forward_mock(
        vec![(seed.token_a.clone(), seed.token_b.clone(), r0, r1, 180)],
        false,
    )
    .await;
    let server = TestServer::new(observed_app(pool, config(&mock), &mock).await);
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
        let required = expected.parse::<u128>().unwrap();
        let target_raw = target.parse::<u128>().unwrap();
        assert!(delivered(required, r0, r1, 180, 0, 0) >= target_raw);
        assert!(delivered(required - 1, r0, r1, 180, 0, 0) < target_raw);

        assert_eq!(body["target_amount_out"], target);
        assert_eq!(body["sizing_mode"], "local_pool_exact_output");
        assert_eq!(body["solver_version"], "local_v1");
        assert_eq!(body["lcd_hybrid_queries"], 0);
        assert!(
            body["spot_amount_out"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
                > 0
        );
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
        0,
        "repeat targets use exact local execution caches without provider validation"
    );
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[serial]
#[tokio::test]
async fn route_exact_output_competing_path_beats_direct_and_unusable_first_path() {
    for direct_reserve in [5000000000000u128, 0] {
        let pool = common::setup_pool().await;
        let seed = seed(&pool, true).await;
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
        let server = TestServer::new(observed_app(pool, config(&mock), &mock).await);
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
        assert!(body["optimality_scope"].as_str().unwrap().contains("local"));
        assert!(mock.received_requests().await.unwrap().is_empty());
    }
}

#[serial]
#[tokio::test]
async fn route_exact_output_fee_tier_and_sender_match_forward() {
    let pool = common::setup_pool().await;
    let seed = seed(&pool, false).await;
    let deep = 10000000000000u128;
    let mock = forward_mock(
        vec![(seed.token_a.clone(), seed.token_b.clone(), deep, deep, 30)],
        true,
    )
    .await;
    let server = TestServer::new(observed_app(pool, config(&mock), &mock).await);
    let resp=server.get(&format!("/api/v1/route/solve?token_in={}&token_out={}&amount_out=1000000&trader=terra1000000000000000000000000000000000001&sender=terra1000000000000000000000000000000000002",seed.token_a,seed.token_b)).await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(
        body["required_amount_in"], "1003010",
        "different fee identities conservatively retain the full fee"
    );
    let same=server.get(&format!("/api/v1/route/solve?token_in={}&token_out={}&amount_out=1000000&trader=terra1000000000000000000000000000000000001&sender=terra1000000000000000000000000000000000001",seed.token_a,seed.token_b)).await;
    same.assert_status_ok();
    let same_body: Value = same.json();
    assert!(
        same_body["required_amount_in"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            < 1_003_010
    );
    assert_eq!(
        same_body["execution_metadata"]["trader"],
        same_body["execution_metadata"]["sender"]
    );
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[serial]
#[tokio::test]
async fn route_exact_output_validation_fails_closed_and_rejects_modes() {
    let pool = common::setup_pool().await;
    let seed = seed(&pool, false).await;
    let mock = lcd_mock::start_ordinary_code_mock().await;
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
    let server = TestServer::new(observed_app(pool, config(&mock), &mock).await);
    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_out=1000000",
        seed.token_a, seed.token_b
    );
    // Successful malformed required observations are unsupported semantics,
    // rather than temporary data loss or amount/protection rejection.
    let malformed = server.get(&url).await;
    malformed.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(malformed.json::<Value>()["error"], "unsupported_semantics");
    assert!(!malformed.text().contains("required_amount_in"));
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
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[serial]
#[tokio::test]
async fn route_exact_output_stale_empty_and_infeasible_are_finite() {
    let pool = common::setup_pool().await;
    let seed = seed(&pool, false).await;
    sqlx::query("WITH stale AS (UPDATE pair_reserves SET snapshot_at=NOW()-INTERVAL '1 hour' RETURNING pair_id,snapshot_at) UPDATE hybrid_pair_reserves SET snapshot_at=stale.snapshot_at FROM stale WHERE hybrid_pair_reserves.pair_id=stale.pair_id")
        .execute(&pool)
        .await
        .unwrap();
    let mock = forward_mock(
        vec![(
            seed.token_a.clone(),
            seed.token_b.clone(),
            10_000_000_000_000,
            10_000_000_000_000,
            30,
        )],
        false,
    )
    .await;
    let server = TestServer::new(observed_app(pool, config(&mock), &mock).await);
    let resp = server
        .get(&format!(
            "/api/v1/route/solve?token_in={}&token_out={}&amount_out=1000000",
            seed.token_a, seed.token_b
        ))
        .await;
    resp.assert_status(axum::http::StatusCode::SERVICE_UNAVAILABLE);
    // The initial expired database tuple was never admitted as an executable
    // observation. Missing usable cached data is distinct from expiration of
    // an original admitted view (covered by the generation/TTL socket suite).
    assert_eq!(resp.json::<Value>()["error"], "missing_observations");
    assert!(mock
        .received_requests()
        .await
        .unwrap()
        .iter()
        .all(|request| lcd_mock::smart_query_from_request(request)
            .get("simulate_swap_operations")
            .is_none()));
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[serial]
#[tokio::test]
async fn route_exact_output_route_local_buy_sell_gross_budget_not_double_counted() {
    let pool = common::setup_pool().await;
    let seed = seed(&pool, false).await;
    for (addr, buy, sell) in [(&seed.token_a, 0i16, 200i16), (&seed.token_b, 1000, 0)] {
        sqlx::query("INSERT INTO community_tokens (contract_address,code_id,buy_bps,sell_bps) VALUES ($1,11611,$2,$3)")
            .bind(addr).bind(buy).bind(sell).execute(&pool).await.unwrap();
    }
    let deep = 10000000000000u128;
    let mock = forward_tax_mock(
        vec![(seed.token_a.clone(), seed.token_b.clone(), deep, deep, 30)],
        false,
        [
            (seed.token_a.clone(), (0, 200)),
            (seed.token_b.clone(), (1000, 0)),
        ]
        .into_iter()
        .collect(),
        false,
    )
    .await;
    let server = TestServer::new(observed_app(pool.clone(), config(&mock), &mock).await);
    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_out=1000000",
        seed.token_a, seed.token_b
    );
    let resp = server.get(&url).await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(body["buy_tax_bps"], 1000);
    let required = body["required_amount_in"]
        .as_str()
        .unwrap()
        .parse::<u128>()
        .unwrap();
    assert!(delivered(required, deep, deep, 30, 200, 1000) >= 1_000_000);
    assert!(delivered(required - 1, deep, deep, 30, 200, 1000) < 1_000_000);

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
            < 1150000,
        "authorized gross input includes locally funded sell tax"
    );
    sqlx::query("UPDATE community_tokens SET buy_bps=10000 WHERE contract_address=$1")
        .bind(&seed.token_b)
        .execute(&pool)
        .await
        .unwrap();
    // Catalog cache edits cannot override trusted live route capability rates.
    let unchanged = server.get(&url).await;
    unchanged.assert_status_ok();
    let refreshed: Value = unchanged.json();
    assert_eq!(refreshed["buy_tax_bps"], 1000);
    assert_eq!(refreshed["required_amount_in"], body["required_amount_in"]);
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[serial]
#[tokio::test]
async fn route_exact_output_bad_intermediary_metadata_does_not_hide_direct_alternate() {
    let pool = common::setup_pool().await;
    let seed = seed(&pool, true).await;
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
    let server = TestServer::new(observed_app(pool, config(&mock), &mock).await);
    let resp = server
        .get(&format!(
            "/api/v1/route/solve?token_in={}&token_out={}&amount_out=1000000",
            seed.token_a, seed.token_c
        ))
        .await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(body["hops"].as_array().unwrap().len(), 1);
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[serial]
#[tokio::test]
async fn route_exact_output_paused_and_blacklisted_paths_cannot_quote() {
    for paused in [true, false] {
        let pool = common::setup_pool().await;
        let seed = seed(&pool, false).await;
        let deep = 10_000_000_000_000;
        let mock = forward_mock(
            vec![(seed.token_a.clone(), seed.token_b.clone(), deep, deep, 30)],
            false,
        )
        .await;
        Mock::given(method("GET")).and(path_regex(r"/cosmwasm/wasm/v1/contract/.*/smart/.*"))
            .and(|request:&Request| {
                let q=lcd_mock::smart_query_from_request(request);
                q.get("is_paused").is_some()||q.get("blacklist_check").is_some()
            }).respond_with(move |request:&Request| {
                let q=lcd_mock::smart_query_from_request(request);
                let data=if q.get("is_paused").is_some(){json!({"paused":paused})}
                else {json!({"wallet_blacklisted":false,"pair_blacklisted":!paused,"blocked":!paused,"blacklisted_tokens":[],"blacklisted_pairs":if paused {json!([])}else{json!(["terra1pairrouteabc"])}})};
                ResponseTemplate::new(200).set_body_json(json!({"data":data}))
            }).with_priority(1).mount(&mock).await;
        let server = TestServer::new(observed_app(pool, config(&mock), &mock).await);
        let resp = server
            .get(&format!(
                "/api/v1/route/solve?token_in={}&token_out={}&amount_out=1000000",
                seed.token_a, seed.token_b
            ))
            .await;
        resp.assert_status_unprocessable_entity();
        assert!(!resp.text().contains("required_amount_in"));
        assert!(mock.received_requests().await.unwrap().is_empty());
    }
}

#[serial]
#[tokio::test]
async fn route_exact_output_sender_only_exemption_matches_execution_context() {
    let pool = common::setup_pool().await;
    let seed = seed(&pool, false).await;
    sqlx::query("INSERT INTO community_tokens (contract_address,code_id,buy_bps,sell_bps,features) VALUES ($1,11611,1000,0,'{\"exemption_directory\":true}'::jsonb)")
        .bind(&seed.token_b).execute(&pool).await.unwrap();
    let deep = 10000000000000u128;
    let mock = forward_tax_mock(
        vec![(seed.token_a.clone(), seed.token_b.clone(), deep, deep, 30)],
        false,
        [(seed.token_b.clone(), (1000, 0))].into_iter().collect(),
        true,
    )
    .await;
    let server = TestServer::new(
        observed_app_with_references(
            pool,
            config(&mock),
            &mock,
            true,
            Some((
                &seed.token_a,
                &seed.token_b,
                "terra1000000000000000000000000000000000002",
            )),
        )
        .await,
    );
    let resp = server.get(&format!("/api/v1/route/solve?token_in={}&token_out={}&amount_out=1000000&sender=terra1000000000000000000000000000000000002",seed.token_a,seed.token_b)).await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(body["tax_kind"], "route_local_v1");
    assert_eq!(body["buy_tax_bps"], 0);
    assert!(
        body["required_amount_in"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            < 1100000
    );
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[serial]
#[tokio::test]
async fn route_exact_output_fair_crossrate_rejects_suspiciously_good_route_for_safe_alternate() {
    let pool = common::setup_pool().await;
    let seed = seed(&pool, true).await;
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
    let server = TestServer::new(observed_app(pool, config(&mock), &mock).await);
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
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[serial]
#[tokio::test]
async fn route_exact_output_large_impact_retains_shared_5_30_warning_fields() {
    let pool = common::setup_pool().await;
    let seed = seed(&pool, false).await;
    let deep = 10000000000000u128;
    let mock = forward_mock(
        vec![(seed.token_a.clone(), seed.token_b.clone(), deep, deep, 30)],
        false,
    )
    .await;
    let server = TestServer::new(observed_app(pool, config(&mock), &mock).await);
    let resp = server
        .get(&format!(
            "/api/v1/route/solve?token_in={}&token_out={}&amount_out={}&max_spread=0.5",
            seed.token_a,
            seed.token_b,
            deep * 30 / 100
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
            > deep * 30 / 100
    );
    assert!(mock.received_requests().await.unwrap().is_empty());
}

#[serial]
#[tokio::test]
async fn route_exact_output_present_standard_reference_unavailable_fails_closed() {
    let pool = common::setup_pool().await;
    let seed = seed(&pool, false).await;
    sqlx::query("UPDATE assets SET symbol='cUSTC' WHERE contract_address=$1")
        .bind(&seed.token_c)
        .execute(&pool)
        .await
        .unwrap();
    let mock = forward_mock(
        vec![(
            seed.token_a.clone(),
            seed.token_b.clone(),
            10_000_000_000_000,
            10_000_000_000_000,
            30,
        )],
        false,
    )
    .await;
    let server = TestServer::new(
        observed_app_with_references(pool, config(&mock), &mock, false, None).await,
    );
    let resp = server
        .get(&format!(
            "/api/v1/route/solve?token_in={}&token_out={}&amount_out=1000000",
            seed.token_a, seed.token_b
        ))
        .await;
    resp.assert_status(axum::http::StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(resp.json::<Value>()["error"], "missing_observations");
    assert!(mock
        .received_requests()
        .await
        .unwrap()
        .iter()
        .all(|request| lcd_mock::smart_query_from_request(request)
            .get("simulate_swap_operations")
            .is_none()));
    assert!(mock.received_requests().await.unwrap().is_empty());
}

/// #1390: reverse sizing preserves observed protocol-trader admission rejection.
#[serial]
#[tokio::test]
async fn route_exact_output_protocol_exempt_guard_rejection_is_observable() {
    let pool = common::setup_pool().await;
    let seed = seed(&pool, false).await;
    let (r0, r1) = (1_000_000_000u128, 1_000_000_000u128);
    reserves(&pool, "terra1pairrouteabc", r0, r1, 30).await;
    let mock = forward_tax_mock(
        vec![(seed.token_a.clone(), seed.token_b.clone(), r0, r1, 30)],
        false,
        std::collections::HashMap::from([(seed.token_b.clone(), (0, 0))]),
        false,
    )
    .await;
    Mock::given(method("GET"))
        .and(path_regex(r"/cosmwasm/wasm/v1/contract/.*/smart/.*"))
        .and(|request:&Request| {
            let q=lcd_mock::smart_query_from_request(request);
            q["is_protocol_exempt"]["address"]=="terra1synthetictrader1390xxxxxxxxxxxxxxxxx"
        })
        .respond_with(|request: &Request| {
            let q = lcd_mock::smart_query_from_request(request);
            if q.get("is_protocol_exempt").is_some() {
                ResponseTemplate::new(200)
                    .set_body_json(json!({"data":{"address":q["is_protocol_exempt"]["address"],"manager":false,"protocol":true}}))
            } else {
                ResponseTemplate::new(500).set_body_string(
                    "query wasm contract failed: router swap requires a non-exempt trader",
                )
            }
        })
        .with_priority(1)
        .mount(&mock)
        .await;
    let server = TestServer::new(observed_app(pool, config(&mock), &mock).await);
    for endpoint in ["solve", "solve/best"] {
        let response = server.get(&format!(
            "/api/v1/route/{endpoint}?token_in={}&token_out={}&amount_out=1000000&trader=terra1synthetictrader1390xxxxxxxxxxxxxxxxx",
            seed.token_a, seed.token_b,
        )).await;
        response.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(response.json::<Value>()["error"], "trading_blocked");
        assert!(!response.text().contains("required_amount_in"));
    }
    assert!(mock.received_requests().await.unwrap().is_empty());
}

/// #1452: provider failure cannot force request RPC or invalidate fresh local observations.
/// Provider gas acceptance remains a separately scheduled #1422/contract verification.
#[serial]
#[tokio::test]
async fn route_exact_output_provider_outage_does_not_invalidate_initialized_local_quote() {
    let pool = common::setup_pool().await;
    let seed = seed(&pool, false).await;
    let (r0, r1) = (1_000_000_000u128, 1_000_000_000u128);
    reserves(&pool, "terra1pairrouteabc", r0, r1, 30).await;
    let mock = forward_tax_mock(
        vec![(seed.token_a.clone(), seed.token_b.clone(), r0, r1, 30)],
        false,
        std::collections::HashMap::new(),
        false,
    )
    .await;
    let server = TestServer::new(observed_app(pool, config(&mock), &mock).await);
    let url = format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_out=1000000",
        seed.token_a, seed.token_b,
    );
    server.get(&url).await.assert_status_ok();
    Mock::given(method("GET"))
        .and(path_regex(r"/cosmwasm/wasm/v1/contract/.*/smart/.*"))
        .and(|request: &Request| lcd_mock::smart_query_from_request(request)
            .get("simulate_swap_operations").is_some())
        .respond_with(ResponseTemplate::new(500).set_body_json(json!({
            "message": "out of gas in location: wasm contract; gasWanted: 3000000, gasUsed: 3043317 http://private-lcd terra1private",
        })))
        .with_priority(1)
        .expect(0)
        .mount(&mock)
        .await;
    for endpoint in ["solve", "solve/best"] {
        let response = server
            .get(&url.replace("/solve?", &format!("/{endpoint}?")))
            .await;
        response.assert_status_ok();
        let body: Value = response.json();
        assert!(
            body["required_amount_in"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
                > 0
        );
        assert!(
            body["estimated_amount_out_net"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
                >= 1_000_000
        );
    }
    assert!(mock.received_requests().await.unwrap().is_empty());
}
