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
    sqlx::query("INSERT INTO oracle_state(ticker,price_usd,quality,accepted_sources,as_of) VALUES('ustc',1,'fresh',ARRAY['mexc','coingecko'],NOW()) ON CONFLICT(ticker) DO UPDATE SET as_of=NOW()")
        .execute(pool).await.unwrap();
    sqlx::query("INSERT INTO economic_token_marks(contract_address,asset_id,price_usd,as_of,anchor_ticker) SELECT contract_address,id,CASE WHEN contract_address=$1 THEN $2 ELSE 1 END,NOW(),'custc' FROM assets WHERE contract_address IS NOT NULL ON CONFLICT(contract_address) DO UPDATE SET price_usd=EXCLUDED.price_usd,as_of=NOW()")
        .bind(CL8Y).bind(cl8y_price).execute(pool).await.unwrap();
}

fn app_state(pool: PgPool, lcd: LcdClient) -> AppState {
    let mut config = common::test_config();
    config.router_address = Some("terra1routerfixture1452".into());
    config.route_solver_db_hybrid = true;
    AppState {
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
    let lcd = LcdClient::new(vec![upstream.uri()], 100, 1, "columbus-5");
    let mut state = app_state(pool.clone(), lcd);
    state.wrap_mapper_address = Some("terra1mapperfixture1452".into());
    // Production ingestion is the sole network-enabled step. Requests below cannot ingest.
    Mock::given(method("GET"))
        .and(wiremock::matchers::path_regex(
            r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/",
        ))
        .respond_with(|request: &wiremock::Request| {
            let query = common::lcd_mock::smart_query_from_request(request);
            let data = if query.get("config").is_some() && request.url.path().contains("terra1mapperfixture1452") {
                json!({"paused":false,"treasury":"terra1treasuryfixture1452","fee_wrap_bps":100,"fee_unwrap_bps":100})
            } else if query.get("config").is_some() && request.url.path().contains("terra1treasuryfixture1452") {
                json!({"wrapping_paused":false})
            } else if query.get("config").is_some() && request.url.path().contains("terra1routerfixture1452") {
                json!({"wrap_mapper":"terra1mapperfixture1452"})
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
            ResponseTemplate::new(200).set_body_json(json!({"data":data}))
        })
        .mount(&upstream)
        .await;
    seed_reference_marks(&pool, 100).await;
    let ingestion_started = Instant::now();
    cl8y_dex_indexer::api::local_quote::ingest_once(&state)
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

    // Controlled observation ages exercise the old 12/15/30/90-second cache boundaries
    // without sleeping or changing economic data. Synthetic admission is confined to this
    // fixture; the earlier socket measurements exercised production ingestion admission.
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
    let state = app_state(
        pool.clone(),
        LcdClient::new(vec![upstream.uri()], 1_000, 1, "columbus-5"),
    );
    seed_reference_marks(&pool, 1).await;
    cl8y_dex_indexer::api::local_quote::ingest_once(&state)
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
