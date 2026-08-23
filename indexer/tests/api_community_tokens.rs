//! GitLab #594 — community tax catalog API.

mod common;

use axum_test::TestServer;
use cl8y_dex_indexer::indexer::community_tokens::{
    ingest_event, parse_community_event, ParsedCommunityEvent,
};
use cl8y_dex_indexer::lcd::Attribute;
use serde_json::Value;
use serial_test::serial;

const LAUNCHER: &str = "terra1af9xm63mev4hnf4z0nmmcsnd9f4lpac2vs205rmaeg3kdqlqudhq894lyz";
const CMM: &str = "terra16j5u6ey7a84g40sr3gd94nzg5w5fm45046k9s2347qhfpwm5fr6sem3lr2";
const TOKEN: &str = "terra1communitytax00000000000000000000000000001";
const OTHER: &str = "terra1communitytax00000000000000000000000000002";
const MANAGER: &str = "terra1manager000000000000000000000000000000001";

fn attrs(pairs: &[(&str, &str)]) -> Vec<Attribute> {
    pairs
        .iter()
        .map(|(k, v)| Attribute {
            key: (*k).to_string(),
            value: (*v).to_string(),
        })
        .collect()
}

fn configured() -> cl8y_dex_indexer::config::Config {
    let mut c = common::test_config();
    c.community_tax_code_id = Some(11611);
    c.community_token_launcher = Some(LAUNCHER.to_string());
    c.cmm_governance_addr = Some(CMM.to_string());
    c
}

async fn insert_row(
    pool: &sqlx::PgPool,
    addr: &str,
    attested: bool,
    manager: &str,
    launcher_tx: Option<&str>,
) {
    sqlx::query(
        r#"
        INSERT INTO community_tokens (
            contract_address, code_id, wasm_admin, manager, launcher_address,
            buy_bps, sell_bps, transfer_bps, features, treasury, name, symbol, decimals,
            attested_cmm, launcher_tx, instantiate_tx, created_at_block
        ) VALUES ($1, 11611, $2, $3, $4, 100, 200, 0, '{"transfer_tax":true}'::jsonb, $3, 'Demo', 'DEMO', 6, $5, $6, $6, 100)
        "#,
    )
    .bind(addr)
    .bind(if attested { CMM } else { MANAGER })
    .bind(manager)
    .bind(LAUNCHER)
    .bind(attested)
    .bind(launcher_tx)
    .execute(pool)
    .await
    .expect("insert community token");
}

#[tokio::test]
#[serial]
async fn unconfigured_list_is_empty_configured_false() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let resp = server.get("/api/v1/community-tokens").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(body["configured"], false);
    assert_eq!(body["items"], serde_json::json!([]));
}

#[tokio::test]
#[serial]
async fn default_list_omits_unattested() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    insert_row(&pool, TOKEN, true, MANAGER, Some("tx-attested")).await;
    insert_row(&pool, OTHER, false, MANAGER, None).await;
    let app = common::build_test_app_with_price_and_config(pool, None, configured()).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/community-tokens").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(body["configured"], true);
    assert_eq!(body["code_id"], 11611);
    assert_eq!(body["items"].as_array().unwrap().len(), 1);
    assert_eq!(body["items"][0]["contract_address"], TOKEN);
    assert_eq!(body["items"][0]["attested_cmm"], true);

    let ops = server
        .get("/api/v1/community-tokens?include_unattested=1")
        .await;
    ops.assert_status_ok();
    assert_eq!(ops.json::<Value>()["items"].as_array().unwrap().len(), 2);
}

#[tokio::test]
#[serial]
async fn manager_filter_is_case_insensitive() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    insert_row(&pool, TOKEN, true, MANAGER, Some("tx1")).await;
    let app = common::build_test_app_with_price_and_config(pool, None, configured()).await;
    let server = TestServer::new(app);
    let resp = server
        .get(&format!(
            "/api/v1/community-tokens?manager={}",
            MANAGER.to_ascii_uppercase()
        ))
        .await;
    resp.assert_status_ok();
    assert_eq!(resp.json::<Value>()["items"].as_array().unwrap().len(), 1);
}

#[tokio::test]
#[serial]
async fn pagination_and_unknown_detail() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, configured()).await;
    let server = TestServer::new(app);

    server
        .get("/api/v1/community-tokens?offset=10001")
        .await
        .assert_status_bad_request();

    let clamped = server.get("/api/v1/community-tokens?limit=-1").await;
    clamped.assert_status_ok();
    assert_eq!(clamped.json::<Value>()["limit"], 1);

    server
        .get("/api/v1/community-tokens/terra1unknown")
        .await
        .assert_status_not_found();
}

#[tokio::test]
#[serial]
async fn detail_and_events_settings_fee_vs_sku() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    insert_row(&pool, TOKEN, true, MANAGER, Some("create-tx")).await;
    sqlx::query(
        r#"
        INSERT INTO community_token_events (contract_address, txhash, block_height, action, kind, sku, invoice)
        VALUES
          ($1, 'tx-sku', 10, 'enable_feature', 'sku_unlock', 'transfer_tax', '50000000'),
          ($1, 'tx-set', 11, 'update_settings', 'settings_fee', NULL, '50000000')
        "#,
    )
    .bind(TOKEN)
    .execute(&pool)
    .await
    .unwrap();

    let app = common::build_test_app_with_price_and_config(pool, None, configured()).await;
    let server = TestServer::new(app);
    let detail = server.get(&format!("/api/v1/community-tokens/{TOKEN}")).await;
    detail.assert_status_ok();
    let token = detail.json::<Value>()["token"].clone();
    assert_eq!(token["buy_bps"], 100);
    assert_eq!(token["sell_bps"], 200);
    assert_eq!(token["manager"], MANAGER);
    assert_eq!(token["features"]["transfer_tax"], true);

    let events = server
        .get(&format!("/api/v1/community-tokens/{TOKEN}/events"))
        .await;
    events.assert_status_ok();
    let items = events.json::<Value>();
    let kinds: Vec<&str> = items
        .as_array()
        .unwrap()
        .iter()
        .map(|e| e["kind"].as_str().unwrap())
        .collect();
    assert!(kinds.contains(&"sku_unlock"));
    assert!(kinds.contains(&"settings_fee"));
}

#[tokio::test]
#[serial]
async fn tokens_detail_embeds_community_tax_only_when_catalogued() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    insert_row(&pool, TOKEN, true, MANAGER, Some("tx")).await;
    sqlx::query(
        "INSERT INTO assets (contract_address, is_cw20, name, symbol, decimals) VALUES ($1, true, 'Demo', 'DEMO', 6)",
    )
    .bind(TOKEN)
    .execute(&pool)
    .await
    .ok();

    let app = common::build_test_app_with_price_and_config(pool, None, configured()).await;
    let server = TestServer::new(app);
    let resp = server.get(&format!("/api/v1/tokens/{TOKEN}")).await;
    if resp.status_code().is_success() {
        let body: Value = resp.json();
        assert!(body.get("community_tax").is_some());
        assert_eq!(body["community_tax"]["contract_address"], TOKEN);
    }
}

#[test]
fn event_spoof_non_launcher_create_is_ignored() {
    let ev = parse_community_event(&attrs(&[
        ("_contract_address", "terra1attacker"),
        ("action", "create_token_ready"),
        ("community_token", TOKEN),
        ("code_id", "11611"),
    ]))
    .expect("parse");
    assert_eq!(ev.emitter, "terra1attacker");
    assert_eq!(ev.action, "create_token_ready");
}

#[tokio::test]
#[serial]
async fn ingest_ignores_non_launcher_create() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let cfg = configured();
    let ev = ParsedCommunityEvent {
        emitter: "terra1attacker".into(),
        action: "create_token_ready".into(),
        community_token: Some(TOKEN.into()),
        token: None,
        sku: None,
        invoice: None,
    };
    ingest_event(&pool, &cfg, &ev, "spoof", 1).await.unwrap();
    let row = cl8y_dex_indexer::db::queries::community_tokens::get_by_address(&pool, TOKEN)
        .await
        .unwrap();
    assert!(row.is_none(), "non-launcher create must not attest");
}

#[tokio::test]
#[serial]
async fn ingest_launcher_create_without_pair() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let cfg = configured();
    let ev = ParsedCommunityEvent {
        emitter: LAUNCHER.into(),
        action: "create_token_ready".into(),
        community_token: Some(TOKEN.into()),
        token: None,
        sku: None,
        invoice: None,
    };
    ingest_event(&pool, &cfg, &ev, "create-hash", 42)
        .await
        .unwrap();
    let row = cl8y_dex_indexer::db::queries::community_tokens::get_by_address(&pool, TOKEN)
        .await
        .unwrap()
        .expect("row");
    assert_eq!(row.launcher_tx.as_deref(), Some("create-hash"));
    assert_eq!(row.created_at_block, Some(42));
}
