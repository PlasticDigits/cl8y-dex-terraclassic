//! Shared token-graph snapshot for route solve (GitLab #485).
//! TTL-cached assets+pairs so concurrent/distant-pair solves reuse one Postgres load.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};
use std::time::{Duration, Instant};

use axum::http::StatusCode;
use sqlx::PgPool;

use crate::api::internal_err;
use crate::api::route_solver::build_id_to_addr_map;
use crate::db::queries::{assets, pairs as db_pairs};

pub const ROUTE_GRAPH_CACHE_TTL: Duration = Duration::from_secs(15);

/// Cached assets + pair adjacency for path enumeration.
pub struct RouteGraphSnapshot {
    pub assets: Arc<Vec<assets::AssetRow>>,
    pub pairs: Arc<Vec<db_pairs::PairRow>>,
    pub id_to_addr: HashMap<i32, String>,
    pub addr_to_id: HashMap<String, i32>,
}

impl RouteGraphSnapshot {
    fn new(all_assets: Vec<assets::AssetRow>, pair_rows: Vec<db_pairs::PairRow>) -> Self {
        let (id_to_addr, addr_to_id) = build_id_to_addr_map(&all_assets);
        Self {
            assets: Arc::new(all_assets),
            pairs: Arc::new(pair_rows),
            id_to_addr,
            addr_to_id,
        }
    }
}

fn graph_cache() -> &'static Mutex<Option<(Arc<RouteGraphSnapshot>, Instant)>> {
    static CACHE: OnceLock<Mutex<Option<(Arc<RouteGraphSnapshot>, Instant)>>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(None))
}

async fn load_graph_snapshot(pool: &PgPool) -> Result<Arc<RouteGraphSnapshot>, (StatusCode, String)> {
    let all_assets = assets::get_all_assets(pool)
        .await
        .map_err(internal_err)?;
    let pair_rows = db_pairs::get_all_pairs(pool)
        .await
        .map_err(internal_err)?;
    Ok(Arc::new(RouteGraphSnapshot::new(all_assets, pair_rows)))
}

/// Return a TTL-cached graph snapshot (double-checked locking).
pub async fn get_route_graph_snapshot(
    pool: &PgPool,
) -> Result<Arc<RouteGraphSnapshot>, (StatusCode, String)> {
    let now = Instant::now();
    if let Ok(guard) = graph_cache().lock() {
        if let Some((snap, at)) = guard.as_ref() {
            if now.duration_since(*at) <= ROUTE_GRAPH_CACHE_TTL {
                return Ok(Arc::clone(snap));
            }
        }
    }

    let snapshot = load_graph_snapshot(pool).await?;

    if let Ok(mut guard) = graph_cache().lock() {
        if let Some((snap, at)) = guard.as_ref() {
            if now.duration_since(*at) <= ROUTE_GRAPH_CACHE_TTL {
                return Ok(Arc::clone(snap));
            }
        }
        *guard = Some((Arc::clone(&snapshot), Instant::now()));
    }

    Ok(snapshot)
}

#[cfg(test)]
mod tests {
    use super::ROUTE_GRAPH_CACHE_TTL;
    use std::time::Duration;

    #[test]
    fn route_graph_cache_ttl_is_15_seconds() {
        assert_eq!(ROUTE_GRAPH_CACHE_TTL, Duration::from_secs(15));
    }
}
