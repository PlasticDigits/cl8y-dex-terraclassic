//! #1452: exercise an ingested market through a real HTTP socket with LCD disabled.
//! Every binary owns its disposable #1396 database; the performance scope is loopback.
mod common;

use bech32::ToBase32;
use cl8y_dex_indexer::{
    api::{build_router, AppState, TickerMapCache},
    db::queries::{assets, pair_reserves, pairs},
    indexer::{fee_discount_registry_health::FeeDiscountRegistryHealth, hub_usd::HubUsdConfig},
    lcd::LcdClient,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::{sync::Arc, time::Instant};
use tokio::task::JoinSet;
use wiremock::{matchers::method, Mock, ResponseTemplate};

const CL8Y: &str = "terra1cl8yfixture145200000000000000000000";
const USTR: &str = "terra1ustrfixture145200000000000000000000";
const CUSTC: &str = "terra1custcfixture145200000000000000000000";
const CLUNC: &str = "terra1cluncfixture145200000000000000000000";
const UST1: &str = "terra1ust1fixture145200000000000000000000";
const ALPHA: &str = "terra1alphafixture14640000000000000000000";

async fn asset(pool: &PgPool, address: &str, decimals: i16) -> i32 {
    sqlx::query_scalar(
        "INSERT INTO assets (contract_address,denom,is_cw20,name,symbol,decimals)
         VALUES ($1,$2,$3,'fixture','untrusted-symbol',$4) RETURNING id",
    )
    .bind(address.starts_with("terra1").then_some(address))
    .bind((!address.starts_with("terra1")).then_some(address))
    .bind(address.starts_with("terra1"))
    .bind(decimals)
    .fetch_one(pool)
    .await
    .unwrap()
}

async fn pair(pool: &PgPool, address: &str, a: i32, b: i32, ra: u128, rb: u128) {
    let id: i32 = sqlx::query_scalar(
        "INSERT INTO pairs(contract_address,asset_0_id,asset_1_id,fee_bps)
         VALUES($1,$2,$3,30) RETURNING id",
    )
    .bind(address)
    .bind(a)
    .bind(b)
    .fetch_one(pool)
    .await
    .unwrap();
    pair_reserves::upsert_pair_reserves(
        pool,
        id,
        &ra.to_string().parse().unwrap(),
        &rb.to_string().parse().unwrap(),
        30,
        Some(1452),
    )
    .await
    .unwrap();
}

/// Eight short candidates put the useful corridor beyond the old five-candidate cap.
/// Every intermediate has six decimals; endpoints have eighteen. Tickers are deliberately useless.
async fn seed_market(pool: &PgPool) {
    common::clean_db(pool).await;
    let cl8y = asset(pool, CL8Y, 18).await;
    let ustr = asset(pool, USTR, 18).await;
    for idx in 0..8 {
        let addr = match idx {
            0 => CLUNC.to_owned(),
            7 => CUSTC.to_owned(),
            _ => format!("terra1thinintermediate{idx}1452"),
        };
        let intermediate = asset(pool, &addr, 6).await;
        pair(
            pool,
            &format!("terra1branch{idx}in1452"),
            cl8y,
            intermediate,
            100 * 10u128.pow(18),
            10_000 * 10u128.pow(6),
        )
        .await;
        let liquidity = if idx == 7 { 10_000 } else { 1 };
        pair(
            pool,
            &format!("terra1branch{idx}out1452"),
            intermediate,
            ustr,
            liquidity * 10u128.pow(6),
            liquidity * 10u128.pow(18),
        )
        .await;
    }
    let custc: i32 = sqlx::query_scalar("SELECT id FROM assets WHERE contract_address=$1")
        .bind(CUSTC)
        .fetch_one(pool)
        .await
        .unwrap();
    let ust1 = asset(pool, UST1, 18).await;
    // The real mixed-decimal corridor remains reachable during bounded deeper search.
    // Give it lower net economics in this fixture so asynchronous work cannot disguise
    // a missing initial baseline; a separate optimizer test proves improvement.
    pair(
        pool,
        "terra1custcust11452",
        custc,
        ust1,
        10_000 * 10u128.pow(6),
        9_000 * 10u128.pow(18),
    )
    .await;
    pair(
        pool,
        "terra1ust1ustr1452",
        ust1,
        ustr,
        10_000 * 10u128.pow(18),
        10_000 * 10u128.pow(18),
    )
    .await;
    // Native legs exercise the same all-swaps entry point without trusting token symbols.
    for native in ["uluna", "uusd"] {
        let id = asset(pool, native, 6).await;
        pair(
            pool,
            &format!("terra1native{native}1452"),
            id,
            ustr,
            10_000 * 10u128.pow(6),
            10_000 * 10u128.pow(18),
        )
        .await;
    }
}

async fn seed_reference_marks(pool: &PgPool, cl8y_price: i64) {
    let mut cfg = common::test_config();
    cfg.hub_custc_address = CUSTC.into();
    cfg.hub_clunc_address = CLUNC.into();
    common::reference_marks::seed_reference_marks(
        pool,
        &HubUsdConfig::from_indexer_config(&cfg),
        &[(CL8Y, cl8y_price.into())],
    )
    .await;
}

const COLD_FIXTURE_HTTP_LIMIT: usize = 512;

fn cold_fixture_lcd(endpoint: String) -> LcdClient {
    // Cold configuration I/O is outside the initialized <250ms quote measurement.
    // Use bounded five-second provider attempts rather than a 100ms loopback
    // assumption; admission's own phase/pass deadlines remain unchanged.
    let policy = cl8y_dex_indexer::lcd::ProviderPolicy {
        // This test drains ~20 pairs, 12 assets, two actors and native mappings
        // consecutively instead of using production's paced scheduled view.
        // Account for that known cold population without changing runtime budgets.
        burst: COLD_FIXTURE_HTTP_LIMIT as u32,
        attempt_timeout_ms: 5_000,
        deadline_ms: 5_000,
        ..Default::default()
    };
    LcdClient::new_with_policy(vec![endpoint], 5_000, 1, "columbus-5", policy)
}
fn app_state(pool: PgPool, lcd: LcdClient) -> AppState {
    let mut config = common::test_config();
    config.router_address = Some("terra1routerfixture1452".into());
    config.route_solver_db_hybrid = true;
    config.hub_custc_address = CUSTC.into();
    config.hub_clunc_address = CLUNC.into();
    AppState {
        indexer_health: Default::default(),
        pool,
        lcd,
        oracle_prices: cl8y_dex_indexer::indexer::oracle::OraclePriceHandles::new(),
        venus_vfdusd: cl8y_dex_indexer::indexer::venus_vfdusd::new_shared_venus(),
        ticker_map_cache: TickerMapCache::default(),
        orderbook_cache: Default::default(),
        router_address: config.router_address.clone(),
        factory_address: Some(config.factory_address.clone()),
        wrap_mapper_address: None,
        fee_discount_address: None,
        fee_discount_registry_health: FeeDiscountRegistryHealth::from_config(None),
        route_solver_db_hybrid: true,
        book_snapshot_max_staleness_ms: 60_000,
        route_fidelity_drift_bps: 100,
        hub_usd: HubUsdConfig::from_indexer_config(&config),
        community_tax: cl8y_dex_indexer::config::CommunityTaxCatalogConfig::from_indexer_config(
            &config,
        ),
    }
}

async fn useful_quote(
    client: &reqwest::Client,
    base: &str,
    input: &str,
    amount: &str,
) -> (Value, f64) {
    let started = Instant::now();
    let response = client
        .get(format!("{base}/api/v1/route/solve"))
        .query(&[
            ("token_in", input),
            ("token_out", USTR),
            ("amount_in", amount),
            ("max_spread", "0.05"),
        ])
        .send()
        .await
        .unwrap();
    let status = response.status();
    let text = response.text().await.unwrap();
    assert!(
        status.is_success(),
        "quote must be useful: status={status} body={text}"
    );
    let body: Value = serde_json::from_str(&text).expect("successful quote JSON");
    assert!(
        body["estimated_amount_out_net"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            > 0
    );
    assert!(!body["router_operations"].as_array().unwrap().is_empty());
    let elapsed_ms = started.elapsed().as_secs_f64() * 1000.0;
    assert!(
        elapsed_ms < 250.0,
        "first useful socket quote took {elapsed_ms:.3} ms"
    );
    (body, elapsed_ms)
}

fn report(label: &str, samples: &mut [f64]) {
    samples.sort_by(f64::total_cmp);
    let percentile = |p: usize| samples[(samples.len() - 1) * p / 100];
    eprintln!(
        "1452 loopback {label}: n={} p50={:.3}ms p95={:.3}ms p99={:.3}ms max={:.3}ms",
        samples.len(),
        percentile(50),
        percentile(95),
        percentile(99),
        samples.last().unwrap()
    );
}

async fn mount_supported_config_mock(upstream: &wiremock::MockServer, wrap_fee_bps: u16) {
    mount_supported_config_mock_with_wrapper_delay(
        upstream,
        wrap_fee_bps,
        std::time::Duration::ZERO,
    )
    .await;
}

async fn mount_supported_config_mock_with_wrapper_delay(
    upstream: &wiremock::MockServer,
    wrap_fee_bps: u16,
    delay: std::time::Duration,
) {
    Mock::given(method("GET"))
        .and(wiremock::matchers::path_regex(
            r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/",
        ))
        .respond_with(move |request: &wiremock::Request| {
            let query = common::lcd_mock::smart_query_from_request(request);
            let data = if query.get("config").is_some() && request.url.path().contains("terra1mapperfixture1452") {
                json!({"paused":false,"treasury":"terra1treasuryfixture1452","fee_wrap_bps":wrap_fee_bps,"fee_unwrap_bps":wrap_fee_bps})
            } else if query.get("config").is_some() && request.url.path().contains("terra1treasuryfixture1452") {
                json!({"wrapping_paused":false})
            } else if query.get("config").is_some() && request.url.path().contains("terra1routerfixture1452") {
                json!({"wrap_mapper":"terra1mapperfixture1452"})
            } else if query.get("execution_capabilities").is_some() {
                json!({"direct_wrapper_floor_version":1})
            } else if query.get("denom_wrappers").is_some() {
                json!({"wrappers":[{"denom":"uluna","wrapper":"terra1mapperfixture1452"},{"denom":"uusd","wrapper":"terra1mapperfixture1452"}]})
            } else if let Some(mapping) = query.get("denom_mapping") {
                let denom = mapping["denom"].as_str().unwrap();
                json!({"denom":denom,"cw20_addr":if denom=="uluna" {CLUNC} else {CUSTC}})
            } else if query.get("rate_limit").is_some() {
                json!({"config":null,"amount_used":"0"})
            } else if query.get("get_hooks").is_some() {
                json!({"hooks":[]})
            } else if query.get("is_paused").is_some() {
                json!({"paused":false})
            } else if query.get("get_asset_code_ids").is_some() {
                json!({"code_ids":[10184,10184]})
            } else if query.get("blacklist_check").is_some() {
                json!({"blocked":false,"wallet_blacklisted":false,"pair_blacklisted":false,"blacklisted_tokens":[],"blacklisted_pairs":[]})
            } else if query.get("get_discount_registry").is_some() {
                json!({"registry":null})
            } else if query.get("is_code_id_whitelisted").is_some() {
                json!({"whitelisted":true})
            } else {
                return ResponseTemplate::new(500);
            };
            let response = ResponseTemplate::new(200).set_body_json(json!({"data":data}));
            if query.get("config").is_some()
                || query.get("denom_wrappers").is_some()
                || query.get("denom_mapping").is_some()
                || query.get("rate_limit").is_some()
            {
                response.set_delay(delay)
            } else {
                response
            }
        })
        .mount(upstream)
        .await;
}

#[tokio::test]
#[serial_test::serial]
async fn recovery1464_slow_optional_wrapper_output_preserves_completed_native_input() {
    let pool = common::setup_pool().await;
    let upstream = common::lcd_mock::start_ordinary_code_mock().await;
    mount_supported_config_mock_with_wrapper_delay(
        &upstream,
        100,
        std::time::Duration::from_millis(900),
    )
    .await;
    Mock::given(method("GET"))
        .and(wiremock::matchers::path("/terra/tax/v1beta1/params"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(json!({"params":{"burn_tax_rate":"0.005"}}))
                .set_delay(std::time::Duration::from_secs(5)),
        )
        .mount(&upstream)
        .await;
    let mut state = app_state(pool, cold_fixture_lcd(upstream.uri()));
    state.wrap_mapper_address = Some("terra1mapperfixture1452".into());
    let before = chrono::Utc::now().timestamp_millis();
    assert!(
        cl8y_dex_indexer::api::local_quote::refresh_wrapper(&state).await,
        "slow optional output must not cancel completed native input"
    );
    cl8y_dex_indexer::api::local_quote::initialize_for_test(
        state.lcd.chain_cache_namespace(),
        Vec::new(),
        std::collections::HashMap::from([(1, CLUNC.into()), (2, CUSTC.into())]),
        std::collections::HashMap::new(),
        20_000,
    );
    let request = serde_json::from_value(json!({
        "token_in":"uluna", "token_out":CLUNC, "amount_in":"10000", "max_spread":"0.05"
    }))
    .unwrap();
    let body = cl8y_dex_indexer::api::local_quote::solve(&state, &request)
        .await
        .unwrap()
        .0;
    let observed = body["execution_metadata"]["observed_at_ms"]
        .as_i64()
        .unwrap();
    assert!(observed >= before);
    assert!(chrono::Utc::now().timestamp_millis() - observed >= 6_000);
    assert_eq!(body["estimated_amount_out_net"], "9900");
    assert_eq!(body["execution_metadata"]["execution_supported"], false);
    assert!(!body["execution_blocker"].is_null());
}

#[tokio::test]
#[serial_test::serial]
async fn recovery1464_reported_direction_corpus_is_executable_fast_and_network_free() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    asset(&pool, "uluna", 6).await;
    let clunc = asset(&pool, CLUNC, 6).await;
    let custc = asset(&pool, CUSTC, 6).await;
    let cl8y = asset(&pool, CL8Y, 18).await;
    let ustr = asset(&pool, USTR, 18).await;
    let alpha = asset(&pool, ALPHA, 6).await;
    for (address, a, b, da, db) in [
        ("terra1clunccustc1464", clunc, custc, 6, 6),
        ("terra1cl8yustr1464", cl8y, ustr, 18, 18),
        ("terra1ustrcustc1464", ustr, custc, 18, 6),
        ("terra1alphacl8y1464", alpha, cl8y, 6, 18),
    ] {
        pair(
            &pool,
            address,
            a,
            b,
            1_000_000 * 10u128.pow(da),
            1_000_000 * 10u128.pow(db),
        )
        .await;
    }
    seed_reference_marks(&pool, 1).await;
    let upstream = common::lcd_mock::start_ordinary_code_mock().await;
    mount_supported_config_mock(&upstream, 0).await;
    let mut state = app_state(pool.clone(), cold_fixture_lcd(upstream.uri()));
    state.wrap_mapper_address = Some("terra1mapperfixture1452".into());
    cl8y_dex_indexer::api::local_quote::ingest_fixture(&state)
        .await
        .unwrap();
    Mock::given(method("GET"))
        .and(wiremock::matchers::path("/terra/tax/v1beta1/params"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"params":{"burn_tax_rate":"0.005"}})),
        )
        .mount(&upstream)
        .await;
    Mock::given(method("GET"))
        .and(wiremock::matchers::path_regex(
            r"^/terra/treasury/v1beta1/tax_caps/",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"tax_cap":"1000000"})))
        .mount(&upstream)
        .await;
    // Independently scheduled fixture observations precede measured request traffic.
    sqlx::query("UPDATE pair_reserves SET snapshot_at=NOW()")
        .execute(&pool)
        .await
        .unwrap();
    seed_reference_marks(&pool, 1).await;
    cl8y_dex_indexer::api::local_quote::refresh_reserves(&state)
        .await
        .unwrap();
    assert!(cl8y_dex_indexer::api::local_quote::refresh_wrapper(&state).await);
    upstream.reset().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&upstream)
        .await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, build_router(state, &common::test_config()))
            .await
            .unwrap()
    });
    let reused = reqwest::Client::new();
    assert!(reused
        .get(format!("{base}/health"))
        .send()
        .await
        .unwrap()
        .status()
        .is_success());
    let corpus = [
        ("uluna", CUSTC, "10000000000", "100000000"),
        (CL8Y, USTR, "1000000000000000000", "10000000000000000"),
        (USTR, CUSTC, "100000000000000000000", "1000000000000000000"),
        (ALPHA, CL8Y, "1000000", "100000"),
    ];
    let mut misses = Vec::new();
    let mut hits = Vec::new();
    for (input, output, ordinary, small) in corpus {
        for (amount, spread) in [(ordinary, "0.05"), (small, "0.01"), (small, "0.30")] {
            let mut first = None;
            // First result is a fresh connection/result miss; second is a reused hit.
            for hit in [false, true] {
                let fresh = reqwest::Client::new();
                let client = if hit { &reused } else { &fresh };
                let start = Instant::now();
                let response = client
                    .get(format!("{base}/api/v1/route/solve"))
                    .query(&[
                        ("token_in", input),
                        ("token_out", output),
                        ("amount_in", amount),
                        ("max_spread", spread),
                        ("pool_only", "true"),
                    ])
                    .send()
                    .await
                    .unwrap();
                let status = response.status();
                let text = response.text().await.unwrap();
                assert_eq!(
                    status,
                    reqwest::StatusCode::OK,
                    "{input}->{output} amount={amount} spread={spread}: {text}"
                );
                let body: Value = serde_json::from_str(&text).unwrap();
                let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                assert!(
                    elapsed < 250.0,
                    "1464 useful loopback sample took {elapsed:.3}ms"
                );
                let meta = &body["execution_metadata"];
                assert_eq!(meta["input_amount"], amount);
                assert!(!meta["generation"].as_str().unwrap().is_empty());
                let now = chrono::Utc::now().timestamp_millis();
                assert!(meta["observed_at_ms"].as_i64().unwrap() <= now);
                assert!(meta["expires_at_ms"].as_i64().unwrap() > now);
                for floor in [
                    body["estimated_amount_out_net"].as_str().unwrap(),
                    meta["minimum_receive"].as_str().unwrap(),
                ] {
                    assert!(floor.parse::<u128>().unwrap() > 0);
                }
                let ops = body["router_operations"].as_array().unwrap();
                let hops = meta["hops"].as_array().unwrap();
                assert!(!ops.is_empty());
                assert_eq!(ops.len(), hops.len());
                for (op, hop) in ops.iter().zip(hops) {
                    assert!(
                        op["terra_swap"]["min_return"]
                            .as_str()
                            .unwrap()
                            .parse::<u128>()
                            .unwrap()
                            > 0
                    );
                    assert_eq!(op["terra_swap"]["min_return"], hop["min_return"]);
                }
                if input == "uluna" {
                    assert_eq!(meta["wrapper"]["input_amount"], amount);
                    assert_eq!(meta["wrapper"]["output_token"], CUSTC);
                    assert_eq!(hops[0]["offer_token"], CLUNC);
                }
                if hit {
                    let initial: Value = first.take().unwrap();
                    assert_eq!(initial["router_operations"], body["router_operations"]);
                    assert_eq!(initial["execution_metadata"], body["execution_metadata"]);
                    hits.push(elapsed);
                } else {
                    first = Some(body);
                    misses.push(elapsed);
                }
            }
        }
    }
    report(
        "1464 four directions new connection/result misses",
        &mut misses,
    );
    report(
        "1464 four directions reused connection/result hits",
        &mut hits,
    );
    assert!(
        upstream.received_requests().await.unwrap().is_empty(),
        "measured corpus cannot invoke providers"
    );
    server.abort();
    let _ = server.await;
}

#[tokio::test]
#[serial_test::serial]
async fn initialized_worker_socket_quotes_are_useful_fast_and_network_free() {
    let pool = common::setup_pool().await;
    seed_market(&pool).await;
    for byte in [1u8, 2u8] {
        let actor =
            bech32::encode("terra", [byte; 20].to_base32(), bech32::Variant::Bech32).unwrap();
        sqlx::query("INSERT INTO traders(address,tier_id,registered) VALUES($1,255,false)")
            .bind(actor)
            .execute(&pool)
            .await
            .unwrap();
    }
    let upstream = common::lcd_mock::start_ordinary_code_mock().await;
    let lcd = cold_fixture_lcd(upstream.uri());
    let mut state = app_state(pool.clone(), lcd);
    state.wrap_mapper_address = Some("terra1mapperfixture1452".into());
    // Production ingestion is the sole network-enabled step. Requests below cannot ingest.
    mount_supported_config_mock(&upstream, 100).await;
    seed_reference_marks(&pool, 100).await;
    let ingestion_started = Instant::now();
    cl8y_dex_indexer::api::local_quote::ingest_fixture(&state)
        .await
        .unwrap();
    Mock::given(method("GET"))
        .and(wiremock::matchers::path("/terra/tax/v1beta1/params"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(json!({"params":{"burn_tax_rate":"0.005"}})),
        )
        .mount(&upstream)
        .await;
    Mock::given(method("GET"))
        .and(wiremock::matchers::path_regex(
            r"^/terra/treasury/v1beta1/tax_caps/",
        ))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({"tax_cap":"1000000"})))
        .mount(&upstream)
        .await;
    // The independently scheduled fixture feed observes its fixed reserve state
    // after slow configuration ingestion; this is separate from measured requests.
    sqlx::query("UPDATE pair_reserves SET snapshot_at=NOW()")
        .execute(&pool)
        .await
        .unwrap();
    seed_reference_marks(&pool, 100).await;
    cl8y_dex_indexer::api::local_quote::refresh_reserves(&state)
        .await
        .unwrap();
    assert!(cl8y_dex_indexer::api::local_quote::refresh_wrapper(&state).await);
    // Prove both admitted mappings before disabling providers. Zero-hop probes
    // neither warm the measured executable route cache nor enqueue deeper jobs.
    for (native, wrapped) in [("uluna", CLUNC), ("uusd", CUSTC)] {
        let probe = serde_json::from_value(
            json!({"token_in":native,"token_out":wrapped,"amount_in":"10000","max_spread":"0.05"}),
        )
        .unwrap();
        let body = cl8y_dex_indexer::api::local_quote::solve(&state, &probe)
            .await
            .expect("cold fixture must initialize each native mapping before provider isolation")
            .0;
        assert_eq!(body["execution_metadata"]["policy"], "wrapper_only");
        assert_eq!(
            body["execution_metadata"]["wrapper"]["mapper"],
            "terra1mapperfixture1452"
        );
        assert_eq!(body["execution_metadata"]["wrapper"]["input_token"], native);
        assert_eq!(
            body["execution_metadata"]["wrapper"]["output_token"],
            wrapped
        );
        assert_eq!(body["estimated_amount_out_net"], "9900");
        assert_eq!(body["execution_metadata"]["execution_supported"], true);
        assert_eq!(
            body["execution_metadata"]["wrapper"]["floor_protocol_version"],
            1
        );
        assert!(body["execution_blocker"].is_null());
    }

    let cold_http_calls = upstream.received_requests().await.unwrap().len();
    assert!(
        cold_http_calls <= COLD_FIXTURE_HTTP_LIMIT,
        "cold fixture exceeded its bounded provider envelope: {cold_http_calls}"
    );
    eprintln!("1452 cold fixture provider calls: {cold_http_calls} (bounded initialization only)");
    eprintln!("1452 independent cold ingestion and wrapper initialization: {:.3}ms (excluded from initialized request latency)",ingestion_started.elapsed().as_secs_f64()*1000.0);
    upstream.reset().await;
    Mock::given(method("GET"))
        .respond_with(
            ResponseTemplate::new(500)
                .set_body_json(json!({"message":"network forbidden during quote"})),
        )
        .mount(&upstream)
        .await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = Arc::new(format!("http://{}", listener.local_addr().unwrap()));
    let server_state = state.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, build_router(server_state, &common::test_config()))
            .await
            .unwrap()
    });
    let client = reqwest::Client::new();
    let mut misses = Vec::new();
    let mut warm = Vec::new();
    let (baseline, elapsed) = useful_quote(&client, &base, CL8Y, "100000000000000000").await;
    misses.push(elapsed);
    eprintln!("1452 baseline generation={} reserve_block={} observed_at={} expires_at={} useful_output={}",
        baseline["execution_metadata"]["generation"],baseline["snapshot_block_height"],baseline["execution_metadata"]["observed_at_ms"],baseline["execution_metadata"]["expires_at_ms"],baseline["estimated_amount_out_net"]);
    assert_eq!(
        baseline["intermediate_tokens"][1], CUSTC,
        "thin protected route must lose to deep executable corridor"
    );
    for input in [CL8Y, CUSTC, UST1] {
        for idx in 0..20 {
            let amount = if input == CL8Y {
                (10u128.pow(16) + idx * 10u128.pow(14)).to_string()
            } else if input == CUSTC {
                (1000 + idx).to_string()
            } else {
                (10u128.pow(15) + idx * 10u128.pow(12)).to_string()
            };
            let (first, miss_ms) = useful_quote(&client, &base, input, &amount).await;
            let (again, hit_ms) = useful_quote(&client, &base, input, &amount).await;
            assert_eq!(
                first["estimated_amount_out_net"],
                again["estimated_amount_out_net"]
            );
            assert_eq!(first["router_operations"], again["router_operations"]);
            misses.push(miss_ms);
            warm.push(hit_ms);
        }
    }
    for native in ["uluna", "uusd"] {
        for gross in [1000, 9999, 10000] {
            let (body, elapsed) = useful_quote(&client, &base, native, &gross.to_string()).await;
            assert_eq!(
                body["execution_metadata"]["input_amount"],
                gross.to_string()
            );
            assert_eq!(
                body["execution_metadata"]["wrapper"]["input_amount"],
                gross.to_string()
            );
            assert_eq!(
                body["execution_metadata"]["wrapper"]["route_input_amount"],
                (gross - gross / 100).to_string()
            );
            assert_eq!(body["execution_metadata"]["wrapper"]["input_token"], native);
            misses.push(elapsed);
        }
    }
    let counters_before = cl8y_dex_indexer::api::local_quote::cache_counters();
    let mut jobs = JoinSet::new();
    for _ in 0..32 {
        let client = client.clone();
        let base = Arc::clone(&base);
        jobs.spawn(async move { useful_quote(&client, &base, CL8Y, "100000000000000000").await });
    }
    while let Some(result) = jobs.join_next().await {
        let (body, elapsed) = result.unwrap();
        assert_eq!(
            body["estimated_amount_out_net"],
            baseline["estimated_amount_out_net"]
        );
        warm.push(elapsed);
    }
    let counters_after = cl8y_dex_indexer::api::local_quote::cache_counters();
    assert!(
        counters_after.0 >= counters_before.0 + 32,
        "identical bursts must use result cache"
    );
    assert_eq!(
        counters_after.1, counters_before.1,
        "cache hits cannot start redundant background jobs"
    );
    assert!(
        counters_after.2 <= 4 && counters_after.3 <= 256,
        "background/cache budgets exceeded"
    );
    eprintln!(
        "1452 cache counters hits={} jobs_started={} active={} results={}",
        counters_after.0, counters_after.1, counters_after.2, counters_after.3
    );
    // Receive sizing and progress are request stages too; neither may start ingestion.
    // The target lies between a viable and an ineligible power-of-two offer; an
    // inverse that only brackets admitted scores would wrongly report no route.
    let exact = client
        .get(format!("{base}/api/v1/route/solve"))
        .query(&[
            ("token_in", CL8Y),
            ("token_out", USTR),
            ("amount_out", "450000000000000000000"),
            ("max_spread", "0.05"),
        ])
        .send()
        .await
        .unwrap();
    let status = exact.status();
    let exact: Value = exact.json().await.unwrap();
    assert!(status.is_success(), "local receive sizing failed: {exact}");
    assert!(
        exact["estimated_amount_out_net"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            >= 450 * 10u128.pow(18)
    );
    assert!(
        exact["required_amount_in"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            > 0
    );
    let progress = client
        .get(format!("{base}/api/v1/route/solve/progress"))
        .query(&[
            ("token_in", CL8Y),
            ("token_out", USTR),
            ("amount_in", "100000000000000000"),
            ("max_spread", "0.05"),
        ])
        .send()
        .await
        .unwrap();
    assert!(progress.status().is_success());
    // Distinct actor identity remains exact even without a configured discount provider.
    for byte in [1u8, 2u8] {
        let actor =
            bech32::encode("terra", [byte; 20].to_base32(), bech32::Variant::Bech32).unwrap();
        let response = client
            .get(format!("{base}/api/v1/route/solve"))
            .query(&[
                ("token_in", CL8Y),
                ("token_out", USTR),
                ("amount_in", "100000000000000000"),
                ("max_spread", "0.05"),
                ("trader", actor.as_str()),
                ("sender", actor.as_str()),
            ])
            .send()
            .await
            .unwrap();
        let status = response.status();
        let body: Value = response.json().await.unwrap();
        assert!(status.is_success(), "actor quote failed: {body}");
        assert_eq!(body["execution_metadata"]["trader"], actor);
        assert_eq!(body["execution_metadata"]["sender"], actor);
        assert_eq!(
            body["estimated_amount_out_net"],
            baseline["estimated_amount_out_net"]
        );
    }
    let strict = client
        .get(format!("{base}/api/v1/route/solve"))
        .query(&[
            ("token_in", CL8Y),
            ("token_out", USTR),
            ("amount_in", "100000000000000000"),
            ("max_spread", "0.00001"),
        ])
        .send()
        .await
        .unwrap();
    assert!(
        !strict.status().is_success(),
        "a cached 5% quote must not satisfy stricter ineligible spread"
    );
    assert_eq!(strict.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        strict.headers()[reqwest::header::CONTENT_TYPE],
        "application/json"
    );
    let rejected: Value = strict.json().await.unwrap();
    assert_eq!(rejected["error"], "protection");
    assert_eq!(rejected["execution_supported"], false);
    assert!(
        rejected["indicative_amount_out"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            > 0
    );
    for field in [
        "router_operations",
        "execution_metadata",
        "minimum_receive",
        "hops",
    ] {
        assert!(
            rejected[field].is_null(),
            "indicative response leaked signing material {field}"
        );
    }
    // Explicit current policy prices the same amount under the UI net floor
    // while keeping a reviewed amount-specific L9 contract ceiling. Legacy
    // requests above retain their original strict market-spread behavior.
    let relative = client
        .get(format!("{base}/api/v1/route/solve"))
        .query(&[
            ("token_in", CL8Y),
            ("token_out", USTR),
            ("amount_in", "100000000000000000"),
            ("max_spread", "0.00001"),
            ("protection_policy", "quote_relative_v1"),
        ])
        .send()
        .await
        .unwrap();
    assert!(relative.status().is_success());
    let relative: Value = relative.json().await.unwrap();
    assert_eq!(
        relative["execution_metadata"]["protection_policy"],
        "quote_relative_v1"
    );
    assert_eq!(relative["execution_metadata"]["max_spread"], "0.00001");
    assert_ne!(
        relative["execution_metadata"]["execution_market_spread"],
        "0.00001"
    );
    assert_eq!(
        relative["estimated_amount_out_net"],
        baseline["estimated_amount_out_net"]
    );
    let progress: Value = progress.json().await.unwrap();
    assert_eq!(progress["quote_generation"], rejected["generation"]);
    for endpoint in ["solve", "solve/best"] {
        for (input, output) in [(CL8Y, USTR), (USTR, CL8Y)] {
            for protection in ["0.01", "0.05", "0.30"] {
                let repeated = client
                    .get(format!("{base}/api/v1/route/{endpoint}"))
                    .query(&[
                        ("token_in", input),
                        ("token_out", output),
                        ("amount_in", "100000000000000000"),
                        ("max_spread", protection),
                        ("protection_policy", "quote_relative_v1"),
                    ])
                    .send()
                    .await
                    .unwrap();
                assert!(
                    repeated.status().is_success(),
                    "amount/direction/protection change must clear obsolete rejection"
                );
                let repeated: Value = repeated.json().await.unwrap();
                assert!(
                    repeated["estimated_amount_out_net"]
                        .as_str()
                        .unwrap()
                        .parse::<u128>()
                        .unwrap()
                        > 0
                );
                assert_eq!(repeated["token_in"], input);
                assert_eq!(repeated["token_out"], output);
                assert!(
                    repeated["execution_metadata"]["expires_at_ms"]
                        .as_i64()
                        .unwrap()
                        > chrono::Utc::now().timestamp_millis()
                );
            }
        }
    }
    assert_eq!(
        upstream.received_requests().await.unwrap().len(),
        0,
        "initial/hit/miss/concurrent request triggered LCD"
    );
    // A previously cached winner becomes locally F6-frozen. Seven short thin paths
    // still exist, so a structural-only fallback would miss the healthy longer route.
    cl8y_dex_indexer::indexer::asset_code_id_freeze::replace_frozen_pair_addresses(
        std::collections::HashSet::from(["terra1branch7out1452".to_owned()]),
    );
    let frozen_started = Instant::now();
    let healthy_alternative = client
        .get(format!("{base}/api/v1/route/solve"))
        .query(&[
            ("token_in", CL8Y),
            ("token_out", USTR),
            ("amount_in", "100000000000000000"),
            ("max_spread", "0.05"),
        ])
        .send()
        .await
        .unwrap();
    let status = healthy_alternative.status();
    let text = healthy_alternative.text().await.unwrap();
    cl8y_dex_indexer::indexer::asset_code_id_freeze::replace_frozen_pair_addresses(
        Default::default(),
    );
    assert!(
        status.is_success(),
        "cached frozen winner must yield a healthy longer alternative: {text}"
    );
    let alternative: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(alternative["hops"].as_array().unwrap().len(), 3);
    assert!(frozen_started.elapsed().as_secs_f64() * 1000.0 < 250.0);
    assert!(alternative["hops"]
        .as_array()
        .unwrap()
        .iter()
        .all(|hop| hop["pair"] != "terra1branch7out1452"));

    let (unfrozen, _) = useful_quote(&client, &base, CL8Y, "100000000000000000").await;
    assert_eq!(
        unfrozen["estimated_amount_out_net"], baseline["estimated_amount_out_net"],
        "cleared F6 admission must invalidate the lower fallback cache"
    );
    assert_ne!(
        unfrozen["execution_metadata"]["generation"],
        alternative["execution_metadata"]["generation"],
        "freeze admission changes must bind quote generation"
    );

    // Controlled ages retain the old 12/15/30/90-second regression boundaries.
    // Only original observations below the hard 20-second quote TTL are usable,
    // even when a synthetic mirror fixture supplies a longer loading max age.
    let addresses: std::collections::HashMap<i32, String> = assets::get_all_assets(&pool)
        .await
        .unwrap()
        .into_iter()
        .filter_map(|asset| asset.contract_address.map(|address| (asset.id, address)))
        .collect();
    let mut pair_rows = pairs::get_all_pairs(&pool).await.unwrap();
    pair_rows.retain(|pair| {
        addresses.contains_key(&pair.asset_0_id) && addresses.contains_key(&pair.asset_1_id)
    });
    let pair_addresses: Vec<_> = pair_rows
        .iter()
        .map(|pair| pair.contract_address.clone())
        .collect();
    let original_mirrors = cl8y_dex_indexer::api::db_orderbook_sim::preload_pool_mirrors_for_pairs(
        &pool,
        &pair_addresses,
        &addresses,
        120_000,
    )
    .await
    .unwrap();
    // Artificially backdated mirrors belong to a separate synthetic namespace.
    // The production-ingested namespace must retain its monotonic fee watermark;
    // replacing it with older synthetic evidence would correctly fail closed.
    server.abort();
    let _ = server.await;
    let state = app_state(pool.clone(), cold_fixture_lcd(upstream.uri()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = Arc::new(format!("http://{}", listener.local_addr().unwrap()));
    let server_state = state.clone();
    let server = tokio::spawn(async move {
        axum::serve(listener, build_router(server_state, &common::test_config()))
            .await
            .unwrap()
    });
    let mut controlled_reference = None;
    for seconds in [12, 15, 30, 90] {
        let mut mirrors = original_mirrors.clone();
        for mirror in mirrors.values_mut() {
            mirror.snapshot_at = chrono::Utc::now() - chrono::Duration::seconds(seconds);
        }
        pair_rows.reverse(); // Insertion order cannot bias the economic winner.
        cl8y_dex_indexer::api::local_quote::initialize_for_test(
            state.lcd.chain_cache_namespace(),
            pair_rows.clone(),
            addresses.clone(),
            mirrors,
            120_000,
        );
        if seconds >= 20 {
            let expired = client
                .get(format!("{base}/api/v1/route/solve"))
                .query(&[
                    ("token_in", CL8Y),
                    ("token_out", USTR),
                    ("amount_in", "100000000000000000"),
                    ("max_spread", "0.05"),
                ])
                .send()
                .await
                .unwrap();
            assert_eq!(expired.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);
            let expired: Value = expired.json().await.unwrap();
            assert_eq!(expired["error"], "stale_observations");
            assert!(expired["indicative_amount_out"].is_null());
            continue;
        }
        let (aged, _) = useful_quote(&client, &base, CL8Y, "100000000000000000").await;
        assert_eq!(
            aged["estimated_amount_out_net"],
            baseline["estimated_amount_out_net"]
        );
        if let Some(reference) = controlled_reference.as_ref() {
            assert_eq!(&aged["spot_amount_out"], reference);
        } else {
            controlled_reference = Some(aged["spot_amount_out"].clone());
        }
        assert_eq!(aged["router_operations"], baseline["router_operations"]);
    }
    // Change reserves inside a fresh published generation: old amount/floors must invalidate.
    let mut changed = original_mirrors.clone();
    let changed_reserve = &mut changed.get_mut("terra1branch7out1452").unwrap().reserve_1;
    *changed_reserve = *changed_reserve / 100 * 98;
    cl8y_dex_indexer::api::local_quote::initialize_for_test(
        state.lcd.chain_cache_namespace(),
        pair_rows.clone(),
        addresses.clone(),
        changed,
        120_000,
    );
    let (updated, _) = useful_quote(&client, &base, CL8Y, "100000000000000000").await;
    assert_ne!(
        updated["execution_metadata"]["generation"],
        baseline["execution_metadata"]["generation"]
    );
    assert!(
        updated["estimated_amount_out_net"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            < baseline["estimated_amount_out_net"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
    );
    // A strictly better three-hop route may replace a useful two-hop baseline only
    // after bounded local work; requests still return a positive baseline immediately.
    let mut improved_mirrors = original_mirrors.clone();
    let longer_reserve = &mut improved_mirrors
        .get_mut("terra1ust1ustr1452")
        .unwrap()
        .reserve_1;
    *longer_reserve = *longer_reserve / 100 * 112;
    cl8y_dex_indexer::api::local_quote::initialize_for_test(
        state.lcd.chain_cache_namespace(),
        pair_rows.clone(),
        addresses.clone(),
        improved_mirrors,
        120_000,
    );
    let improvement_started = Instant::now();
    let (initial, first_ms) = useful_quote(&client, &base, CL8Y, "100000000000000000").await;
    assert_eq!(initial["hops"].as_array().unwrap().len(), 2);
    let mut improvement = None;
    for _ in 0..100 {
        let (next, _) = useful_quote(&client, &base, CL8Y, "100000000000000000").await;
        if next["estimated_amount_out_net"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            > initial["estimated_amount_out_net"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
        {
            improvement = Some(next);
            break;
        }
        tokio::task::yield_now().await;
    }
    let improvement =
        improvement.expect("bounded deeper search must publish the better three-hop corridor");
    assert_eq!(improvement["hops"].as_array().unwrap().len(), 3);
    eprintln!(
        "1452 useful baseline {:.3}ms, deeper improvement {:.3}ms, generation={}",
        first_ms,
        improvement_started.elapsed().as_secs_f64() * 1000.0,
        improvement["execution_metadata"]["generation"]
    );

    // Stale and future timestamps never fall back to live validation.
    for seconds in [121, -1] {
        let mut mirrors = original_mirrors.clone();
        for mirror in mirrors.values_mut() {
            mirror.snapshot_at = chrono::Utc::now() - chrono::Duration::seconds(seconds);
        }
        cl8y_dex_indexer::api::local_quote::initialize_for_test(
            state.lcd.chain_cache_namespace(),
            pair_rows.clone(),
            addresses.clone(),
            mirrors,
            120_000,
        );
        let response = client
            .get(format!("{base}/api/v1/route/solve"))
            .query(&[
                ("token_in", CL8Y),
                ("token_out", USTR),
                ("amount_in", "100000000000000000"),
                ("max_spread", "0.05"),
            ])
            .send()
            .await
            .unwrap();
        assert!(
            !response.status().is_success(),
            "stale/future observation must fail closed"
        );
    }
    // Independent reference outage fails explicitly and recovers through scheduled
    // local ingestion; it cannot reuse stale impact or trigger live reference solves.
    sqlx::query("DELETE FROM economic_token_marks")
        .execute(&pool)
        .await
        .unwrap();
    cl8y_dex_indexer::api::local_quote::refresh_reserves(&state)
        .await
        .unwrap();
    let unavailable_reference = client
        .get(format!("{base}/api/v1/route/solve"))
        .query(&[
            ("token_in", CL8Y),
            ("token_out", USTR),
            ("amount_in", "100000000000000000"),
            ("max_spread", "0.05"),
        ])
        .send()
        .await
        .unwrap();
    assert!(
        !unavailable_reference.status().is_success(),
        "missing references must not fabricate zero impact or reuse old fields"
    );
    seed_reference_marks(&pool, 100).await;
    cl8y_dex_indexer::api::local_quote::refresh_reserves(&state)
        .await
        .unwrap();
    let (recovered, _) = useful_quote(&client, &base, CL8Y, "100000000000000000").await;
    assert_eq!(
        recovered["estimated_amount_out_net"],
        baseline["estimated_amount_out_net"]
    );
    assert_eq!(upstream.received_requests().await.unwrap().len(), 0);
    report("route-response miss", &mut misses);
    report("warm and concurrent", &mut warm);
    server.abort();
}

#[tokio::test]
#[serial_test::serial]
async fn scheduled_hook_observation_prices_each_deduction_once_without_quote_rpc() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let input_asset = asset(&pool, CL8Y, 18).await;
    let output_asset = asset(&pool, USTR, 18).await;
    let reserve = 1_000 * 10u128.pow(18);
    pair(
        &pool,
        "terra1hookpair1452",
        input_asset,
        output_asset,
        reserve,
        reserve,
    )
    .await;
    let upstream = common::lcd_mock::start_ordinary_code_mock().await;
    Mock::given(method("GET"))
        .and(wiremock::matchers::path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/"))
        .respond_with(|request: &wiremock::Request| {
            let query = common::lcd_mock::smart_query_from_request(request);
            let data = if query.get("get_hooks").is_some() { json!({"hooks":["terra1taxhook1452","terra1burnhook1452"]}) }
                else if query.get("get_config").is_some() && request.url.path().contains("terra1taxhook1452") { json!({"recipient":"terra1fixturetreasury","tax_token":USTR,"tax_percentage_bps":333}) }
                else if query.get("get_config").is_some() && request.url.path().contains("terra1burnhook1452") { json!({"burn_token":USTR,"burn_percentage_bps":333}) }
                else if query.get("is_paused").is_some() { json!({"paused":false}) }
                else if query.get("get_asset_code_ids").is_some() { json!({"code_ids":[10184,10184]}) }
                else if query.get("blacklist_check").is_some() { json!({"blocked":false,"wallet_blacklisted":false,"pair_blacklisted":false,"blacklisted_tokens":[],"blacklisted_pairs":[]}) }
                else if query.get("is_code_id_whitelisted").is_some() { json!({"whitelisted":true}) }
                else { return ResponseTemplate::new(500) };
            ResponseTemplate::new(200).set_body_json(json!({"data":data}))
        }).mount(&upstream).await;
    let state = app_state(pool.clone(), cold_fixture_lcd(upstream.uri()));
    seed_reference_marks(&pool, 1).await;
    cl8y_dex_indexer::api::local_quote::ingest_fixture(&state)
        .await
        .unwrap();
    sqlx::query("UPDATE pair_reserves SET snapshot_at=NOW()")
        .execute(&pool)
        .await
        .unwrap();
    seed_reference_marks(&pool, 1).await;
    cl8y_dex_indexer::api::local_quote::refresh_reserves(&state)
        .await
        .unwrap();
    upstream.reset().await;
    Mock::given(method("GET"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&upstream)
        .await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(listener, build_router(state, &common::test_config()))
            .await
            .unwrap()
    });
    let (body, _) = useful_quote(&reqwest::Client::new(), &base, CL8Y, "1000000000000000000").await;
    let input = 10u128.pow(18);
    use bigdecimal::num_bigint::BigInt;
    let denom = BigInt::from(reserve + input);
    let remaining: u128 =
        ((BigInt::from(reserve) * BigInt::from(reserve) + &denom - BigInt::from(1u8)) / &denom)
            .to_string()
            .parse()
            .unwrap();
    let gross = reserve - remaining;
    let pool_net = gross - cl8y_dex_indexer::api::orderbook_sim::swap_fee_amount(gross, 30);
    let each_deduction = pool_net * 333 / 10_000;
    let final_net = pool_net - each_deduction - each_deduction;
    assert_eq!(body["estimated_amount_out_net"], final_net.to_string());
    assert_eq!(
        body["execution_metadata"]["hops"][0]["pool_return_amount"],
        pool_net.to_string()
    );
    assert_eq!(
        body["execution_metadata"]["hops"][0]["return_amount"],
        final_net.to_string()
    );
    assert!(
        body["execution_metadata"]["minimum_receive"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            > 0
    );
    assert_eq!(upstream.received_requests().await.unwrap().len(), 0);
    server.abort();
}

/// Integration of #1452 and #1453: new pool observations cannot erase usable
/// hybrid books or renew their original age, including a successfully empty book.
#[tokio::test]
#[serial_test::serial]
async fn local_mirror_preserves_hybrid_age_and_falls_back_to_independent_pool() {
    use cl8y_dex_indexer::api::db_orderbook_sim::load_local_mirror;
    use std::collections::HashMap;
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let a = asset(&pool, CL8Y, 18).await;
    let b = asset(&pool, USTR, 18).await;
    pair(&pool, "terra1generation1452", a, b, 1000, 2000).await;
    let p = pairs::get_pair_by_address(&pool, "terra1generation1452")
        .await
        .unwrap()
        .unwrap();
    let original = pair_reserves::get_hybrid_pair_reserves(&pool, p.id)
        .await
        .unwrap()
        .unwrap();
    // New pool-only reserves have a different height and amount; the fee is
    // subsequently observed as in the independent production pool lane.
    pair_reserves::upsert_pool_observation(
        &pool,
        p.id,
        &3000.into(),
        &4000.into(),
        Some(1453),
        chrono::Utc::now(),
    )
    .await
    .unwrap();
    sqlx::query("UPDATE pair_reserves SET fee_bps=30 WHERE pair_id=$1")
        .bind(p.id)
        .execute(&pool)
        .await
        .unwrap();
    let addresses = HashMap::from([(a, CL8Y.to_string()), (b, USTR.to_string())]);
    let hybrid = load_local_mirror(&pool, &p, &addresses, 60_000)
        .await
        .unwrap();
    assert_eq!(hybrid.reserve_0, 1000);
    assert_eq!(hybrid.block_height, Some(1452));
    assert_eq!(hybrid.snapshot_at, original.snapshot_at);
    sqlx::query(
        "UPDATE hybrid_pair_reserves SET snapshot_at=NOW()-INTERVAL '5 minutes' WHERE pair_id=$1",
    )
    .bind(p.id)
    .execute(&pool)
    .await
    .unwrap();
    let current = load_local_mirror(&pool, &p, &addresses, 60_000)
        .await
        .unwrap();
    assert_eq!(current.reserve_0, 3000);
    assert_eq!(current.block_height, Some(1453));
    assert!(current.bids.is_empty() && current.asks.is_empty());
}

/// #1464: a scheduled transient read failure cannot discard fresh qualification,
/// renew its observation time, or hide a subsequently observed pause in a warm cache.
#[tokio::test]
#[serial_test::serial]
async fn transient_admission_refresh_preserves_original_times_then_known_pause_invalidates() {
    use cl8y_dex_indexer::api::local_quote;
    use std::sync::atomic::{AtomicU8, Ordering};
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let a = asset(&pool, CL8Y, 18).await;
    let b = asset(&pool, USTR, 18).await;
    pair(
        &pool,
        "terra1recovery1464",
        a,
        b,
        10_000 * 10u128.pow(18),
        10_000 * 10u128.pow(18),
    )
    .await;
    seed_reference_marks(&pool, 1).await;
    let upstream = common::lcd_mock::start_ordinary_code_mock().await;
    let phase = Arc::new(AtomicU8::new(0));
    let observed_phase = phase.clone();
    Mock::given(method("GET"))
        .and(wiremock::matchers::path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/"))
        .respond_with(move |request: &wiremock::Request| {
            let query = common::lcd_mock::smart_query_from_request(request);
            if observed_phase.load(Ordering::SeqCst) == 1 {
                return ResponseTemplate::new(503);
            }
            let data = if query.get("is_paused").is_some() {
                json!({"paused":observed_phase.load(Ordering::SeqCst) == 2})
            } else if query.get("get_hooks").is_some() {
                json!({"hooks":[]})
            } else if query.get("get_asset_code_ids").is_some() {
                json!({"code_ids":[10184,10184]})
            } else if query.get("blacklist_check").is_some() {
                json!({"blocked":false,"wallet_blacklisted":false,"pair_blacklisted":false,"blacklisted_tokens":[],"blacklisted_pairs":[]})
            } else if query.get("is_code_id_whitelisted").is_some() {
                json!({"whitelisted":true})
            } else if query.get("get_discount_registry").is_some() {
                json!({"registry":null})
            } else {
                return ResponseTemplate::new(500);
            };
            ResponseTemplate::new(200).set_body_json(json!({"data":data}))
        }).mount(&upstream).await;
    let policy = cl8y_dex_indexer::lcd::ProviderPolicy {
        burst: 1024,
        max_backoff_ms: 1,
        attempt_timeout_ms: 5_000,
        deadline_ms: 5_000,
        ..Default::default()
    };
    let lcd = LcdClient::new_with_policy(vec![upstream.uri()], 5_000, 1, "columbus-5", policy);
    let state = app_state(pool.clone(), lcd);
    local_quote::ingest_once(&state).await.unwrap();
    let request = serde_json::from_value(json!({
        "token_in": CL8Y, "token_out": USTR,
        "amount_in": "1000000000000000000",
        "max_spread": "0.05", "pool_only": true
    }))
    .unwrap();
    let original = local_quote::solve(&state, &request).await.unwrap().0;
    phase.store(1, Ordering::SeqCst);
    local_quote::ingest_once(&state).await.unwrap();
    let preserved = local_quote::solve(&state, &request).await.unwrap().0;
    for field in ["observed_at_ms", "expires_at_ms"] {
        assert_eq!(
            preserved["execution_metadata"][field], original["execution_metadata"][field],
            "failed refresh must not renew {field}"
        );
    }
    assert_eq!(
        preserved["estimated_amount_out_net"],
        original["estimated_amount_out_net"]
    );
    phase.store(2, Ordering::SeqCst);
    tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    local_quote::ingest_once(&state).await.unwrap();
    assert_eq!(
        local_quote::solve(&state, &request).await.unwrap_err().0,
        axum::http::StatusCode::UNPROCESSABLE_ENTITY
    );
    phase.store(0, Ordering::SeqCst);
    local_quote::ingest_once(&state).await.unwrap();
    assert!(
        local_quote::solve(&state, &request).await.is_ok(),
        "fresh explicit recovery must restore quotes"
    );
    state.indexer_health.halt("reorg");
    assert_eq!(
        local_quote::solve(&state, &request).await.unwrap_err().0,
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        local_quote::progress(&state, &request).unwrap_err().0,
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        local_quote::solve_post(&state, &request, None)
            .await
            .unwrap_err()
            .0,
        axum::http::StatusCode::SERVICE_UNAVAILABLE
    );
}

#[path = "scenarios/fee_generation_recovery_1464.rs"]
mod fee_generation_recovery;

#[path = "scenarios/route_cost_attribution_1483.rs"]
mod route_cost_attribution;

#[path = "scenarios/route_discovery_1489.rs"]
mod route_discovery_1489;
