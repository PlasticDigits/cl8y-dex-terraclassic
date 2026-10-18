//! Two independently owned real HTTP APIs and an actual forwarding proxy.
//! Source observations/LCD are explicit local fixtures; admission, quotes,
//! block processing and advisory leadership use production code unchanged.
//! This is not OS binary-process or Coolify/Traefik execution evidence.
use super::*;
use axum::{
    body::Body,
    extract::State,
    http::{Request, StatusCode},
    response::Response,
    routing::get as route_get,
    Router,
};
use cl8y_dex_indexer::{
    api::{build_router_with_api_readiness, rolling_readiness::ApiReadinessProfile},
    db::queries::state as checkpoints,
    indexer::{block_indexer, oracle},
};
use sqlx::Connection;
use std::{
    sync::{
        atomic::{AtomicUsize, Ordering},
        Mutex,
    },
    time::Duration,
};
use tokio::sync::RwLock;
use tokio_util::sync::CancellationToken;
use wiremock::matchers::{path, query_param};

fn address(seed: u8) -> String {
    bech32::encode("terra", vec![seed; 32].to_base32(), bech32::Variant::Bech32).unwrap()
}
const MAX_CLIENT_REQUESTS: usize = 150;
#[derive(Clone)]
struct ClientBudget {
    inner: reqwest::Client,
    used: Arc<AtomicUsize>,
}
impl ClientBudget {
    fn get(&self, url: impl reqwest::IntoUrl) -> reqwest::RequestBuilder {
        let count = self.used.fetch_add(1, Ordering::SeqCst) + 1;
        assert!(
            count <= MAX_CLIENT_REQUESTS,
            "owned handoff client HTTP budget exhausted"
        );
        self.inner.get(url)
    }
}
#[derive(Clone)]
struct Backend {
    name: &'static str,
    base: String,
    inflight: Arc<AtomicUsize>,
}
#[derive(Clone)]
struct Forwarder {
    client: reqwest::Client,
    active: Arc<RwLock<Vec<Backend>>>,
    cursor: Arc<AtomicUsize>,
}
struct Inflight(Arc<AtomicUsize>);
impl Drop for Inflight {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}
async fn proxy(State(state): State<Forwarder>, request: Request<Body>) -> Response {
    let node = {
        let nodes = state.active.read().await;
        let selected = nodes[state.cursor.fetch_add(1, Ordering::SeqCst) % nodes.len()].clone();
        // Reserve under the routing read lock so removal/drain cannot miss a
        // selected request which has not begun its transport yet.
        selected.inflight.fetch_add(1, Ordering::SeqCst);
        selected
    };
    let _guard = Inflight(node.inflight.clone());
    let target = format!(
        "{}{}",
        node.base,
        request.uri().path_and_query().unwrap().as_str()
    );
    // Forward the original request once. Never fabricate quote/health responses,
    // retry another node or hide a failed attempt behind the old API.
    let fetched = state.client.get(target).send().await;
    match fetched {
        Ok(response) => {
            let status = response.status();
            let headers = response.headers().clone();
            match response.bytes().await {
                Ok(bytes) if bytes.len() <= 262_144 => {
                    let mut outgoing = Response::builder()
                        .status(status)
                        .header("x-rehearsal-upstream", node.name);
                    for key in ["content-type", "cache-control"] {
                        if let Some(value) = headers.get(key) {
                            outgoing = outgoing.header(key, value);
                        }
                    }
                    outgoing.body(Body::from(bytes)).unwrap()
                }
                _ => Response::builder()
                    .status(StatusCode::BAD_GATEWAY)
                    .body(Body::empty())
                    .unwrap(),
            }
        }
        Err(_) => Response::builder()
            .status(StatusCode::BAD_GATEWAY)
            .body(Body::empty())
            .unwrap(),
    }
}
async fn spawn_api(
    state: AppState,
    config: &cl8y_dex_indexer::config::Config,
    profile: ApiReadinessProfile,
    name: &'static str,
) -> (Backend, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let app = build_router_with_api_readiness(state, config, profile);
    let task = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });
    (
        Backend {
            name,
            base,
            inflight: Arc::new(AtomicUsize::new(0)),
        },
        task,
    )
}
async fn get(client: &ClientBudget, base: &str, route: &str) -> (reqwest::StatusCode, Value) {
    let response = client.get(format!("{base}{route}")).send().await.unwrap();
    let status = response.status();
    (status, response.json().await.unwrap())
}
fn quote_paths(token: &str) -> [String; 2] {
    [format!("/api/v1/route/solve?token_in=uusd&token_out={token}&amount_in=1000000&max_spread=0.05&protection_policy=quote_relative_v1&hybrid_optimize=true&max_maker_fills=8"),
     format!("/api/v1/route/solve?token_in={token}&token_out=uusd&amount_in=1000000000000000000&max_spread=0.05&protection_policy=quote_relative_v1&hybrid_optimize=true&max_maker_fills=8")]
}
async fn index_controlled_block(
    upstream: &wiremock::MockServer,
    state: &AppState,
    config: &cl8y_dex_indexer::config::Config,
    market: &str,
    token: &str,
    offer: &str,
    height: i64,
    tx_hash: &str,
) {
    let hash = format!("OWNED-ROLLING-{height}");
    let trader = address(113);
    Mock::given(method("GET")).and(path(format!("/cosmos/base/tendermint/v1beta1/blocks/{height}"))).respond_with(ResponseTemplate::new(200).set_body_json(json!({"block_id":{"hash":hash},"block":{"header":{"chain_id":"columbus-5","height":height.to_string(),"time":chrono::Utc::now().to_rfc3339()}}}))).mount(upstream).await;
    Mock::given(method("GET")).and(path("/cosmos/tx/v1beta1/txs")).and(query_param("query",format!("tx.height={height}"))).respond_with(ResponseTemplate::new(200).set_body_json(json!({"tx_responses":[{"height":height.to_string(),"txhash":tx_hash,"timestamp":chrono::Utc::now().to_rfc3339(),"logs":[{"events":[{"type":"wasm","attributes":[{"key":"_contract_address","value":market},{"key":"action","value":"swap"},{"key":"sender","value":trader},{"key":"offer_amount","value":"1000000"},{"key":"return_amount","value":"950000000000000000"},{"key":"commission_amount","value":"30000000000000000"},{"key":"spread_amount","value":"20000000000000000"},{"key":"offer_asset","value":offer},{"key":"ask_asset","value":token}]}]}]}],"pagination":{"total":"1"}}))).mount(upstream).await;
    let meta = block_indexer::index_block_with_retries_cancel(
        &state.pool,
        &state.lcd,
        config,
        height,
        &oracle::new_shared_price(),
        &CancellationToken::new(),
        &state.indexer_health,
    )
    .await
    .unwrap();
    assert_eq!(meta.tx_count, 1);
    assert_eq!(
        checkpoints::get_indexer_checkpoint(&state.pool)
            .await
            .unwrap(),
        (height, Some(hash))
    );
    assert_eq!(
        sqlx::query_scalar::<_, i64>("SELECT count(*) FROM swap_events WHERE tx_hash=$1")
            .bind(tx_hash)
            .fetch_one(&state.pool)
            .await
            .unwrap(),
        1
    );
}
fn executable(body: &Value) {
    assert!(
        body["estimated_amount_out_net"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            > 0
    );
    let m = &body["execution_metadata"];
    assert!(
        m["minimum_receive"]
            .as_str()
            .unwrap()
            .parse::<u128>()
            .unwrap()
            > 0
    );
    assert!(m["expires_at_ms"].as_i64().unwrap() > chrono::Utc::now().timestamp_millis());
    assert!(body["snapshot_age_ms"]
        .as_i64()
        .is_some_and(|age| (0..=20_000).contains(&age)));
    assert_ne!(body["execution_supported"], false);
    assert_ne!(m["execution_supported"], false);
    assert!(m["trader"].is_null() && m["sender"].is_null());
}
async fn admitted(client: &ClientBudget, node: &Backend, paths: &[String; 2]) -> bool {
    if get(client, &node.base, "/health/serving").await.0 != reqwest::StatusCode::OK {
        return false;
    }
    if get(client, &node.base, "/ready-api").await.0 != reqwest::StatusCode::OK {
        return false;
    }
    for path in paths {
        let (status, body) = get(client, &node.base, path).await;
        if !status.is_success() {
            return false;
        }
        executable(&body);
    }
    true
}

#[tokio::test]
#[serial_test::serial]
async fn rolling_proxy_keeps_old_until_standby_native_admission_then_real_block_handoff() {
    tokio::time::timeout(Duration::from_secs(48), rehearsal())
        .await
        .expect("owned entire handoff fixture48s");
}
async fn rehearsal() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    let token = address(111);
    let token_id = asset(&pool, &token, 18).await;
    let custc = asset(&pool, CUSTC, 6).await;
    let market = address(112);
    pair(
        &pool,
        &market,
        custc,
        token_id,
        10_000 * 10u128.pow(6),
        10_000 * 10u128.pow(18),
    )
    .await;
    // Quote-only fixtures use an intentionally noncanonical mapper asset. Real
    // block parsing instead uses a separate canonical CW20 market.
    let ingest_offer = address(114);
    let ingest_market = address(115);
    let ingest_asset = asset(&pool, &ingest_offer, 6).await;
    pair(
        &pool,
        &ingest_market,
        ingest_asset,
        token_id,
        10_000 * 10u128.pow(6),
        10_000 * 10u128.pow(18),
    )
    .await;
    let mut config = common::test_config();
    config.hub_custc_address = CUSTC.into();
    config.hub_clunc_address = CLUNC.into();
    common::reference_marks::seed_reference_marks(
        &pool,
        &HubUsdConfig::from_indexer_config(&config),
        &[(&token, 1.into())],
    )
    .await;
    let upstream = Arc::new(common::lcd_mock::start_ordinary_code_mock().await);
    mount_supported_config_mock(&upstream, 0).await;
    Mock::given(method("GET"))
        .and(path("/terra/tax/v1beta1/params"))
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
    let mut old_state = app_state(pool.clone(), cold_fixture_lcd(upstream.uri()));
    old_state.wrap_mapper_address = Some("terra1mapperfixture1452".into());
    let mut new_state = app_state(pool.clone(), cold_fixture_lcd(upstream.uri()));
    new_state.wrap_mapper_address = old_state.wrap_mapper_address.clone();
    assert_ne!(
        old_state.lcd.chain_cache_namespace(),
        new_state.lcd.chain_cache_namespace(),
        "new admission cannot borrow old cache/namespace"
    );
    let profile=json!([{"token_in":"uusd","token_out":token,"amount_in":"1000000","max_spread":"0.05"},{"token_in":token,"token_out":"uusd","amount_in":"1000000000000000000","max_spread":"0.05"}]).to_string();
    let (old, old_server) = spawn_api(
        old_state.clone(),
        &config,
        ApiReadinessProfile::from_json(&profile),
        "old",
    )
    .await;
    let (new, new_server) = spawn_api(
        new_state.clone(),
        &config,
        ApiReadinessProfile::from_json(&profile),
        "new",
    )
    .await;
    let (bad, bad_server) = spawn_api(
        app_state(pool.clone(), cold_fixture_lcd(upstream.uri())),
        &config,
        ApiReadinessProfile::missing(),
        "never-ready",
    )
    .await;
    let client = ClientBudget {
        inner: reqwest::Client::builder()
            .timeout(Duration::from_secs(2))
            .build()
            .unwrap(),
        used: Arc::new(AtomicUsize::new(0)),
    };
    let paths = quote_paths(&token);
    let mut old_leader = sqlx::PgConnection::connect_with(pool.connect_options().as_ref())
        .await
        .unwrap();
    assert!(checkpoints::try_acquire_poller_lock(&mut old_leader)
        .await
        .unwrap());
    let mut next_leader = sqlx::PgConnection::connect_with(pool.connect_options().as_ref())
        .await
        .unwrap();
    assert!(!checkpoints::try_acquire_poller_lock(&mut next_leader)
        .await
        .unwrap());
    index_controlled_block(
        &upstream,
        &old_state,
        &config,
        &ingest_market,
        &token,
        &ingest_offer,
        14520,
        "OWNED-OLD-SWAP",
    )
    .await;
    let source_cancel = CancellationToken::new();
    let source_stop = source_cancel.clone();
    let source_pool = pool.clone();
    let source_hub = old_state.hub_usd.clone();
    let source_token = token.clone();
    // Explicit synthetic raw observations are independently scheduled, never
    // generated by a request. No quote output or admission result is injected.
    let source = tokio::spawn(async move {
        let mut timer = tokio::time::interval(Duration::from_secs(2));
        loop {
            tokio::select! {_=source_stop.cancelled()=>break,_=timer.tick()=>{
                sqlx::query("UPDATE pair_reserves SET snapshot_at=now()").execute(&source_pool).await.unwrap();
                common::reference_marks::seed_reference_marks(&source_pool,&source_hub,&[(&source_token,1.into())]).await;
            }}
        }
    });
    let old_cancel = CancellationToken::new();
    let mut old_producers = cl8y_dex_indexer::api::local_quote::start_ingestion_with_cancel(
        old_state.clone(),
        old_cancel.clone(),
    );
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    while !admitted(&client, &old, &paths).await {
        assert!(
            tokio::time::Instant::now() < deadline,
            "old scheduled native admission"
        );
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    let routing = Arc::new(RwLock::new(vec![old.clone()]));
    let forward = Forwarder {
        client: client.inner.clone(),
        active: routing.clone(),
        cursor: Arc::new(AtomicUsize::new(0)),
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_base = format!("http://{}", listener.local_addr().unwrap());
    let proxy_server = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new().fallback(route_get(proxy)).with_state(forward),
        )
        .await
        .unwrap();
    });
    let traffic_cancel = CancellationToken::new();
    let traffic_stop = traffic_cancel.clone();
    let samples = Arc::new(Mutex::new(Vec::new()));
    let measured = samples.clone();
    let traffic_client = client.clone();
    let traffic_base = proxy_base.clone();
    let traffic_paths = paths.clone();
    let phase = Arc::new(AtomicUsize::new(0));
    let traffic_phase = phase.clone();
    let traffic_old = old_state.clone();
    let traffic_upstream = upstream.clone();
    let traffic = tokio::spawn(async move {
        while !traffic_stop.is_cancelled() {
            for (direction, path) in traffic_paths.iter().enumerate() {
                let started = Instant::now();
                let phase_at_send = traffic_phase.load(Ordering::SeqCst);
                let response = traffic_client
                    .get(format!("{traffic_base}{path}"))
                    .send()
                    .await
                    .unwrap();
                let status = response.status();
                let node = response
                    .headers()
                    .get("x-rehearsal-upstream")
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned();
                let body: Value = response.json().await.unwrap();
                if status != reqwest::StatusCode::OK {
                    let progress_path = path.replace("/route/solve?", "/route/solve/progress?");
                    let (_, progress) = get(&traffic_client, &traffic_base, &progress_path).await;
                    let counters = traffic_old.lcd.provider_counters().await;
                    let mut classes = std::collections::BTreeMap::<String, usize>::new();
                    for request in traffic_upstream.received_requests().await.unwrap() {
                        let query = common::lcd_mock::smart_query_from_request(&request);
                        for key in query.as_object().into_iter().flat_map(|q| q.keys()) {
                            if key.len() <= 64
                                && key.bytes().all(|b| b.is_ascii_lowercase() || b == b'_')
                            {
                                *classes.entry(key.clone()).or_default() += 1;
                            }
                        }
                    }
                    eprintln!(
                        "HANDOFF_FAILURE_DIAGNOSTIC {}",
                        json!({"node":node,"query_classes":classes,"provider_attempts":counters.iter().map(|c|c.attempts).collect::<Vec<_>>(),"provider_failures":counters.iter().map(|c|c.failures).collect::<Vec<_>>(),"circuit_open":counters.iter().map(|c|c.circuit_open).collect::<Vec<_>>(),"eligibility":progress["quote_eligibility"],"blocker":progress["quote_blocker"]})
                    );
                }
                assert_eq!(
                    status,
                    reqwest::StatusCode::OK,
                    "real proxied native response {direction}: {body}"
                );
                executable(&body);
                measured.lock().unwrap().push((
                    node,
                    direction,
                    started.elapsed().as_millis(),
                    phase_at_send,
                ));
                // Focused continuity sampling, not a stress/throughput benchmark.
                tokio::time::sleep(Duration::from_millis(200)).await;
            }
        }
    });
    // Failed/cold candidates do not affect routing or the active leader.
    assert_eq!(
        get(&client, &new.base, "/health").await.0,
        reqwest::StatusCode::OK
    );
    assert!(!admitted(&client, &new, &paths).await);
    for _ in 0..3 {
        assert!(!admitted(&client, &bad, &paths).await);
    }
    assert_eq!(routing.read().await.len(), 1);
    assert_eq!(routing.read().await[0].name, "old");
    assert!(!checkpoints::try_acquire_poller_lock(&mut next_leader)
        .await
        .unwrap());
    assert_eq!(
        checkpoints::get_last_indexed_height(&pool).await.unwrap(),
        14520
    );
    index_controlled_block(
        &upstream,
        &old_state,
        &config,
        &ingest_market,
        &token,
        &ingest_offer,
        14521,
        "OWNED-OLD-CONTINUED",
    )
    .await;
    bad_server.abort();
    let new_cancel = CancellationToken::new();
    let mut new_producers = cl8y_dex_indexer::api::local_quote::start_ingestion_with_cancel(
        new_state.clone(),
        new_cancel.clone(),
    );
    while !admitted(&client, &new, &paths).await {
        assert!(
            tokio::time::Instant::now() < deadline,
            "standby native admission must precede handoff"
        );
        tokio::time::sleep(Duration::from_millis(500)).await;
    }
    assert_eq!(
        get(&client, &new.base, "/ready").await.0,
        reqwest::StatusCode::SERVICE_UNAVAILABLE
    );
    assert!(
        !checkpoints::try_acquire_poller_lock(&mut next_leader)
            .await
            .unwrap(),
        "standby is useful before acquiring old leadership"
    );
    *routing.write().await = vec![old.clone(), new.clone()];
    phase.store(1, Ordering::SeqCst);
    tokio::time::timeout(Duration::from_secs(2), async {
        while !samples
            .lock()
            .unwrap()
            .iter()
            .any(|(node, _, _, at)| node == "new" && *at == 1)
        {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("actual new proxy response before leader handoff");
    // Real ownership transfer and actual block RPC/parser/checkpoint processing.
    // Do not fabricate health.progress; /ready is not used to admit the API.
    old_leader.close().await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if checkpoints::try_acquire_poller_lock(&mut next_leader)
                .await
                .unwrap()
            {
                break;
            }
            tokio::time::sleep(Duration::from_millis(500)).await;
        }
    })
    .await
    .expect("same production500ms lock-wait guard");
    let height = 14522_i64;
    index_controlled_block(
        &upstream,
        &new_state,
        &config,
        &ingest_market,
        &token,
        &ingest_offer,
        height,
        "OWNED-NEW-SWAP",
    )
    .await;
    // Remove old from the real forwarding set, then wait for original in-flight
    // requests to drain before retiring its listener and producer owner.
    *routing.write().await = vec![new.clone()];
    phase.store(2, Ordering::SeqCst);
    tokio::time::timeout(Duration::from_secs(2), async {
        while old.inflight.load(Ordering::SeqCst) != 0 {
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    })
    .await
    .unwrap();
    old_server.abort();
    old_cancel.cancel();
    old_producers.shutdown().await;
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            let both = (0..2).all(|direction| {
                samples
                    .lock()
                    .unwrap()
                    .iter()
                    .any(|(node, d, _, at)| node == "new" && *d == direction && *at == 2)
            });
            if both {
                break;
            }
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    })
    .await
    .expect("both native directions through sole new proxy after handoff");
    traffic_cancel.cancel();
    traffic.await.unwrap();
    let recorded = samples.lock().unwrap();
    for node in ["old", "new"] {
        for direction in 0..2 {
            assert!(recorded
                .iter()
                .any(|(seen, d, _, _)| seen == node && *d == direction));
        }
    }
    let count = recorded.len();
    let maximum = recorded.iter().map(|(_, _, ms, _)| *ms).max().unwrap();
    drop(recorded);
    // Stop producer work before the request-only guard, preserving their original
    // observation clock. Requests themselves must add no validation/refresh RPC.
    new_cancel.cancel();
    new_producers.shutdown().await;
    source_cancel.cancel();
    source.await.unwrap();
    let baseline = upstream.received_requests().await.unwrap().len();
    for _ in 0..3 {
        assert!(admitted(&client, &new, &paths).await);
    }
    assert_eq!(upstream.received_requests().await.unwrap().len(), baseline);
    // Controlled source loss is a negative scope after the continuous-positive
    // stream ended: dynamic native503 cannot unroute the only admitted API.
    sqlx::query("DELETE FROM pair_reserves")
        .execute(&pool)
        .await
        .unwrap();
    cl8y_dex_indexer::api::local_quote::refresh_reserves(&new_state)
        .await
        .unwrap();
    assert_eq!(
        get(&client, &new.base, "/ready-api").await.0,
        reqwest::StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        get(&client, &proxy_base, "/health/serving").await.0,
        reqwest::StatusCode::OK
    );
    assert_eq!(
        get(
            &client,
            &proxy_base,
            "/api/v1/evidence/digest?day=2026-10-18"
        )
        .await
        .0,
        reqwest::StatusCode::OK
    );
    assert_eq!(
        get(&client, &proxy_base, &paths[0]).await.0,
        reqwest::StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(upstream.received_requests().await.unwrap().len(), baseline);
    next_leader.close().await.unwrap();
    new_server.abort();
    proxy_server.abort();
    println!(
        "ROLLING_HANDOFF_RECEIPT {}",
        json!({"schema":"owned_api_rolling_handoff_v1","scope":"independent_http_appstates_and_production_block_indexer; not_os_binaries_or_coolify_proxy","measurement_profile":"paced_native_200ms_total_client_cap150_v1","client_http_requests":client.used.load(Ordering::SeqCst),"client_http_cap":MAX_CLIENT_REQUESTS,"proxy_requests":count,"native_quote_successes":count,"http_failures":0,"request_lcd_calls":0,"parsed_blocks_before_handoff":2,"parsed_blocks_after_handoff":1,"max_loopback_ms":maximum,"independent_namespaces":true,"standby_indexer_ready_status":503,"cold_candidate_rejected":true,"never_ready_candidate_left_old_leader_and_api_unchanged":true,"actual_advisory_lock_transfer":true,"actual_checkpoint_height":height,"old_inflight_drained":true,"source_loss_dynamic503_latch200_cheap_api200":true,"positive_scope":"continuous_proxied_bidirectional_native_quotes; intentional_source_loss_control_separate"})
    );
}
