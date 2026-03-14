mod cg;
mod cmc;
pub mod hooks;
mod orderbook_sim;
mod overview;
mod pairs;
mod tokens;
mod traders;

use std::collections::HashMap;
use std::net::SocketAddr;

use axum::http::{header, HeaderValue, Method, StatusCode};
use axum::routing::get;
use axum::Router;
use sqlx::PgPool;
use tower_governor::governor::GovernorConfigBuilder;
use tower_governor::GovernorLayer;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;
use utoipa::OpenApi;
use utoipa_swagger_ui::SwaggerUi;

use crate::config::Config;
use crate::db::queries::{assets, pairs as db_pairs};
use crate::lcd::LcdClient;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub lcd: LcdClient,
}

pub fn internal_err(e: impl std::fmt::Display) -> (StatusCode, String) {
    tracing::error!("Internal error: {}", e);
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Internal server error".to_string(),
    )
}

async fn build_asset_map(
    pool: &PgPool,
) -> Result<HashMap<i32, assets::AssetRow>, sqlx::Error> {
    let all = assets::get_all_assets(pool).await?;
    Ok(all.into_iter().map(|a| (a.id, a)).collect())
}

async fn find_pair_by_ticker(
    state: &AppState,
    ticker_id: &str,
) -> Result<String, (StatusCode, String)> {
    let parts: Vec<&str> = ticker_id.split('_').collect();
    if parts.len() != 2 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Invalid ticker_id format, expected BASE_TARGET".to_string(),
        ));
    }
    let (base_sym, target_sym) = (parts[0], parts[1]);

    let all_pairs = db_pairs::get_all_pairs(&state.pool)
        .await
        .map_err(internal_err)?;
    let asset_map = build_asset_map(&state.pool)
        .await
        .map_err(internal_err)?;

    for p in &all_pairs {
        if let (Some(a0), Some(a1)) =
            (asset_map.get(&p.asset_0_id), asset_map.get(&p.asset_1_id))
        {
            if a0.symbol == base_sym && a1.symbol == target_sym {
                return Ok(p.contract_address.clone());
            }
        }
    }

    Err((
        StatusCode::NOT_FOUND,
        format!("Pair not found for ticker: {}", ticker_id),
    ))
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "CL8Y DEX Indexer API",
        version = "1.0.0",
        description = "Indexer API for CL8Y DEX on Terra Classic — charting, analytics, CoinGecko and CoinMarketCap integrations.",
    ),
    paths(
        pairs::list_pairs,
        pairs::get_pair,
        pairs::get_pair_candles,
        pairs::get_pair_trades,
        pairs::get_pair_stats,
        tokens::list_tokens,
        tokens::get_token,
        tokens::get_token_pairs,
        traders::get_trader_profile,
        traders::get_trader_trades,
        traders::get_trader_positions,
        traders::leaderboard,
        overview::get_overview,
        cg::cg_pairs,
        cg::cg_tickers,
        cg::cg_orderbook,
        cg::cg_historical_trades,
        cmc::cmc_summary,
        cmc::cmc_assets,
        cmc::cmc_ticker,
        cmc::cmc_orderbook,
        cmc::cmc_trades,
    ),
    components(schemas(
        pairs::PairResponse,
        pairs::AssetBrief,
        pairs::CandleResponse,
        pairs::TradeResponse,
        pairs::PairStatsResponse,
        tokens::TokenResponse,
        tokens::TokenDetailResponse,
        tokens::VolumeStatResponse,
        traders::TraderResponse,
        traders::PositionResponse,
        overview::OverviewResponse,
        cg::CgPairResponse,
        cg::CgTickerResponse,
        cg::CgOrderbookResponse,
        cg::CgTradeEntry,
        cg::CgHistoricalTradesResponse,
        cmc::CmcSummaryEntry,
        cmc::CmcAssetEntry,
        cmc::CmcTickerEntry,
        cmc::CmcOrderbookResponse,
        cmc::CmcTradeEntry,
    )),
    tags(
        (name = "Pairs", description = "Trading pair endpoints"),
        (name = "Tokens", description = "Token/asset endpoints"),
        (name = "Traders", description = "Trader profile and leaderboard"),
        (name = "Overview", description = "Global DEX statistics"),
        (name = "CoinGecko", description = "CoinGecko-compatible endpoints"),
        (name = "CoinMarketCap", description = "CoinMarketCap-compatible endpoints"),
    )
)]
struct ApiDoc;

async fn health() -> axum::Json<serde_json::Value> {
    axum::Json(serde_json::json!({"status": "ok"}))
}

pub fn build_router(state: AppState, config: &Config) -> Router {
    let mut origins = Vec::new();
    for o in &config.cors_origins {
        match o.parse::<HeaderValue>() {
            Ok(v) => origins.push(v),
            Err(_) => {
                tracing::warn!("Skipping invalid CORS origin: {}", o);
                continue;
            }
        }
    }

    let cors = CorsLayer::new()
        .allow_origin(origins)
        .allow_methods([Method::GET])
        .allow_headers([header::CONTENT_TYPE, header::ACCEPT]);

    let governor_conf = GovernorConfigBuilder::default()
        .per_second(config.rate_limit_rps)
        .burst_size(config.rate_limit_rps as u32 * 2)
        .use_headers()
        .finish()
        .expect("failed to build governor config");

    let governor_limiter = governor_conf.limiter().clone();
    let cleanup_interval = std::time::Duration::from_secs(60);
    tokio::spawn(async move {
        loop {
            tokio::time::sleep(cleanup_interval).await;
            governor_limiter.retain_recent();
        }
    });

    Router::new()
        .route("/health", get(health))
        .route("/api/v1/pairs", get(pairs::list_pairs))
        .route("/api/v1/pairs/{addr}", get(pairs::get_pair))
        .route("/api/v1/pairs/{addr}/candles", get(pairs::get_pair_candles))
        .route("/api/v1/pairs/{addr}/trades", get(pairs::get_pair_trades))
        .route("/api/v1/pairs/{addr}/stats", get(pairs::get_pair_stats))
        .route("/api/v1/tokens", get(tokens::list_tokens))
        .route("/api/v1/tokens/{addr}", get(tokens::get_token))
        .route("/api/v1/tokens/{addr}/pairs", get(tokens::get_token_pairs))
        .route("/api/v1/traders/leaderboard", get(traders::leaderboard))
        .route("/api/v1/traders/{addr}", get(traders::get_trader_profile))
        .route(
            "/api/v1/traders/{addr}/trades",
            get(traders::get_trader_trades),
        )
        .route(
            "/api/v1/traders/{addr}/positions",
            get(traders::get_trader_positions),
        )
        .route("/api/v1/hooks", get(hooks::get_hook_events))
        .route("/api/v1/overview", get(overview::get_overview))
        .route("/cg/pairs", get(cg::cg_pairs))
        .route("/cg/tickers", get(cg::cg_tickers))
        .route("/cg/orderbook", get(cg::cg_orderbook))
        .route("/cg/historical_trades", get(cg::cg_historical_trades))
        .route("/cmc/summary", get(cmc::cmc_summary))
        .route("/cmc/assets", get(cmc::cmc_assets))
        .route("/cmc/ticker", get(cmc::cmc_ticker))
        .route("/cmc/orderbook/{market_pair}", get(cmc::cmc_orderbook))
        .route("/cmc/trades/{market_pair}", get(cmc::cmc_trades))
        .merge(
            SwaggerUi::new("/swagger-ui")
                .url("/api-docs/openapi.json", ApiDoc::openapi()),
        )
        .layer(GovernorLayer::new(governor_conf))
        .layer(cors)
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

pub async fn serve(
    pool: PgPool,
    lcd: LcdClient,
    config: Config,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state = AppState { pool, lcd };
    let app = build_router(state, &config);

    let addr = format!("{}:{}", config.api_bind, config.api_port);
    tracing::info!("API server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await?;

    Ok(())
}
