use axum::Json;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::{AppState, build_asset_map, internal_err};
use crate::db::queries::{assets, pairs as db_pairs, volume};

const TOKEN_LIST_LIMIT_DEFAULT: i64 = 200;
const TOKEN_LIST_LIMIT_MAX: i64 = 500;
const TOKEN_LIST_OFFSET_MAX: i64 = 10_000;

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct ListTokensQuery {
    /// Page size (default 200, max 500)
    pub limit: Option<i64>,
    /// Offset for pagination (max 10_000)
    pub offset: Option<i64>,
}

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
    params(ListTokensQuery),
    responses(
        (status = 200, description = "Paginated token list", body = Vec<TokenResponse>),
        (status = 400, description = "Invalid query parameters"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "Tokens"
)]
pub async fn list_tokens(
    State(state): State<AppState>,
    Query(q): Query<ListTokensQuery>,
) -> Result<Json<Vec<TokenResponse>>, (StatusCode, String)> {
    let limit = q
        .limit
        .unwrap_or(TOKEN_LIST_LIMIT_DEFAULT)
        .clamp(1, TOKEN_LIST_LIMIT_MAX);
    let offset = q.offset.unwrap_or(0).max(0);
    if offset > TOKEN_LIST_OFFSET_MAX {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "offset exceeds maximum of {} (deep pagination disabled)",
                TOKEN_LIST_OFFSET_MAX
            ),
        ));
    }

    let rows = assets::list_assets_paginated(&state.pool, limit, offset)
        .await
        .map_err(internal_err)?;

    Ok(Json(rows.iter().map(TokenResponse::from).collect()))
}

#[derive(Serialize, ToSchema)]
pub struct TokenDetailResponse {
    pub token: TokenResponse,
    pub volume_stats: Vec<VolumeStatResponse>,
    /// Present only when this CW20 is in the community-tax catalog (#594).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub community_tax: Option<super::community_tokens::CommunityTokenItem>,
}

#[derive(Serialize, ToSchema)]
pub struct VolumeStatResponse {
    pub window: String,
    pub volume: String,
    pub volume_usd: String,
    pub trade_count: i64,
    pub unique_traders: i64,
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
            volume_usd: v.volume_usd.to_string(),
            trade_count: v.trade_count,
            unique_traders: i64::from(v.unique_traders),
        })
        .collect();

    let community_tax = if let Some(ref addr) = asset.contract_address {
        super::community_tokens::community_tax_for_asset(&state.pool, addr)
            .await
            .map_err(internal_err)?
    } else {
        None
    };

    Ok(Json(TokenDetailResponse {
        token: TokenResponse::from(&asset),
        volume_stats,
        community_tax,
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

    let asset_map = build_asset_map(&state.pool).await.map_err(internal_err)?;

    let mut result = Vec::new();
    for p in &pair_rows {
        if let (Some(a0), Some(a1)) = (asset_map.get(&p.asset_0_id), asset_map.get(&p.asset_1_id)) {
            result.push(super::pairs::pair_to_response(p, a0, a1, None, None));
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
