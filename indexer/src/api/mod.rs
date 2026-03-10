mod cg;
mod cmc;
mod orderbook_sim;
mod overview;
mod pairs;
mod tokens;
mod traders;

use std::collections::HashMap;

use axum::http::StatusCode;
use axum::routing::get;
use axum::Router;
use sqlx::PgPool;
use tower_http::cors::CorsLayer;

use crate::config::Config;
use crate::db::queries::{assets, pairs as db_pairs};
use crate::lcd::LcdClient;

#[derive(Clone)]
pub struct AppState {
    pub pool: PgPool,
    pub lcd: LcdClient,
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
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let asset_map = build_asset_map(&state.pool)
        .await
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

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

pub async fn serve(
    pool: PgPool,
    lcd: LcdClient,
    config: Config,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state = AppState { pool, lcd };
    let cors = CorsLayer::permissive();

    let app = Router::new()
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
        .layer(cors)
        .with_state(state);

    let addr = format!("0.0.0.0:{}", config.api_port);
    tracing::info!("API server listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
