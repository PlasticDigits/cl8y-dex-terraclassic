//! Independent scheduled bounded quote observation workers.
use super::*;
use crate::api::{local_reference_quote, local_wrapper_quote};

pub(super) async fn query(state: &AppState, address: &str, value: Value) -> Option<Value> {
    state
        .lcd
        .query_contract_bounded(address, &value, 32_768)
        .await
        .ok()
}

pub(super) fn actor_context(
    blocked: Option<bool>,
    discount: Option<u16>,
    previous: Option<(u16, bool)>,
) -> Option<(u16, bool)> {
    match blocked {
        Some(true) => Some((0, true)),
        Some(false) => Some((discount.unwrap_or(0), false)),
        None => previous
            .filter(|(_, was_blocked)| *was_blocked)
            .map(|_| (0, true)),
    }
}
pub(super) const MAX_ACTOR_CONTEXTS: usize = 2048;
const RECENT_ACTORS: usize = 120;
const HISTORICAL_BLOCK_REFRESH: usize = 8;
pub(super) fn retained_blocks(
    previous: &HashMap<String, (u16, bool)>,
) -> Result<HashMap<String, (u16, bool)>, sqlx::Error> {
    let blocks = previous
        .iter()
        .filter(|(_, (_, blocked))| *blocked)
        .map(|(address, _)| (address.clone(), (0, true)))
        .collect::<HashMap<_, _>>();
    if blocks.len() > MAX_ACTOR_CONTEXTS {
        return Err(actor_capacity_error());
    }
    Ok(blocks)
}
fn actor_capacity_error() -> sqlx::Error {
    sqlx::Error::Protocol("Local actor observation capacity exhausted".into())
}
/// Fees can fall back conservatively; an observed block is never a capacity victim.
pub(super) fn insert_actor(
    actors: &mut HashMap<String, (u16, bool)>,
    address: String,
    context: (u16, bool),
) -> Result<(), sqlx::Error> {
    if !actors.contains_key(&address) && actors.len() >= MAX_ACTOR_CONTEXTS {
        let victim = actors
            .iter()
            .filter(|(_, (_, blocked))| !*blocked)
            .map(|(address, _)| address)
            .min()
            .cloned();
        if let Some(victim) = victim {
            actors.remove(&victim);
        } else if context.1 {
            return Err(actor_capacity_error());
        } else {
            return Ok(());
        }
    }
    actors.insert(address, context);
    Ok(())
}
/// Reserve eight observations for forgotten-but-still-blocked wallets. Rotation
/// is per namespace and advances independently of request traffic or snapshot epochs.
pub(super) fn actor_refresh_addresses(
    recent: Vec<String>,
    previous: &HashMap<String, (u16, bool)>,
    cursor: usize,
) -> (Vec<String>, usize) {
    let mut selected = recent.into_iter().take(RECENT_ACTORS).collect::<Vec<_>>();
    let mut historical = previous
        .iter()
        .filter(|(address, (_, blocked))| *blocked && !selected.contains(address))
        .map(|(address, _)| address.clone())
        .collect::<Vec<_>>();
    historical.sort();
    if historical.is_empty() {
        return (selected, 0);
    }
    let count = historical.len().min(HISTORICAL_BLOCK_REFRESH);
    for offset in 0..count {
        selected.push(historical[(cursor % historical.len() + offset) % historical.len()].clone());
    }
    (
        selected,
        (cursor % historical.len() + count) % historical.len(),
    )
}
/// Refresh immutable reserves independently of slow configuration/network admission.
/// Only scheduled workers and test setup call this function.
pub async fn refresh_reserves(state: &AppState) -> Result<(), sqlx::Error> {
    let ns = state.lcd.chain_cache_namespace();
    let snapshot = store().lock().unwrap().snapshots.get(&ns).cloned();
    let Some(s) = snapshot else {
        return Ok(());
    };
    if s.config_expiry_ms <= now_ms() {
        return Ok(());
    }
    let mut mirrors = HashMap::new();
    for pair in &s.pairs {
        if s.config_expiries
            .get(&pair.contract_address)
            .copied()
            .unwrap_or(s.config_expiry_ms)
            <= now_ms()
        {
            continue;
        }
        if crate::indexer::asset_code_id_freeze::is_pair_code_id_frozen(&pair.contract_address) {
            continue;
        }
        if let Ok(m) = sim::load_local_mirror(&state.pool, pair, &s.addresses, s.max_age_ms).await {
            mirrors.insert(pair.contract_address.clone(), m);
        }
    }
    publish(
        ns,
        s.pairs.clone(),
        s.addresses.clone(),
        mirrors,
        s.max_age_ms,
        s.hooks.clone(),
        s.config_expiry_ms,
        s.config_expiries.clone(),
        local_reference_quote::ReferencePrices::observe(
            &state.pool,
            &sqlx::query_as::<_, assets::AssetRow>("SELECT * FROM assets ORDER BY id LIMIT 2048")
                .fetch_all(&state.pool)
                .await?,
            &state.hub_usd,
        )
        .await?,
        s.actors.clone(),
        s.discounted_pairs.clone(),
        s.policy_identity.clone(),
        s.taxes.clone(),
        Some(s.config_epoch),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_wallet_block_survives_missing_fee_and_failed_blacklist_refresh() {
        assert_eq!(actor_context(Some(true), None, None), Some((0, true)));
        assert_eq!(
            actor_context(None, None, Some((250, true))),
            Some((0, true))
        );
        assert_eq!(
            actor_context(Some(false), Some(250), Some((0, true))),
            Some((250, false))
        );
        assert_eq!(actor_context(None, None, Some((250, false))), None);
        assert_eq!(actor_context(Some(false), None, None), Some((0, false)));
    }
    #[test]
    fn previous_block_absent_from_recent_census_is_retained_and_fresh_unblock_needs_no_fee() {
        let previous = HashMap::from([
            ("historical-block".into(), (250, true)),
            ("old-fee".into(), (500, false)),
        ]);
        let mut actors = retained_blocks(&previous).unwrap();
        assert_eq!(actors.get("historical-block"), Some(&(0, true)));
        assert!(!actors.contains_key("old-fee"));
        let context = actor_context(None, None, previous.get("historical-block").copied()).unwrap();
        insert_actor(&mut actors, "historical-block".into(), context).unwrap();
        assert!(actors["historical-block"].1);
        let cleared =
            actor_context(Some(false), None, previous.get("historical-block").copied()).unwrap();
        insert_actor(&mut actors, "historical-block".into(), cleared).unwrap();
        assert_eq!(actors["historical-block"], (0, false));
    }
    #[test]
    fn historical_blocks_rotate_outside_recent_census_with_fixed_observation_budget() {
        let previous = (0..20)
            .map(|i| (format!("historical-{i:02}"), (0, true)))
            .collect::<HashMap<_, _>>();
        let recent = (0..120)
            .map(|i| format!("recent-{i:03}"))
            .collect::<Vec<_>>();
        let mut seen = HashSet::new();
        let mut cursor = 0;
        for _ in 0..3 {
            let (selected, next) = actor_refresh_addresses(recent.clone(), &previous, cursor);
            assert_eq!(selected.len(), 128);
            assert_eq!(&selected[..120], recent.as_slice());
            seen.extend(selected[120..].iter().cloned());
            cursor = next;
        }
        assert_eq!(seen.len(), 20);
    }
    #[test]
    fn actor_capacity_never_evicts_blocks_and_new_block_overflow_declines_publication() {
        let mut actors = (0..MAX_ACTOR_CONTEXTS)
            .map(|i| (format!("blocked-{i:04}"), (0, true)))
            .collect::<HashMap<_, _>>();
        let original = actors.clone();
        insert_actor(&mut actors, "unknown-fee".into(), (500, false)).unwrap();
        assert_eq!(actors, original);
        assert!(insert_actor(&mut actors, "new-block".into(), (0, true)).is_err());
        assert_eq!(actors, original);
        actors.insert("blocked-0000".into(), (0, false));
        insert_actor(&mut actors, "new-block".into(), (0, true)).unwrap();
        assert_eq!(actors.len(), MAX_ACTOR_CONTEXTS);
        assert!(!actors.contains_key("blocked-0000"));
        assert!(actors["new-block"].1);
    }
}
/// Scheduled wrapper observation pass; requests never call it.
pub async fn refresh_wrapper(state: &AppState) -> bool {
    if let Some(w) = local_wrapper_quote::observe(state).await {
        publish_wrapper(state.lcd.chain_cache_namespace(), w);
        true
    } else {
        false
    }
}
/// Counters contain no actor/input information and make cache/coalescing evidence testable.
pub fn cache_counters() -> (u64, u64, usize, usize) {
    let g = store().lock().unwrap();
    (g.hits, g.started, g.jobs.len(), g.results.len())
}
/// Independent scheduled workers, each with one outstanding pass. Missed ticks skip;
/// requests never enqueue, retry or accelerate ingestion.
pub fn start_ingestion(mut state: AppState) {
    state.lcd = state.lcd.with_ingestion_budget();
    let wrapper_state = state.clone();
    tokio::spawn(async move {
        let mut timer = tokio::time::interval(Duration::from_secs(10));
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            timer.tick().await;
            if let Some(w) = local_wrapper_quote::observe(&wrapper_state).await {
                publish_wrapper(wrapper_state.lcd.chain_cache_namespace(), w);
            }
        }
    });
    let reserves = state.clone();
    tokio::spawn(async move {
        let mut timer = tokio::time::interval(Duration::from_secs(2));
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            timer.tick().await;
            match tokio::time::timeout(Duration::from_secs(2), refresh_reserves(&reserves)).await {
                Ok(Ok(())) => {}
                Ok(Err(e)) => tracing::warn!("Local reserve ingestion unavailable: {e}"),
                Err(_) => tracing::warn!("Local reserve ingestion exceeded deadline"),
            }
        }
    });
    tokio::spawn(async move {
        let mut timer = tokio::time::interval(Duration::from_secs(2));
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            timer.tick().await;
            match tokio::time::timeout(
                Duration::from_secs(60),
                super::admission::ingest_once(&state),
            )
            .await
            {
                Ok(Ok(())) => {}
                Ok(Err(e)) => tracing::warn!("Local quote admission unavailable: {e}"),
                Err(_) => tracing::warn!("Local quote admission exhausted pass budget"),
            }
        }
    });
}
