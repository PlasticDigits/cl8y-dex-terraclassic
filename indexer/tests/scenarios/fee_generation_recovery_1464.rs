//! Actual scheduled producer/publication races. Reuse the parent market fixture;
//! each test process owns its disposable database and only loopback HTTP.
use super::*;
use base64::Engine;
use cl8y_dex_indexer::{
    api::local_quote,
    indexer::{book_snapshot, fee_phase},
};
use std::{
    sync::atomic::{AtomicUsize, Ordering},
    time::Duration,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    sync::{Notify, Semaphore},
};

const PAIR: &str = "terra1feephase1464";
const RAW: u128 = 10_000 * 10u128.pow(18);

/// Gate a real HTTP fee response after raw publication. A permanent response
/// can expose its headers while its body remains blocked, exercising the LCD
/// safety observer rather than relying on producer completion.
struct GatedProvider {
    uri: String,
    entered: Arc<Semaphore>,
    release: Arc<Notify>,
    calls: Arc<AtomicUsize>,
    task: tokio::task::JoinHandle<()>,
}
impl Drop for GatedProvider {
    fn drop(&mut self) {
        self.task.abort();
    }
}
impl GatedProvider {
    async fn start(upstream: String, permanent: bool, fee_bps: u16) -> Self {
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let uri = format!("http://{}", listener.local_addr().unwrap());
        let entered = Arc::new(Semaphore::new(0));
        let release = Arc::new(Notify::new());
        let calls = Arc::new(AtomicUsize::new(0));
        let (signal, gate, counter) = (entered.clone(), release.clone(), calls.clone());
        let task = tokio::spawn(async move {
            let mut children = JoinSet::new();
            loop {
                let (mut socket, _) = listener.accept().await.unwrap();
                let (upstream, signal, gate, counter) = (
                    upstream.clone(),
                    signal.clone(),
                    gate.clone(),
                    counter.clone(),
                );
                children.spawn(async move {
                    let mut bytes = Vec::new();
                    while !bytes.ends_with(b"\r\n\r\n") {
                        let mut byte = [0];
                        if socket.read_exact(&mut byte).await.is_err() { return; }
                        bytes.push(byte[0]);
                        assert!(bytes.len() <= 8192);
                    }
                    counter.fetch_add(1, Ordering::SeqCst);
                    let text = String::from_utf8(bytes).unwrap();
                    let path = text.lines().next().unwrap().split_whitespace().nth(1).unwrap();
                    let url = reqwest::Url::parse(&format!("http://fixture{path}")).unwrap();
                    let height = url.query_pairs().find(|(key,_)| key=="height").map(|(_,value)| value.into_owned()).unwrap_or_else(|| "101".into());
                    let query: Value = url.path().split("/smart/").nth(1).and_then(|encoded| base64::engine::general_purpose::STANDARD.decode(encoded).ok()).and_then(|bytes| serde_json::from_slice(&bytes).ok()).unwrap_or(Value::Null);
                    let (status, body) = if query.get("get_fee_config").is_some() {
                        let body = if permanent { json!({"error":"invalid fee request"}) } else { json!({"data":{"fee_config":{"fee_bps":fee_bps,"treasury":"fixture"}}}) }.to_string();
                        if permanent {
                            socket.write_all(format!("HTTP/1.1 400 Bad Request\r\nContent-Type: application/json\r\nContent-Length: {}\r\nx-cosmos-block-height: {height}\r\nConnection: close\r\n\r\n", body.len()).as_bytes()).await.unwrap();
                        }
                        signal.add_permits(1);
                        gate.notified().await;
                        if permanent { let _=socket.write_all(body.as_bytes()).await; return; }
                        (200, body)
                    } else if query.get("pool").is_some() {
                        (200, json!({"data":{"assets":[{"info":{"token":{"contract_addr":CL8Y}},"amount":(RAW*2).to_string()},{"info":{"token":{"contract_addr":USTR}},"amount":(RAW*2).to_string()}],"total_share":"1"}}).to_string())
                    } else if query.get("order_book_head").is_some() {
                        (200, json!({"data":null}).to_string())
                    } else {
                        let response = reqwest::Client::new().get(format!("{upstream}{path}")).send().await.unwrap();
                        (response.status().as_u16(), response.text().await.unwrap())
                    };
                    let header = format!("HTTP/1.1 {status} Fixture\r\nContent-Type: application/json\r\nContent-Length: {}\r\nx-cosmos-block-height: {height}\r\nConnection: close\r\n\r\n", body.len());
                    let _ = socket.write_all(header.as_bytes()).await;
                    let _ = socket.write_all(body.as_bytes()).await;
                });
            }
        });
        Self {
            uri,
            entered,
            release,
            calls,
            task,
        }
    }
    async fn waiting(&self) {
        tokio::time::timeout(Duration::from_secs(3), self.entered.acquire())
            .await
            .unwrap()
            .unwrap()
            .forget();
    }
}

async fn fixture(
    permanent: bool,
) -> (
    AppState,
    pairs::PairRow,
    wiremock::MockServer,
    GatedProvider,
) {
    fixture_fee(permanent, 35).await
}
async fn fixture_fee(
    permanent: bool,
    fee_bps: u16,
) -> (
    AppState,
    pairs::PairRow,
    wiremock::MockServer,
    GatedProvider,
) {
    fixture_fee_with_ingestion(permanent, fee_bps, true).await
}

async fn fixture_fee_with_ingestion(
    permanent: bool,
    fee_bps: u16,
    initialize: bool,
) -> (
    AppState,
    pairs::PairRow,
    wiremock::MockServer,
    GatedProvider,
) {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let a = asset(&pool, CL8Y, 18).await;
    let b = asset(&pool, USTR, 18).await;
    pair(&pool, PAIR, a, b, RAW, RAW).await;
    sqlx::query("DELETE FROM hybrid_pair_reserves")
        .execute(&pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE pair_reserves SET block_height=100,snapshot_at=NOW()-INTERVAL '6.5 seconds'",
    )
    .execute(&pool)
    .await
    .unwrap();
    seed_reference_marks(&pool, 1).await;
    let upstream = common::lcd_mock::start_ordinary_code_mock().await;
    mount_supported_config_mock(&upstream, 0).await;
    let provider = GatedProvider::start(upstream.uri(), permanent, fee_bps).await;
    let mut state = app_state(pool.clone(), cold_fixture_lcd(provider.uri.clone()));
    state.book_snapshot_max_staleness_ms = 20_000;
    if initialize {
        local_quote::ingest_fixture(&state).await.unwrap();
    }
    let pair = sqlx::query_as("SELECT * FROM pairs WHERE contract_address=$1")
        .bind(PAIR)
        .fetch_one(&pool)
        .await
        .unwrap();
    (state, pair, upstream, provider)
}
async fn quote(state: &AppState) -> Result<Value, ()> {
    let request = serde_json::from_value(json!({"token_in":CL8Y,"token_out":USTR,"amount_in":"1000000000000000000","max_spread":"0.05","pool_only":true})).unwrap();
    local_quote::solve(state, &request)
        .await
        .map(|body| body.0)
        .map_err(|_| ())
}
fn useful(body: &Value) {
    assert!(
        body["estimated_amount_out_net"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            > 0
    );
    assert!(!body["router_operations"].as_array().unwrap().is_empty());
}
async fn publishers(state: &AppState) {
    local_quote::refresh_reserves(state).await.unwrap();
    local_quote::ingest_once(state).await.unwrap();
}

#[tokio::test]
#[serial_test::serial]
async fn recovery1464_pending_fee_keeps_exact_complete_generation_in_both_publishers() {
    let (state, pair, _upstream, provider) = fixture(false).await;
    let baseline = quote(&state).await.unwrap();
    useful(&baseline);
    let (pool, lcd) = (state.pool.clone(), state.lcd.clone());
    let worker = tokio::spawn(async move {
        book_snapshot::snapshot_single_pair(&pool, &lcd, &pair, Some(101))
            .await
            .map_err(|error| error.to_string())
    });
    provider.waiting().await;
    let raw = pair_reserves::get_all_pair_reserves(&state.pool)
        .await
        .unwrap();
    let pending = raw.values().next().unwrap();
    assert_eq!(pending.block_height, Some(101));
    assert_eq!(pending.fee_bps, -1);
    let original = pending.snapshot_at;
    publishers(&state).await;
    let before_calls = provider.calls.load(Ordering::SeqCst);
    let retained = quote(&state).await.unwrap();
    useful(&retained);
    assert_eq!(
        retained["snapshot_block_height"],
        baseline["snapshot_block_height"]
    );
    assert_eq!(
        retained["estimated_amount_out_net"],
        baseline["estimated_amount_out_net"]
    );
    assert_eq!(
        retained["execution_metadata"]["observed_at_ms"],
        baseline["execution_metadata"]["observed_at_ms"]
    );
    assert_eq!(
        retained["execution_metadata"]["expires_at_ms"],
        baseline["execution_metadata"]["expires_at_ms"]
    );
    assert_eq!(
        provider.calls.load(Ordering::SeqCst),
        before_calls,
        "quote must trigger zero upstream requests"
    );
    provider.release.notify_waiters();
    worker.await.unwrap().unwrap();
    publishers(&state).await;
    let complete = quote(&state).await.unwrap();
    useful(&complete);
    assert_eq!(complete["snapshot_block_height"], 101);
    let raw = pair_reserves::get_all_pair_reserves(&state.pool)
        .await
        .unwrap();
    let row = raw.values().next().unwrap();
    assert_eq!(row.snapshot_at, original);
    assert_eq!(row.fee_bps, 35);
    assert_ne!(
        complete["execution_metadata"]["generation"],
        baseline["execution_metadata"]["generation"]
    );
}

#[tokio::test]
#[serial_test::serial]
async fn recovery1464_fee_header_negative_invalidates_cached_quote_before_body_completion() {
    let (state, pair, _upstream, provider) = fixture(true).await;
    useful(&quote(&state).await.unwrap());
    let (pool, lcd) = (state.pool.clone(), state.lcd.clone());
    let worker = tokio::spawn(async move {
        book_snapshot::snapshot_single_pair(&pool, &lcd, &pair, Some(101))
            .await
            .map_err(|error| error.to_string())
    });
    provider.waiting().await;
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if quote(&state).await.is_err() {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("permanent header must revoke before slow body completes");
    assert!(!worker.is_finished());
    publishers(&state).await;
    assert!(quote(&state).await.is_err());
    worker.abort();
    let _ = worker.await;
    assert!(quote(&state).await.is_err());
    let raw = pair_reserves::get_all_pair_reserves(&state.pool)
        .await
        .unwrap();
    assert_eq!(
        raw.values().next().unwrap().block_height,
        Some(101),
        "raw liquidity survives fee failure"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn recovery1464_cancelled_fee_and_obsolete_completion_cannot_restore_prior_quote() {
    let (state, pair, _upstream, provider) = fixture(false).await;
    useful(&quote(&state).await.unwrap());
    let (pool, lcd, worker_pair) = (state.pool.clone(), state.lcd.clone(), pair.clone());
    let worker = tokio::spawn(async move {
        book_snapshot::snapshot_single_pair(&pool, &lcd, &worker_pair, Some(101))
            .await
            .map_err(|error| error.to_string())
    });
    provider.waiting().await;
    publishers(&state).await;
    useful(&quote(&state).await.unwrap());
    worker.abort();
    let _ = worker.await;
    assert!(quote(&state).await.is_err());
    publishers(&state).await;
    assert!(quote(&state).await.is_err());
    let namespace = state.lcd.chain_cache_namespace();
    let at = chrono::Utc::now();
    let old = fee_phase::begin(
        &namespace,
        pair.id,
        (pair.asset_0_id, pair.asset_1_id),
        102,
        at,
        (RAW * 2, RAW * 2),
    )
    .unwrap();
    let newer = fee_phase::begin(
        &namespace,
        pair.id,
        (pair.asset_0_id, pair.asset_1_id),
        103,
        chrono::Utc::now(),
        (RAW * 2, RAW * 2),
    )
    .unwrap();
    newer.finish_negative();
    old.finish_validated();
    publishers(&state).await;
    assert!(quote(&state).await.is_err());
}

#[tokio::test]
#[serial_test::serial]
async fn recovery1464_completed_maximum_fee_cannot_reuse_old_positive_cached_economics() {
    let (state, pair, _upstream, provider) = fixture_fee(false, 10_000).await;
    useful(&quote(&state).await.unwrap());
    let (pool, lcd) = (state.pool.clone(), state.lcd.clone());
    let worker = tokio::spawn(async move {
        book_snapshot::snapshot_single_pair(&pool, &lcd, &pair, Some(101))
            .await
            .map_err(|error| error.to_string())
    });
    provider.waiting().await;
    publishers(&state).await;
    useful(&quote(&state).await.unwrap());
    provider.release.notify_waiters();
    worker.await.unwrap().unwrap();
    assert!(
        quote(&state).await.is_err(),
        "authoritative fee change must revoke warm results before a publisher runs"
    );
    publishers(&state).await;
    assert!(quote(&state).await.is_err());
    let raw = pair_reserves::get_all_pair_reserves(&state.pool)
        .await
        .unwrap();
    assert_eq!(raw.values().next().unwrap().fee_bps, 10_000);
}

#[tokio::test]
#[serial_test::serial]
async fn recovery1464_unchanged_completed_fee_keeps_original_quote_until_publication() {
    let (state, pair, _upstream, provider) = fixture_fee(false, 30).await;
    let baseline = quote(&state).await.unwrap();
    useful(&baseline);
    let (pool, lcd) = (state.pool.clone(), state.lcd.clone());
    let worker = tokio::spawn(async move {
        book_snapshot::snapshot_single_pair(&pool, &lcd, &pair, Some(101))
            .await
            .map_err(|error| error.to_string())
    });
    provider.waiting().await;
    publishers(&state).await;
    provider.release.notify_waiters();
    worker.await.unwrap().unwrap();
    let still_fresh = quote(&state).await.unwrap();
    useful(&still_fresh);
    assert_eq!(
        still_fresh["execution_metadata"]["observed_at_ms"],
        baseline["execution_metadata"]["observed_at_ms"]
    );
    assert_eq!(
        still_fresh["execution_metadata"]["expires_at_ms"],
        baseline["execution_metadata"]["expires_at_ms"]
    );
    publishers(&state).await;
    assert_eq!(quote(&state).await.unwrap()["snapshot_block_height"], 101);
}

#[tokio::test]
#[serial_test::serial]
async fn recovery1464_cold_fee_rejection_cannot_admit_older_hybrid_before_or_after_qualification() {
    let (warm, pair, _upstream, provider) = fixture(true).await;
    sqlx::query("INSERT INTO hybrid_pair_reserves SELECT * FROM pair_reserves")
        .execute(&warm.pool)
        .await
        .unwrap();
    let cold = app_state(warm.pool.clone(), cold_fixture_lcd(provider.uri.clone()));
    let (pool, lcd, p) = (cold.pool.clone(), cold.lcd.clone(), pair.clone());
    let worker = tokio::spawn(async move {
        book_snapshot::snapshot_single_pair(&pool, &lcd, &p, Some(101))
            .await
            .map_err(|error| error.to_string())
    });
    provider.waiting().await;
    publishers(&cold).await;
    let calls = provider.calls.load(Ordering::SeqCst);
    assert!(
        quote(&cold).await.is_err(),
        "cold publication cannot infer a pending proof from an old hybrid"
    );
    assert_eq!(provider.calls.load(Ordering::SeqCst), calls);
    provider.release.notify_waiters();
    assert!(worker.await.unwrap().is_err());
    publishers(&cold).await;
    assert!(
        quote(&cold).await.is_err(),
        "qualification cannot erase required fee rejection"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn recovery1464_cold_changed_fee_cannot_reuse_old_hybrid_economics() {
    let (warm, pair, _upstream, provider) = fixture_fee(false, 10_000).await;
    sqlx::query("INSERT INTO hybrid_pair_reserves SELECT * FROM pair_reserves")
        .execute(&warm.pool)
        .await
        .unwrap();
    let cold = app_state(warm.pool.clone(), cold_fixture_lcd(provider.uri.clone()));
    let (pool, lcd, p) = (cold.pool.clone(), cold.lcd.clone(), pair.clone());
    let worker = tokio::spawn(async move {
        book_snapshot::snapshot_single_pair(&pool, &lcd, &p, Some(101))
            .await
            .map_err(|error| error.to_string())
    });
    provider.waiting().await;
    publishers(&cold).await;
    assert!(quote(&cold).await.is_err());
    provider.release.notify_waiters();
    worker.await.unwrap().unwrap();
    publishers(&cold).await;
    assert!(
        quote(&cold).await.is_err(),
        "a complete maximum fee cannot borrow a prior positive fee"
    );
    let raw = pair_reserves::get_all_pair_reserves(&cold.pool)
        .await
        .unwrap();
    assert_eq!(raw.values().next().unwrap().fee_bps, 10_000);
}

/// Give the scheduled publisher both real generations. The old book is deliberately
/// kept in SQL: deleting it would bypass the preferential-hybrid bug under test.
async fn seed_independent_views(state: &AppState) -> (i64, i64) {
    let at = chrono::Utc::now();
    // Leave 3.75 seconds for owned cold scheduled admission. The original
    // hybrid still expires inside the four-second pending-fee window, even
    // before subtracting admission time. Neither production TTL is extended.
    let hybrid_at = at - chrono::Duration::milliseconds(16_250);
    let pool_at = at - chrono::Duration::milliseconds(9_000);
    sqlx::query("INSERT INTO hybrid_pair_reserves SELECT * FROM pair_reserves ON CONFLICT(pair_id) DO NOTHING")
        .execute(&state.pool).await.unwrap();
    sqlx::query("UPDATE hybrid_pair_reserves SET snapshot_at=$1,block_height=100")
        .bind(hybrid_at)
        .execute(&state.pool)
        .await
        .unwrap();
    sqlx::query(
        "UPDATE pair_reserves SET snapshot_at=$1,block_height=101,reserve_0=$2,reserve_1=$2",
    )
    .bind(pool_at)
    .bind(bigdecimal::BigDecimal::from(RAW * 3))
    .execute(&state.pool)
    .await
    .unwrap();
    sqlx::query("INSERT INTO resting_limit_orders(pair_id,order_id,side,price,remaining,block_height,snapshot_at,walk_index) SELECT id,1,'bid',1.01,$1,100,$2,0 FROM pairs WHERE contract_address=$3")
        .bind(bigdecimal::BigDecimal::from(RAW)).bind(hybrid_at).bind(PAIR)
        .execute(&state.pool).await.unwrap();
    (hybrid_at.timestamp_millis(), pool_at.timestamp_millis())
}

async fn publish_view(state: &AppState, admission: bool) {
    if admission {
        local_quote::ingest_once(state).await.unwrap();
    } else {
        local_quote::refresh_reserves(state).await.unwrap();
    }
}

fn view_request(receive: bool, book: bool) -> Value {
    json!({
        "token_in":CL8Y,"token_out":USTR,"max_spread":"0.05",
        "amount_in":if receive {Value::Null} else {json!("1000000000000000000")},
        "amount_out":if receive {json!("1000000000000000000")} else {Value::Null},
        "pool_only":!book
    })
}

async fn view_quote(
    state: &AppState,
    receive: bool,
    book: bool,
) -> Result<Value, (axum::http::StatusCode, String)> {
    let request = serde_json::from_value(view_request(receive, book)).unwrap();
    local_quote::solve(state, &request).await.map(|body| body.0)
}

async fn manual_book_quote(state: &AppState) -> Result<Value, (axum::http::StatusCode, String)> {
    let plan = [Some(
        serde_json::from_value(json!({
            "pool_input":"0","book_input":"1000000000000000000",
            "max_maker_fills":8,"book_start_hint":null
        }))
        .unwrap(),
    )];
    let request = serde_json::from_value(view_request(false, true)).unwrap();
    local_quote::solve_post(state, &request, Some(&plan))
        .await
        .map(|body| body.0)
}

fn original_view(body: &Value, height: i64, observed: i64) {
    useful(body);
    assert_eq!(
        body["snapshot_block_height"], height,
        "a view must preserve its own height"
    );
    assert_eq!(body["execution_metadata"]["observed_at_ms"], observed);
    assert_eq!(
        body["execution_metadata"]["expires_at_ms"],
        observed + 20_000,
        "publication and fallback must not renew the original 20-second deadline"
    );
}

async fn after_original_deadline(observed: i64) {
    let remaining = (observed + 20_025 - chrono::Utc::now().timestamp_millis()).max(0);
    tokio::time::sleep(Duration::from_millis(remaining as u64)).await;
}

async fn both_views_rejected(state: &AppState, provider: &GatedProvider) {
    let before_calls = provider.calls.load(Ordering::SeqCst);
    assert!(view_quote(state, false, false).await.is_err());
    assert!(view_quote(state, true, false).await.is_err());
    assert!(view_quote(state, false, true).await.is_err());
    assert!(manual_book_quote(state).await.is_err());
    assert_eq!(
        provider.calls.load(Ordering::SeqCst),
        before_calls,
        "negative cached quotes trigger zero RPC"
    );
}

#[tokio::test]
#[serial_test::serial]
async fn recovery1464_dual_views_survive_hybrid_expiry_without_another_scheduled_pass() {
    for admission in [false, true] {
        let (state, _pair, _upstream, provider) =
            fixture_fee_with_ingestion(false, 35, false).await;
        let (hybrid_at, pool_at) = seed_independent_views(&state).await;
        if !admission {
            local_quote::ingest_fixture(&state).await.unwrap();
        }
        publish_view(&state, admission).await;
        let before_calls = provider.calls.load(Ordering::SeqCst);
        eprintln!(
            "dual-view publisher_admission={admission} first_quote_age_ms={}",
            chrono::Utc::now().timestamp_millis() - hybrid_at
        );
        assert!(
            chrono::Utc::now().timestamp_millis() < hybrid_at + 20_000,
            "scheduled fixture crossed its original hybrid deadline before quoting"
        );
        original_view(&manual_book_quote(&state).await.unwrap(), 100, hybrid_at);
        let optimized = view_quote(&state, false, true).await.unwrap();
        original_view(&optimized, 100, hybrid_at);
        assert!(!optimized["router_operations"][0]["terra_swap"]["hybrid"].is_null());
        original_view(&manual_book_quote(&state).await.unwrap(), 100, hybrid_at);
        // Warm exact-input and receive cache entries before the hybrid expires.
        original_view(
            &view_quote(&state, false, false).await.unwrap(),
            101,
            pool_at,
        );
        original_view(
            &view_quote(&state, true, false).await.unwrap(),
            101,
            pool_at,
        );
        after_original_deadline(hybrid_at).await;
        let pool = view_quote(&state, false, false).await.expect(
            "still-fresh complete pool must survive hybrid expiry without a publisher pass",
        );
        original_view(&pool, 101, pool_at);
        assert!(pool["router_operations"][0]["terra_swap"]["hybrid"].is_null());
        original_view(
            &view_quote(&state, true, false).await.unwrap(),
            101,
            pool_at,
        );
        let automatic = view_quote(&state, false, true).await.unwrap();
        original_view(&automatic, 101, pool_at);
        assert!(
            automatic["router_operations"][0]["terra_swap"]["hybrid"].is_null(),
            "the fresh pool must never splice in an expired book"
        );
        assert_eq!(
            manual_book_quote(&state).await.unwrap_err().0,
            axum::http::StatusCode::SERVICE_UNAVAILABLE,
            "an expired declared book is unavailable and cannot fall back silently"
        );
        assert_eq!(
            provider.calls.load(Ordering::SeqCst),
            before_calls,
            "quotes trigger zero RPC"
        );
        after_original_deadline(pool_at).await;
        both_views_rejected(&state, &provider).await;
        assert_eq!(provider.calls.load(Ordering::SeqCst), before_calls);
    }
}

#[tokio::test]
#[serial_test::serial]
async fn recovery1464_dual_views_pending_fee_keeps_pool_and_shares_immediate_invalidation() {
    // Each publisher must capture the newer complete pool even while the older
    // hybrid remains eligible. Then a real producer commits a third, fee=-1 raw.
    for admission in [false, true] {
        for outcome in ["cancel", "changed", "negative", "deadline"] {
            let (state, pair, _upstream, provider) =
                fixture_fee_with_ingestion(outcome == "negative", 10_000, false).await;
            let (hybrid_at, pool_at) = seed_independent_views(&state).await;
            if !admission {
                local_quote::ingest_fixture(&state).await.unwrap();
            }
            publish_view(&state, admission).await;
            eprintln!(
                "dual-view publisher_admission={admission} outcome={outcome} first_quote_age_ms={}",
                chrono::Utc::now().timestamp_millis() - hybrid_at
            );
            assert!(
                chrono::Utc::now().timestamp_millis() < hybrid_at + 20_000,
                "scheduled fixture crossed its original hybrid deadline before quoting"
            );
            original_view(&manual_book_quote(&state).await.unwrap(), 100, hybrid_at);
            original_view(
                &view_quote(&state, false, true).await.unwrap(),
                100,
                hybrid_at,
            );
            original_view(&manual_book_quote(&state).await.unwrap(), 100, hybrid_at);
            let complete_pool = view_quote(&state, false, false).await.unwrap();
            original_view(&complete_pool, 101, pool_at);
            original_view(
                &view_quote(&state, true, false).await.unwrap(),
                101,
                pool_at,
            );
            let (pool, lcd, p) = (state.pool.clone(), state.lcd.clone(), pair.clone());
            let worker = tokio::spawn(async move {
                book_snapshot::snapshot_single_pair(&pool, &lcd, &p, Some(102))
                    .await
                    .map_err(|error| error.to_string())
            });
            provider.waiting().await;
            let fee_waiting = Instant::now();
            let raw = pair_reserves::get_all_pair_reserves(&state.pool)
                .await
                .unwrap();
            assert_eq!(raw.values().next().unwrap().block_height, Some(102));
            assert_eq!(raw.values().next().unwrap().fee_bps, -1);
            if outcome == "negative" {
                // Permanent headers revoke both immutable views while the body
                // remains gated, before either publisher can observe completion.
                tokio::time::timeout(Duration::from_secs(1), async {
                    while view_quote(&state, false, false).await.is_ok() {
                        tokio::task::yield_now().await;
                    }
                })
                .await
                .unwrap();
                both_views_rejected(&state, &provider).await;
                assert!(!worker.is_finished());
            } else {
                publish_view(&state, admission).await;
                let quote_calls = provider.calls.load(Ordering::SeqCst);
                original_view(
                    &view_quote(&state, false, true).await.unwrap(),
                    100,
                    hybrid_at,
                );
                original_view(&manual_book_quote(&state).await.unwrap(), 100, hybrid_at);
                if outcome == "cancel" || outcome == "changed" {
                    assert!(
                        chrono::Utc::now().timestamp_millis() < hybrid_at + 20_000,
                        "invalidation must be exercised while BOTH original views are fresh"
                    );
                    if outcome == "changed" {
                        provider.release.notify_waiters();
                        worker.await.unwrap().unwrap();
                    } else {
                        worker.abort();
                        let _ = worker.await;
                    }
                    let completed_calls = provider.calls.load(Ordering::SeqCst);
                    both_views_rejected(&state, &provider).await;
                    assert_eq!(provider.calls.load(Ordering::SeqCst), completed_calls);
                    publish_view(&state, admission).await;
                    both_views_rejected(&state, &provider).await;
                    continue;
                }
                after_original_deadline(hybrid_at).await;
                let retained_pool = view_quote(&state, false, false).await.expect(
                    "pending raw must retain the independently captured complete pool, not only old hybrid");
                original_view(&retained_pool, 101, pool_at);
                assert_eq!(
                    retained_pool["estimated_amount_out_net"],
                    complete_pool["estimated_amount_out_net"],
                    "pending reserves cannot borrow the retained fee or pool economics"
                );
                let automatic = view_quote(&state, false, true).await.unwrap();
                original_view(&automatic, 101, pool_at);
                assert!(automatic["router_operations"][0]["terra_swap"]["hybrid"].is_null());
                original_view(
                    &view_quote(&state, true, false).await.unwrap(),
                    101,
                    pool_at,
                );
                assert_eq!(manual_book_quote(&state).await.unwrap_err().0,
                    axum::http::StatusCode::SERVICE_UNAVAILABLE,
                    "expired manual book must be an availability blocker while the complete pool remains fresh");
                assert_eq!(provider.calls.load(Ordering::SeqCst), quote_calls);
                if outcome == "deadline" {
                    // The four-second pending guard remains shorter than the
                    // pool's original deadline; a stalled fee cannot extend it.
                    tokio::time::sleep(
                        Duration::from_millis(4_025).saturating_sub(fee_waiting.elapsed()),
                    )
                    .await;
                    both_views_rejected(&state, &provider).await;
                }
            }
            worker.abort();
            let _ = worker.await;
            both_views_rejected(&state, &provider).await;
            publish_view(&state, admission).await;
            both_views_rejected(&state, &provider).await;
        }
    }
}
