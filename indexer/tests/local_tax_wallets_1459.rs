//! Independently scheduled wallet admission beyond the trader census (#1459).
//! This binary owns a disposable DB and uses only public synthetic addresses.
mod common;

use axum_test::TestServer;
use cl8y_dex_indexer::{api::local_quote, lcd::LcdClient};
use common::lcd_mock;
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicU8, Ordering},
    Arc,
};
use wiremock::{
    matchers::{method, path_regex},
    Mock, Request, ResponseTemplate,
};

const ELIGIBLE: &str = "terra1000000000000000000000000000000000001";
const CAPPED: &str = "terra1000000000000000000000000000000000002";
const BLOCKED: &str = "terra1000000000000000000000000000000000003";
const PROTOCOL: &str = "terra1000000000000000000000000000000000004";
const ENROLLED: &str = "terra1000000000000000000000000000000000006";
const MANAGER: &str = "terra1000000000000000000000000000000000007";
const SECOND_PAGE: &str = "terra1000000000000000000000000000000000010";
const UNKNOWN: &str = "terra1000000000000000000000000000000000005";

#[tokio::test]
async fn scheduled_uncensused_holders_keep_tax_identity_caps_blackout_and_zero_request_rpc() {
    // This dedicated integration process has one wallet test. Restore an inherited
    // private list without printing it; the synthetic enrolled buyer has zero balance
    // and appears in neither the trader census nor CW20 account enumeration.
    struct EnrollmentGuard(Option<std::ffi::OsString>);
    impl Drop for EnrollmentGuard {
        fn drop(&mut self) {
            if let Some(value) = self.0.take() {
                std::env::set_var("LOCAL_QUOTE_WALLETS", value);
            } else {
                std::env::remove_var("LOCAL_QUOTE_WALLETS");
            }
        }
    }
    let _enrollment = EnrollmentGuard(std::env::var_os("LOCAL_QUOTE_WALLETS"));
    std::env::set_var("LOCAL_QUOTE_WALLETS", format!("{ENROLLED},{MANAGER}"));
    let pool = common::setup_pool().await;
    let mut seeded = common::seed_route_solve_with_mirror(&pool).await;
    for token in [
        &mut seeded.token_a,
        &mut seeded.token_b,
        &mut seeded.token_c,
    ] {
        let valid = format!("{token}0000000000000000000000");
        sqlx::query("UPDATE assets SET contract_address=$1 WHERE contract_address=$2")
            .bind(&valid)
            .bind(&*token)
            .execute(&pool)
            .await
            .unwrap();
        *token = valid;
    }
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM traders WHERE address=ANY($1)")
        .bind(vec![ELIGIBLE, CAPPED, BLOCKED, PROTOCOL, ENROLLED, MANAGER])
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(
        count, 0,
        "holder coverage must not depend on trader enrollment"
    );
    sqlx::query("INSERT INTO oracle_state(ticker,price_usd,as_of,quality,accepted_sources) VALUES('ustc',1,NOW(),'fresh',ARRAY['mexc','coingecko']) ON CONFLICT(ticker) DO UPDATE SET as_of=NOW(),quality='fresh'")
        .execute(&pool).await.unwrap();
    sqlx::query("INSERT INTO economic_token_marks(contract_address,anchor_ticker,price_usd,as_of) SELECT contract_address,'custc',1,NOW() FROM assets WHERE contract_address IS NOT NULL ON CONFLICT(contract_address) DO UPDATE SET as_of=NOW()")
        .execute(&pool).await.unwrap();
    let provider = lcd_mock::start_ordinary_code_mock().await;
    let taxed = seeded.token_b.clone();
    let tax_code_token = taxed.clone();
    Mock::given(method("GET")).and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+$"))
        .respond_with(move |r: &Request| {
            let address = r.url.path().rsplit('/').next().unwrap();
            ResponseTemplate::new(200).set_body_json(json!({"contract_info":{"code_id":if address == tax_code_token {"11667"} else {"10184"},"admin":""}}))
        }).with_priority(1).mount(&provider).await;
    let mode = Arc::new(AtomicU8::new(0));
    let observed_mode = mode.clone();
    Mock::given(method("GET")).and(path_regex(r"/cosmwasm/wasm/v1/contract/.*/smart/.*"))
        .respond_with(move |request: &Request| {
            let q = lcd_mock::smart_query_from_request(request);
            let mode = observed_mode.load(Ordering::Relaxed);
            let data = if q.get("config").is_some() { json!({"funding_version":1,"factory":"terra1factory"}) }
                else if q.get("get_config").is_some() { json!({"factory":"terra1factory","router":"terra1routertest","buy_bps":250,"sell_bps":500,"treasury":"terra1treasury","sinks":[],"launch_guards":{"trading_enabled":true,"cooldown_blocks":if mode==1 {1}else{0},"max_wallet":"100000"}}) }
                else if q.get("get_features").is_some() { json!({"launch_guards":true,"auto_v2_lp":mode==2,"split_router":false,"exemption_directory":true}) }
                else if q.get("route_tax_capability").is_some() { json!({"funding_version":1,"router":"terra1routertest","buy_bps":250,"sell_bps":500}) }
                else if q.get("route_tax_preview").is_some() { json!({"funding_version":1,"gross":"0","economic":"0","net":"0","tax":"0"}) }
                else if q.get("all_accounts").is_some() {
                    if q["all_accounts"]["start_after"].is_null() { json!({"accounts":[ELIGIBLE,CAPPED,BLOCKED,PROTOCOL]}) }
                    else if q["all_accounts"]["start_after"]==PROTOCOL { json!({"accounts":[SECOND_PAGE,"terra1000000000000000000000000000000000011","terra1000000000000000000000000000000000012","terra1000000000000000000000000000000000013"]}) }
                    else { json!({"accounts":[]}) }
                }
                else if let Some(flags) = q.get("is_protocol_exempt") { json!({"address":flags["address"],"protocol":flags["address"]==PROTOCOL,"manager":flags["address"]==CAPPED || flags["address"]==MANAGER}) }
                else if let Some(balance) = q.get("balance") { json!({"balance":if balance["address"]==CAPPED {"99999"}else{"0"}}) }
                else if q.get("is_paused").is_some() { json!({"paused":false}) }
                else if q.get("get_hooks").is_some() { json!({"hooks":[]}) }
                else if q.get("get_discount_registry").is_some() { json!({"registry":null}) }
                else if q.get("is_code_id_whitelisted").is_some() { json!({"whitelisted":true}) }
                else if q.get("get_asset_code_ids").is_some() { json!({"code_ids":[10184,11667]}) }
                else if let Some(blacklist) = q.get("blacklist_check") { json!({"wallet_blacklisted":blacklist["wallet"]==BLOCKED,"blacklisted_tokens":[],"blacklisted_pairs":[],"blocked":false}) }
                else { return ResponseTemplate::new(500); };
            ResponseTemplate::new(200).set_body_json(json!({"data":data}))
        }).mount(&provider).await;
    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![lcd_mock::lcd_base_url(&provider)];
    cfg.router_address = Some("terra1routertest".into());
    cfg.trading_route_tax_code_ids.insert(11667);
    let policy = cl8y_dex_indexer::lcd::ProviderPolicy {
        attempt_timeout_ms: 5_000,
        deadline_ms: 5_000,
        requests_per_second: 1024,
        burst: 1024,
        ..cfg.lcd_provider_policy.clone()
    };
    let lcd = LcdClient::new_with_policy(
        cfg.lcd_urls.clone(),
        5_000,
        1,
        cfg.expected_chain_id.clone(),
        policy,
    );
    let state = common::local_quotes::state(pool.clone(), &cfg, lcd);
    local_quote::ingest_once(&state).await.unwrap();
    let quote = |actor: &str| {
        serde_json::from_value(json!({"token_in":seeded.token_a,"token_out":seeded.token_b,"amount_in":"1000","pool_only":true,"sender":actor})).unwrap()
    };
    let successful = local_quote::solve(&state, &quote(ELIGIBLE))
        .await
        .unwrap()
        .0;
    assert_eq!(successful["tax_kind"], "route_local_v1");
    assert_eq!(successful["buy_tax_bps"], 250);
    assert_eq!(successful["execution_metadata"]["sender"], ELIGIBLE);
    let enrolled = local_quote::solve(&state, &quote(ENROLLED))
        .await
        .unwrap()
        .0;
    assert_eq!(enrolled["execution_metadata"]["sender"], ENROLLED);
    assert_eq!(enrolled["buy_tax_bps"], 250);
    for actor in [CAPPED, BLOCKED, PROTOCOL, UNKNOWN] {
        assert!(
            local_quote::solve(&state, &quote(actor)).await.is_err(),
            "unsafe actor unexpectedly quoted: {actor}"
        );
    }
    let manager = local_quote::solve(&state, &quote(MANAGER)).await.unwrap().0;
    assert_eq!(manager["buy_tax_bps"], 0);
    // Page-two and terminal observations cannot replace page-one wallet state.
    for _ in 0..2 {
        local_quote::ingest_once(&state).await.unwrap();
        assert!(local_quote::solve(&state, &quote(ELIGIBLE)).await.is_ok());
        assert!(local_quote::solve(&state, &quote(SECOND_PAGE))
            .await
            .is_ok());
    }
    for unsupported in [1, 2] {
        mode.store(unsupported, Ordering::Relaxed);
        local_quote::ingest_once(&state).await.unwrap();
        assert!(
            local_quote::solve(&state, &quote(ELIGIBLE)).await.is_err(),
            "stateful unsupported mode became executable"
        );
        assert!(
            local_quote::solve(&state, &quote(MANAGER)).await.is_err(),
            "manager exemption waived stateful unsupported admission"
        );
    }
    mode.store(0, Ordering::Relaxed);
    sqlx::query("UPDATE oracle_state SET as_of=NOW()")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query("UPDATE economic_token_marks SET as_of=NOW()")
        .execute(&pool)
        .await
        .unwrap();
    // A terminal holder page resets the crawl; the next pass observes its first page.
    local_quote::ingest_once(&state).await.unwrap();
    local_quote::ingest_once(&state).await.unwrap();
    local_quote::solve(&state, &quote(ELIGIBLE))
        .await
        .expect("supported holder policy must recover after fresh scheduled observations");
    assert!(local_quote::solve(&state, &quote(SECOND_PAGE))
        .await
        .is_ok());
    provider.reset().await;
    let server = TestServer::new(cl8y_dex_indexer::api::build_router(state.clone(), &cfg));
    for actor in [ELIGIBLE, ELIGIBLE, ENROLLED, SECOND_PAGE] {
        let response = server.get(&format!("/api/v1/route/solve?token_in={}&token_out={}&amount_in=1000&pool_only=true&sender={actor}",seeded.token_a,seeded.token_b)).await;
        response.assert_status_ok();
        let body: Value = response.json();
        assert_eq!(body["buy_tax_bps"], 250);
    }
    assert!(local_quote::solve(&state, &quote(BLOCKED)).await.is_err());
    assert!(local_quote::solve(&state, &quote(UNKNOWN)).await.is_err());
    assert_eq!(
        provider.received_requests().await.unwrap().len(),
        0,
        "quotes/cache/missing-wallet discovery performed provider I/O"
    );
    // Executed scheduled blackout removes failed admission and keeps known negatives.
    local_quote::ingest_once(&state).await.unwrap();
    assert!(local_quote::solve(&state, &quote(ELIGIBLE)).await.is_err());
    assert!(local_quote::solve(&state, &quote(BLOCKED)).await.is_err());
}
