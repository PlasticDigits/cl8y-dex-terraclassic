//! Forgejo #1276: generic `GET /health` liveness + optional `git_sha`.

mod common;

use axum_test::TestServer;
use serde_json::Value;
use serial_test::serial;

const HEX40: &str = "0123456789abcdef0123456789abcdef01234567";
const HEX7: &str = "abcdef0";

fn legacy_health_fields(mut body: Value) -> Value {
    let identity = body
        .as_object_mut()
        .unwrap()
        .remove("chain_identity")
        .expect("chain health metadata");
    assert_eq!(identity["expected_chain_id"], "columbus-5");
    assert!(identity["cache_namespace"]
        .as_str()
        .unwrap()
        .starts_with("columbus-5:"));
    body
}

struct CommitEnvGuard;

impl Drop for CommitEnvGuard {
    fn drop(&mut self) {
        clear_commit_env();
    }
}

fn clear_commit_env() {
    // SAFETY: tests that mutate these keys are `#[serial]`.
    unsafe {
        std::env::remove_var("GIT_SHA");
        std::env::remove_var("SOURCE_COMMIT");
    }
}

fn set_commit_env(git_sha: Option<&str>, source_commit: Option<&str>) -> CommitEnvGuard {
    clear_commit_env();
    unsafe {
        if let Some(value) = git_sha {
            std::env::set_var("GIT_SHA", value);
        }
        if let Some(value) = source_commit {
            std::env::set_var("SOURCE_COMMIT", value);
        }
    }
    CommitEnvGuard
}

#[serial]
#[tokio::test]
async fn health_returns_ok() {
    let _guard = set_commit_env(None, None);
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/health").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(body["status"], "ok");
}

#[serial]
#[tokio::test]
async fn health_ok_only_when_commit_env_unset() {
    let pool = common::setup_pool().await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let _guard = set_commit_env(None, None);

    let resp = server.get("/health").await;
    resp.assert_status_ok();
    assert_eq!(
        legacy_health_fields(resp.json::<Value>()),
        serde_json::json!({ "status": "ok" })
    );
}

#[serial]
#[tokio::test]
async fn health_emits_git_sha_for_valid_hex() {
    let pool = common::setup_pool().await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let _g40 = set_commit_env(Some(HEX40), None);
    let resp = server.get("/health").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["git_sha"], HEX40);

    drop(_g40);
    let _g7 = set_commit_env(Some(HEX7), None);
    let body: Value = server.get("/health").await.json();
    assert_eq!(body["git_sha"], HEX7);

    drop(_g7);
    let _gup = set_commit_env(Some("ABCDEF0123456789ABCDEF0123456789ABCDEF01"), None);
    let body: Value = server.get("/health").await.json();
    assert_eq!(body["git_sha"], "abcdef0123456789abcdef0123456789abcdef01");
}

#[serial]
#[tokio::test]
async fn health_empty_git_sha_falls_through_to_source_commit() {
    let pool = common::setup_pool().await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let _g = set_commit_env(Some(""), Some(HEX40));
    let body: Value = server.get("/health").await.json();
    assert_eq!(body["git_sha"], HEX40);

    drop(_g);
    let _gws = set_commit_env(Some("   "), Some(HEX7));
    let body: Value = server.get("/health").await.json();
    assert_eq!(body["git_sha"], HEX7);

    drop(_gws);
    let _gunset = set_commit_env(None, Some(HEX40));
    let body: Value = server.get("/health").await.json();
    assert_eq!(body["git_sha"], HEX40);

    drop(_gunset);
    let _gws_src = set_commit_env(None, Some("   "));
    let body: Value = server.get("/health").await.json();
    assert_eq!(
        legacy_health_fields(body),
        serde_json::json!({ "status": "ok" }),
        "whitespace-only selected SOURCE_COMMIT must omit"
    );
}

#[serial]
#[tokio::test]
async fn health_omits_git_sha_on_non_empty_reject() {
    let pool = common::setup_pool().await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let len41 = format!("{HEX40}a");
    for git_sha in [
        "HEAD",
        "head",
        "main",
        "refs/heads/main",
        "N|not-a-commit",
        "Bearer abcdef01234567",
        "sk-abcdef01234567",
        "glpat-abcdef01234567",
        "abcdef",
        len41.as_str(),
        "abc def0",
        "GIT_SHA=abcdef0",
    ] {
        let _g = set_commit_env(Some(git_sha), Some(HEX40));
        let resp = server.get("/health").await;
        resp.assert_status_ok();
        let body: Value = resp.json();
        assert_eq!(
            legacy_health_fields(body),
            serde_json::json!({ "status": "ok" }),
            "git_sha={git_sha}"
        );
    }
}

#[serial]
#[tokio::test]
async fn health_trailing_newline_on_40_char_hex_is_present() {
    let pool = common::setup_pool().await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    let _g = set_commit_env(Some(&format!("{HEX40}\n")), None);
    let body: Value = server.get("/health").await.json();
    assert_eq!(body["git_sha"], HEX40);
}

#[serial]
#[tokio::test]
async fn health_has_no_inventory_or_auth_requirement() {
    let _guard = set_commit_env(None, None);
    let pool = common::setup_pool().await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/health").await;
    resp.assert_status_ok();
    let text = resp.text();
    let body: Value = serde_json::from_str(&text).expect("json");
    assert_eq!(body["status"], "ok");
    for forbidden in [
        "uuid",
        "coolify",
        "token",
        "bearer",
        "glpat",
        "inventory",
        "127.0.0.1",
        "indexer.dex.cl8y.com",
    ] {
        assert!(
            !text.to_lowercase().contains(forbidden),
            "health body must not leak {forbidden}: {text}"
        );
    }
}

/// #1392: cheap liveness returns unverified metadata without depending on LCD or Postgres.
#[serial]
#[tokio::test]
async fn health_reports_explicit_local_identity_without_upstream_io() {
    let mock = wiremock::MockServer::start().await;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    let mut config = common::test_config();
    config.lcd_urls = vec![mock.uri()];
    config.expected_chain_id = "localterra".into();
    let app = common::build_test_app_with_price_and_config(pool, None, config).await;
    let server = TestServer::new(app);
    let body: Value = server.get("/health").await.json();
    assert_eq!(body["status"], "ok");
    assert_eq!(body["chain_identity"]["expected_chain_id"], "localterra");
    assert_eq!(body["chain_identity"]["endpoints"][0]["verified"], false);
    assert!(body["chain_identity"]["endpoints"][0]["checked_at"].is_null());
    assert!(mock.received_requests().await.unwrap().is_empty());
}

/// #1392/#1452: scheduled attestation quarantines the provider; local requests never probe it.
#[serial]
#[tokio::test]
async fn wrong_chain_quotes_fail_closed_before_db_and_health_exposes_quarantine() {
    let mock = wiremock::MockServer::start().await;
    wiremock::Mock::given(wiremock::matchers::path(
        "/cosmos/base/tendermint/v1beta1/node_info",
    ))
    .respond_with(
        wiremock::ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "default_node_info":{"network":"localterra"}
        })),
    )
    .mount(&mock)
    .await;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    let mut config = common::test_config();
    config.lcd_urls = vec![mock.uri()];
    // Model the independently scheduled identity observation before request handling.
    let lcd = cl8y_dex_indexer::lcd::LcdClient::new(config.lcd_urls.clone(), 100, 1, "columbus-5");
    assert!(lcd.attest_endpoints().await.is_err());
    let observed = lcd.chain_identity_health().await;
    assert_eq!(
        observed.endpoints[0].observed_chain_id.as_deref(),
        Some("localterra")
    );
    assert!(!observed.endpoints[0].verified);
    assert_eq!(mock.received_requests().await.unwrap().len(), 1);
    let state = common::local_quotes::state(pool, &config, lcd);
    let app = cl8y_dex_indexer::api::build_router(state, &config);
    let server = TestServer::new(app);
    server
        .get("/api/v1/route/solve")
        .await
        .assert_status_bad_request();
    server
        .get("/api/v1/route/solve?token_in=terra1syntheticinput&token_out=terra1syntheticoutput&amount_in=1000000")
        .await
        .assert_status_service_unavailable();
    let body: Value = server.get("/health").await.json();
    let endpoint = &body["chain_identity"]["endpoints"][0];
    assert_eq!(endpoint["observed_chain_id"], "localterra");
    assert_eq!(endpoint["verified"], false);
    assert!(endpoint["checked_at"].is_string());
    assert_eq!(
        mock.received_requests().await.unwrap().len(),
        1,
        "health must not reprobe quarantined providers"
    );
}

/// #1464: readiness remains a local lifecycle observation while API liveness survives halt.
#[tokio::test]
async fn recovery1464_ready_is_local_and_truthful_through_outage_recovery_and_halt() {
    let mock = wiremock::MockServer::start().await;
    let pool = sqlx::postgres::PgPoolOptions::new()
        .connect_lazy("postgres://localhost/unused")
        .unwrap();
    let mut config = common::test_config();
    config.lcd_urls = vec![mock.uri()];
    let lcd = cl8y_dex_indexer::lcd::LcdClient::new(config.lcd_urls.clone(), 100, 1, "columbus-5");
    let state = common::local_quotes::state(pool, &config, lcd);
    let health = state.indexer_health.clone();
    let app = cl8y_dex_indexer::api::build_router(state, &config);
    let server = TestServer::new(app);
    server
        .get("/ready")
        .await
        .assert_status_service_unavailable();
    health.progress(40, 41);
    server
        .get("/ready")
        .await
        .assert_status_service_unavailable();
    health.unavailable(Some(41));
    let outage = server.get("/ready").await;
    outage.assert_status_service_unavailable();
    assert_eq!(outage.json::<Value>()["status"], "provider_unavailable");
    assert_eq!(outage.json::<Value>()["checkpoint_height"], 40);
    health.progress(41, 41);
    let ready = server.get("/ready").await;
    ready.assert_status_ok();
    assert_eq!(ready.json::<Value>()["ready"], true);
    health.halt("reorg");
    health.progress(42, 42);
    let halted = server.get("/ready").await;
    halted.assert_status_service_unavailable();
    assert_eq!(halted.json::<Value>()["status"], "halted");
    assert_eq!(halted.json::<Value>()["reason"], "reorg");
    server.get("/health").await.assert_status_ok();
    assert!(
        mock.received_requests().await.unwrap().is_empty(),
        "readiness must not probe providers or the database"
    );
}

#[tokio::test]
async fn recovery1464_halted_and_cancelled_ingestion_workers_join_without_upstream_work() {
    for halted in [false, true] {
        let mock = wiremock::MockServer::start().await;
        let pool = sqlx::postgres::PgPoolOptions::new()
            .connect_lazy("postgres://localhost/unused")
            .unwrap();
        let mut config = common::test_config();
        config.lcd_urls = vec![mock.uri()];
        let lcd =
            cl8y_dex_indexer::lcd::LcdClient::new(config.lcd_urls.clone(), 100, 1, "columbus-5");
        let state = common::local_quotes::state(pool, &config, lcd);
        let cancel = tokio_util::sync::CancellationToken::new();
        if halted {
            state.indexer_health.halt("reorg");
        } else {
            cancel.cancel();
        }
        let mut workers =
            cl8y_dex_indexer::api::local_quote::start_ingestion_with_cancel(state, cancel);
        let mut joined = 0;
        while let Some(result) =
            tokio::time::timeout(std::time::Duration::from_secs(1), workers.join_next())
                .await
                .unwrap()
        {
            result.unwrap();
            joined += 1;
        }
        assert_eq!(joined, 3);
        assert!(mock.received_requests().await.unwrap().is_empty());
    }
}
