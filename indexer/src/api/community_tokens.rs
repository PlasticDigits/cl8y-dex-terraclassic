//! Community tax token catalog API (GitLab #594).
//! List handlers read Postgres only — no per-row LCD (no amplification).

use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::{IntoParams, ToSchema};

use super::{internal_err, AppState};
use crate::db::queries::community_tokens as db;

const LIST_LIMIT_DEFAULT: i64 = 50;
const LIST_LIMIT_MAX: i64 = 100;
const LIST_OFFSET_MAX: i64 = 10_000;

#[derive(Debug, Deserialize, IntoParams, ToSchema)]
pub struct ListCommunityTokensQuery {
    pub manager: Option<String>,
    pub include_unattested: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CommunityTokenFeaturesJson {
    pub mint_control: bool,
    pub transfer_tax: bool,
    pub split_router: bool,
    pub auto_v2_lp: bool,
    pub exemption_directory: bool,
    pub variable_rates: bool,
    pub launch_guards: bool,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CommunityTokenItem {
    pub contract_address: String,
    pub code_id: Option<i64>,
    pub wasm_admin: Option<String>,
    pub manager: Option<String>,
    pub treasury: Option<String>,
    pub launcher_address: Option<String>,
    pub buy_bps: Option<i16>,
    pub sell_bps: Option<i16>,
    pub transfer_bps: Option<i16>,
    pub features: CommunityTokenFeaturesJson,
    pub name: Option<String>,
    pub symbol: Option<String>,
    pub decimals: Option<i16>,
    pub attested_cmm: bool,
    pub launcher_tx: Option<String>,
    pub instantiate_tx: Option<String>,
    pub created_at_block: Option<i64>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CommunityTokenListResponse {
    pub configured: bool,
    pub code_id: Option<u64>,
    pub items: Vec<CommunityTokenItem>,
    pub total: i64,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CommunityTokenDetailResponse {
    pub configured: bool,
    pub token: CommunityTokenItem,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct CommunityTokenEventItem {
    pub id: i64,
    pub txhash: String,
    pub block_height: i64,
    pub action: String,
    pub kind: String,
    pub sku: Option<String>,
    pub invoice: Option<String>,
}

fn features_from_value(v: &serde_json::Value) -> CommunityTokenFeaturesJson {
    CommunityTokenFeaturesJson {
        mint_control: v.get("mint_control").and_then(|x| x.as_bool()).unwrap_or(false),
        transfer_tax: v.get("transfer_tax").and_then(|x| x.as_bool()).unwrap_or(false),
        split_router: v.get("split_router").and_then(|x| x.as_bool()).unwrap_or(false),
        auto_v2_lp: v.get("auto_v2_lp").and_then(|x| x.as_bool()).unwrap_or(false),
        exemption_directory: v
            .get("exemption_directory")
            .and_then(|x| x.as_bool())
            .unwrap_or(false),
        variable_rates: v.get("variable_rates").and_then(|x| x.as_bool()).unwrap_or(false),
        launch_guards: v.get("launch_guards").and_then(|x| x.as_bool()).unwrap_or(false),
    }
}

fn item_from_row(row: &db::CommunityTokenRow) -> CommunityTokenItem {
    CommunityTokenItem {
        contract_address: row.contract_address.clone(),
        code_id: row.code_id,
        wasm_admin: row.wasm_admin.clone(),
        manager: row.manager.clone(),
        treasury: row.treasury.clone(),
        launcher_address: row.launcher_address.clone(),
        buy_bps: row.buy_bps,
        sell_bps: row.sell_bps,
        transfer_bps: row.transfer_bps,
        features: features_from_value(&row.features),
        name: row.name.clone(),
        symbol: row.symbol.clone(),
        decimals: row.decimals,
        attested_cmm: row.attested_cmm,
        launcher_tx: row.launcher_tx.clone(),
        instantiate_tx: row.instantiate_tx.clone(),
        created_at_block: row.created_at_block,
    }
}

fn include_unattested_flag(raw: Option<&str>) -> bool {
    matches!(raw, Some("1") | Some("true") | Some("yes"))
}

#[utoipa::path(
    get,
    path = "/api/v1/community-tokens",
    params(ListCommunityTokensQuery),
    responses(
        (status = 200, description = "Community tax token catalog", body = CommunityTokenListResponse),
        (status = 400, description = "Invalid query parameters"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "CommunityTokens"
)]
pub async fn list_community_tokens(
    State(state): State<AppState>,
    Query(q): Query<ListCommunityTokensQuery>,
) -> Result<Json<CommunityTokenListResponse>, (StatusCode, String)> {
    let limit = q.limit.unwrap_or(LIST_LIMIT_DEFAULT).clamp(1, LIST_LIMIT_MAX);
    let offset = q.offset.unwrap_or(0).max(0);
    if offset > LIST_OFFSET_MAX {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "offset exceeds maximum of {} (deep pagination disabled)",
                LIST_OFFSET_MAX
            ),
        ));
    }
    if !catalog_configured_state(&state) {
        return Ok(Json(CommunityTokenListResponse {
            configured: false,
            code_id: None,
            items: vec![],
            total: 0,
            limit,
            offset,
        }));
    }

    let include = include_unattested_flag(q.include_unattested.as_deref());
    let manager = q
        .manager
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty());

    let (rows, total) = db::list_tokens(&state.pool, manager, include, limit, offset)
        .await
        .map_err(internal_err)?;

    Ok(Json(CommunityTokenListResponse {
        configured: true,
        code_id: state.community_tax.code_id,
        items: rows.iter().map(item_from_row).collect(),
        total,
        limit,
        offset,
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/community-tokens/{addr}",
    params(("addr" = String, Path, description = "Token contract address")),
    responses(
        (status = 200, description = "Community tax token detail", body = CommunityTokenDetailResponse),
        (status = 404, description = "Not in catalog"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "CommunityTokens"
)]
pub async fn get_community_token(
    State(state): State<AppState>,
    Path(addr): Path<String>,
) -> Result<Json<CommunityTokenDetailResponse>, (StatusCode, String)> {
    if !catalog_configured_state(&state) {
        return Err((StatusCode::NOT_FOUND, "Token not found".to_string()));
    }
    let row = db::get_by_address(&state.pool, &addr)
        .await
        .map_err(internal_err)?;
    let Some(row) = row else {
        return Err((StatusCode::NOT_FOUND, "Token not found".to_string()));
    };
    Ok(Json(CommunityTokenDetailResponse {
        configured: true,
        token: item_from_row(&row),
    }))
}

#[utoipa::path(
    get,
    path = "/api/v1/community-tokens/{addr}/events",
    params(("addr" = String, Path, description = "Token contract address")),
    responses(
        (status = 200, description = "Lifecycle events", body = Vec<CommunityTokenEventItem>),
        (status = 404, description = "Not in catalog"),
        (status = 400, description = "Invalid query parameters"),
        (status = 500, description = "Internal server error"),
    ),
    tag = "CommunityTokens"
)]
pub async fn get_community_token_events(
    State(state): State<AppState>,
    Path(addr): Path<String>,
    Query(q): Query<ListCommunityTokensQuery>,
) -> Result<Json<Vec<CommunityTokenEventItem>>, (StatusCode, String)> {
    if !catalog_configured_state(&state) {
        return Err((StatusCode::NOT_FOUND, "Token not found".to_string()));
    }
    let exists = db::get_by_address(&state.pool, &addr)
        .await
        .map_err(internal_err)?;
    if exists.is_none() {
        return Err((StatusCode::NOT_FOUND, "Token not found".to_string()));
    }
    let limit = q.limit.unwrap_or(LIST_LIMIT_DEFAULT).clamp(1, LIST_LIMIT_MAX);
    let offset = q.offset.unwrap_or(0).max(0);
    if offset > LIST_OFFSET_MAX {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "offset exceeds maximum of {} (deep pagination disabled)",
                LIST_OFFSET_MAX
            ),
        ));
    }
    let rows = db::list_events(&state.pool, &addr, limit, offset)
        .await
        .map_err(internal_err)?;
    Ok(Json(
        rows.into_iter()
            .map(|r| CommunityTokenEventItem {
                id: r.id,
                txhash: r.txhash,
                block_height: r.block_height,
                action: r.action,
                kind: r.kind,
                sku: r.sku,
                invoice: r.invoice,
            })
            .collect(),
    ))
}

fn catalog_configured_state(state: &AppState) -> bool {
    state.community_tax.is_configured()
}

/// Snapshot attached to `GET /tokens/{addr}` when the asset is a catalogued tax token.
pub async fn community_tax_for_asset(
    pool: &sqlx::PgPool,
    contract_address: &str,
) -> Result<Option<CommunityTokenItem>, sqlx::Error> {
    let row = db::get_by_address(pool, contract_address).await?;
    Ok(row.map(|r| item_from_row(&r)))
}
