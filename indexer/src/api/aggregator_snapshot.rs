//! Shared CG/CMC aggregator snapshot loading — set-based 24h stats + volume-ranked pagination (GitLab #288).

use axum::http::StatusCode;
use serde::Deserialize;
use sqlx::PgPool;
use utoipa::{IntoParams, ToSchema};

use crate::db::queries::{assets, pairs as db_pairs, swap_events};
use crate::db::queries::pairs::PairRow;
use crate::db::queries::swap_events::PairStats;

use super::consolidated_stats;

/// Default page size for CG/CMC ticker/summary endpoints — top pairs by 24h quote volume.
pub const AGGREGATOR_LIST_LIMIT_DEFAULT: i64 = 100;
pub const AGGREGATOR_LIST_LIMIT_MAX: i64 = 1000;
pub const AGGREGATOR_LIST_OFFSET_MAX: i64 = 10_000;

#[derive(Deserialize, IntoParams, ToSchema)]
pub struct AggregatorListQuery {
    /// Max pairs returned (default 100, max 1000). Ordered by 24h quote volume descending.
    pub limit: Option<i64>,
    /// Offset for pagination (max 10_000).
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Copy)]
pub struct AggregatorListParams {
    pub limit: i64,
    pub offset: i64,
}

pub fn parse_aggregator_list_query(
    q: AggregatorListQuery,
) -> Result<AggregatorListParams, (StatusCode, String)> {
    let limit = q
        .limit
        .unwrap_or(AGGREGATOR_LIST_LIMIT_DEFAULT)
        .clamp(1, AGGREGATOR_LIST_LIMIT_MAX);
    let offset = q.offset.unwrap_or(0).max(0);
    if offset > AGGREGATOR_LIST_OFFSET_MAX {
        return Err((
            StatusCode::BAD_REQUEST,
            format!(
                "offset exceeds maximum of {} (deep pagination disabled)",
                AGGREGATOR_LIST_OFFSET_MAX
            ),
        ));
    }
    Ok(AggregatorListParams { limit, offset })
}

pub fn aggregator_cache_key(endpoint: &str, params: AggregatorListParams) -> String {
    format!("{}:{}:{}", endpoint, params.limit, params.offset)
}

#[derive(Clone)]
pub struct AggregatorPairRow {
    pub pair: PairRow,
    pub asset_0: assets::AssetRow,
    pub asset_1: assets::AssetRow,
    pub stats: PairStats,
    pub extensions: Option<consolidated_stats::Cl8yConsolidatedExtensions>,
}

/// Load pairs ranked by 24h quote volume with set-based stats (O(1) queries, not O(pairs)).
pub async fn load_aggregator_pairs(
    pool: &PgPool,
    params: AggregatorListParams,
    include_extensions: bool,
) -> Result<Vec<AggregatorPairRow>, sqlx::Error> {
    let all_pairs = db_pairs::get_all_pairs(pool).await?;
    let all_assets = assets::get_all_assets(pool).await?;
    let asset_map: std::collections::HashMap<i32, assets::AssetRow> =
        all_assets.into_iter().map(|a| (a.id, a)).collect();

    let stats_map = swap_events::get_24h_stats_all_pairs(pool).await?;
    let hybrid_map = if include_extensions {
        Some(swap_events::get_24h_hybrid_breakdown_all_pairs(pool).await?)
    } else {
        None
    };

    let mut ranked: Vec<AggregatorPairRow> = Vec::new();
    for p in all_pairs {
        let (a0, a1) = match (asset_map.get(&p.asset_0_id), asset_map.get(&p.asset_1_id)) {
            (Some(a0), Some(a1)) => (a0.clone(), a1.clone()),
            _ => continue,
        };

        let stats = stats_map.get(&p.id).cloned().unwrap_or_default();
        let extensions = hybrid_map.as_ref().map(|m| {
            let b = m.get(&p.id).cloned().unwrap_or_default();
            consolidated_stats::extensions_from_breakdown(&b)
        });

        ranked.push(AggregatorPairRow {
            pair: p,
            asset_0: a0,
            asset_1: a1,
            stats,
            extensions,
        });
    }

    ranked.sort_by(|a, b| {
        b.stats
            .volume_quote
            .cmp(&a.stats.volume_quote)
            .then_with(|| a.pair.id.cmp(&b.pair.id))
    });

    let start = params.offset.min(ranked.len() as i64) as usize;
    let end = (params.offset + params.limit).min(ranked.len() as i64) as usize;
    if start >= ranked.len() {
        return Ok(Vec::new());
    }
    Ok(ranked[start..end].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_limit_is_100() {
        let p = parse_aggregator_list_query(AggregatorListQuery {
            limit: None,
            offset: None,
        })
        .unwrap();
        assert_eq!(p.limit, 100);
        assert_eq!(p.offset, 0);
    }

    #[test]
    fn rejects_excessive_offset() {
        let err = parse_aggregator_list_query(AggregatorListQuery {
            limit: None,
            offset: Some(99_999),
        })
        .unwrap_err();
        assert_eq!(err.0, StatusCode::BAD_REQUEST);
    }
}
