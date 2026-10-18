//! Explicit owned-chain acceptance; run after localterra-cohort-1211.py emits fresh LCD receipts.
mod common;
use axum_test::TestServer;
use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use cl8y_dex_indexer::{
    api::reset_protocol_fees_cache,
    db::queries::{oracle as oracle_db, protocol_fees},
    indexer::{oracle, parser},
    lcd::{LcdClient, TxResponse},
};
use serde_json::{json, Value};
use std::path::Path;

#[tokio::test]
#[ignore = "Requires explicitly prepared owned LocalTerra contracts and fresh actual LCD receipts"]
async fn actual_swap_registration_wrap_ingest_and_public_aggregate() {
    let raw = std::fs::read("/tmp/dex-cohort-1211-receipts.json").unwrap();
    let fixture: Value = serde_json::from_slice(&raw).unwrap();
    assert_eq!(fixture["chain_id"], "localterra");
    let field = |key: &str| fixture[key].as_str().unwrap().to_owned();
    let actor = field("manager");
    let registry = field("registry");
    let mut config = common::test_config(); // loads only the private marked QA credential file
    config.expected_chain_id = "localterra".into();
    config.lcd_urls = vec!["http://127.0.0.1:1317".into()];
    config.factory_address = field("factory");
    config.fee_discount_address = Some(registry.clone());
    config.wrap_mapper_address = Some(field("mapper"));
    config.usdt_cw20_address = field("stable");
    config.ustc_denom = Some("uusd".into());
    let pool = common::setup_pool().await; // this process owns its disposable migrated database
                                           // Explicit owned QA price observations. No real USD feed or production peg is claimed.
    oracle_db::record_observation(
        &pool,
        oracle::OracleTicker::Ustc,
        Some(&1.into()),
        &["mexc".into(), "coingecko".into()],
        "fresh",
    )
    .await
    .unwrap();
    sqlx::query("INSERT INTO economic_token_marks(contract_address,price_usd,anchor_ticker,as_of) VALUES($1,1,'custc',NOW())")
        .bind(field("stable")).execute(&pool).await.unwrap();
    let price = oracle::new_shared_price();
    *price.write().await = Some(1.into());
    let lcd = LcdClient::new(config.lcd_urls.clone(), 1500, 1, "localterra");
    let txs: Vec<TxResponse> = fixture["txs"]
        .as_array()
        .unwrap()
        .iter()
        .cloned()
        .map(|row| serde_json::from_value(row).unwrap())
        .collect();
    assert_eq!(txs.len(), 3);
    let heights: Vec<i64> = txs.iter().map(|tx| tx.height.parse().unwrap()).collect();
    assert!(heights.windows(2).all(|pair| pair[0] < pair[1]));
    assert_eq!(parser::parse_swaps(&txs[0]).len(), 1);
    assert!(parser::parse_swaps(&txs[0])[0]
        .commission_amount
        .as_ref()
        .is_some_and(|v| v > &BigDecimal::from(0)));
    for tx in &txs {
        let height = tx.height.parse().unwrap();
        let time = DateTime::parse_from_rfc3339(tx.timestamp.as_deref().unwrap())
            .unwrap()
            .with_timezone(&Utc);
        parser::process_block_txs(
            &pool,
            &lcd,
            &config,
            std::slice::from_ref(tx),
            height,
            time,
            &price,
        )
        .await
        .unwrap();
    }
    let rows: Vec<(String, String, Option<BigDecimal>, BigDecimal)> = sqlx::query_as(
        "SELECT e.source,a.cohort,e.fee_usd,e.amount_raw FROM protocol_fee_events e JOIN protocol_fee_attributions a ON a.fee_event_id=e.id ORDER BY e.block_height,e.id")
        .fetch_all(&pool).await.unwrap();
    assert_eq!(rows.len(), 2);
    assert_eq!(rows[0].0, "swap_amm");
    assert_eq!(rows[0].1, "retail");
    assert_eq!(rows[1].0, "wrap");
    assert_eq!(rows[1].1, "mm");
    assert!(rows.iter().all(|(_, _, usd, raw)| usd
        .as_ref()
        .is_some_and(|v| v > &BigDecimal::from(0))
        && raw > &BigDecimal::from(0)));
    let snapshots: Vec<(i64, bool, i16)> = sqlx::query_as(
        "SELECT block_height,registered,tier_id FROM fee_registry_snapshots WHERE registry=$1 AND actor=$2 ORDER BY block_height")
        .bind(&registry).bind(&actor).fetch_all(&pool).await.unwrap();
    assert!(snapshots.contains(&(heights[0] - 1, false, 0)));
    assert!(snapshots.contains(&(heights[2] - 1, true, 0)));
    // Replay after the actual registration must preserve the earlier retail fee.
    let tx = &txs[0];
    parser::process_block_txs(
        &pool,
        &lcd,
        &config,
        std::slice::from_ref(tx),
        heights[0],
        Utc::now(),
        &price,
    )
    .await
    .unwrap();
    let cohorts: Vec<String> =
        sqlx::query_scalar("SELECT cohort FROM protocol_fee_attributions ORDER BY fee_event_id")
            .fetch_all(&pool)
            .await
            .unwrap();
    assert_eq!(cohorts, ["retail", "mm"]);
    protocol_fees::refresh_protocol_fees_with_sources(&pool, true, false, false, false)
        .await
        .unwrap();
    reset_protocol_fees_cache();
    let app =
        common::build_test_app_with_price_and_config(pool.clone(), Some(1.into()), config).await;
    let server = TestServer::new(app);
    let response = server.get("/api/v1/protocol/fees?window=24h").await;
    response.assert_status_ok();
    let body = response.text();
    assert!(!body.contains(&actor));
    let public: Value = serde_json::from_str(&body).unwrap();
    let aggregate = public["by_cohort"].as_array().unwrap();
    assert_eq!(aggregate.len(), 3);
    assert_eq!(aggregate[0]["cohort"], "retail");
    assert_eq!(aggregate[0]["event_count"], 1);
    assert_eq!(aggregate[1]["cohort"], "mm");
    assert_eq!(aggregate[1]["event_count"], 1);
    for row in aggregate {
        let keys: std::collections::BTreeSet<_> = row
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        assert_eq!(
            keys,
            ["cohort", "amount_usd", "share_pct", "event_count"]
                .into_iter()
                .collect()
        );
    }
    let result = json!({"status":"PASS","chain_id":"localterra", "pricing_scope":"explicit owned QA economic mark and USTC oracle fixture; not real USD feed acceptance",
        "production_ingestion":"process_block_txs", "real_h_minus_one_registry_queries":true,
        "unregistered_paid_swap":"retail", "registered_tier_zero_paid_wrap":"mm",
        "replay_keeps_historical_cohort":true,"public_cohort_json_has_no_actor":true,
        "swap_tx":txs[0].txhash,"registration_tx":txs[1].txhash,"wrap_tx":txs[2].txhash,
        "heights":heights,"by_cohort":aggregate});
    std::fs::write(
        Path::new("/tmp/dex-cohort-1211-ingestion-result.json"),
        serde_json::to_vec_pretty(&result).unwrap(),
    )
    .unwrap();
}
