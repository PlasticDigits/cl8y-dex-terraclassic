//! Owned HTTP/lock rehearsal with explicitly synthetic source observations.
//! No quote/result is injected; scheduled production API producers own admission.
use super::*;
use cl8y_dex_indexer::api::{
    build_router_with_api_readiness, rolling_readiness::ApiReadinessProfile,
};
use cl8y_dex_indexer::db::queries::state as checkpoints;
use sqlx::Connection;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

fn checked_address(seed: u8) -> String {
    bech32::encode("terra", vec![seed; 32].to_base32(), bech32::Variant::Bech32).unwrap()
}
async fn json_get(
    client: &reqwest::Client,
    base: &str,
    path: &str,
) -> (reqwest::StatusCode, Value) {
    let response = client.get(format!("{base}{path}")).send().await.unwrap();
    let status = response.status();
    if matches!(path, "/ready-api" | "/health/serving") {
        assert_eq!(response.headers()["cache-control"], "no-store");
    }
    (status, response.json().await.unwrap())
}

#[tokio::test]
#[serial_test::serial]
async fn api_rolling_readiness_standby_native_cache_and_owned_leader_handoff() {
    tokio::time::timeout(Duration::from_secs(48), rehearsal())
        .await
        .expect("owned readiness rehearsal must stay bounded");
}
async fn rehearsal() {
    let pool = common::setup_pool().await;
    common::clean_db(&pool).await;
    cl8y_dex_indexer::database_security::migrate_and_grant(&pool, "dex_indexer_runtime")
        .await
        .unwrap();
    let token = checked_address(91);
    let token_id = asset(&pool, &token, 18).await;
    let custc = asset(&pool, CUSTC, 6).await;
    let pair_address = checked_address(92);
    pair(
        &pool,
        &pair_address,
        custc,
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
    let upstream = common::lcd_mock::start_ordinary_code_mock().await;
    mount_supported_config_mock(&upstream, 0).await;
    Mock::given(method("GET"))
        .and(wiremock::matchers::path("/terra/tax/v1beta1/params"))
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
    let mut state = app_state(pool.clone(), cold_fixture_lcd(upstream.uri()));
    state.wrap_mapper_address = Some("terra1mapperfixture1452".into());
    let health = state.indexer_health.clone();
    let mut old_leader = sqlx::PgConnection::connect_with(pool.connect_options().as_ref())
        .await
        .unwrap();
    assert!(checkpoints::try_acquire_poller_lock(&mut old_leader)
        .await
        .unwrap());
    let mut standby = sqlx::PgConnection::connect_with(pool.connect_options().as_ref())
        .await
        .unwrap();
    assert!(!checkpoints::try_acquire_poller_lock(&mut standby)
        .await
        .unwrap());
    let profile = ApiReadinessProfile::from_json(
        &json!([{"token_in":"uusd","token_out":token,"amount_in":"1000000","max_spread":"0.05"}])
            .to_string(),
    );
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let app = build_router_with_api_readiness(state.clone(), &config, profile);
    let server = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .unwrap();
    let before = upstream.received_requests().await.unwrap().len();
    let (status, cold) = json_get(&client, &base, "/ready-api").await;
    assert_eq!(status, reqwest::StatusCode::SERVICE_UNAVAILABLE);
    assert!(!cold["ready"].as_bool().unwrap());
    assert_eq!(
        upstream.received_requests().await.unwrap().len(),
        before,
        "cold readiness cannot start RPC"
    );
    assert_eq!(
        json_get(&client, &base, "/ready").await.0,
        reqwest::StatusCode::SERVICE_UNAVAILABLE
    );
    let source_cancel = CancellationToken::new();
    let source_stop = source_cancel.clone();
    let source_pool = pool.clone();
    let source_hub = state.hub_usd.clone();
    let source_token = token.clone();
    // This controlled old-publisher fixture explicitly publishes NEW synthetic
    // observations on its own schedule. Readiness never wakes or renews it.
    let source = tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(2));
        loop {
            tokio::select! {_ = source_stop.cancelled()=>break,_=tick.tick()=>{
                sqlx::query("UPDATE pair_reserves SET snapshot_at=NOW()").execute(&source_pool).await.unwrap();
                common::reference_marks::seed_reference_marks(&source_pool,&source_hub,&[(&source_token,1.into())]).await;
            }}
        }
    });
    let producer_cancel = CancellationToken::new();
    let mut producers = cl8y_dex_indexer::api::local_quote::start_ingestion_with_cancel(
        state.clone(),
        producer_cancel.clone(),
    );
    let warm_deadline = tokio::time::Instant::now() + Duration::from_secs(32);
    loop {
        let (status, body) = json_get(&client, &base, "/ready-api").await;
        if status.is_success() {
            assert_eq!(body, json!({"ready":true,"reason":null,"scenario_count":1}));
            break;
        }
        assert!(
            tokio::time::Instant::now() < warm_deadline,
            "scheduled native cache did not warm: {body}"
        );
        tokio::time::sleep(Duration::from_millis(200)).await;
    }
    assert_eq!(
        json_get(&client, &base, "/ready").await.0,
        reqwest::StatusCode::SERVICE_UNAVAILABLE,
        "API usefulness does not require leadership"
    );
    assert!(!checkpoints::try_acquire_poller_lock(&mut standby)
        .await
        .unwrap());
    producer_cancel.cancel();
    producers.shutdown().await;
    source_cancel.cancel();
    source.await.unwrap();
    assert_eq!(
        json_get(&client, &base, "/health/serving").await.0,
        reqwest::StatusCode::OK
    );
    let baseline = upstream.received_requests().await.unwrap().len();
    let quote=format!("/api/v1/route/solve?token_in=uusd&token_out={token}&amount_in=1000000&max_spread=0.05&protection_policy=quote_relative_v1&hybrid_optimize=true&max_maker_fills=8");
    for _ in 0..10 {
        assert_eq!(
            json_get(&client, &base, "/ready-api").await.0,
            reqwest::StatusCode::OK
        );
        assert_eq!(
            json_get(&client, &base, &quote).await.0,
            reqwest::StatusCode::OK
        );
    }
    assert_eq!(
        upstream.received_requests().await.unwrap().len(),
        baseline,
        "readiness and quote requests must add zero LCD traffic"
    );
    // Real dedicated sessions hand over the SAME production advisory key. The
    // lifecycle observation models the normal first caught-up poll after handoff;
    // full process/startup/online SQL compatibility is a separate owned test.
    old_leader.close().await.unwrap();
    assert!(checkpoints::try_acquire_poller_lock(&mut standby)
        .await
        .unwrap());
    health.progress(1452, 1452);
    assert_eq!(
        json_get(&client, &base, "/ready").await.0,
        reqwest::StatusCode::OK
    );
    for _ in 0..5 {
        assert_eq!(
            json_get(&client, &base, "/ready-api").await.0,
            reqwest::StatusCode::OK
        );
        assert_eq!(
            json_get(&client, &base, &quote).await.0,
            reqwest::StatusCode::OK
        );
    }
    assert_eq!(upstream.received_requests().await.unwrap().len(), baseline);
    // Scheduled source loss revokes dynamic readiness and real quote execution;
    // the already admitted process remains routed, without authorizing a quote.
    sqlx::query("DELETE FROM pair_reserves")
        .execute(&pool)
        .await
        .unwrap();
    cl8y_dex_indexer::api::local_quote::refresh_reserves(&state)
        .await
        .unwrap();
    assert_eq!(
        json_get(&client, &base, "/ready-api").await.0,
        reqwest::StatusCode::SERVICE_UNAVAILABLE
    );
    assert_eq!(
        json_get(&client, &base, &quote).await.0,
        reqwest::StatusCode::SERVICE_UNAVAILABLE
    );
    let (status, latched) = json_get(&client, &base, "/health/serving").await;
    assert_eq!(status, reqwest::StatusCode::OK);
    assert_eq!(latched["admitted"], true);
    assert_eq!(upstream.received_requests().await.unwrap().len(), baseline);
    // A fresh process/gate never inherits the previous memory latch.
    let fresh=build_router_with_api_readiness(state.clone(),&config,ApiReadinessProfile::from_json(&json!([{"token_in":"uusd","token_out":token,"amount_in":"1000000","max_spread":"0.05"}]).to_string()));
    use tower::ServiceExt;
    let reset = fresh
        .oneshot(
            axum::http::Request::builder()
                .uri("/health/serving")
                .extension(axum::extract::ConnectInfo(std::net::SocketAddr::from((
                    [127, 0, 0, 1],
                    1,
                ))))
                .body(axum::body::Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(reset.status(), axum::http::StatusCode::SERVICE_UNAVAILABLE);
    sqlx::query("UPDATE indexer_schema_version SET version=1 WHERE singleton=true")
        .execute(&pool)
        .await
        .unwrap();
    let (status, body) = json_get(&client, &base, "/ready-api").await;
    assert_eq!(status, reqwest::StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(body["reason"], "database_unavailable");
    assert_eq!(upstream.received_requests().await.unwrap().len(), baseline);
    sqlx::query("UPDATE indexer_schema_version SET version=20261018110000 WHERE singleton=true")
        .execute(&pool)
        .await
        .unwrap();
    server.abort();
    standby.close().await.unwrap();
}
