//! Forgejo #1276: generic `GET /health` liveness + optional `git_sha`.

mod common;

use axum_test::TestServer;
use serde_json::Value;
use serial_test::serial;

const HEX40: &str = "0123456789abcdef0123456789abcdef01234567";
const HEX7: &str = "abcdef0";

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
    assert_eq!(resp.json::<Value>(), serde_json::json!({ "status": "ok" }));
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
            body,
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
