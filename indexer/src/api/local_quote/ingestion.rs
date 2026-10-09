//! Independent scheduled bounded quote observation workers.
use super::*;
use crate::api::{
    community_tax_rank, local_hook_pricing, local_reference_quote, local_wrapper_quote,
    trading_tax_discovery,
};

async fn query(state: &AppState, address: &str, value: Value) -> Option<Value> {
    state
        .lcd
        .query_contract_bounded(address, &value, 32_768)
        .await
        .ok()
}

fn actor_context(
    blocked: Option<bool>,
    discount: Option<u16>,
    previous: Option<(u16, bool)>,
) -> Option<(u16, bool)> {
    match blocked {
        Some(true) => Some((0, true)),
        Some(false) => discount.map(|bps| (bps, false)),
        None => previous
            .filter(|(_, was_blocked)| *was_blocked)
            .map(|_| (0, true)),
    }
}
/// One scheduled observation pass, also used by integration fixtures. No request invokes it.
/// Sequential admission has one outstanding provider operation and a per-pair timeout;
/// failed or unsupported config is omitted rather than interpreted as tax/hook free.
pub async fn ingest_once(state: &AppState) -> Result<(), sqlx::Error> {
    let all_assets: Vec<assets::AssetRow> =
        sqlx::query_as("SELECT * FROM assets ORDER BY id LIMIT 2048")
            .fetch_all(&state.pool)
            .await?;
    let (addresses, _) = route_solver::build_id_to_addr_map(&all_assets);
    let all_pairs: Vec<pairs::PairRow> =
        sqlx::query_as("SELECT * FROM pairs ORDER BY contract_address LIMIT 512")
            .fetch_all(&state.pool)
            .await?;
    let policy = community_tax_rank::RouterHopTaxPolicy::from_catalog(&state.community_tax);
    let mut ordinary = HashMap::new();
    let mut asset_addresses = addresses.values().collect::<Vec<_>>();
    asset_addresses.sort();
    for address in asset_addresses.into_iter().take(MAX_PAIRS) {
        let verdict = trading_tax_discovery::discover(state, address, &policy).await;
        if let trading_tax_discovery::TradingTax::Ordinary { code_id: Some(id) } = verdict {
            ordinary.insert(address.clone(), id as u64);
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    let mut admitted = Vec::new();
    let mut hook_prices = HashMap::new();
    let mut discounted_pairs = HashSet::new();
    let mut policy_identity = Vec::new();
    let config_started = now_ms();
    let references =
        local_reference_quote::ReferencePrices::observe(&state.pool, &all_assets, &state.hub_usd)
            .await?;
    let actor_addresses: Vec<String> = sqlx::query_scalar(
        "SELECT address FROM traders ORDER BY last_trade_at DESC NULLS LAST,address LIMIT 128",
    )
    .fetch_all(&state.pool)
    .await?;
    let mut actors = HashMap::new();
    let previous_actors = store()
        .lock()
        .unwrap()
        .snapshots
        .get(&state.lcd.chain_cache_namespace())
        .map(|s| s.actors.clone())
        .unwrap_or_default();
    if let Some(factory) = state.factory_address.as_deref() {
        for address in actor_addresses {
            let verdict = query(
                state,
                factory,
                json!({"blacklist_check":{"wallet":address,"tokens":[],"pairs":[],"pair":null}}),
            )
            .await;
            let blocked = verdict.and_then(|v| v["wallet_blacklisted"].as_bool());
            let discount = if blocked != Some(false) {
                None
            } else if let Some(registry) = state.fee_discount_address.as_deref() {
                state
                    .lcd
                    .query_contract_bounded::<Value>(
                        registry,
                        &json!({"get_discount":{"trader":address,"sender":address}}),
                        32_768,
                    )
                    .await
                    .ok()
                    .and_then(|v| v["discount_bps"].as_u64())
                    .filter(|d| *d <= 10_000)
                    .map(|d| d as u16)
            } else {
                Some(0)
            };
            if let Some(context) =
                actor_context(blocked, discount, previous_actors.get(&address).copied())
            {
                actors.insert(address, context);
            }
            tokio::time::sleep(Duration::from_millis(250)).await;
        }
    }
    for pair in all_pairs.into_iter().take(MAX_PAIRS) {
        if crate::indexer::asset_code_id_freeze::is_pair_code_id_frozen(&pair.contract_address) {
            continue;
        }
        let (Some(a), Some(b)) = (
            addresses.get(&pair.asset_0_id),
            addresses.get(&pair.asset_1_id),
        ) else {
            continue;
        };
        if crate::indexer::defillama::is_columbus5_gem_address(a)
            || crate::indexer::defillama::is_columbus5_gem_address(b)
        {
            continue;
        }
        let (Some(code_a), Some(code_b)) = (ordinary.get(a), ordinary.get(b)) else {
            continue;
        };
        let Some(factory) = state.factory_address.as_deref() else {
            continue;
        };
        let checked=tokio::time::timeout(Duration::from_secs(6),async{
            let paused=query(state,&pair.contract_address,json!({"is_paused":{}})).await?;
            let hooks=query(state,&pair.contract_address,json!({"get_hooks":{}})).await?;
            let pins=query(state,&pair.contract_address,json!({"get_asset_code_ids":{}})).await?;
            let blacklist=query(state,factory,json!({"blacklist_check":{"tokens":[a,b],"pairs":[pair.contract_address],"wallet":null,"pair":null}})).await?;
            let wa=query(state,factory,json!({"is_code_id_whitelisted":{"code_id":code_a}})).await?;
            let wb=query(state,factory,json!({"is_code_id_whitelisted":{"code_id":code_b}})).await?;
            let hook_list=hooks["hooks"].as_array()?;
            if hook_list.len()>8{return None;}
            let mut hook_obs=Vec::new();
            for hook in hook_list {let addr=hook.as_str()?;let cfg=query(state,addr,json!({"get_config":{}})).await?;hook_obs.push((addr.to_string(),cfg));}
            let prices=local_hook_pricing::CachedHookPrices::from_observation(&hook_obs).ok()?;
            let registry=query(state,&pair.contract_address,json!({"get_discount_registry":{}})).await;
            let discounted=state.fee_discount_address.as_deref().is_some_and(|configured|registry.as_ref().and_then(|v|v["registry"].as_str())==Some(configured));
            let identity=format!("{}|{pins}|{:?}",pair.contract_address,registry);
            Some((prices,discounted,identity,paused["paused"].as_bool()==Some(false) && pins["code_ids"]==json!([code_a,code_b])
                && blacklist["blacklisted_tokens"].as_array().is_some_and(|v|v.is_empty()) && blacklist["blacklisted_pairs"].as_array().is_some_and(|v|v.is_empty())
                && wa["whitelisted"].as_bool()==Some(true) && wb["whitelisted"].as_bool()==Some(true)))
        }).await.ok().flatten();
        if let Some((prices, discount_supported, identity, true)) = checked {
            policy_identity.push(identity);
            if discount_supported {
                discounted_pairs.insert(pair.contract_address.clone());
            }
            hook_prices.insert(pair.contract_address.clone(), prices);
            admitted.push(pair);
        }
        tokio::time::sleep(Duration::from_millis(250)).await;
    }
    let mut mirrors = HashMap::new();
    for pair in &admitted {
        if let Ok(m) = sim::load_local_mirror(
            &state.pool,
            pair,
            &addresses,
            state.book_snapshot_max_staleness_ms,
        )
        .await
        {
            mirrors.insert(pair.contract_address.clone(), m);
        }
    }
    publish(
        state.lcd.chain_cache_namespace(),
        admitted,
        addresses,
        mirrors,
        state.book_snapshot_max_staleness_ms,
        hook_prices,
        config_started + CONFIG_TTL_MS,
        references,
        actors,
        discounted_pairs,
        format!(
            "{:?}|{:?}|{policy_identity:?}",
            state.factory_address, state.router_address
        ),
        None,
    );
    Ok(())
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
        Some(s.config_epoch),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::actor_context;

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
        assert_eq!(actor_context(Some(false), None, None), None);
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
        let mut timer = tokio::time::interval(Duration::from_secs(60));
        timer.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            timer.tick().await;
            match tokio::time::timeout(
                Duration::from_millis(CONFIG_TTL_MS as u64),
                ingest_once(&state),
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
