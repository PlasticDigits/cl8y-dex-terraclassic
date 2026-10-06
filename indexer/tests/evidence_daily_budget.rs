//! #1397 database budgets, ordered daily scans and pool reuse.
mod common;

use chrono::{DateTime, Duration as ChronoDuration, TimeZone, Utc};
use cl8y_dex_indexer::db::queries::evidence_daily::{
    EvidenceSortKey, EvidenceSurfaceSet, list_evidence_rows,
};
use common::{seed_db, setup_pool, test_config};
use serial_test::serial;
use sqlx::{PgPool, postgres::PgPoolOptions};
use std::time::{Duration, Instant};

fn day() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 6, 10, 0, 0, 0).unwrap()
}

fn swaps() -> EvidenceSurfaceSet {
    EvidenceSurfaceSet {
        swap: true,
        wrap: false,
        limit: false,
        lp: false,
    }
}

async fn rows(
    pool: &PgPool,
    cursor: Option<&EvidenceSortKey>,
    limit: i64,
) -> Result<Vec<cl8y_dex_indexer::db::queries::evidence_daily::EvidenceRawRow>, sqlx::Error> {
    list_evidence_rows(
        pool,
        day(),
        day() + ChronoDuration::days(1),
        swaps(),
        cursor,
        limit,
    )
    .await
}

async fn single_connection_pool() -> PgPool {
    PgPoolOptions::new()
        .max_connections(1)
        .connect(&test_config().database_url)
        .await
        .unwrap()
}

async fn assert_reusable(pool: &PgPool) {
    let mut connection = tokio::time::timeout(Duration::from_secs(3), pool.acquire())
        .await
        .expect("cancelled evidence must release its pool slot")
        .unwrap();
    let value: i32 = sqlx::query_scalar("SELECT 1")
        .fetch_one(&mut *connection)
        .await
        .unwrap();
    assert_eq!(value, 1);
    // SET LOCAL must not leak into unrelated queries on this same connection.
    let statement: String = sqlx::query_scalar("SHOW statement_timeout")
        .fetch_one(&mut *connection)
        .await
        .unwrap();
    let lock: String = sqlx::query_scalar("SHOW lock_timeout")
        .fetch_one(&mut *connection)
        .await
        .unwrap();
    assert_eq!(statement, "0");
    assert_eq!(lock, "0");
}

fn assert_index_budget(
    plan: &serde_json::Value,
    day_index: &str,
    require_day_index: bool,
    label: &str,
) {
    fn visit(node: &serde_json::Value, scanned: &mut u64, indexes: &mut Vec<String>) {
        if let Some(index) = node["Index Name"].as_str() {
            indexes.push(index.into());
        }
        if node.get("Relation Name").is_some() {
            let rows = node["Actual Rows"].as_u64().unwrap_or(0)
                + node["Rows Removed by Filter"].as_u64().unwrap_or(0)
                + node["Rows Removed by Index Recheck"].as_u64().unwrap_or(0);
            *scanned += rows * node["Actual Loops"].as_u64().unwrap_or(1);
        }
        if let Some(children) = node["Plans"].as_array() {
            for child in children {
                visit(child, scanned, indexes);
            }
        }
    }
    let mut scanned = 0;
    let mut indexes = Vec::new();
    visit(&plan[0]["Plan"], &mut scanned, &mut indexes);
    let text = plan.to_string();
    assert!(!indexes.is_empty(), "{label}: {text}");
    assert!(!text.contains("Seq Scan"), "{label}: {text}");
    if require_day_index {
        assert!(
            indexes.iter().any(|name| name == day_index),
            "{label}: {text}"
        );
    }
    assert!(scanned <= 512, "{label}: read {scanned} rows: {text}");
    let elapsed = plan[0]["Execution Time"].as_f64().unwrap();
    assert!(elapsed < 1500.0, "{label}: {text}");
    eprintln!(
        "{label}: indexes={indexes:?}, scanned={scanned}, execution_ms={elapsed}, shared_blocks={}",
        plan[0]["Plan"]["Shared Hit Blocks"].as_u64().unwrap_or(0)
            + plan[0]["Plan"]["Shared Read Blocks"].as_u64().unwrap_or(0)
    );
}

#[serial]
#[tokio::test]
async fn evidence_dense_day_cursor_filter_and_index_scan() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    sqlx::query("INSERT INTO swap_events (pair_id, swap_index, block_height, block_timestamp, tx_hash, sender, offer_asset_id, ask_asset_id, offer_amount, return_amount, price)
        SELECT $1, 0, n, $2, 'dense-' || lpad(n::text, 8, '0'), 'terra1privateactor', $3, $4, 1000, 990, 0.99 FROM generate_series(1, 20000) n")
        .bind(seed.pair_id).bind(day()).bind(seed.asset_0_id).bind(seed.asset_1_id)
        .execute(&pool).await.unwrap();
    sqlx::query("ANALYZE swap_events")
        .execute(&pool)
        .await
        .unwrap();
    let start = Instant::now();
    let first = rows(&pool, None, 101).await.unwrap();
    assert!(start.elapsed() < Duration::from_secs(2));
    assert_eq!(first.len(), 101);
    let last = first.last().unwrap();
    let cursor = EvidenceSortKey {
        block_height: last.block_height,
        tx_hash: last.tx_hash.clone(),
        surface: last.surface.clone(),
        kind: last.kind.clone(),
        ordinal: last.ordinal,
        row_id: last.row_id,
    };
    let next = rows(&pool, Some(&cursor), 101).await.unwrap();
    assert_eq!(next.len(), 101);
    assert_eq!(next[0].block_height, last.block_height + 1);
    let far_cursor = EvidenceSortKey {
        block_height: 19900,
        tx_hash: "dense-00019900".into(),
        row_id: i64::MAX,
        ..cursor
    };
    let tail = rows(&pool, Some(&far_cursor), 101).await.unwrap();
    assert_eq!(tail.len(), 100);
    assert_eq!(tail[0].block_height, 19901);
    let empty = list_evidence_rows(
        &pool,
        day() + ChronoDuration::days(2),
        day() + ChronoDuration::days(3),
        EvidenceSurfaceSet::all(),
        None,
        101,
    )
    .await
    .unwrap();
    assert!(empty.is_empty());
    let only_wrap = EvidenceSurfaceSet {
        swap: false,
        wrap: true,
        limit: false,
        lp: false,
    };
    assert!(
        list_evidence_rows(
            &pool,
            day(),
            day() + ChronoDuration::days(1),
            only_wrap,
            None,
            101
        )
        .await
        .unwrap()
        .is_empty()
    );

    // Natural planner choices are permitted when actual source reads stay
    // bounded. Empty first pages must seek directly by UTC day rather than
    // walking unrelated block history. No sequential scans are disabled.
    for target_day in [
        day().date_naive(),
        (day() + ChronoDuration::days(2)).date_naive(),
    ] {
        for late in [false, true] {
            let predicate = if late {
                "AND (s.block_height,s.tx_hash,'swap'::text,'swap'::text,COALESCE(s.swap_index,0)::bigint,s.id) > (19900,'dense-00019900','swap','swap',0,0)"
            } else {
                ""
            };
            let sql = format!(
                "EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON) SELECT s.block_height,s.tx_hash,COALESCE(s.swap_index,0)::bigint,s.id FROM swap_events s WHERE (s.block_timestamp AT TIME ZONE 'UTC')::date = $1 {predicate} ORDER BY s.block_height,s.tx_hash,COALESCE(s.swap_index,0)::bigint,s.id LIMIT 101"
            );
            let plan: serde_json::Value = sqlx::query_scalar(&sql)
                .bind(target_day)
                .fetch_one(&pool)
                .await
                .unwrap();
            assert_index_budget(
                &plan,
                "evidence_swaps_day_order_idx",
                !late && target_day != day().date_naive(),
                &format!("swap {target_day} late={late}"),
            );
        }
    }
    pool.close().await;
}

#[serial]
#[tokio::test]
async fn evidence_every_source_has_dense_late_page_and_empty_index_plans() {
    let pool = setup_pool().await;
    let seed = seed_db(&pool).await;
    for insert in [
        "INSERT INTO liquidity_events (pair_id,block_height,block_timestamp,tx_hash,provider,event_type) SELECT $1,n,$2,'dense-'||lpad(n::text,8,'0'),'terra1privateactor',CASE WHEN n%2=0 THEN 'add' ELSE 'remove' END FROM generate_series(1,20000)n",
        "INSERT INTO limit_order_placements (pair_id,block_height,block_timestamp,tx_hash,order_id) SELECT $1,n,$2,'dense-'||lpad(n::text,8,'0'),n FROM generate_series(1,20000)n",
        "INSERT INTO limit_order_cancellations (pair_id,block_height,block_timestamp,tx_hash,order_id) SELECT $1,n,$2,'dense-'||lpad(n::text,8,'0'),n FROM generate_series(1,20000)n",
        "INSERT INTO limit_order_fills (pair_id,block_height,block_timestamp,tx_hash,order_id,side,maker,price,token0_amount,token1_amount,commission_amount) SELECT $1,n,$2,'dense-'||lpad(n::text,8,'0'),n,'bid','terra1privateactor',1,1000,1000,1 FROM generate_series(1,20000)n",
    ] {
        sqlx::query(insert)
            .bind(seed.pair_id)
            .bind(day())
            .execute(&pool)
            .await
            .unwrap();
    }
    sqlx::query("INSERT INTO protocol_fee_events (asset_id,block_height,block_timestamp,tx_hash,source,ordinal,amount_raw,decimals) SELECT $1,n,$2,'dense-'||lpad(n::text,8,'0'),CASE WHEN n%2=0 THEN 'wrap' ELSE 'unwrap' END,0,1000,6 FROM generate_series(1,20000)n")
        .bind(seed.asset_0_id).bind(day()).execute(&pool).await.unwrap();
    // Distinct source kinds and surfaces are present at each block. The global
    // merge must stay ordered and filtered late pages must survive branch caps.
    let cursor = EvidenceSortKey {
        block_height: 19900,
        tx_hash: "dense-00019900".into(),
        surface: "swap".into(),
        kind: "swap".into(),
        ordinal: 0,
        row_id: i64::MAX,
    };
    let merged = list_evidence_rows(
        &pool,
        day(),
        day() + ChronoDuration::days(1),
        EvidenceSurfaceSet::all(),
        Some(&cursor),
        101,
    )
    .await
    .unwrap();
    assert_eq!(merged.len(), 101);
    assert_eq!(merged[0].surface, "wrap");
    assert_eq!(merged[0].block_height, 19900);
    for (table, index, surface, kind, ordinal, filter) in [
        (
            "liquidity_events",
            "evidence_lp_day_order_idx",
            "lp",
            "event_type::text",
            "id",
            "TRUE",
        ),
        (
            "limit_order_placements",
            "evidence_placements_day_order_idx",
            "limit",
            "'place'::text",
            "id",
            "TRUE",
        ),
        (
            "limit_order_cancellations",
            "evidence_cancellations_day_order_idx",
            "limit",
            "'cancel'::text",
            "id",
            "TRUE",
        ),
        (
            "limit_order_fills",
            "evidence_fills_day_order_idx",
            "limit",
            "'fill'::text",
            "id",
            "TRUE",
        ),
        (
            "protocol_fee_events",
            "evidence_wrap_day_order_idx",
            "wrap",
            "source",
            "ordinal",
            "source IN ('wrap','unwrap')",
        ),
    ] {
        sqlx::query(&format!("ANALYZE {table}"))
            .execute(&pool)
            .await
            .unwrap();
        for target_day in [
            day().date_naive(),
            (day() + ChronoDuration::days(2)).date_naive(),
        ] {
            for late in [false, true] {
                let predicate = if late {
                    format!(
                        "AND (block_height,tx_hash,'{surface}'::text,{kind},{ordinal},id) > (19900,'dense-00019900','swap','swap',0,0)"
                    )
                } else {
                    String::new()
                };
                let sql = format!("EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON) SELECT block_height,tx_hash,{kind},{ordinal},id FROM {table}
                    WHERE (block_timestamp AT TIME ZONE 'UTC')::date = $1 AND {filter} {predicate}
                    ORDER BY block_height,tx_hash,{kind},{ordinal},id LIMIT 101");
                let plan: serde_json::Value = sqlx::query_scalar(&sql)
                    .bind(target_day)
                    .fetch_one(&pool)
                    .await
                    .unwrap();
                assert_index_budget(
                    &plan,
                    index,
                    !late && target_day != day().date_naive(),
                    &format!("{table} {target_day} late={late}"),
                );
            }
        }
    }
    pool.close().await;
}

#[serial]
#[tokio::test]
async fn evidence_pool_acquisition_is_bounded() {
    let setup = setup_pool().await;
    seed_db(&setup).await;
    let pool = single_connection_pool().await;
    let held = pool.acquire().await.unwrap();
    let start = Instant::now();
    assert!(matches!(
        rows(&pool, None, 2).await,
        Err(sqlx::Error::PoolTimedOut)
    ));
    assert!(start.elapsed() < Duration::from_secs(1));
    drop(held);
    assert_reusable(&pool).await;
    pool.close().await;
    setup.close().await;
}

#[serial]
#[tokio::test]
async fn evidence_lock_timeout_and_cancel_release_transaction() {
    let setup = setup_pool().await;
    seed_db(&setup).await;
    let pool = single_connection_pool().await;
    let mut blocker = setup.begin().await.unwrap();
    sqlx::query("LOCK TABLE swap_events IN ACCESS EXCLUSIVE MODE")
        .execute(&mut *blocker)
        .await
        .unwrap();
    let start = Instant::now();
    let error = rows(&pool, None, 2).await.unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("55P03")
    );
    assert!(start.elapsed() < Duration::from_secs(2));
    assert_reusable(&pool).await;

    // Cancel an in-flight read before the 500ms lock timeout, while blocked.
    let worker_pool = pool.clone();
    let worker = tokio::spawn(async move { rows(&worker_pool, None, 2).await });
    let mut blocked = false;
    for _ in 0..25 {
        blocked = sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM pg_stat_activity WHERE state='active' AND wait_event_type='Lock' AND query LIKE 'SELECT * FROM (%' AND pid <> pg_backend_pid())")
            .fetch_one(&setup).await.unwrap();
        if blocked {
            break;
        }
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    assert!(blocked, "test must cancel an executing evidence query");
    worker.abort();
    assert!(worker.await.unwrap_err().is_cancelled());
    assert_reusable(&pool).await;
    blocker.rollback().await.unwrap();
    assert!(rows(&pool, None, 2).await.unwrap().is_empty());
    pool.close().await;
    setup.close().await;
}

#[serial]
#[tokio::test]
async fn evidence_statement_timeout_releases_connection() {
    let setup = setup_pool().await;
    let seed = seed_db(&setup).await;
    sqlx::query("INSERT INTO swap_events (pair_id, swap_index, block_height, block_timestamp, tx_hash, sender, offer_asset_id, ask_asset_id, offer_amount, return_amount, price) VALUES ($1,0,1,$2,'slow-test','terra1privateactor',$3,$4,1000,990,0.99)")
        .bind(seed.pair_id).bind(day()).bind(seed.asset_0_id).bind(seed.asset_1_id)
        .execute(&setup).await.unwrap();
    // An isolated-test-DB view forces real server execution to sleep rather
    // than relying on fixture size or host speed to trigger statement_timeout.
    let mut ddl = setup.begin().await.unwrap();
    sqlx::query("ALTER TABLE swap_events RENAME TO evidence_slow_swaps")
        .execute(&mut *ddl)
        .await
        .unwrap();
    sqlx::query("CREATE VIEW swap_events AS SELECT s.* FROM evidence_slow_swaps s CROSS JOIN (SELECT pg_sleep(3)) delay")
        .execute(&mut *ddl).await.unwrap();
    ddl.commit().await.unwrap();
    let pool = single_connection_pool().await;
    let start = Instant::now();
    let result = rows(&pool, None, 2).await;
    let elapsed = start.elapsed();
    // Restore names before assertions so a failed regression leaves the test
    // database usable by later binaries.
    let mut ddl = setup.begin().await.unwrap();
    sqlx::query("DROP VIEW swap_events")
        .execute(&mut *ddl)
        .await
        .unwrap();
    sqlx::query("ALTER TABLE evidence_slow_swaps RENAME TO swap_events")
        .execute(&mut *ddl)
        .await
        .unwrap();
    ddl.commit().await.unwrap();
    let error = result.unwrap_err();
    assert_eq!(
        error.as_database_error().unwrap().code().as_deref(),
        Some("57014")
    );
    assert!(elapsed >= Duration::from_millis(1000));
    assert!(elapsed < Duration::from_secs(3));
    assert_reusable(&pool).await;
    assert_eq!(rows(&pool, None, 2).await.unwrap().len(), 1);
    pool.close().await;
    setup.close().await;
}

#[serial]
#[tokio::test]
async fn evidence_invalid_fetch_limits_fail_without_database_work() {
    let pool = PgPoolOptions::new()
        .connect_lazy("postgres://unused:unused@127.0.0.1:1/unused")
        .unwrap();
    for limit in [0, -1, 1002] {
        assert!(matches!(
            rows(&pool, None, limit).await,
            Err(sqlx::Error::Protocol(_))
        ));
    }
}
