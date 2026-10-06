//! #1394: discovery admission and negative cache must preserve legitimate persistence.
//! Shared Postgres scenarios are serialized; all LCD responses are local wiremock fixtures.

mod common;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::time::Duration;

use base64::Engine;
use bech32::ToBase32;
use cl8y_dex_indexer::db::queries::pairs;
use cl8y_dex_indexer::indexer::pair_discovery::{
    DiscoverPairError, discover_new_pair, get_or_discover_pair, is_rejected_pair_error,
};
use cl8y_dex_indexer::lcd::{LcdClient, LcdError};
use serde_json::{Value, json};
use serial_test::serial;
use sqlx::PgPool;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, Request, ResponseTemplate};

fn address(seed: u8) -> String {
    bech32::encode("terra", [seed; 32].to_base32(), bech32::Variant::Bech32).unwrap()
}

fn pair_info(pair: &str) -> Value {
    json!({
        "asset_infos": [
            {"native_token": {"denom": "uluna"}},
            {"native_token": {"denom": "uusd"}}
        ],
        "contract_addr": pair,
        "liquidity_token": address(250)
    })
}

fn contract(req: &Request) -> &str {
    req.url.path().split('/').nth(5).unwrap()
}

async fn mount(
    server: &MockServer,
    responder: impl Fn(&Request) -> ResponseTemplate + Send + Sync + 'static,
) {
    common::lcd_mock::mock_chain_identity(server, "columbus-5").await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/.+$"))
        .respond_with(responder)
        .mount(server)
        .await;
}

fn success_response(req: &Request, factory: &str, pair: &str) -> ResponseTemplate {
    let query = common::lcd_mock::smart_query_from_request(req);
    let data = success_data(contract(req), &query, factory, pair);
    ResponseTemplate::new(200).set_body_json(json!({"data": data}))
}

fn success_data(contract: &str, query: &Value, factory: &str, pair: &str) -> Value {
    if contract == factory {
        assert!(query["pair"]["asset_infos"].is_array());
        json!({"pair": pair_info(pair)})
    } else if query.get("pair").is_some() {
        assert_eq!(contract, pair);
        pair_info(pair)
    } else if query.get("get_fee_config").is_some() {
        json!({"fee_config": {"fee_bps": 30, "treasury": address(251)}})
    } else if query.get("get_hooks").is_some() {
        json!({"hooks": [address(252)]})
    } else {
        panic!("unexpected discovery query: {query}");
    }
}

async fn legitimate_server(factory: &str, pair: &str) -> MockServer {
    let server = MockServer::start().await;
    let factory = factory.to_owned();
    let pair = pair.to_owned();
    mount(&server, move |req| success_response(req, &factory, &pair)).await;
    server
}

async fn smart_requests(server: &MockServer) -> usize {
    server
        .received_requests()
        .await
        .unwrap()
        .iter()
        .filter(|r| r.url.path().contains("/smart/"))
        .count()
}

fn client(server: &MockServer) -> LcdClient {
    LcdClient::new(vec![server.uri()], 1_000, 0, "columbus-5")
}

async fn assert_no_writes(pool: &PgPool) {
    for table in ["pairs", "assets"] {
        let count: i64 = sqlx::query_scalar(&format!("SELECT COUNT(*) FROM {table}"))
            .fetch_one(pool)
            .await
            .unwrap();
        assert_eq!(count, 0, "rejected discovery wrote to {table}");
    }
}

async fn assert_persisted(pool: &PgPool, pair: &str) {
    let row = pairs::get_pair_by_address(pool, pair)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.contract_address, pair);
    assert_eq!(row.fee_bps, Some(30));
    assert_eq!(row.hooks, vec![address(252)]);
    assert_eq!(row.lp_token.as_deref(), Some(address(250).as_str()));
    let assets: Vec<(String, String)> =
        sqlx::query_as("SELECT denom, symbol FROM assets ORDER BY denom")
            .fetch_all(pool)
            .await
            .unwrap();
    assert_eq!(
        assets,
        vec![
            ("uluna".into(), "LUNC".into()),
            ("uusd".into(), "USTC".into())
        ]
    );
    let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM pairs")
        .fetch_one(pool)
        .await
        .unwrap();
    assert_eq!(count, 1);
}

#[tokio::test]
#[serial]
async fn rejection_on_lcd_a_does_not_suppress_legitimate_pair_on_lcd_b() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let pair = address(1);
    let factory = address(2);
    let foreign = MockServer::start().await;
    mount(&foreign, |_| {
        ResponseTemplate::new(500).set_body_string("unknown variant `pair`")
    })
    .await;
    let err = discover_new_pair(&pool, &client(&foreign), &factory, &pair)
        .await
        .unwrap_err();
    assert!(is_rejected_pair_error(&*err));
    assert_no_writes(&pool).await;

    let legitimate = legitimate_server(&factory, &pair).await;
    discover_new_pair(&pool, &client(&legitimate), &factory, &pair)
        .await
        .unwrap();
    assert_persisted(&pool, &pair).await;
    assert_eq!(smart_requests(&foreign).await, 1);
    assert_eq!(smart_requests(&legitimate).await, 4);
}

#[tokio::test]
#[serial]
async fn rejection_in_factory_a_does_not_suppress_factory_b_on_same_client() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let pair = address(3);
    let factory_a = address(4);
    let factory_b = address(5);
    let server = MockServer::start().await;
    let a = factory_a.clone();
    let b = factory_b.clone();
    let p = pair.clone();
    mount(&server, move |req| {
        if contract(req) == a {
            ResponseTemplate::new(500).set_body_string("pair not found")
        } else {
            success_response(req, &b, &p)
        }
    })
    .await;
    let lcd = client(&server);
    let err = discover_new_pair(&pool, &lcd, &factory_a, &pair)
        .await
        .unwrap_err();
    assert!(is_rejected_pair_error(&*err));
    assert_no_writes(&pool).await;
    discover_new_pair(&pool, &lcd, &factory_b, &pair)
        .await
        .unwrap();
    assert_persisted(&pool, &pair).await;
    assert_eq!(smart_requests(&server).await, 6);
}

#[tokio::test]
#[serial]
async fn new_client_retries_changed_endpoint_without_inheriting_old_rejection() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let pair = address(6);
    let factory = address(7);
    let server = MockServer::start().await;
    let reject = Arc::new(AtomicBool::new(true));
    let r = reject.clone();
    let f = factory.clone();
    let p = pair.clone();
    mount(&server, move |req| {
        if r.load(Ordering::SeqCst) {
            ResponseTemplate::new(500).set_body_string("unknown variant `pair`")
        } else {
            success_response(req, &f, &p)
        }
    })
    .await;
    let old_client = client(&server);
    assert!(
        get_or_discover_pair(&pool, &old_client, &factory, &pair)
            .await
            .unwrap()
            .is_none()
    );
    reject.store(false, Ordering::SeqCst);
    assert!(
        get_or_discover_pair(&pool, &old_client.clone(), &factory, &pair)
            .await
            .unwrap()
            .is_none()
    );
    assert_eq!(smart_requests(&server).await, 1);
    discover_new_pair(&pool, &client(&server), &factory, &pair)
        .await
        .unwrap();
    assert_persisted(&pool, &pair).await;
    assert_eq!(smart_requests(&server).await, 5);
}

#[tokio::test]
#[serial]
async fn repeated_foreign_pair_has_no_database_writes_or_repeat_lcd_queries() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let pair = address(8);
    let factory = address(9);
    let server = MockServer::start().await;
    let f = factory.clone();
    let p = pair.clone();
    mount(&server, move |req| {
        if contract(req) == f {
            ResponseTemplate::new(500).set_body_string("pair not found")
        } else {
            success_response(req, &f, &p)
        }
    })
    .await;
    let lcd = client(&server);
    let first = discover_new_pair(&pool, &lcd, &factory, &pair)
        .await
        .unwrap_err();
    assert!(is_rejected_pair_error(&*first));
    let cached = discover_new_pair(&pool, &lcd.clone(), &factory, &pair)
        .await
        .unwrap_err();
    assert!(is_rejected_pair_error(&*cached));
    assert!(cached.to_string().contains("recently rejected"));
    assert!(format!("{cached:?}").contains("Unlisted"));
    assert!(
        get_or_discover_pair(&pool, &lcd, &factory, &pair)
            .await
            .unwrap()
            .is_none()
    );
    assert_no_writes(&pool).await;
    assert_eq!(smart_requests(&server).await, 2);
}

#[tokio::test]
#[serial]
async fn transient_500_and_timeout_do_not_negative_cache_legitimate_pairs() {
    let pool = common::setup_pool().await;
    for (seed, timeout) in [(10, false), (12, true)] {
        common::clean_db(&pool).await;
        let pair = address(seed);
        let factory = address(seed + 1);
        let server = MockServer::start().await;
        let calls = Arc::new(AtomicUsize::new(0));
        let c = calls.clone();
        let f = factory.clone();
        let p = pair.clone();
        mount(&server, move |req| {
            if c.fetch_add(1, Ordering::SeqCst) == 0 {
                if timeout {
                    success_response(req, &f, &p).set_delay(Duration::from_secs(2))
                } else {
                    ResponseTemplate::new(500).set_body_string("temporary upstream unavailable")
                }
            } else {
                success_response(req, &f, &p)
            }
        })
        .await;
        let lcd = LcdClient::new(
            vec![server.uri()],
            if timeout { 1_000 } else { 5_000 },
            0,
            "columbus-5",
        );
        let err = discover_new_pair(&pool, &lcd, &factory, &pair)
            .await
            .unwrap_err();
        assert!(
            !is_rejected_pair_error(&*err),
            "transient error was cached: {err}"
        );
        assert_no_writes(&pool).await;
        discover_new_pair(&pool, &lcd, &factory, &pair)
            .await
            .unwrap();
        assert_persisted(&pool, &pair).await;
        assert_eq!(calls.load(Ordering::SeqCst), 5);
    }
}

#[tokio::test]
#[serial]
async fn concurrent_discovery_survives_failover_and_persists_only_one_pair() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let pair = address(14);
    let factory = address(15);
    let failing = MockServer::start().await;
    mount(&failing, |_| ResponseTemplate::new(503)).await;
    let healthy = legitimate_server(&factory, &pair).await;
    let lcd = LcdClient::new(
        vec![failing.uri(), healthy.uri()],
        1_000,
        30_000,
        "columbus-5",
    );
    let other = lcd.clone();
    let (first, second) = tokio::join!(
        discover_new_pair(&pool, &lcd, &factory, &pair),
        discover_new_pair(&pool, &other, &factory, &pair),
    );
    let (winner, deferred) = match (first, second) {
        (Ok(winner), Err(deferred)) | (Err(deferred), Ok(winner)) => (winner, deferred),
        other => panic!("one concurrent discovery must win and one must defer: {other:?}"),
    };
    assert!(!is_rejected_pair_error(&*deferred));
    assert!(matches!(
        deferred.downcast_ref::<DiscoverPairError>(),
        Some(DiscoverPairError::Deferred { .. })
    ));
    let retried = get_or_discover_pair(&pool, &other, &factory, &pair)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(winner.id, retried.id);
    assert_persisted(&pool, &pair).await;
    assert!(!failing.received_requests().await.unwrap().is_empty());
    assert_eq!(smart_requests(&healthy).await, 4);
}

#[tokio::test]
#[serial]
async fn getter_waits_for_coalesced_discovery_without_dropping_legitimate_pair() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let pair = address(100);
    let factory = address(101);
    let (started_tx, started_rx) = tokio::sync::oneshot::channel();
    let started_tx = Arc::new(std::sync::Mutex::new(Some(started_tx)));
    let release = Arc::new(tokio::sync::Notify::new());
    let hits = Arc::new(AtomicUsize::new(0));
    let f = factory.clone();
    let p = pair.clone();
    let response_release = release.clone();
    let request_hits = hits.clone();
    let app = axum::Router::new()
        .route(
            "/cosmos/base/tendermint/v1beta1/node_info",
            axum::routing::get(|| async {
                axum::Json(json!({"default_node_info":{"network":"columbus-5"}}))
            }),
        )
        .fallback(axum::routing::get(move |uri: axum::http::Uri| {
            let f = f.clone();
            let p = p.clone();
            let started_tx = started_tx.clone();
            let release = response_release.clone();
            let hits = request_hits.clone();
            async move {
                hits.fetch_add(1, Ordering::SeqCst);
                let contract = uri.path().split('/').nth(5).unwrap();
                let encoded = uri.path().split("/smart/").nth(1).unwrap();
                let bytes = base64::engine::general_purpose::STANDARD
                    .decode(encoded)
                    .unwrap();
                let query: Value = serde_json::from_slice(&bytes).unwrap();
                if contract == p && query.get("pair").is_some() {
                    let tx = started_tx.lock().unwrap().take().unwrap();
                    tx.send(()).unwrap();
                    // Keep the winner's permit held until the test observes the
                    // getter's coalesced deferral. No elapsed-time race is involved.
                    release.notified().await;
                }
                axum::Json(json!({"data": success_data(contract, &query, &f, &p)}))
            }
        }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel();
    let server = tokio::spawn(async move {
        axum::serve(listener, app)
            .with_graceful_shutdown(async {
                let _ = shutdown_rx.await;
            })
            .await
            .unwrap();
    });
    let lcd = LcdClient::new(vec![endpoint], 10_000, 0, "columbus-5");
    let winning_pool = pool.clone();
    let winning_lcd = lcd.clone();
    let winning_factory = factory.clone();
    let winning_pair = pair.clone();
    let winning_discovery = tokio::spawn(async move {
        discover_new_pair(&winning_pool, &winning_lcd, &winning_factory, &winning_pair).await
    });
    started_rx.await.unwrap();
    let waiting_pool = pool.clone();
    let waiting_lcd = lcd.clone();
    let waiting_factory = factory.clone();
    let waiting_pair = pair.clone();
    let waiting_getter = tokio::spawn(async move {
        get_or_discover_pair(&waiting_pool, &waiting_lcd, &waiting_factory, &waiting_pair).await
    });
    tokio::time::timeout(Duration::from_secs(5), async {
        while lcd.pair_discovery_counters().deferred_coalesced == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("getter must defer while the winner's admission permit is held");
    release.notify_one();
    let winner = winning_discovery.await.unwrap().unwrap();
    let discovered = tokio::time::timeout(Duration::from_secs(5), waiting_getter)
        .await
        .expect("getter must retry after the winning discovery releases admission")
        .unwrap()
        .unwrap()
        .expect("coalesced admission must not discard a legitimate pair event");
    assert_eq!(discovered.id, winner.id);
    assert_eq!(discovered.contract_address, pair);
    assert_persisted(&pool, &pair).await;
    assert_eq!(hits.load(Ordering::SeqCst), 4);
    let counters = lcd.pair_discovery_counters();
    assert!(counters.deferred_coalesced >= 1);
    assert_eq!(counters.discovered, 1);
    shutdown_tx.send(()).unwrap();
    server.await.unwrap();
}

#[tokio::test]
#[serial]
async fn getter_transient_failure_soft_skips_and_direct_discovery_preserves_error() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let pair = address(102);
    let factory = address(103);
    let server = MockServer::start().await;
    mount(&server, |_| {
        ResponseTemplate::new(503).set_body_string("upstream temporarily unavailable")
    })
    .await;
    let lcd = client(&server);
    assert!(
        get_or_discover_pair(&pool, &lcd, &factory, &pair)
            .await
            .expect("foreign/transient discovery must not halt block indexing")
            .is_none()
    );
    assert_eq!(lcd.pair_discovery_counters().transient_failures, 1);
    assert_no_writes(&pool).await;
    assert_eq!(smart_requests(&server).await, 1);
    let err = discover_new_pair(&pool, &lcd, &factory, &pair)
        .await
        .expect_err("direct discovery must preserve the original LCD error");
    assert!(!is_rejected_pair_error(&*err));
    assert!(err.downcast_ref::<LcdError>().is_some());
    assert_no_writes(&pool).await;
    assert_eq!(smart_requests(&server).await, 2);
    assert_eq!(lcd.pair_discovery_counters().transient_failures, 2);
}

#[tokio::test]
#[serial]
async fn factory_transient_error_type_is_preserved_and_getter_retry_succeeds() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let pair = address(104);
    let factory = address(105);
    let server = MockServer::start().await;
    let factory_calls = Arc::new(AtomicUsize::new(0));
    let calls = factory_calls.clone();
    let f = factory.clone();
    let p = pair.clone();
    mount(&server, move |req| {
        if contract(req) == f && calls.fetch_add(1, Ordering::SeqCst) == 0 {
            ResponseTemplate::new(503).set_body_string("factory upstream temporarily unavailable")
        } else {
            success_response(req, &f, &p)
        }
    })
    .await;
    let lcd = client(&server);
    let err = discover_new_pair(&pool, &lcd, &factory, &pair)
        .await
        .expect_err("direct discovery must preserve the original factory LCD error");
    assert!(err.downcast_ref::<LcdError>().is_some());
    assert!(!is_rejected_pair_error(&*err));
    assert_no_writes(&pool).await;
    assert_eq!(smart_requests(&server).await, 2);

    let discovered = get_or_discover_pair(&pool, &lcd, &factory, &pair)
        .await
        .unwrap()
        .expect("factory recovery must permit discovery on the same client");
    assert_eq!(discovered.contract_address, pair);
    assert_persisted(&pool, &pair).await;
    assert_eq!(smart_requests(&server).await, 6);
    assert_eq!(factory_calls.load(Ordering::SeqCst), 2);
    let counters = lcd.pair_discovery_counters();
    assert_eq!(counters.transient_failures, 1);
    assert_eq!(counters.discovered, 1);
    assert_eq!(counters.cached_unlisted, 0);
    assert_eq!(counters.cached_not_pair, 0);
    assert_eq!(counters.rejected_unlisted, 0);
    assert_eq!(counters.rejected_not_pair, 0);
}
