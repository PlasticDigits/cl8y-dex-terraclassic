//! #1464: required tax policy and optional wallets own independent admission phases.
mod common;
#[path = "common/local_tax_quotes.rs"]
mod local_tax_quotes;

use axum_test::TestServer;
use cl8y_dex_indexer::api::{build_router, local_quote};
use common::lcd_mock::smart_query_from_request;
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicU8, AtomicUsize, Ordering},
    Arc,
};
use wiremock::{
    matchers::{method, path_regex},
    Mock, Request, ResponseTemplate,
};

async fn fixture() -> (
    cl8y_dex_indexer::api::AppState,
    cl8y_dex_indexer::config::Config,
    common::RouteSolveSeed,
    wiremock::MockServer,
) {
    let pool = common::setup_pool().await;
    let initial = common::seed_route_solve_with_mirror(&pool).await;
    let seed = local_tax_quotes::pad_seed(&pool, initial).await;
    local_tax_quotes::seed_reserves(&pool).await;
    local_tax_quotes::seed_marks(&pool).await;
    let provider = local_tax_quotes::mock_tax_lcd(&seed.token_b, Some(1), false).await;
    let mut config = local_tax_quotes::config(&provider, true);
    // Repeated cold passes exercise policy transitions without modeling a
    // provider rate outage. This finite fixture budget never changes production.
    config.lcd_provider_policy.burst = 1024;
    config.lcd_cooldown_ms = 1;
    config.lcd_provider_policy.max_backoff_ms = 1;
    config.lcd_timeout_ms = 5000;
    config.lcd_provider_policy.attempt_timeout_ms = 5000;
    config.lcd_provider_policy.deadline_ms = 8000;
    let lcd = cl8y_dex_indexer::lcd::LcdClient::new_with_policy(
        config.lcd_urls.clone(),
        5000,
        1,
        "columbus-5",
        config.lcd_provider_policy.clone(),
    );
    let state = common::local_quotes::state(pool, &config, lcd);
    (state, config, seed, provider)
}
fn request(seed: &common::RouteSolveSeed) -> String {
    format!(
        "/api/v1/route/solve?token_in={}&token_out={}&amount_in=1000000&pool_only=true",
        seed.token_a, seed.token_b
    )
}
fn assert_useful(body: &Value) {
    assert!(
        body["estimated_amount_out_net"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            > 0
    );
    assert!(!body["router_operations"].as_array().unwrap().is_empty());
    assert!(
        body["execution_metadata"]["minimum_receive"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            > 0
    );
    assert_eq!(body["tax_funding_version"], 1);
}

#[tokio::test]
#[serial_test::serial]
async fn recovery1464_completed_tax_outage_preserves_original_policy_and_actual_unsupported_invalidates(
) {
    let (state, config, seed, provider) = fixture().await;
    let phase = Arc::new(AtomicU8::new(0));
    let observed_changes = Arc::new(AtomicUsize::new(0));
    let feature_changes = observed_changes.clone();
    let controlled = phase.clone();
    Mock::given(method("GET")).and(path_regex(r"/cosmwasm/wasm/v1/contract/.*/smart/.*"))
        .and(|req:&Request|smart_query_from_request(req).get("get_features").is_some())
        .respond_with(move |_:&Request| {
            let current=controlled.load(Ordering::SeqCst);
            if [2,4].contains(&current) { feature_changes.fetch_or(1 << current, Ordering::SeqCst); }
            if current==1 {return ResponseTemplate::new(429).insert_header("retry-after","0");}
            if current==3 {return ResponseTemplate::new(503);}
            ResponseTemplate::new(200).set_body_json(json!({"data":{"launch_guards":false,"auto_v2_lp":current==2,"split_router":false,"exemption_directory":current==4}}))
        }).with_priority(1).mount(&provider).await;
    let config_phase = phase.clone();
    let config_changes = observed_changes.clone();
    Mock::given(method("GET")).and(path_regex(r"/cosmwasm/wasm/v1/contract/.*/smart/.*"))
        .and(|req:&Request|smart_query_from_request(req).get("get_config").is_some())
        .respond_with(move |_:&Request| {
            if config_phase.load(Ordering::SeqCst)==3 { config_changes.fetch_or(1 << 3, Ordering::SeqCst); }
            ResponseTemplate::new(200).set_body_json(json!({"data":{
            "router":local_tax_quotes::ROUTER,"factory":"terra1factory","buy_bps":450,"sell_bps":100,
            "treasury":if config_phase.load(Ordering::SeqCst)==3 {"terra1changedtreasury"}else{"terra1treasury1381"},
            "autolp":null,"launch_guards":null,"sinks":[]
        }}))}).with_priority(1).mount(&provider).await;
    let flag_phase = phase.clone();
    Mock::given(method("GET"))
        .and(path_regex(r"/cosmwasm/wasm/v1/contract/.*/smart/.*"))
        .and(move |req: &Request| {
            flag_phase.load(Ordering::SeqCst) == 4
                && smart_query_from_request(req)
                    .get("is_protocol_exempt")
                    .is_some()
        })
        .respond_with(ResponseTemplate::new(503))
        .with_priority(1)
        .mount(&provider)
        .await;
    local_quote::ingest_once(&state).await.unwrap();
    let server = TestServer::new(build_router(state.clone(), &config));
    let first = server.get(&request(&seed)).await;
    first.assert_status_ok();
    let first: Value = first.json();
    assert_useful(&first);
    for _ in 0..2 {
        phase.store(1, Ordering::SeqCst);
        local_quote::ingest_once(&state).await.unwrap();
        let recovered = server.get(&request(&seed)).await;
        recovered.assert_status_ok();
        let body: Value = recovered.json();
        assert_useful(&body);
        assert_eq!(
            body["execution_metadata"]["expires_at_ms"],
            first["execution_metadata"]["expires_at_ms"]
        );
        assert_eq!(
            body["execution_metadata"]["observed_at_ms"],
            first["execution_metadata"]["observed_at_ms"]
        );
        phase.store(0, Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
    }
    phase.store(2, Ordering::SeqCst);
    local_quote::ingest_once(&state).await.unwrap();
    let rejected = server.get(&request(&seed)).await;
    rejected.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(rejected.json::<Value>()["error"], "unsupported_semantics");
    assert!(!rejected.text().contains("estimated_amount_out"));
    phase.store(0, Ordering::SeqCst);
    local_quote::ingest_once(&state).await.unwrap();
    server.get(&request(&seed)).await.assert_status_ok();
    for changed in [3, 4] {
        phase.store(changed, Ordering::SeqCst);
        local_quote::ingest_once(&state).await.unwrap();
        let rejected = server.get(&request(&seed)).await;
        rejected.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(rejected.json::<Value>()["error"], "unsupported_semantics");
        assert!(
            !rejected.text().contains("estimated_amount_out"),
            "completed policy change cannot hide behind a later provider outage"
        );
        phase.store(0, Ordering::SeqCst);
        tokio::time::sleep(std::time::Duration::from_millis(2)).await;
        local_quote::ingest_once(&state).await.unwrap();
        server.get(&request(&seed)).await.assert_status_ok();
    }
    assert_eq!(
        observed_changes.load(Ordering::SeqCst),
        (1 << 2) | (1 << 3) | (1 << 4)
    );
    assert_no_capacity_deferrals(&state).await;
}

async fn assert_no_capacity_deferrals(state: &cl8y_dex_indexer::api::AppState) {
    let counters = state.lcd.provider_counters().await;
    assert!(
        counters
            .iter()
            .all(|counter| counter.rate_deferrals == 0 && counter.concurrency_deferrals == 0),
        "controlled policy fixture exhausted provider capacity: {counters:?}"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn recovery1464_static_tax_baseline_precedes_slow_optional_actor_and_quote_has_zero_rpc() {
    use bech32::ToBase32;
    let (state, config, seed, provider) = fixture().await;
    let actor = bech32::encode("terra", [7u8; 20].to_base32(), bech32::Variant::Bech32).unwrap();
    sqlx::query("INSERT INTO traders(address,tier_id,registered) VALUES($1,255,false)")
        .bind(&actor)
        .execute(&state.pool)
        .await
        .unwrap();
    let actor_started = Arc::new(tokio::sync::Notify::new());
    let started = actor_started.clone();
    let actor_match = actor.clone();
    Mock::given(method("GET")).and(path_regex(r"/cosmwasm/wasm/v1/contract/.*/smart/.*"))
        .and(move |req:&Request| smart_query_from_request(req)["is_protocol_exempt"]["address"]==actor_match)
        .respond_with(move |req:&Request| {
            started.notify_one();let q=smart_query_from_request(req);
            ResponseTemplate::new(200).set_body_json(json!({"data":{"address":q["is_protocol_exempt"]["address"],"manager":false,"protocol":false}})).set_delay(std::time::Duration::from_secs(12))
        }).with_priority(1).mount(&provider).await;
    let worker_state = state.clone();
    let worker = tokio::spawn(async move { local_quote::ingest_once(&worker_state).await });
    tokio::time::timeout(std::time::Duration::from_secs(5), actor_started.notified())
        .await
        .unwrap();
    assert!(!worker.is_finished(), "wallet read remains in progress");
    let before = provider.received_requests().await.unwrap().len();
    let server = TestServer::new(build_router(state.clone(), &config));
    let anonymous = server.get(&request(&seed)).await;
    anonymous.assert_status_ok();
    assert_useful(&anonymous.json::<Value>());
    assert_eq!(
        before,
        provider.received_requests().await.unwrap().len(),
        "quote must not trigger policy RPC"
    );
    let named = server
        .get(&format!("{}&trader={actor}", request(&seed)))
        .await;
    named.assert_status(axum::http::StatusCode::SERVICE_UNAVAILABLE);
    assert!(!named.text().contains("estimated_amount_out"));
    worker.abort();
    assert!(worker.await.unwrap_err().is_cancelled());
}

#[tokio::test]
#[serial_test::serial]
async fn recovery1464_known_bad_preview_revokes_before_later_outage_unknown_alternative_stays_local(
) {
    let (state, config, seed, provider) = fixture().await;
    let server = TestServer::new(build_router(state.clone(), &config));
    local_quote::ingest_once(&state).await.unwrap();
    server.get(&request(&seed)).await.assert_status_ok();
    sqlx::query("INSERT INTO pairs(contract_address,asset_0_id,asset_1_id,lp_token,fee_bps) SELECT 'terra1pairroutebcd',b.id,c.id,'terra1lproutebc',30 FROM assets b,assets c WHERE b.contract_address=$1 AND c.contract_address=$2")
        .bind(&seed.token_b).bind(&seed.token_c).execute(&state.pool).await.unwrap();
    local_tax_quotes::seed_reserves(&state.pool).await;
    let phase = Arc::new(AtomicU8::new(1));
    let controlled = phase.clone();
    let negative_reads = Arc::new(AtomicUsize::new(0));
    let observed_negative = negative_reads.clone();
    let later_reads = Arc::new(AtomicUsize::new(0));
    let observed_later = later_reads.clone();
    Mock::given(method("GET")).and(path_regex(r"/cosmwasm/wasm/v1/contract/.*/smart/.*"))
        .and(|req:&Request| smart_query_from_request(req).get("route_tax_preview").is_some())
        .respond_with(move |req:&Request| {
            let q=smart_query_from_request(req); let pair=q["route_tax_preview"]["pair"].as_str().unwrap();
            let current=controlled.load(Ordering::SeqCst);
            if (current==1 && pair=="terra1pairroutebcd") || (current==2 && pair=="terra1pairrouteabc") {
                observed_negative.fetch_add(1,Ordering::SeqCst);
                return ResponseTemplate::new(200).set_body_json(json!({"data":{"funding_version":0}}));
            }
            if current==2 && pair=="terra1pairroutebcd" {
                observed_later.fetch_add(1,Ordering::SeqCst);
                return ResponseTemplate::new(503);
            }
            ResponseTemplate::new(200).set_body_json(json!({"data":{"funding_version":1,"gross":"0","economic":"0","net":"0","tax":"0"}}))
        }).with_priority(1).mount(&provider).await;
    // An unregistered alternative cannot erase a healthy registered route.
    local_quote::ingest_once(&state).await.unwrap();
    let useful = server.get(&request(&seed)).await;
    useful.assert_status_ok();
    assert_useful(&useful.json::<Value>());
    assert!(negative_reads.load(Ordering::SeqCst) > 0);
    phase.store(0, Ordering::SeqCst);
    local_quote::ingest_once(&state).await.unwrap();
    server.get(&request(&seed)).await.assert_status_ok();
    negative_reads.store(0, Ordering::SeqCst);
    phase.store(2, Ordering::SeqCst);
    local_quote::ingest_once(&state).await.unwrap();
    let rejected = server.get(&request(&seed)).await;
    rejected.assert_status(axum::http::StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(rejected.json::<Value>()["error"], "unsupported_semantics");
    assert!(!rejected.text().contains("estimated_amount_out"));
    assert!(
        negative_reads.load(Ordering::SeqCst) > 0,
        "known negative was read"
    );
    assert_eq!(
        later_reads.load(Ordering::SeqCst),
        0,
        "known negative revokes before another required await"
    );
    assert_no_capacity_deferrals(&state).await;
}

#[tokio::test]
#[serial_test::serial]
async fn recovery1464_manager_revocation_survives_cancelled_balance_then_nonexempt_recovers() {
    use bech32::ToBase32;
    let (state, config, seed, provider) = fixture().await;
    let actor = bech32::encode("terra", [9u8; 20].to_base32(), bech32::Variant::Bech32).unwrap();
    sqlx::query("INSERT INTO traders(address,tier_id,registered) VALUES($1,255,false)")
        .bind(&actor)
        .execute(&state.pool)
        .await
        .unwrap();
    let phase = Arc::new(AtomicU8::new(0));
    let flag_phase = phase.clone();
    let flag_actor = actor.clone();
    let flag_reads = Arc::new(AtomicUsize::new(0));
    let observed_flags = flag_reads.clone();
    Mock::given(method("GET")).and(path_regex(r"/cosmwasm/wasm/v1/contract/.*/smart/.*"))
        .and(move |req:&Request| smart_query_from_request(req)["is_protocol_exempt"]["address"]==flag_actor)
        .respond_with(move |req:&Request| {
            let current = flag_phase.load(Ordering::SeqCst);
            if current==1 { observed_flags.fetch_add(1,Ordering::SeqCst); }
            ResponseTemplate::new(200).set_body_json(json!({"data":{"address":smart_query_from_request(req)["is_protocol_exempt"]["address"],"protocol":false,"manager":current==0}}))
        }).with_priority(1).mount(&provider).await;
    let balance_started = Arc::new(tokio::sync::Notify::new());
    let started = balance_started.clone();
    let balance_phase = phase.clone();
    let balance_actor = actor.clone();
    Mock::given(method("GET"))
        .and(path_regex(r"/cosmwasm/wasm/v1/contract/.*/smart/.*"))
        .and(move |req: &Request| {
            smart_query_from_request(req)["balance"]["address"] == balance_actor
        })
        .respond_with(move |_: &Request| {
            let response =
                ResponseTemplate::new(200).set_body_json(json!({"data":{"balance":"0"}}));
            if balance_phase.load(Ordering::SeqCst) == 1 {
                started.notify_one();
                response.set_delay(std::time::Duration::from_secs(64))
            } else {
                response
            }
        })
        .with_priority(1)
        .mount(&provider)
        .await;
    let server = TestServer::new(build_router(state.clone(), &config));
    let named_request = format!("{}&sender={actor}", request(&seed));
    local_quote::ingest_once(&state).await.unwrap();
    let initial = server.get(&named_request).await;
    initial.assert_status_ok();
    assert_eq!(initial.json::<Value>()["buy_tax_bps"], 0);
    phase.store(1, Ordering::SeqCst);
    let worker_state = state.clone();
    let worker = tokio::spawn(async move { local_quote::ingest_once(&worker_state).await });
    tokio::time::timeout(
        std::time::Duration::from_secs(5),
        balance_started.notified(),
    )
    .await
    .unwrap();
    assert!(
        flag_reads.load(Ordering::SeqCst) > 0,
        "manager=false response completed before balance wait"
    );
    assert!(!worker.is_finished());
    let before = provider.received_requests().await.unwrap().len();
    let revoked = server.get(&named_request).await;
    revoked.assert_status(axum::http::StatusCode::SERVICE_UNAVAILABLE);
    assert!(!revoked.text().contains("estimated_amount_out"));
    server
        .get(&request(&seed))
        .await
        .assert_status(axum::http::StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(
        before,
        provider.received_requests().await.unwrap().len(),
        "revocation check is local"
    );
    worker.abort();
    assert!(worker.await.unwrap_err().is_cancelled());
    server
        .get(&named_request)
        .await
        .assert_status(axum::http::StatusCode::SERVICE_UNAVAILABLE);
    phase.store(2, Ordering::SeqCst);
    local_quote::ingest_once(&state).await.unwrap();
    let recovered = server.get(&named_request).await;
    recovered.assert_status_ok();
    let body: Value = recovered.json();
    assert_eq!(body["buy_tax_bps"], 450);
    assert_useful(&body);
    assert_no_capacity_deferrals(&state).await;
}
