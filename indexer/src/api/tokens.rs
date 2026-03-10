use axum::extract::{Path, State};
use axum::http::StatusCode;
use axum::Json;
use serde::Serialize;
use utoipa::ToSchema;

use super::{build_asset_map, internal_err, AppState};
use crate::db::queries::{assets, pairs as db_pairs, volume};

#[derive(Serialize, ToSchema)]
pub struct TokenResponse {
    pub id: i32,
    pub contract_address: Option<String>,
    pub denom: Option<String>,
    pub is_cw20: bool,
    pub name: String,
    pub symbol: String,
    pub decimals: i16,
    pub logo_url: Option<String>,
    pub coingecko_id: Option<String>,
    pub cmc_id: Option<i32>,
}

impl From<&assets::AssetRow> for TokenResponse {
    fn from(a: &assets::AssetRow) -> Self {
        Self {
            id: a.id,
            contract_address: a.contract_address.clone(),
            denom: a.denom.clone(),
            is_cw20: a.is_cw20,
            name: a.name.clone(),
            symbol: a.symbol.clone(),
            decimals: a.decimals,
            logo_url: a.logo_url.clone(),
            coingecko_id: a.coingecko_id.clone(),
            cmc_id: a.cmc_id,
        }
    }
}

#[utoipa::path(
    get,
    path = "/api/v1/tokens",
    responses(
        (status = 200, description = "List of all tokens", body = Vec<TokenResponse>),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Tokens"
)]
pub async fn list_tokens(
    State(state): State<AppState>,
) -> Result<Json<Vec<TokenResponse>>, (StatusCode, String)> {
    let all = assets::get_all_assets(&state.pool)
        .await
        .map_err(internal_err)?;

    Ok(Json(all.iter().map(TokenResponse::from).collect()))
}

#[derive(Serialize, ToSchema)]
pub struct TokenDetailResponse {
    pub token: TokenResponse,
    pub volume_stats: Vec<VolumeStatResponse>,
}

#[derive(Serialize, ToSchema)]
pub struct VolumeStatResponse {
    pub window: String,
    pub volume: String,
    pub trade_count: i64,
}

#[utoipa::path(
    get,
    path = "/api/v1/tokens/{addr}",
    params(
        ("addr" = String, Path, description = "Token contract address or native denom"),
    ),
    responses(
        (status = 200, description = "Token details with volume stats", body = TokenDetailResponse),
        (status = 404, description = "Token not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Tokens"
)]
pub async fn get_token(
    State(state): State<AppState>,
    Path(addr): Path<String>,
) -> Result<Json<TokenDetailResponse>, (StatusCode, String)> {
    let asset = find_asset(&state, &addr).await?;

    let vol_rows = volume::get_token_volume(&state.pool, asset.id)
        .await
        .map_err(internal_err)?;

    let volume_stats = vol_rows
        .iter()
        .map(|v| VolumeStatResponse {
            window: v.window.clone(),
            volume: v.volume.to_string(),
            trade_count: v.trade_count,
        })
        .collect();

    Ok(Json(TokenDetailResponse {
        token: TokenResponse::from(&asset),
        volume_stats,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/tokens/{addr}/pairs",
    params(
        ("addr" = String, Path, description = "Token contract address or native denom"),
    ),
    responses(
        (status = 200, description = "Pairs containing this token", body = Vec<super::pairs::PairResponse>),
        (status = 404, description = "Token not found"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Tokens"
)]
pub async fn get_token_pairs(
    State(state): State<AppState>,
    Path(addr): Path<String>,
) -> Result<Json<Vec<super::pairs::PairResponse>>, (StatusCode, String)> {
    let asset = find_asset(&state, &addr).await?;

    let pair_rows = db_pairs::get_pairs_for_asset(&state.pool, asset.id)
        .await
        .map_err(internal_err)?;

    let asset_map = build_asset_map(&state.pool)
        .await
        .map_err(internal_err)?;

    let mut result = Vec::new();
    for p in &pair_rows {
        if let (Some(a0), Some(a1)) =
            (asset_map.get(&p.asset_0_id), asset_map.get(&p.asset_1_id))
        {
            result.push(super::pairs::PairResponse {
                pair_address: p.contract_address.clone(),
                asset_0: super::pairs::AssetBrief::from(a0),
                asset_1: super::pairs::AssetBrief::from(a1),
                lp_token: p.lp_token.clone(),
                fee_bps: p.fee_bps,
                is_active: true,
            });
        }
    }

    Ok(Json(result))
}

async fn find_asset(
    state: &AppState,
    addr: &str,
) -> Result<assets::AssetRow, (StatusCode, String)> {
    if let Some(a) = assets::get_asset_by_contract(&state.pool, addr)
        .await
        .map_err(internal_err)?
    {
        return Ok(a);
    }
    if let Some(a) = assets::get_asset_by_denom(&state.pool, addr)
        .await
        .map_err(internal_err)?
    {
        return Ok(a);
    }
    Err((StatusCode::NOT_FOUND, "Token not found".to_string()))
}
