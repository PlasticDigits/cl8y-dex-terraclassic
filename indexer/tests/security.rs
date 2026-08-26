mod common;

use std::env;
use std::net::SocketAddr;

use axum::http::{header, HeaderValue, StatusCode};
use axum_test::TestServer;
use cl8y_dex_indexer::api::LCD_UPSTREAM_GATEWAY_MSG;
use cl8y_dex_indexer::config::{Config, RunMode, DEFAULT_RATE_LIMIT_LCD_HEAVY_RPS};
use serde_json::Value;
use serial_test::serial;
use wiremock::matchers::{method, path_regex};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[tokio::test]
async fn invalid_interval_rejected() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let bad_intervals = &[
        "3h",
        "2m",
        "12h",
        "1M",
        "abc",
        "%27%3B%20DROP%20TABLE%20pairs%3B%20--",
    ];
    for interval in bad_intervals {
        let resp = server
            .get(&format!(
                "/api/v1/pairs/{}/candles?interval={}",
                seed.pair_address, interval
            ))
            .await;
        resp.assert_status_bad_request();
    }
}

#[tokio::test]
async fn all_valid_intervals_accepted() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let valid_intervals = &["1m", "5m", "15m", "1h", "4h", "1d", "1w"];
    for interval in valid_intervals {
        let resp = server
            .get(&format!(
                "/api/v1/pairs/{}/candles?interval={}",
                seed.pair_address, interval
            ))
            .await;
        resp.assert_status_ok();
    }
}

#[tokio::test]
async fn invalid_sort_rejected() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let bad_sorts = &[
        "unknown",
        "id",
        "address",
        "%27%3B%20DROP%20TABLE%20traders%3B%20--",
    ];
    for sort in bad_sorts {
        let resp = server
            .get(&format!("/api/v1/traders/leaderboard?sort={}", sort))
            .await;
        resp.assert_status_bad_request();
    }
}

#[tokio::test]
async fn error_responses_do_not_leak_internals() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/pairs/nonexistent/stats").await;
    let body = resp.text();

    assert!(
        !body.contains("sqlx"),
        "error should not contain SQL library names"
    );
    assert!(
        !body.contains("SELECT"),
        "error should not contain SQL fragments"
    );
    assert!(
        !body.contains("postgres"),
        "error should not contain DB details"
    );
}

#[tokio::test]
async fn cors_allowed_origin_gets_headers() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get("/api/v1/overview")
        .add_header(
            header::ORIGIN,
            "https://dex.cl8y.com".parse::<HeaderValue>().unwrap(),
        )
        .await;
    resp.assert_status_ok();

    let acao = resp.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN);
    assert!(
        acao.is_some(),
        "should return ACAO header for allowed origin"
    );
    assert_eq!(acao.unwrap().to_str().unwrap(), "https://dex.cl8y.com");
}

#[tokio::test]
async fn cors_disallowed_origin_no_acao() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get("/api/v1/overview")
        .add_header(
            header::ORIGIN,
            "https://evil.com".parse::<HeaderValue>().unwrap(),
        )
        .await;

    let acao = resp.headers().get(header::ACCESS_CONTROL_ALLOW_ORIGIN);
    assert!(
        acao.is_none(),
        "should NOT return ACAO header for disallowed origin"
    );
}

#[tokio::test]
async fn trades_limit_capped_at_200() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/trades?limit=99999",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(body.len() <= 200);
}

#[tokio::test]
async fn candles_limit_capped_at_1000() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/candles?limit=99999",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();

    let body: Vec<Value> = resp.json();
    assert!(body.len() <= 1000);
}

#[tokio::test]
async fn swagger_ui_available() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/swagger-ui/").await;
    resp.assert_status_ok();
    let body = resp.text();
    assert!(body.contains("swagger"), "swagger UI page should load");
}

#[tokio::test]
async fn cg_ticker_id_attack_matrix_all_400() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let bad_tickers = [
        "",
        "_",
        "A",
        "A_",
        "_B",
        "A_B_C",
        "LUNC_USTC_EXTRA",
        "SINGLESEGMENT",
    ];
    for t in bad_tickers {
        let url = if t.is_empty() {
            "/cg/historical_trades?ticker_id=".to_string()
        } else {
            format!("/cg/historical_trades?ticker_id={}", t)
        };
        let resp = server.get(&url).await;
        resp.assert_status_bad_request();
    }
}

#[tokio::test]
async fn oracle_history_limit_capped_at_1000() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api/v1/oracle/history/ustc?limit=999999").await;
    resp.assert_status_ok();
    let body: Value = resp.json();
    let prices = body["prices"].as_array().expect("prices array");
    assert!(prices.len() <= 1000);
}

#[tokio::test]
async fn leaderboard_all_documented_sort_columns_accepted() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let sorts = [
        "total_volume",
        "total_volume_usd",
        "volume_24h",
        "volume_7d",
        "volume_30d",
        "total_trades",
        "total_realized_pnl",
        "best_trade_pnl",
        "worst_trade_pnl",
        "total_fees_paid",
    ];
    for sort in sorts {
        let resp = server
            .get(&format!("/api/v1/traders/leaderboard?sort={}", sort))
            .await;
        resp.assert_status_ok();
    }
}

#[tokio::test]
async fn leaderboard_pair_sql_injection_and_best_trade_rejected() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let payloads = [
        format!(
            "/api/v1/traders/leaderboard?pair={}&sort=best_trade_pnl",
            seed.pair_address
        ),
        "/api/v1/traders/leaderboard?pair=%27%3B%20DROP%20TABLE%20traders%3B%20--&sort=total_volume_usd"
            .to_string(),
        format!(
            "/api/v1/traders/leaderboard?pair={}&sort=%27%3B%20DROP%20TABLE%20traders%3B%20--",
            seed.pair_address
        ),
        "/api/v1/traders/leaderboard?pair=1%20OR%201%3D1&sort=total_volume_usd".to_string(),
    ];
    for path in payloads {
        let resp = server.get(&path).await;
        assert!(
            resp.status_code() == StatusCode::BAD_REQUEST
                || resp.status_code() == StatusCode::NOT_FOUND,
            "{path} → {}",
            resp.status_code()
        );
        let body = resp.text();
        assert!(!body.contains("sqlx"), "{path}");
        assert!(!body.contains("SELECT"), "{path}");
        assert!(!body.contains("postgres"), "{path}");
    }
}

#[tokio::test]
async fn trader_trades_limit_capped_at_200() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/traders/{}/trades?limit=99999",
            seed.trader_address
        ))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert!(body.len() <= 200);
}

#[tokio::test]
async fn limit_fills_limit_capped_at_200() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/limit-fills?limit=99999",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert!(body.len() <= 200);
}

#[tokio::test]
async fn limit_placements_limit_capped_at_200() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/limit-placements?limit=99999",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert!(body.len() <= 200);
}

#[tokio::test]
async fn limit_cancellations_limit_capped_at_200() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/limit-cancellations?limit=99999",
            seed.pair_address
        ))
        .await;
    resp.assert_status_ok();
    let body: Vec<Value> = resp.json();
    assert!(body.len() <= 200);
}

/// GitLab #431 (SEC-F05): negative/zero `limit` on capped list routes must not reach Postgres
/// as a negative/zero SQL `LIMIT` (historical HTTP 500 — #284). Contract: clamp to **1** via
/// `.clamp(1, max)` → **200** with at most one row (bare arrays) or nested list capped (oracle).
async fn assert_negative_or_zero_limit_clamps_to_one(server: &TestServer, path: &str) {
    let sep = if path.contains('?') { '&' } else { '?' };
    for v in ["-1", "0"] {
        let url = format!("{path}{sep}limit={v}");
        let resp = server.get(&url).await;
        resp.assert_status_ok();
        let body: Value = resp.json();
        match &body {
            Value::Array(arr) => {
                assert!(
                    arr.len() <= 1,
                    "{url}: negative/zero limit must clamp to 1 row, got {}",
                    arr.len()
                );
            }
            Value::Object(obj) if obj.get("prices").is_some() => {
                let prices = obj["prices"].as_array().expect("prices array");
                assert!(
                    prices.len() <= 1,
                    "{url}: negative/zero limit must clamp oracle prices to 1 row, got {}",
                    prices.len()
                );
            }
            other => panic!("{url}: unexpected JSON shape for limit lower-bound test: {other}"),
        }
    }
}

#[tokio::test]
async fn pair_trades_negative_and_zero_limit_clamp_to_one() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    assert_negative_or_zero_limit_clamps_to_one(
        &server,
        &format!("/api/v1/pairs/{}/trades", seed.pair_address),
    )
    .await;
}

#[tokio::test]
async fn pair_candles_negative_and_zero_limit_clamp_to_one() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    assert_negative_or_zero_limit_clamps_to_one(
        &server,
        &format!(
            "/api/v1/pairs/{}/candles?interval=1h",
            seed.pair_address
        ),
    )
    .await;
}

#[tokio::test]
async fn oracle_history_negative_and_zero_limit_clamp_to_one() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    assert_negative_or_zero_limit_clamps_to_one(&server, "/api/v1/oracle/history/ustc").await;
}

#[tokio::test]
async fn trader_trades_negative_and_zero_limit_clamp_to_one() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    assert_negative_or_zero_limit_clamps_to_one(
        &server,
        &format!("/api/v1/traders/{}/trades", seed.trader_address),
    )
    .await;
}

#[tokio::test]
async fn limit_fills_negative_and_zero_limit_clamp_to_one() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    assert_negative_or_zero_limit_clamps_to_one(
        &server,
        &format!("/api/v1/pairs/{}/limit-fills", seed.pair_address),
    )
    .await;
}

#[tokio::test]
async fn limit_placements_negative_and_zero_limit_clamp_to_one() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    assert_negative_or_zero_limit_clamps_to_one(
        &server,
        &format!("/api/v1/pairs/{}/limit-placements", seed.pair_address),
    )
    .await;
}

#[tokio::test]
async fn limit_cancellations_negative_and_zero_limit_clamp_to_one() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);
    assert_negative_or_zero_limit_clamps_to_one(
        &server,
        &format!("/api/v1/pairs/{}/limit-cancellations", seed.pair_address),
    )
    .await;
}

#[tokio::test]
async fn lcd_failure_returns_sanitized_502_body() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/.+$"))
        .respond_with(ResponseTemplate::new(500).set_body_string("internal LCD failure"))
        .mount(&mock)
        .await;

    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];

    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/pairs/{}/order-book-head?side=bid",
            seed.pair_address
        ))
        .await;
    assert_eq!(resp.status_code(), StatusCode::BAD_GATEWAY);
    let body = resp.text();
    assert_eq!(body, LCD_UPSTREAM_GATEWAY_MSG);
    assert!(
        !body.contains("LCD query failed:"),
        "must not echo legacy LcdError display prefix"
    );
    assert!(
        !body.contains("All LCD endpoints failed"),
        "must not echo LcdError variant text"
    );
    assert!(
        !body.contains(&mock.uri()),
        "must not leak LCD endpoint URL"
    );
    assert!(!body.contains("cosmwasm"), "must not leak LCD path details");
}

/// GitLab #363: production config path clamps `RATE_LIMIT_LCD_HEAVY_RPS=0` and still enforces 429.
#[tokio::test]
#[serial]
async fn prod_lcd_heavy_rate_limit_enforced_when_env_zero() {
    for key in [
        "RUN_MODE",
        "LCD_URLS",
        "DATABASE_URL",
        "FACTORY_ADDRESS",
        "CORS_ORIGINS",
        "RATE_LIMIT_RPS",
        "RATE_LIMIT_LCD_HEAVY_RPS",
    ] {
        env::remove_var(key);
    }
    env::set_var("RUN_MODE", "prod");
    env::set_var("LCD_URLS", "https://lcd.example.com");
    env::set_var("DATABASE_URL", "postgres://localhost/db");
    env::set_var("FACTORY_ADDRESS", "terra1factory");
    env::set_var("CORS_ORIGINS", "https://app.example.com");
    env::set_var("RATE_LIMIT_RPS", "0");
    env::set_var("RATE_LIMIT_LCD_HEAVY_RPS", "0");

    let prod_config = Config::from_env().expect("prod config from env");
    assert_eq!(prod_config.run_mode, RunMode::Prod);
    assert_eq!(
        prod_config.rate_limit_lcd_heavy_rps,
        DEFAULT_RATE_LIMIT_LCD_HEAVY_RPS,
        "prod must clamp LCD-heavy limit when env is 0"
    );

    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let mut config = common::test_config();
    config.run_mode = RunMode::Prod;
    config.rate_limit_rps = 0;
    config.rate_limit_lcd_heavy_rps = prod_config.rate_limit_lcd_heavy_rps;
    let app = common::build_test_app_with_price_and_config(pool, None, config).await;
    let server = TestServer::builder()
        .http_transport()
        .build(app.into_make_service_with_connect_info::<SocketAddr>());

    let url = format!(
        "/api/v1/pairs/{}/order-book-head?side=bid",
        seed.pair_address
    );
    let mut saw_429 = false;
    for _ in 0..80 {
        let resp = server.get(&url).await;
        if resp.status_code() == StatusCode::TOO_MANY_REQUESTS {
            saw_429 = true;
            assert!(
                resp.headers()
                    .get(header::RETRY_AFTER)
                    .is_some(),
                "429 should include Retry-After header"
            );
            break;
        }
    }
    assert!(
        saw_429,
        "prod-clamped LCD-heavy governor should return 429 when env had RATE_LIMIT_LCD_HEAVY_RPS=0"
    );
}

#[tokio::test]
async fn lcd_heavy_route_rate_limit_returns_429() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let mut config = common::test_config();
    config.rate_limit_rps = 0;
    config.rate_limit_lcd_heavy_rps = 5;
    let app = common::build_test_app_with_price_and_config(pool, None, config).await;
    let server = TestServer::builder()
        .http_transport()
        .build(app.into_make_service_with_connect_info::<SocketAddr>());

    let url = format!(
        "/api/v1/pairs/{}/order-book-head?side=bid",
        seed.pair_address
    );
    let mut saw_429 = false;
    for _ in 0..80 {
        let resp = server.get(&url).await;
        if resp.status_code() == StatusCode::TOO_MANY_REQUESTS {
            saw_429 = true;
            break;
        }
    }
    assert!(
        saw_429,
        "LCD-heavy governor should return 429 even when global RATE_LIMIT_RPS=0"
    );
}

/// GitLab #363: prod profile clamps `RATE_LIMIT_LCD_HEAVY_RPS=0` → 10 at config load; HTTP governor must still 429.
#[tokio::test]
async fn prod_lcd_heavy_rate_limit_enforced_when_config_clamped() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let mut config = common::test_config();
    config.run_mode = cl8y_dex_indexer::config::RunMode::Prod;
    config.rate_limit_rps = 0;
    config.rate_limit_lcd_heavy_rps = 0;
    // Integration tests skip Config::from_env; mirror prod clamp (#363) before build_router.
    if config.run_mode == cl8y_dex_indexer::config::RunMode::Prod && config.rate_limit_lcd_heavy_rps == 0
    {
        config.rate_limit_lcd_heavy_rps = 10;
    }
    let app = common::build_test_app_with_price_and_config(pool, None, config).await;
    let server = TestServer::builder()
        .http_transport()
        .build(app.into_make_service_with_connect_info::<SocketAddr>());

    let url = format!(
        "/api/v1/pairs/{}/order-book-head?side=bid",
        seed.pair_address
    );
    let mut saw_429 = false;
    for _ in 0..80 {
        let resp = server.get(&url).await;
        if resp.status_code() == StatusCode::TOO_MANY_REQUESTS {
            saw_429 = true;
            break;
        }
    }
    assert!(
        saw_429,
        "prod-clamped LCD-heavy governor should return 429 when global RATE_LIMIT_RPS=0 (#363)"
    );
}

/// GitLab #278: CG/CMC orderbook mirrors share `lcd_heavy_router` with native book routes.
#[tokio::test]
async fn cg_cmc_orderbook_lcd_heavy_rate_limit_returns_429() {
    let mock = common::lcd_mock::start_pool_query_mock().await;
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let mut config = common::test_config();
    config.rate_limit_rps = 0;
    config.rate_limit_lcd_heavy_rps = 5;
    config.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];
    let app = common::build_test_app_with_price_and_config(pool, None, config).await;
    let server = TestServer::builder()
        .http_transport()
        .build(app.into_make_service_with_connect_info::<SocketAddr>());

    for path in [
        "/cg/orderbook?ticker_id=LUNC_USTC&depth=5",
        "/cmc/orderbook/LUNC_USTC?depth=5",
    ] {
        let mut saw_429 = false;
        for _ in 0..80 {
            let resp = server.get(path).await;
            if resp.status_code() == StatusCode::TOO_MANY_REQUESTS {
                saw_429 = true;
                break;
            }
        }
        assert!(
            saw_429,
            "LCD-heavy governor should return 429 on {path} when global RATE_LIMIT_RPS=0 (#278)"
        );
    }
}

/// GitLab #355: after burst drain, replenish must match configured RPS (not 1 token / RPS seconds).
#[tokio::test]
async fn rate_limit_sustained_throughput_matches_rps() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let mut config = common::test_config();
    config.rate_limit_rps = 5;
    config.rate_limit_lcd_heavy_rps = 0;
    let app = common::build_test_app_with_price_and_config(pool, None, config).await;
    let server = TestServer::builder()
        .http_transport()
        .build(app.into_make_service_with_connect_info::<SocketAddr>());

    for _ in 0..12 {
        let _ = server.get("/health").await;
    }
    assert_eq!(
        server.get("/health").await.status_code(),
        StatusCode::TOO_MANY_REQUESTS,
        "burst should be exhausted at 5 RPS (burst 10)"
    );

    tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

    let mut ok = 0u32;
    for _ in 0..5 {
        if server.get("/health").await.status_code() == StatusCode::OK {
            ok += 1;
        }
    }
    assert!(
        ok >= 4,
        "expected ~5 successful requests after 1.1s at 5 RPS, got {ok}"
    );
}

#[tokio::test]
async fn rate_limit_returns_429_when_exceeded() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let mut config = common::test_config();
    config.rate_limit_rps = 10;
    let app = common::build_test_app_with_price_and_config(pool, None, config).await;
    // Governor's default PeerIpKeyExtractor needs ConnectInfo; mock transport omits it unless we use a real TCP server.
    let server = TestServer::builder()
        .http_transport()
        .build(app.into_make_service_with_connect_info::<SocketAddr>());

    let mut saw_429 = false;
    for _ in 0..120 {
        let resp = server.get("/health").await;
        if resp.status_code() == StatusCode::TOO_MANY_REQUESTS {
            saw_429 = true;
            break;
        }
    }
    assert!(saw_429, "expected governor to return 429 after burst");
}

#[tokio::test]
async fn trader_trades_invalid_format_returns_400() {
    let pool = common::setup_pool().await;
    let seed = common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server
        .get(&format!(
            "/api/v1/traders/{}/trades?format=xml",
            seed.trader_address
        ))
        .await;
    resp.assert_status_bad_request();
}

#[tokio::test]
async fn openapi_spec_available() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let resp = server.get("/api-docs/openapi.json").await;
    resp.assert_status_ok();

    let body: Value = resp.json();
    assert_eq!(body["info"]["title"], "CL8Y DEX Indexer API");
    assert!(body["paths"].is_object());
    assert!(body["paths"]["/api/v1/pairs"].is_object());
}

/// GitLab #379 (M-05): dev config accepts dual-zero rate limits (governors disabled).
#[test]
#[serial]
fn dev_dual_zero_rate_limits_load_without_clamp() {
    for key in [
        "RUN_MODE",
        "LCD_URLS",
        "DATABASE_URL",
        "FACTORY_ADDRESS",
        "CORS_ORIGINS",
        "RATE_LIMIT_RPS",
        "RATE_LIMIT_LCD_HEAVY_RPS",
    ] {
        env::remove_var(key);
    }
    env::set_var("DATABASE_URL", "postgres://localhost/db");
    env::set_var("FACTORY_ADDRESS", "terra1factory");
    env::set_var("CORS_ORIGINS", "http://localhost:5173");
    env::set_var("RATE_LIMIT_RPS", "0");
    env::set_var("RATE_LIMIT_LCD_HEAVY_RPS", "0");
    let c = Config::from_env().expect("dev dual-zero config");
    assert_eq!(c.run_mode, RunMode::Dev);
    assert_eq!(c.rate_limit_rps, 0);
    assert_eq!(c.rate_limit_lcd_heavy_rps, 0);
}

/// GitLab #379 (L-05): blacklist-check LCD failure → sanitized 502.
#[tokio::test]
async fn blacklist_check_lcd_failure_returns_sanitized_502() {
    let mock = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path_regex(r"^/cosmwasm/wasm/v1/contract/[^/]+/smart/.+$"))
        .respond_with(ResponseTemplate::new(503).set_body_string("upstream unavailable"))
        .mount(&mock)
        .await;

    let mut cfg = common::test_config();
    cfg.lcd_urls = vec![common::lcd_mock::lcd_base_url(&mock)];

    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app_with_price_and_config(pool, None, cfg).await;
    let server = TestServer::new(app);

    let resp = server
        .get("/api/v1/compliance/blacklist-check?wallet=terra1wallet")
        .await;
    assert_eq!(resp.status_code(), StatusCode::BAD_GATEWAY);
    let body = resp.text();
    assert_eq!(body, LCD_UPSTREAM_GATEWAY_MSG);
    assert!(!body.contains(&mock.uri()));
    assert!(!body.contains("cosmwasm"));
}

/// GitLab #379 (L-08): POST route solve rejects oversized JSON bodies with 413.
#[tokio::test]
async fn route_solve_post_oversized_body_returns_413() {
    let pool = common::setup_pool().await;
    common::seed_db(&pool).await;
    let app = common::build_test_app(pool).await;
    let server = TestServer::new(app);

    let pad = "x".repeat(cl8y_dex_indexer::config::ROUTE_SOLVE_POST_BODY_LIMIT_BYTES + 1);
    let body = serde_json::json!({
        "token_in": "terra1a",
        "token_out": "terra1b",
        "amount_in": "100",
        "hybrid_by_hop": null,
        "trader": null,
        "sender": null,
        "pad": pad,
    });

    let resp = server
        .post("/api/v1/route/solve")
        .json(&body)
        .await;
    assert_eq!(resp.status_code(), StatusCode::PAYLOAD_TOO_LARGE);
}
