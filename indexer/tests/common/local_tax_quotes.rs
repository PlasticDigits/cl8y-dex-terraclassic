//! Scheduled synthetic v1 admission shared by tax and receive-sizing API regression tests.
use crate::common;
use cl8y_dex_indexer::{
    api::{build_router, local_quote},
    config::Config,
    lcd::LcdClient,
};
use common::lcd_mock::smart_query_from_request;
use serde_json::json;
use sqlx::PgPool;
use wiremock::{
    matchers::{method, path_regex},
    Mock, MockServer, Request, ResponseTemplate,
};
pub const ROUTER: &str = "terra1router1381";

pub async fn mock_tax_lcd(taxed: &str, version: Option<u16>, fail: bool) -> MockServer {
    let mock = MockServer::start().await;
    common::lcd_mock::mock_chain_identity(&mock, "columbus-5").await;
    let tax = taxed.to_owned();
    Mock::given(method("GET")).and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+$"))
        .respond_with(move |req:&Request| {
            let addr=req.url.path().rsplit('/').next().unwrap();
            ResponseTemplate::new(200).set_body_json(json!({"contract_info":{"code_id":if addr==tax {"11667"} else {"10184"},"admin":"terra1governance"}}))
        }).mount(&mock).await;
    Mock::given(method("GET")).and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/.+$"))
        .respond_with(move |req:&Request| {
            let q=smart_query_from_request(req);
            let addr=req.url.path().split('/').nth(5).unwrap();
            let data=if q.get("route_tax_capability").is_some() {
                if fail {return ResponseTemplate::new(503);}
                if version.is_none() {return ResponseTemplate::new(500).set_body_string("query wasm contract failed: unknown variant");}
                json!({"funding_version":version,"router":ROUTER,"buy_bps":450,"sell_bps":100})
            } else if q.get("get_config").is_some() {
                json!({"router":ROUTER,"factory":"terra1factory","buy_bps":450,"sell_bps":100,"treasury":"terra1treasury1381","autolp":null,"launch_guards":null,"sinks":[]})
            } else if q.get("get_features").is_some() {json!({"launch_guards":false,"auto_v2_lp":false,"split_router":false})}
            else if q.get("config").is_some() {json!({"funding_version":1,"factory":"terra1factory"})}
            else if let Some(actor)=q.get("is_protocol_exempt") {json!({"address":actor["address"],"manager":false,"protocol":false})}
            else if q.get("balance").is_some() {json!({"balance":"0"})}
            else if q.get("route_tax_preview").is_some() {json!({"funding_version":1,"gross":"0","economic":"0","tax":"0","net":"0"})}
            else if q.get("is_paused").is_some() {json!({"paused":false})}
            else if q.get("get_hooks").is_some() {json!({"hooks":[]})}
            else if q.get("get_asset_code_ids").is_some() {
                let ids=if addr.ends_with("abc")||addr.ends_with("mpab") {[10184,11667]} else if addr.ends_with("bcd")||addr.ends_with("mpbc") {[11667,10184]} else {[10184,10184]};
                json!({"code_ids":ids})
            } else if q.get("blacklist_check").is_some() {json!({"wallet_blacklisted":false,"blacklisted_tokens":[],"blacklisted_pairs":[]})}
            else if q.get("is_code_id_whitelisted").is_some() {json!({"whitelisted":true})}
            else if q.get("get_discount_registry").is_some() {json!({"registry":null})}
            else {return ResponseTemplate::new(500).set_body_string("unexpected quote-triggered RPC");};
            ResponseTemplate::new(200).set_body_json(json!({"data":data}))
        }).mount(&mock).await;
    mock
}
pub fn config(mock: &MockServer, trusted: bool) -> Config {
    let mut cfg = common::test_config();
    cfg.router_address = Some(ROUTER.into());
    cfg.lcd_urls = vec![mock.uri()];
    cfg.trading_tax_free_code_ids.insert(10184);
    if trusted {
        cfg.trading_route_tax_code_ids.insert(11667);
    }
    cfg.community_tax_code_id = Some(11630);
    cfg
}
/// Marks are independently produced fixture observations, never route-derived request values.
pub async fn seed_marks(pool: &PgPool) {
    sqlx::query("INSERT INTO oracle_state(ticker,price_usd,quality,accepted_sources,as_of) VALUES('ustc',1,'fresh',ARRAY['mexc','coingecko'],NOW()) ON CONFLICT(ticker) DO UPDATE SET as_of=NOW()")
        .execute(pool).await.unwrap();
    sqlx::query("INSERT INTO economic_token_marks(contract_address,asset_id,price_usd,as_of,anchor_ticker) SELECT contract_address,id,1,NOW(),'custc' FROM assets WHERE contract_address IS NOT NULL ON CONFLICT(contract_address) DO UPDATE SET price_usd=EXCLUDED.price_usd,as_of=NOW()")
        .execute(pool).await.unwrap();
}
pub async fn seed_reserves(pool: &PgPool) {
    for pair in cl8y_dex_indexer::db::queries::pairs::get_all_pairs(pool)
        .await
        .unwrap()
    {
        cl8y_dex_indexer::db::queries::pair_reserves::upsert_pair_reserves(
            pool,
            pair.id,
            &"10000000000000".parse().unwrap(),
            &"10000000000000".parse().unwrap(),
            30,
            Some(1381),
        )
        .await
        .unwrap();
    }
}
pub async fn app(pool: PgPool, cfg: Config, mock: &MockServer) -> axum::Router {
    seed_marks(&pool).await;
    let lcd = LcdClient::new_with_policy(
        cfg.lcd_urls.clone(),
        cfg.lcd_timeout_ms,
        cfg.lcd_cooldown_ms,
        cfg.expected_chain_id.clone(),
        cfg.lcd_provider_policy.clone(),
    );
    let state = common::local_quotes::state(pool, &cfg, lcd);
    local_quote::ingest_fixture(&state).await.unwrap();
    let ingested = mock.received_requests().await.unwrap();
    assert!(
        !ingested.is_empty(),
        "fixture must exercise scheduled admission"
    );
    mock.reset().await;
    // Every request after this point is observable and has no usable LCD responses.
    build_router(state, &cfg)
}
pub async fn assert_no_request_rpc(mock: &MockServer) {
    assert!(
        mock.received_requests().await.unwrap().is_empty(),
        "local quote touched provider after scheduled ingestion"
    );
}
pub fn pool_net(input: u128, reserve_in: u128, reserve_out: u128) -> u128 {
    let gross = input * reserve_out / (reserve_in + input);
    gross - gross * 30 / 10000
}
pub fn taxed_two_hop(input: u128) -> u128 {
    let first = pool_net(input, 10_000_000_000_000, 10_000_000_000_000);
    let after_buy = first - first * 450 / 10000;
    let economic = ((after_buy + 1) * 10000 - 1) / 10100;
    pool_net(economic, 10_000_000_000_000, 10_000_000_000_000)
}
/// Legacy short synthetic names predate the persisted economic mark address guard.
pub async fn pad_seed(pool: &PgPool, mut seed: common::RouteSolveSeed) -> common::RouteSolveSeed {
    for token in [&mut seed.token_a, &mut seed.token_b, &mut seed.token_c]
        .into_iter()
        .chain(seed.token_d.iter_mut())
        .chain(seed.token_e.iter_mut())
    {
        let padded = format!("{token:0<44}");
        sqlx::query("UPDATE assets SET contract_address=$1 WHERE contract_address=$2")
            .bind(&padded)
            .bind(&*token)
            .execute(pool)
            .await
            .unwrap();
        *token = padded;
    }
    seed
}
