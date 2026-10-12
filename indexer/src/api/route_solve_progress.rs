//! Local route-solve progress (#1452); historical registry is test-only.
//! Progress is advisory only — not a quote. No estimated_amount_out. Opaque key = hybrid cache key.

#[cfg(test)]
use std::collections::HashMap;
#[cfg(test)]
use std::sync::Mutex;
#[cfg(test)]
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use axum::extract::{Query, State};
use axum::http::StatusCode;
use axum::Json;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::api::route_solver::SolveRouteParams;
use crate::api::AppState;

#[cfg(test)]
const MAX_ENTRIES: usize = 256;
#[cfg(test)]
const ENTRY_TTL: Duration = Duration::from_secs(30);
#[cfg(test)]
const MAX_AGE: Duration = Duration::from_secs(120);

/// Advisory solve stage (snake_case strings in JSON).
#[cfg(test)]
pub type SolveStage = &'static str;

#[cfg(test)]
pub const STAGE_IDLE: SolveStage = "idle";
#[cfg(test)]
pub const STAGE_GRAPH_LOAD: SolveStage = "graph_load";
#[cfg(test)]
pub const STAGE_ENUMERATING: SolveStage = "enumerating";
#[cfg(test)]
pub const STAGE_LOADING_MIRRORS: SolveStage = "loading_mirrors";
#[cfg(test)]
pub const STAGE_EVALUATING: SolveStage = "evaluating";
/// Reserved for future hop-level router-sim milestones (API contract / docs).
#[allow(dead_code)]
#[cfg(test)]
pub const STAGE_SIMULATING: SolveStage = "simulating";
#[cfg(test)]
pub const STAGE_ENRICHING: SolveStage = "enriching";
#[cfg(test)]
pub const STAGE_DONE: SolveStage = "done";
#[cfg(test)]
pub const STAGE_ERROR: SolveStage = "error";
#[cfg(test)]
pub const STAGE_CACHED: SolveStage = "cached";

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema, PartialEq, Eq)]
pub struct SolveProgress {
    pub stage: String,
    pub done: u32,
    pub total: u32,
    pub label: String,
    pub cache_hit: bool,
    pub updated_at_ms: u64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quote_generation: Option<String>,
    /// Search completion is separate from executable quote eligibility (#1479).
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quote_eligibility: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub quote_blocker: Option<String>,
}

#[derive(Clone)]
#[cfg(test)]
pub(crate) struct ProgressEntry {
    progress: SolveProgress,
    started_at: Instant,
    terminal_at: Option<Instant>,
}

#[cfg(test)]
static REGISTRY: Mutex<Option<HashMap<String, ProgressEntry>>> = Mutex::new(None);

#[cfg(test)]
fn registry() -> std::sync::MutexGuard<'static, Option<HashMap<String, ProgressEntry>>> {
    let mut guard = REGISTRY.lock().expect("progress registry poisoned");
    if guard.is_none() {
        *guard = Some(HashMap::new());
    }
    guard
}

#[cfg(test)]
fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
pub(crate) fn progress_cleanup_locked(map: &mut HashMap<String, ProgressEntry>, now: Instant) {
    map.retain(|_, entry| {
        if let Some(term) = entry.terminal_at {
            return now.duration_since(term) <= ENTRY_TTL;
        }
        now.duration_since(entry.started_at) <= MAX_AGE
    });
    if map.len() > MAX_ENTRIES {
        let mut keys: Vec<_> = map
            .iter()
            .map(|(k, e)| {
                let sort_key = e.terminal_at.unwrap_or(e.started_at);
                (k.clone(), sort_key)
            })
            .collect();
        keys.sort_by_key(|(_, at)| *at);
        let excess = map.len().saturating_sub(MAX_ENTRIES);
        for (k, _) in keys.into_iter().take(excess) {
            map.remove(&k);
        }
    }
}

#[cfg(test)]
fn set_progress(
    map: &mut HashMap<String, ProgressEntry>,
    key: &str,
    stage: SolveStage,
    done: u32,
    total: u32,
    label: impl Into<String>,
    cache_hit: bool,
    terminal: bool,
) {
    let now = Instant::now();
    progress_cleanup_locked(map, now);
    let label = label.into();
    let entry = map.entry(key.to_string()).or_insert_with(|| ProgressEntry {
        progress: SolveProgress {
            stage: STAGE_IDLE.into(),
            done: 0,
            total: 0,
            label: String::new(),
            cache_hit: false,
            updated_at_ms: now_ms(),
            quote_generation: None,
            quote_eligibility: None,
            quote_blocker: None,
        },
        started_at: now,
        terminal_at: None,
    });
    entry.progress = SolveProgress {
        stage: stage.to_string(),
        done,
        total,
        label,
        cache_hit,
        updated_at_ms: now_ms(),
        quote_generation: None,
        quote_eligibility: None,
        quote_blocker: None,
    };
    if terminal {
        entry.terminal_at = Some(now);
    }
}

/// Begin tracking progress for a hybrid solve (key = hybrid cache key).
#[cfg(test)]
pub fn progress_begin(key: &str) {
    let mut guard = registry();
    let map = guard.as_mut().expect("registry init");
    set_progress(
        map,
        key,
        STAGE_GRAPH_LOAD,
        0,
        0,
        "Loading token graph…",
        false,
        false,
    );
}

/// Update in-flight progress.
#[cfg(test)]
pub fn progress_update(
    key: &str,
    stage: SolveStage,
    done: u32,
    total: u32,
    label: impl Into<String>,
) {
    let mut guard = registry();
    let map = guard.as_mut().expect("registry init");
    set_progress(map, key, stage, done, total, label, false, false);
}

/// Mark solve complete (terminal).
#[cfg(test)]
pub fn progress_complete(key: &str, cache_hit: bool) {
    let mut guard = registry();
    let map = guard.as_mut().expect("registry init");
    let stage = if cache_hit { STAGE_CACHED } else { STAGE_DONE };
    let label = if cache_hit {
        "Quote ready (cached)"
    } else {
        "Quote ready"
    };
    set_progress(map, key, stage, 1, 1, label, cache_hit, true);
}

/// Mark solve failed (terminal).
#[cfg(test)]
pub fn progress_fail(key: &str, label: &str) {
    let mut guard = registry();
    let map = guard.as_mut().expect("registry init");
    set_progress(map, key, STAGE_ERROR, 0, 0, label.to_string(), false, true);
}

/// Fetch current progress for a key (runs TTL/eviction cleanup).
#[cfg(test)]
pub fn progress_get(key: &str) -> Option<SolveProgress> {
    let mut guard = registry();
    let map = guard.as_mut().expect("registry init");
    progress_cleanup_locked(map, Instant::now());
    map.get(key).map(|e| e.progress.clone())
}

#[cfg(test)]
fn idle_progress() -> SolveProgress {
    SolveProgress {
        stage: STAGE_IDLE.into(),
        done: 0,
        total: 0,
        label: "Waiting…".into(),
        cache_hit: false,
        updated_at_ms: now_ms(),
        quote_generation: None,
        quote_eligibility: None,
        quote_blocker: None,
    }
}

/// `GET /api/v1/route/solve/progress` — advisory poll (same query params as solve).
#[utoipa::path(
    get,
    path = "/api/v1/route/solve/progress",
    params(SolveRouteParams),
    responses(
        (status = 200, description = "Advisory solve progress", body = SolveProgress),
        (status = 400, description = "Missing or invalid amount_in"),
    ),
    tag = "Routing"
)]
pub async fn solve_route_progress(
    State(state): State<AppState>,
    Query(q): Query<SolveRouteParams>,
) -> Result<Json<SolveProgress>, (StatusCode, String)> {
    super::local_quote::progress(&state, &q).map(Json)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn unique_key(suffix: &str) -> String {
        format!("test-progress-{suffix}-{}", now_ms())
    }

    #[test]
    fn begin_update_complete_lifecycle() {
        let key = unique_key("lifecycle");
        progress_begin(&key);
        let p = progress_get(&key).expect("begin");
        assert_eq!(p.stage, STAGE_GRAPH_LOAD);

        progress_update(&key, STAGE_EVALUATING, 2, 5, "Searching 2 of 5 paths…");
        let p = progress_get(&key).expect("update");
        assert_eq!(p.stage, STAGE_EVALUATING);
        assert_eq!(p.done, 2);
        assert_eq!(p.total, 5);
        assert!(p.label.contains("2 of 5"));

        progress_complete(&key, false);
        let p = progress_get(&key).expect("complete");
        assert_eq!(p.stage, STAGE_DONE);
        assert!(!p.cache_hit);
    }

    #[test]
    fn progress_fail_sets_error_stage() {
        let key = unique_key("fail");
        progress_begin(&key);
        progress_fail(&key, "No route found");
        let p = progress_get(&key).expect("fail");
        assert_eq!(p.stage, STAGE_ERROR);
        assert_eq!(p.label, "No route found");
    }

    #[test]
    fn progress_get_unknown_returns_none() {
        assert!(progress_get(&unique_key("missing")).is_none());
    }

    #[test]
    fn progress_complete_cache_hit_sets_cached() {
        let key = unique_key("cached");
        progress_begin(&key);
        progress_complete(&key, true);
        let p = progress_get(&key).expect("cached");
        assert_eq!(p.stage, STAGE_CACHED);
        assert!(p.cache_hit);
    }

    #[test]
    fn concurrent_keys_do_not_crosstalk() {
        let key_a = unique_key("a");
        let key_b = unique_key("b");
        progress_begin(&key_a);
        progress_begin(&key_b);
        progress_update(&key_a, STAGE_EVALUATING, 1, 3, "A path 1 of 3");
        progress_update(&key_b, STAGE_LOADING_MIRRORS, 2, 4, "B mirror 2 of 4");
        let pa = progress_get(&key_a).unwrap();
        let pb = progress_get(&key_b).unwrap();
        assert_eq!(pa.stage, STAGE_EVALUATING);
        assert_eq!(pb.stage, STAGE_LOADING_MIRRORS);
        assert_ne!(pa.label, pb.label);
    }

    #[test]
    fn eviction_when_over_max_entries() {
        for i in 0..MAX_ENTRIES + 5 {
            let key = format!("evict-{i}-{}", now_ms());
            progress_begin(&key);
            progress_complete(&key, false);
        }
        let guard = registry();
        let map = guard.as_ref().expect("registry");
        assert!(map.len() <= MAX_ENTRIES);
    }
}
