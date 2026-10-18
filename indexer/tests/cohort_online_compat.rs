//! Owned-local67 ->68 transition with actual runtime login, HTTP and ingestion.
//! A dedicated loopback container harness provisions the empty marked database.
mod common;
use bech32::ToBase32;
use chrono::Utc;
use cl8y_dex_indexer::{
    api, database_security as security,
    db::queries::protocol_fees as fees,
    indexer::{
        fee_cohorts::{attribute_transaction, AttributionBudget},
        oracle, parser,
    },
    lcd::{LcdClient, TxResponse},
};
use serde_json::{json, Value};
use sqlx::{postgres::PgPoolOptions, PgPool};
use std::{
    borrow::Cow,
    sync::{
        atomic::{AtomicBool, AtomicU64, Ordering},
        Arc,
    },
};

async fn history(pool: &PgPool) -> Vec<Value> {
    sqlx::query_scalar("SELECT row_to_json(m)::jsonb FROM _sqlx_migrations m ORDER BY version")
        .fetch_all(pool)
        .await
        .unwrap()
}
async fn start_api(pool: PgPool) -> (String, tokio::task::JoinHandle<()>) {
    // Exercise the same least-privilege/version startup boundary as run_server.
    security::verify_serving_runtime(&pool).await.unwrap();
    let app = common::build_test_app(pool).await;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let task = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        )
        .await
        .unwrap();
    });
    (format!("http://{address}"), task)
}
async fn response(client: &reqwest::Client, base: &str) -> Value {
    api::reset_protocol_fees_cache();
    let r = client
        .get(format!("{base}/api/v1/protocol/fees"))
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), 200);
    r.json().await.unwrap()
}
fn assert_cohort_state(body: &Value) {
    match body["cohort_status"].as_str().unwrap() {
        "available" => {
            let cohorts = body["by_cohort"].as_array().unwrap();
            assert_eq!(cohorts.len(), 3);
            let n: i64 = cohorts
                .iter()
                .map(|r| r["event_count"].as_i64().unwrap())
                .sum();
            let sources: i64 = body["by_source"]
                .as_array()
                .unwrap()
                .iter()
                .map(|r| r["event_count"].as_i64().unwrap())
                .sum();
            assert_eq!(
                n, sources,
                "cohorts and source counts must belong to the same publication"
            );
        }
        "pending_schema" | "pending_publication" => assert!(body.get("by_cohort").is_none()),
        other => panic!("unexpected status {other}"),
    }
}
fn transaction(n: u64, trader: &str) -> TxResponse {
    serde_json::from_value(json!({"height":n.to_string(),"txhash":format!("compat-{n}"),"logs":[{"events":[{"type":"wasm","attributes":[
        {"key":"_contract_address","value":"terra1compatpair"},{"key":"action","value":"swap"},
        {"key":"sender","value":trader},{"key":"offer_amount","value":"1000000"},
        {"key":"return_amount","value":"950000"},{"key":"spread_amount","value":"20000"},
        {"key":"commission_amount","value":"30000"},{"key":"offer_asset","value":"uluna"},
        {"key":"ask_asset","value":"terra1compatustc"}]}]}]})).unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn real67_to68_keeps_runtime_api_ingestion_and_joint_publication_safe() {
    let url = std::env::var("COHORT_COMPAT_DATABASE_URL").expect("use owned container harness");
    let parsed = reqwest::Url::parse(&url).unwrap();
    assert_eq!(parsed.host_str(), Some("127.0.0.1"));
    let admin = PgPoolOptions::new()
        .max_connections(2)
        .connect(&url)
        .await
        .unwrap();
    let marker:Option<String>=sqlx::query_scalar("SELECT shobj_description(oid,'pg_database') FROM pg_database WHERE datname=current_database()")
        .fetch_one(&admin).await.unwrap();
    let expected_marker = std::env::var("COHORT_COMPAT_DATABASE_MARKER").unwrap();
    assert!(expected_marker.starts_with("cl8y-dex:cohort-compat:") && expected_marker.len() > 40);
    assert_eq!(marker.as_deref(), Some(expected_marker.as_str()));
    let options = admin.connect_options().as_ref().clone();
    let owner = PgPool::connect_with(options.clone().username("cohort_compat_migrator"))
        .await
        .unwrap();
    assert!(
        sqlx::query_scalar::<_, bool>("SELECT to_regclass('public._sqlx_migrations') IS NULL")
            .fetch_one(&owner)
            .await
            .unwrap()
    );
    let prefix = security::MIGRATOR
        .iter()
        .filter(|m| {
            m.version < security::COHORT_SCHEMA_VERSION && !m.migration_type.is_down_migration()
        })
        .cloned()
        .collect::<Vec<_>>();
    assert_eq!(prefix.len(), 67);
    sqlx::migrate::Migrator {
        migrations: Cow::Owned(prefix),
        ignore_missing: false,
        locking: true,
        no_tx: false,
    }
    .run(&owner)
    .await
    .unwrap();
    sqlx::raw_sql("CREATE TABLE indexer_schema_version(singleton BOOLEAN PRIMARY KEY CHECK(singleton),version BIGINT NOT NULL); INSERT INTO indexer_schema_version VALUES(true,20261016120000);
      GRANT USAGE ON SCHEMA public TO cohort_compat_runtime; GRANT SELECT,INSERT,UPDATE,DELETE ON ALL TABLES IN SCHEMA public TO cohort_compat_runtime;
      GRANT USAGE,SELECT ON ALL SEQUENCES IN SCHEMA public TO cohort_compat_runtime; REVOKE ALL ON _sqlx_migrations,indexer_schema_version FROM cohort_compat_runtime; GRANT SELECT ON indexer_schema_version TO cohort_compat_runtime;
      INSERT INTO assets(denom,is_cw20,name,symbol,decimals) VALUES('uluna',false,'Luna','LUNC',6);
      INSERT INTO assets(contract_address,is_cw20,name,symbol,decimals) VALUES('terra1compatustc',true,'USTC','USTC',6);
      INSERT INTO pairs(contract_address,asset_0_id,asset_1_id,lp_token,fee_bps) SELECT 'terra1compatpair',1,2,'terra1compatlp',30;
      INSERT INTO global_stats_24h(id) VALUES(1) ON CONFLICT DO NOTHING;")
        .execute(&owner).await.unwrap();
    let recovery_script = include_str!("../../scripts/indexer-reorg-recover.sh");
    let start = recovery_script.find("DO \\$cohort\\$").unwrap();
    let tail = &recovery_script[start..];
    let end = tail.find("\\$cohort\\$;").unwrap() + "\\$cohort\\$;".len();
    let recovery_sql = tail[..end].replace("\\$", "$").replace("${HEIGHT}", "1");
    let before = history(&owner).await;
    let runtime_options = options.username("cohort_compat_runtime");
    let runtime = PgPoolOptions::new()
        .max_connections(8)
        .connect_with(runtime_options.clone())
        .await
        .unwrap();
    sqlx::raw_sql(&recovery_sql)
        .execute(&runtime)
        .await
        .unwrap();
    assert!(
        security::verify_runtime(&runtime).await.is_err(),
        "strict maintenance/QA version validation stays strict on67"
    );
    let security_before:Value=sqlx::query_scalar("SELECT jsonb_agg(to_jsonb(r) ORDER BY rolname) FROM pg_roles r WHERE rolname IN('cohort_compat_runtime','cohort_compat_migrator')")
        .fetch_one(&admin).await.unwrap();
    let (base, server) = start_api(runtime.clone()).await;
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(2))
        .build()
        .unwrap();
    let initial = response(&client, &base).await;
    assert_eq!(initial["cohort_status"], "pending_schema");
    assert_cohort_state(&initial);
    let specification: Value = client
        .get(format!("{base}/api-docs/openapi.json"))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    let schema = &specification["components"]["schemas"]["ProtocolFeesResponse"];
    assert!(!schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|name| name == "by_cohort"));
    assert!(schema["required"]
        .as_array()
        .unwrap()
        .iter()
        .any(|name| name == "cohort_status"));
    let digest = client
        .get(format!("{base}/api/v1/evidence/digest?day=2026-10-18"))
        .send()
        .await
        .unwrap();
    assert_eq!(digest.status(), 200);
    // Restart before expansion uses the same real authenticated runtime identity.
    server.abort();
    let fresh_runtime = PgPool::connect_with(runtime_options.clone()).await.unwrap();
    let (base, server) = start_api(fresh_runtime).await;
    let trader = bech32::encode("terra", [31; 20].to_base32(), bech32::Variant::Bech32).unwrap();
    let lcd = LcdClient::new(vec!["http://127.0.0.1:9".into()], 50, 1, "columbus-5");
    let cfg = common::test_config();
    let price = oracle::new_shared_price();
    let first = transaction(1, &trader);
    parser::process_block_txs(
        &runtime,
        &lcd,
        &cfg,
        &[first.clone()],
        1,
        Utc::now(),
        &price,
    )
    .await
    .unwrap();
    let mut old_block = AttributionBudget::default();
    attribute_transaction(&runtime, &lcd, &cfg, &first, 1, 0, &mut old_block)
        .await
        .unwrap();
    fees::refresh_protocol_fees(&runtime, true, false)
        .await
        .unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let ingest_count = Arc::new(AtomicU64::new(0));
    let reader_count = Arc::new(AtomicU64::new(0));
    let writer = {
        let (pool, stop, count, trader) = (
            runtime.clone(),
            stop.clone(),
            ingest_count.clone(),
            trader.clone(),
        );
        tokio::spawn(async move {
            let lcd = LcdClient::new(vec!["http://127.0.0.1:9".into()], 50, 1, "columbus-5");
            let cfg = common::test_config();
            let price = oracle::new_shared_price();
            while !stop.load(Ordering::SeqCst) {
                let n = count.fetch_add(1, Ordering::SeqCst) + 10;
                parser::process_block_txs(
                    &pool,
                    &lcd,
                    &cfg,
                    &[transaction(n, &trader)],
                    n as i64,
                    Utc::now(),
                    &price,
                )
                .await
                .unwrap();
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        })
    };
    let reader = {
        let (client, base, stop, count) = (
            client.clone(),
            base.clone(),
            stop.clone(),
            reader_count.clone(),
        );
        tokio::spawn(async move {
            while !stop.load(Ordering::SeqCst) {
                let body = response(&client, &base).await;
                assert_cohort_state(&body);
                count.fetch_add(1, Ordering::SeqCst);
                tokio::time::sleep(std::time::Duration::from_millis(5)).await;
            }
        })
    };
    tokio::time::sleep(std::time::Duration::from_millis(60)).await;
    // Hold an old67 publication snapshot across actual SQLx68 application. Its
    // eventual old-style commit must invalidate cohort readiness rather than
    // querying missing tables or presenting the migration seed as a joint run.
    let mut publication_lock = owner.begin().await.unwrap();
    sqlx::query("SELECT id FROM global_stats_24h WHERE id=1 FOR UPDATE")
        .execute(&mut *publication_lock)
        .await
        .unwrap();
    let old_publisher = {
        let pool = runtime.clone();
        tokio::spawn(async move {
            fees::refresh_protocol_fees(&pool, true, false)
                .await
                .unwrap();
        })
    };
    tokio::time::timeout(std::time::Duration::from_secs(3),async {
        loop {
            let blocked:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM pg_stat_activity WHERE wait_event_type='Lock' AND query LIKE '%UPDATE global_stats_24h SET%')")
                .fetch_one(&admin).await.unwrap();
            if blocked { break; }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
    }).await.expect("actual67 publisher must reach its global row write before migration");
    security::migrate_and_grant(&owner, "cohort_compat_runtime")
        .await
        .unwrap();
    assert_eq!(before, history(&owner).await[..67]);
    sqlx::raw_sql(&recovery_sql)
        .execute(&runtime)
        .await
        .unwrap();
    let staged = response(&client, &base).await;
    assert_eq!(staged["cohort_status"], "pending_publication");
    assert_cohort_state(&staged);
    // A block which chose67 cannot acquire cohort work after the marker advanced.
    attribute_transaction(&runtime, &lcd, &cfg, &first, 1, 0, &mut old_block)
        .await
        .unwrap();
    assert_eq!(sqlx::query_scalar::<_,i64>("SELECT count(*) FROM protocol_fee_attributions a JOIN protocol_fee_events e ON e.id=a.fee_event_id WHERE e.tx_hash='compat-1'").fetch_one(&runtime).await.unwrap(),0);
    publication_lock.commit().await.unwrap();
    old_publisher.await.unwrap();
    let pending = response(&client, &base).await;
    assert_eq!(pending["cohort_status"], "pending_publication");
    fees::refresh_protocol_fees(&runtime, true, false)
        .await
        .unwrap();
    let current = response(&client, &base).await;
    assert_eq!(current["cohort_status"], "available");
    assert_cohort_state(&current);
    tokio::time::sleep(std::time::Duration::from_millis(60)).await;
    stop.store(true, Ordering::SeqCst);
    writer.await.unwrap();
    reader.await.unwrap();
    assert!(ingest_count.load(Ordering::SeqCst) > 1);
    assert!(reader_count.load(Ordering::SeqCst) > 1);
    // Simulate a legacy67 publisher after68: its independent global clock cannot
    // make older cohorts appear current, even while all3 row values are valid.
    sqlx::query("UPDATE global_stats_24h SET updated_at=clock_timestamp() WHERE id=1")
        .execute(&runtime)
        .await
        .unwrap();
    assert_eq!(
        response(&client, &base).await["cohort_status"],
        "pending_publication"
    );
    fees::refresh_protocol_fees(&runtime, true, false)
        .await
        .unwrap();
    server.abort();
    let restarted = PgPool::connect_with(runtime_options).await.unwrap();
    let (base, server) = start_api(restarted.clone()).await;
    assert_eq!(response(&client, &base).await["cohort_status"], "available");
    for table in [
        "protocol_fee_stats_by_source",
        "protocol_fee_stats_by_token",
        "protocol_fee_stats_by_cohort",
    ] {
        sqlx::query(&format!("UPDATE {table} SET updated_at=updated_at-interval '1 microsecond' WHERE \"window\"='24h'"))
            .execute(&runtime).await.unwrap();
        let mismatch = response(&client, &base).await;
        assert_eq!(mismatch["cohort_status"], "pending_publication");
        assert_cohort_state(&mismatch);
        fees::refresh_protocol_fees(&runtime, true, false)
            .await
            .unwrap();
        assert_eq!(response(&client, &base).await["cohort_status"], "available");
    }
    security::verify_runtime(&restarted).await.unwrap();
    for forbidden in [
        "CREATE TABLE forbidden_compat_runtime(id int)",
        "SELECT * FROM _sqlx_migrations",
        "UPDATE indexer_schema_version SET version=version WHERE false",
        "CREATE ROLE forbidden_compat_runtime NOLOGIN",
    ] {
        assert!(sqlx::query(forbidden).execute(&restarted).await.is_err());
    }
    for bad in [20261015120000_i64, 20261019110000_i64] {
        sqlx::query("UPDATE indexer_schema_version SET version=$1")
            .bind(bad)
            .execute(&owner)
            .await
            .unwrap();
        assert!(security::verify_serving_runtime(&restarted).await.is_err());
        assert!(security::cohorts_ready(&restarted).await.is_err());
    }
    sqlx::query("UPDATE indexer_schema_version SET version=20261018110000")
        .execute(&owner)
        .await
        .unwrap();
    let security_after:Value=sqlx::query_scalar("SELECT jsonb_agg(to_jsonb(r) ORDER BY rolname) FROM pg_roles r WHERE rolname IN('cohort_compat_runtime','cohort_compat_migrator')").fetch_one(&admin).await.unwrap();
    assert_eq!(security_before, security_after);
    assert_eq!(before, history(&owner).await[..67]);
    server.abort();
    println!("COHORT_COMPAT_PASS baseline=67 final=68 runtime_denials=4 continuing_http={} continuing_blocks={}",reader_count.load(Ordering::SeqCst),ingest_count.load(Ordering::SeqCst));
}
