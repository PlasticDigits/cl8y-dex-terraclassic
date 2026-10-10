//! Scheduled observations and integer gross funding for supported community tokens.
//! Unknown actor identity and unobservable stateful cooldowns never become tax-free quotes.
use super::*;
use crate::api::community_tax_rank::{sell_economic, tax_amount, CataloguedTaxToken};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub(super) struct ActorObservation {
    pub observed_at_ms: i64,
    pub protocol: bool,
    pub manager: bool,
    pub balance: Option<u128>,
}
#[derive(Clone, Debug)]
pub(super) struct TokenObservation {
    pub policy_identity: String,
    pub code_id: u64,
    pub buy_bps: u16,
    pub sell_bps: u16,
    pub observed_at_ms: i64,
    pub max_wallet: Option<u128>,
    pub pairs: HashSet<String>,
    pub actors: BTreeMap<String, ActorObservation>,
}
impl TokenObservation {
    pub fn merge(&mut self, newer: Self) {
        if self.policy_identity != newer.policy_identity
            || self.code_id != newer.code_id
            || self.buy_bps != newer.buy_bps
            || self.sell_bps != newer.sell_bps
            || self.max_wallet != newer.max_wallet
        {
            let quarantined = self.observed_at_ms == 0 || newer.observed_at_ms == 0;
            if newer.observed_at_ms >= self.observed_at_ms {
                let previous = std::mem::replace(self, newer);
                self.inherit_wallet_contexts(&previous);
            } else {
                // An older incompatible pair cannot rewrite current positive policy;
                // retain its negatives unless a newer explicit observation cleared them.
                self.merge_actor_observations(
                    newer
                        .actors
                        .into_iter()
                        .filter(|(_, actor)| actor.protocol)
                        .collect(),
                );
            }
            // Partial, unordered pair publication is never protocol-inventory recovery.
            if quarantined {
                self.observed_at_ms = 0;
            }
            return;
        }
        self.pairs.extend(newer.pairs);
        self.merge_actor_observations(newer.actors);
        self.observed_at_ms = self.observed_at_ms.min(newer.observed_at_ms);
    }

    /// Wallet state is independent of pair slices. Carry original observations,
    /// never previous registered pairs or configuration times. A policy change
    /// drops positive approvals; a protocol negative needs an explicit fresh clear.
    pub fn inherit_wallet_contexts(&mut self, previous: &Self) {
        let compatible = self.policy_identity == previous.policy_identity
            && self.code_id == previous.code_id
            && self.buy_bps == previous.buy_bps
            && self.sell_bps == previous.sell_bps
            && self.max_wallet == previous.max_wallet;
        let fresh = std::mem::take(&mut self.actors);
        self.actors = previous
            .actors
            .iter()
            .filter(|(_, observation)| {
                observation.protocol || (compatible && observation.fresh_at(now_ms()))
            })
            .map(|(actor, observation)| (actor.clone(), observation.clone()))
            .collect();
        self.merge_actor_observations(fresh);
        // Zero is the fail-closed capacity marker, never a configuration time.
        // A partial page cannot prove that an omitted protocol negative cleared.
        if previous.observed_at_ms == 0 {
            self.observed_at_ms = 0;
        }
    }
    fn merge_actor_observations(&mut self, observations: BTreeMap<String, ActorObservation>) {
        for (actor, observation) in observations {
            if self
                .actors
                .get(&actor)
                .is_some_and(|old| old.observed_at_ms > observation.observed_at_ms)
            {
                continue;
            }
            if !self.actors.contains_key(&actor)
                && self.actors.len() >= ingestion::MAX_ACTOR_CONTEXTS
            {
                // Protocol negatives survive eviction; exhaustion cannot authorize new wallets.
                let victim = self
                    .actors
                    .iter()
                    .filter(|(_, obs)| !obs.protocol)
                    .min_by_key(|(address, obs)| (obs.observed_at_ms, *address))
                    .map(|(address, _)| address.clone());
                if let Some(victim) = victim {
                    self.actors.remove(&victim);
                } else if observation.protocol {
                    self.observed_at_ms = 0;
                } else {
                    continue;
                }
            }
            if self.actors.len() < ingestion::MAX_ACTOR_CONTEXTS || self.actors.contains_key(&actor)
            {
                self.actors.insert(actor, observation);
            }
        }
    }
    pub fn rates(&self, actor: Option<&str>, pair: &str) -> Option<(u16, u16)> {
        if now_ms() < self.observed_at_ms
            || now_ms() - self.observed_at_ms >= CONFIG_TTL_MS
            || !self.pairs.contains(pair)
        {
            return None;
        }
        let exempt = match actor {
            Some(actor) => {
                let observed = self.actors.get(actor)?;
                if observed.protocol || !observed.fresh_at(now_ms()) {
                    return None;
                }
                observed.manager
            }
            None => false,
        };
        Some(if exempt {
            (0, 0)
        } else {
            (self.sell_bps, self.buy_bps)
        })
    }
    pub fn actor_expiry(&self, actor: Option<&str>) -> Option<i64> {
        self.actors
            .get(actor?)
            .map(|observation| observation.observed_at_ms.saturating_add(CONFIG_TTL_MS))
    }
    pub fn permits_credit(&self, actor: Option<&str>, amount: u128) -> bool {
        self.max_wallet.is_none_or(|max| {
            actor
                .and_then(|a| self.actors.get(a))
                .filter(|a| !a.protocol && a.fresh_at(now_ms()))
                .and_then(|a| a.balance)
                .and_then(|balance| balance.checked_add(amount))
                .is_some_and(|new| new <= max)
        })
    }
}
impl ActorObservation {
    fn fresh_at(&self, at: i64) -> bool {
        at >= self.observed_at_ms && at - self.observed_at_ms < CONFIG_TTL_MS
    }
}
pub(super) fn sell(gross: u128, bps: u16) -> Option<(u128, u128, u128)> {
    if bps > 10_000 {
        return None;
    }
    if bps == 0 {
        return Some((gross, 0, 0));
    }
    let economic = sell_economic(gross, bps);
    let tax = tax_amount(economic, bps);
    Some((
        economic,
        tax,
        gross.checked_sub(economic.checked_add(tax)?)?,
    ))
}
pub(super) fn buy(gross: u128, bps: u16) -> Option<(u128, u128)> {
    if bps > 10_000 {
        return None;
    }
    let tax = tax_amount(gross, bps);
    Some((gross.checked_sub(tax)?, tax))
}
/// Smallest gross buy amount delivering the net target. One floored tax is monotone.
pub(super) fn inverse_buy(target: u128, bps: u16) -> Option<u128> {
    if bps == 0 {
        return Some(target);
    }
    if bps > 10_000 || buy(u128::MAX, bps)?.0 < target {
        return None;
    }
    let (mut lo, mut hi) = (target, u128::MAX);
    while lo < hi {
        let mid = lo + (hi - lo) / 2;
        if buy(mid, bps)?.0 >= target {
            hi = mid;
        } else {
            lo = mid + 1;
        }
    }
    Some(lo)
}
async fn query(state: &AppState, token: &str, query: Value) -> Option<Value> {
    state
        .lcd
        .query_contract_bounded(token, &query, 32_768)
        .await
        .ok()
}
pub(super) async fn observe_actor(
    state: &AppState,
    token: &str,
    actor: &str,
) -> Option<ActorObservation> {
    let observed_at_ms = now_ms();
    let flags = query(
        state,
        token,
        json!({"is_protocol_exempt":{"address":actor}}),
    )
    .await?;
    if flags["address"].as_str() != Some(actor) {
        return None;
    }
    let protocol = flags["protocol"].as_bool()? || actor == token;
    if protocol {
        protocol_guards::record(
            &state.lcd.chain_cache_namespace(),
            token,
            actor,
            observed_at_ms,
        )?;
    }
    let manager = flags["manager"].as_bool()?;
    let balance = query(state, token, json!({"balance":{"address":actor}}))
        .await
        .and_then(|v| v["balance"].as_str()?.parse().ok());
    Some(ActorObservation {
        observed_at_ms,
        protocol,
        manager,
        balance,
    })
}
/// Public queries expose static guards but no cooldown last-trade state. Unsupported
/// dynamic actions remain unavailable before any amount-specific arithmetic.
fn supported_guards(config: &Value, features: &Value) -> Option<Option<u128>> {
    if features["auto_v2_lp"].as_bool() != Some(false) {
        return None;
    }
    if features["launch_guards"].as_bool()? && !config["launch_guards"].is_null() {
        let guard = &config["launch_guards"];
        if guard["trading_enabled"].as_bool() != Some(true)
            || guard["cooldown_blocks"].as_u64() != Some(0)
        {
            return None;
        }
        if !guard["max_wallet"].is_null() {
            return Some(Some(guard["max_wallet"].as_str()?.parse().ok()?));
        }
    }
    Some(None)
}
/// Invoked only by independently scheduled ingestion. Discovery already attests immutable code
/// and v1 router identity; this adds supported config, pair registration and actor observations.
pub(super) async fn observe(
    state: &AppState,
    token: &CataloguedTaxToken,
    pair_addresses: &[String],
    actors: &[(String, i64)],
) -> Option<TokenObservation> {
    if token.route_funding_version != Some(1) || token.buy_bps > 10_000 || token.sell_bps > 10_000 {
        return None;
    }
    let started = now_ms();
    let router = state.router_address.as_deref()?;
    let router_config = query(state, router, json!({"config":{}})).await?;
    if router_config["funding_version"].as_u64() != Some(1)
        || router_config["factory"].as_str() != state.factory_address.as_deref()
    {
        return None;
    }
    let config = query(state, &token.contract_address, json!({"get_config":{}})).await?;
    let features = query(state, &token.contract_address, json!({"get_features":{}})).await?;
    if config["router"].as_str() != Some(router)
        || config["factory"].as_str() != state.factory_address.as_deref()
        || config["buy_bps"].as_u64() != Some(token.buy_bps.into())
        || config["sell_bps"].as_u64() != Some(token.sell_bps.into())
    {
        return None;
    }
    let max_wallet = supported_guards(&config, &features)?;
    // Reject tax sinks that could add unrelated router/pair balances. Conservatively
    // disallow protocol sinks; AutoLP execution is outside supported observations.
    let configured_sinks = config["sinks"].as_array()?;
    if configured_sinks.len() > 4 {
        return None;
    }
    let mut sinks = Vec::new();
    if features["split_router"].as_bool()? && !configured_sinks.is_empty() {
        for sink in configured_sinks {
            if sink["bps"].as_u64()? > 10_000 {
                return None;
            }
            match sink["kind"].as_str()? {
                "treasury" => sinks.push(config["treasury"].as_str()?.to_owned()),
                "wallet" => sinks.push(sink["addr"].as_str()?.to_owned()),
                "burn" => {}
                // Sister threshold/settlement requires a richer local observation.
                "auto_lp" => return None,
                _ => return None,
            }
        }
    } else {
        sinks.push(config["treasury"].as_str()?.to_owned());
    }
    let sink_addresses = sinks.iter().cloned().collect::<HashSet<_>>();
    for address in sinks {
        // A configured tax recipient cannot be the routed trader. Preserve that
        // negative before any dependent sink/actor/config/code await can fail.
        protocol_guards::record(
            &state.lcd.chain_cache_namespace(),
            &token.contract_address,
            &address,
            started,
        )?;
        if address == router || pair_addresses.contains(&address) {
            return None;
        }
        let flags = query(
            state,
            &token.contract_address,
            json!({"is_protocol_exempt":{"address":address}}),
        )
        .await?;
        if flags["address"].as_str() != Some(&address) || flags["protocol"].as_bool()? {
            return None;
        }
    }
    let mut pairs = HashSet::new();
    for pair in pair_addresses.iter().take(MAX_PAIRS) {
        if let Some(preview) = query(
            state,
            &token.contract_address,
            json!({"route_tax_preview":{"kind":"sell","amount":"0","trader":null,"pair":pair}}),
        )
        .await
        {
            if preview["funding_version"].as_u64() == Some(1)
                && preview["gross"] == "0"
                && preview["economic"] == "0"
                && preview["net"] == "0"
                && preview["tax"] == "0"
            {
                pairs.insert(pair.clone());
            }
        }
    }
    if pairs.is_empty() {
        return None;
    }
    let mut observed_actors = BTreeMap::new();
    for (actor, factory_observed_at) in actors.iter().take(ingestion::MAX_TAX_ACTORS_PER_PASS) {
        if let Some(mut observation) = observe_actor(state, &token.contract_address, actor).await {
            // Blacklist identity predates token flags/balance; later dependent reads
            // cannot renew that earlier guard through a provider blackout.
            observation.observed_at_ms = observation.observed_at_ms.min(*factory_observed_at);
            if sink_addresses.contains(actor) {
                observation.protocol = true;
            }
            observed_actors.insert(actor.clone(), observation);
        }
    }
    // Recheck capability/code after the dependent reads to reject migrations/rotation.
    if query(state, &token.contract_address, json!({"get_config":{}})).await? != config
        || query(state, &token.contract_address, json!({"get_features":{}})).await? != features
    {
        return None;
    }
    let cap = query(
        state,
        &token.contract_address,
        json!({"route_tax_capability":{}}),
    )
    .await?;
    let code_id = u64::try_from(token.code_id?).ok()?;
    if state
        .lcd
        .get_contract_code_id(&token.contract_address)
        .await
        .ok()?
        != code_id
        || cap["funding_version"].as_u64() != Some(1)
        || cap["router"].as_str() != Some(router)
        || cap["buy_bps"].as_u64() != Some(token.buy_bps.into())
        || cap["sell_bps"].as_u64() != Some(token.sell_bps.into())
    {
        return None;
    }
    Some(TokenObservation {
        policy_identity: format!("{config}|{features}"),
        code_id,
        buy_bps: token.buy_bps,
        sell_bps: token.sell_bps,
        observed_at_ms: started,
        max_wallet,
        pairs,
        actors: observed_actors,
    })
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn incompatible_pair_policy_merge_cannot_overwrite_protocol_capacity_quarantine_in_either_order(
    ) {
        let at = now_ms();
        let quarantined = TokenObservation {
            policy_identity: "current-policy".into(),
            code_id: 2,
            buy_bps: 250,
            sell_bps: 500,
            observed_at_ms: 0,
            max_wallet: None,
            pairs: HashSet::from(["pair".into()]),
            actors: Default::default(),
        };
        let mut stale = quarantined.clone();
        stale.policy_identity = "old-policy".into();
        stale.code_id = 1;
        stale.observed_at_ms = at;
        stale.actors.insert(
            "wallet".into(),
            ActorObservation {
                observed_at_ms: at,
                protocol: false,
                manager: true,
                balance: Some(0),
            },
        );
        let mut current_first = quarantined.clone();
        current_first.merge(stale.clone());
        assert_eq!(current_first.observed_at_ms, 0);
        assert!(current_first.rates(Some("wallet"), "pair").is_none());
        let mut old_first = stale;
        old_first.merge(quarantined);
        assert_eq!(old_first.observed_at_ms, 0);
        assert!(old_first.rates(Some("wallet"), "pair").is_none());
    }
    #[test]
    fn protocol_capacity_quarantine_survives_partial_wallet_pages_and_policy_changes() {
        let at = now_ms();
        let mut previous = TokenObservation {
            policy_identity: "policy".into(),
            code_id: 1,
            buy_bps: 250,
            sell_bps: 500,
            observed_at_ms: at,
            max_wallet: None,
            pairs: HashSet::from(["pair".into()]),
            actors: (0..ingestion::MAX_ACTOR_CONTEXTS)
                .map(|i| {
                    (
                        format!("blocked-{i:04}"),
                        ActorObservation {
                            observed_at_ms: at,
                            protocol: true,
                            manager: false,
                            balance: None,
                        },
                    )
                })
                .collect(),
        };
        previous.merge_actor_observations(BTreeMap::from([(
            "overflow-protocol".into(),
            ActorObservation {
                observed_at_ms: at,
                protocol: true,
                manager: false,
                balance: None,
            },
        )]));
        assert_eq!(previous.observed_at_ms, 0);
        assert_eq!(previous.actors.len(), ingestion::MAX_ACTOR_CONTEXTS);
        let mut partial = previous.clone();
        partial.observed_at_ms = at;
        partial.actors = BTreeMap::from([(
            "blocked-0000".into(),
            ActorObservation {
                observed_at_ms: at,
                protocol: false,
                manager: true,
                balance: Some(0),
            },
        )]);
        partial.inherit_wallet_contexts(&previous);
        assert_eq!(partial.observed_at_ms, 0);
        assert!(partial.rates(Some("blocked-0000"), "pair").is_none());
        let mut changed = partial.clone();
        changed.policy_identity = "new-policy".into();
        changed.observed_at_ms = at;
        changed.actors.clear();
        changed.inherit_wallet_contexts(&partial);
        assert_eq!(changed.observed_at_ms, 0);
    }
    #[test]
    fn independent_wallet_ledger_survives_pair_pages_but_never_renews_time_or_mixes_policy() {
        let at = now_ms();
        let previous = TokenObservation {
            policy_identity: "old-policy".into(),
            code_id: 1,
            buy_bps: 250,
            sell_bps: 500,
            observed_at_ms: at - 1000,
            max_wallet: Some(100),
            pairs: HashSet::from(["old-pair".into()]),
            actors: BTreeMap::from([
                (
                    "page-one".into(),
                    ActorObservation {
                        observed_at_ms: at - 1000,
                        protocol: false,
                        manager: true,
                        balance: Some(0),
                    },
                ),
                (
                    "old-protocol".into(),
                    ActorObservation {
                        observed_at_ms: at - CONFIG_TTL_MS,
                        protocol: true,
                        manager: false,
                        balance: None,
                    },
                ),
            ]),
        };
        let mut fresh = previous.clone();
        fresh.observed_at_ms = at;
        fresh.pairs = HashSet::from(["new-pair".into()]);
        fresh.actors = BTreeMap::from([(
            "page-two".into(),
            ActorObservation {
                observed_at_ms: at,
                protocol: false,
                manager: false,
                balance: Some(10),
            },
        )]);
        fresh.inherit_wallet_contexts(&previous);
        assert_eq!(fresh.pairs, HashSet::from(["new-pair".into()]));
        assert_eq!(fresh.observed_at_ms, at);
        assert_eq!(fresh.actors["page-one"].observed_at_ms, at - 1000);
        assert_eq!(fresh.actors.len(), 3);
        let ledger = fresh.clone();
        fresh.actors.clear(); // Terminal page/failed wallet read preserves old times.
        fresh.inherit_wallet_contexts(&ledger);
        assert_eq!(fresh.actors["page-one"].observed_at_ms, at - 1000);
        let mut expired = ledger.clone();
        expired.actors.get_mut("page-one").unwrap().observed_at_ms = at - CONFIG_TTL_MS;
        fresh.actors.clear();
        fresh.inherit_wallet_contexts(&expired);
        assert!(!fresh.actors.contains_key("page-one"));
        assert!(fresh.actors.contains_key("old-protocol"));
        fresh.policy_identity = "changed-policy".into();
        fresh.actors.clear();
        fresh.inherit_wallet_contexts(&ledger);
        assert_eq!(fresh.actors.len(), 1);
        assert!(fresh.actors["old-protocol"].protocol);
        let guarded = fresh.clone();
        fresh.actors = BTreeMap::from([(
            "old-protocol".into(),
            ActorObservation {
                observed_at_ms: at,
                protocol: false,
                manager: false,
                balance: Some(0),
            },
        )]);
        fresh.inherit_wallet_contexts(&guarded);
        assert!(!fresh.actors["old-protocol"].protocol);
    }
    #[test]
    fn wallet_identity_and_cap_observations_expire_independently_and_cannot_be_renewed_by_merge() {
        let at = now_ms();
        let mut token = TokenObservation {
            policy_identity: "policy".into(),
            code_id: 1,
            buy_bps: 250,
            sell_bps: 500,
            observed_at_ms: at,
            max_wallet: Some(100),
            pairs: HashSet::from(["pair".into()]),
            actors: BTreeMap::from([(
                "wallet".into(),
                ActorObservation {
                    observed_at_ms: at - CONFIG_TTL_MS,
                    protocol: false,
                    manager: true,
                    balance: Some(0),
                },
            )]),
        };
        assert!(token.rates(Some("wallet"), "pair").is_none());
        assert!(!token.permits_credit(Some("wallet"), 1));
        let mut newer = token.clone();
        newer.observed_at_ms = at + 1;
        newer.actors.clear(); // A failed actor read cannot make old approval fresh.
        token.merge(newer);
        assert_eq!(token.actor_expiry(Some("wallet")), Some(at));
        assert!(token.rates(Some("wallet"), "pair").is_none());
        token.actors.get_mut("wallet").unwrap().observed_at_ms = at + 10_000;
        assert!(token.rates(Some("wallet"), "pair").is_none());
        assert!(!token.permits_credit(Some("wallet"), 1));
    }
    #[test]
    fn token_wallet_capacity_preserves_protocol_negatives_and_newest_independent_observations() {
        let at = now_ms();
        let mut token = TokenObservation {
            policy_identity: "policy".into(),
            code_id: 1,
            buy_bps: 250,
            sell_bps: 500,
            observed_at_ms: at,
            max_wallet: None,
            pairs: HashSet::from(["pair".into()]),
            actors: (0..ingestion::MAX_ACTOR_CONTEXTS)
                .map(|i| {
                    (
                        format!("blocked-{i:04}"),
                        ActorObservation {
                            observed_at_ms: at,
                            protocol: true,
                            manager: false,
                            balance: None,
                        },
                    )
                })
                .collect(),
        };
        let mut newer = token.clone();
        newer.actors = BTreeMap::from([(
            "new-wallet".into(),
            ActorObservation {
                observed_at_ms: at + 1,
                protocol: false,
                manager: true,
                balance: Some(0),
            },
        )]);
        token.merge(newer.clone());
        assert_eq!(token.actors.len(), ingestion::MAX_ACTOR_CONTEXTS);
        assert!(!token.actors.contains_key("new-wallet"));
        token.actors.get_mut("blocked-0000").unwrap().protocol = false;
        token.merge(newer);
        assert_eq!(token.actors.len(), ingestion::MAX_ACTOR_CONTEXTS);
        assert!(!token.actors.contains_key("blocked-0000"));
        assert!(token.actors["new-wallet"].manager);
        let mut older = token.clone();
        older.actors.get_mut("new-wallet").unwrap().observed_at_ms = at - 1;
        older.actors.get_mut("new-wallet").unwrap().manager = false;
        token.merge(older);
        assert!(token.actors["new-wallet"].manager);
    }
    #[test]
    fn disabled_trading_cooldown_and_autolp_never_become_static_quote_approval() {
        let mut cfg = json!({"launch_guards":{"max_wallet":"100","cooldown_blocks":0,"trading_enabled":true}});
        let mut features = json!({"auto_v2_lp":false,"launch_guards":true});
        assert_eq!(supported_guards(&cfg, &features), Some(Some(100)));
        cfg["launch_guards"]["trading_enabled"] = json!(false);
        assert_eq!(supported_guards(&cfg, &features), None);
        cfg["launch_guards"]["trading_enabled"] = json!(true);
        cfg["launch_guards"]["cooldown_blocks"] = json!(1);
        assert_eq!(supported_guards(&cfg, &features), None);
        cfg["launch_guards"]["cooldown_blocks"] = json!(0);
        features["auto_v2_lp"] = json!(true);
        assert_eq!(supported_guards(&cfg, &features), None);
        features["auto_v2_lp"] = json!(false);
        cfg["launch_guards"]["max_wallet"] = json!("malformed");
        assert_eq!(supported_guards(&cfg, &features), None);
    }
    #[test]
    fn funded_rounding_maximality_and_no_overflow() {
        for gross in [1, 9_999, 10_000, 123_457, u128::MAX] {
            for rate in [0, 1, 333, 2500, 10000] {
                let (economic, tax, residue) = sell(gross, rate).unwrap();
                assert_eq!(economic.checked_add(tax).unwrap() + residue, gross);
                if economic < gross {
                    assert!(economic
                        .checked_add(1)
                        .and_then(|n| n.checked_add(tax_amount(n, rate)))
                        .is_none_or(|next| next > gross));
                }
                let (net, buy_tax) = buy(gross, rate).unwrap();
                assert_eq!(net + buy_tax, gross);
            }
        }
    }
    #[test]
    fn identity_and_max_wallet_never_waived_by_manager_exemption() {
        let mut token = TokenObservation {
            policy_identity: "synthetic".into(),
            code_id: 1,
            buy_bps: 333,
            sell_bps: 250,
            observed_at_ms: now_ms(),
            max_wallet: Some(100),
            pairs: HashSet::from(["pair".into()]),
            actors: BTreeMap::from([
                (
                    "manager".into(),
                    ActorObservation {
                        observed_at_ms: now_ms(),
                        protocol: false,
                        manager: true,
                        balance: Some(90),
                    },
                ),
                (
                    "protocol".into(),
                    ActorObservation {
                        observed_at_ms: now_ms(),
                        protocol: true,
                        manager: false,
                        balance: Some(0),
                    },
                ),
            ]),
        };
        assert_eq!(token.rates(Some("manager"), "pair"), Some((0, 0)));
        assert!(token.permits_credit(Some("manager"), 10));
        assert!(!token.permits_credit(Some("manager"), 11));
        assert!(!token.permits_credit(None, 1));
        assert!(token.rates(Some("unknown"), "pair").is_none());
        assert!(token.rates(Some("protocol"), "pair").is_none());
        assert!(token.rates(Some("manager"), "unknown-pair").is_none());
        token.observed_at_ms = now_ms() + 1000;
        assert!(token.rates(None, "pair").is_none());
    }
}

#[cfg(test)]
mod route_tests {
    use super::*;
    fn observation(s: &Snapshot, sell_bps: u16, buy_bps: u16) -> TokenObservation {
        TokenObservation {
            policy_identity: "synthetic".into(),
            code_id: 202,
            sell_bps,
            buy_bps,
            observed_at_ms: now_ms(),
            max_wallet: None,
            pairs: s.pairs.iter().map(|p| p.contract_address.clone()).collect(),
            actors: BTreeMap::from([
                (
                    "wallet".into(),
                    ActorObservation {
                        observed_at_ms: now_ms(),
                        protocol: false,
                        manager: false,
                        balance: Some(0),
                    },
                ),
                (
                    "manager".into(),
                    ActorObservation {
                        observed_at_ms: now_ms(),
                        protocol: false,
                        manager: true,
                        balance: Some(0),
                    },
                ),
            ]),
        }
    }
    #[test]
    fn named_wallet_original_blacklist_expiry_bounds_response_and_cannot_survive_refresh() {
        let mut snapshot = super::super::tests::fixture();
        let path = vec![("pair7a".into(), 1, 10), ("pair7b".into(), 10, 2)];
        let input = 10u128.pow(18);
        let mut token = observation(&snapshot, 250, 0);
        let expiry = now_ms() + 5_000;
        token.actors.get_mut("wallet").unwrap().observed_at_ms = expiry - CONFIG_TTL_MS;
        snapshot.taxes.insert("in18".into(), token);
        snapshot.tax_trader = Some("wallet".into());
        let candidate = score(&snapshot, &path, input, SCALE / 20).unwrap();
        let body = response(
            &snapshot,
            &super::super::tests::request(input),
            &candidate,
            input,
            SCALE / 20,
            1,
        );
        assert_eq!(body["execution_metadata"]["expires_at_ms"], expiry);
        // Configuration remains fresh while the required wallet context expires.
        snapshot
            .taxes
            .get_mut("in18")
            .unwrap()
            .actors
            .get_mut("wallet")
            .unwrap()
            .observed_at_ms = now_ms() - CONFIG_TTL_MS;
        assert!(score(&snapshot, &path, input, SCALE / 20).is_none());
    }
    #[test]
    fn taxed_intermediate_uses_only_preceding_net_and_boundary_specific_rates() {
        let mut s = super::super::tests::fixture();
        let path = vec![("pair7a".into(), 1, 10), ("pair7b".into(), 10, 2)];
        let input = 10u128.pow(18);
        let untaxed = score(&s, &path, input, SCALE / 20).unwrap();
        s.taxes.insert("in18".into(), observation(&s, 250, 0));
        s.taxes.insert("middle76".into(), observation(&s, 400, 333));
        s.taxes.insert("out18".into(), observation(&s, 0, 125));
        s.tax_trader = Some("wallet".into());
        let quoted = score(&s, &path, input, SCALE / 20).unwrap();
        let first_economic = sell(input, 250).unwrap().0;
        let first_leg =
            sim::quote_local_leg(&s.mirrors["pair7a"], "in18", first_economic, 0, 8).unwrap();
        let first_net = buy(first_leg.return_amount, 333).unwrap().0;
        let second_economic = sell(first_net, 400).unwrap().0;
        let second_leg =
            sim::quote_local_leg(&s.mirrors["pair7b"], "middle76", second_economic, 0, 8).unwrap();
        let final_net = buy(second_leg.return_amount, 125).unwrap().0;
        assert_eq!(quoted.output, final_net);
        assert!(quoted.output < untaxed.output);
        assert_eq!(
            quoted.hops[0]["economic_offer_amount"],
            first_economic.to_string()
        );
        assert_eq!(quoted.hops[1]["offer_amount"], first_net.to_string());
        assert_eq!(
            quoted.hops[1]["economic_offer_amount"],
            second_economic.to_string()
        );
        let response = response(
            &s,
            &super::super::tests::request(input),
            &quoted,
            input,
            SCALE / 20,
            1,
        );
        assert_eq!(response["tax_funding_version"], 1);
        assert_eq!(
            response["estimated_amount_out"],
            response["estimated_amount_out_net"]
        );
        s.tax_trader = Some("manager".into());
        assert_eq!(
            score(&s, &path, input, SCALE / 20).unwrap().output,
            untaxed.output
        );
        s.tax_trader = Some("unknown".into());
        assert!(score(&s, &path, input, SCALE / 20).is_none());
    }
    #[test]
    fn purchased_intermediate_cap_applies_to_returned_residue_and_final_delivery() {
        let mut s = super::super::tests::fixture();
        let path = vec![("pair7a".into(), 1, 10), ("pair7b".into(), 10, 2)];
        let mut middle = observation(&s, 400, 333);
        middle.max_wallet = Some(1);
        s.taxes.insert("middle76".into(), middle);
        s.tax_trader = Some("wallet".into());
        let quoted = score(&s, &path, 10u128.pow(18), SCALE / 20).unwrap();
        assert!(
            quoted.hops[0]["return_amount"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
                > 1
        );
        assert!(
            quoted.hops[1]["sell_residue"]
                .as_str()
                .unwrap()
                .parse::<u128>()
                .unwrap()
                <= 1
        );
        let mut final_token = observation(&s, 0, 0);
        final_token.max_wallet = Some(1);
        s.taxes.insert("out18".into(), final_token);
        assert!(score(&s, &path, 10u128.pow(18), SCALE / 20).is_none());
    }
    #[test]
    fn tax_capability_repetition_stays_rejected_with_zero_rates_and_manager_exemption() {
        let mut s = super::super::tests::fixture();
        s.taxes.insert("in18".into(), observation(&s, 0, 0));
        s.tax_trader = Some("manager".into());
        let repeated = vec![("pair7a".into(), 1, 10), ("pair7a".into(), 10, 1)];
        assert!(score(&s, &repeated, 10u128.pow(18), SCALE / 20).is_none());
    }
    #[test]
    fn configured_change_cannot_merge_old_registered_pair_or_actor_approval() {
        let s = super::super::tests::fixture();
        let mut previous = observation(&s, 250, 333);
        let mut current = previous.clone();
        current.policy_identity = "rotated-router".into();
        current.pairs = HashSet::from(["new-pair".into()]);
        current.actors.clear();
        current.observed_at_ms += 1;
        previous.merge(current);
        assert!(!previous.pairs.contains("pair7a"));
        assert!(previous.actors.is_empty());
    }
}

#[cfg(test)]
mod inverse_tests {
    use super::*;
    #[test]
    fn inverse_buy_delivers_target_at_minimal_gross_including_max_boundary() {
        for target in [1, 9_999, 10_000, 123_457, u128::MAX] {
            for rate in [0, 1, 333, 2500, 10000] {
                if let Some(gross) = inverse_buy(target, rate) {
                    assert!(buy(gross, rate).unwrap().0 >= target);
                    assert!(gross == 0 || buy(gross - 1, rate).unwrap().0 < target);
                } else {
                    assert!(buy(u128::MAX, rate).unwrap().0 < target);
                }
            }
        }
    }
    #[test]
    fn funded_two_hop_inverse_is_minimal_and_forward_covers_net_target() {
        let mut s = super::super::tests::fixture();
        let token = TokenObservation {
            policy_identity: "synthetic".into(),
            code_id: 202,
            buy_bps: 333,
            sell_bps: 250,
            observed_at_ms: now_ms(),
            max_wallet: None,
            pairs: s.pairs.iter().map(|p| p.contract_address.clone()).collect(),
            actors: BTreeMap::new(),
        };
        s.taxes.insert("middle76".into(), token);
        let path = vec![("pair7a".into(), 1, 10), ("pair7b".into(), 10, 2)];
        let target = 10u128.pow(18);
        let (gross, quote) = select_inverse(&s, &[path.clone()], target, SCALE / 20).unwrap();
        assert!(quote.output >= target);
        assert!(score(&s, &path, gross - 1, SCALE / 20).is_none_or(|q| q.output < target));
    }
}

#[cfg(test)]
mod wrapper_tests {
    use super::*;
    use crate::api::local_wrapper_quote::{NativeMapping, WrapperSnapshot};
    #[test]
    fn native_entry_retains_gross_budget_and_taxed_intermediate_net_and_stale_tax_fails() {
        let mut s = super::super::tests::fixture();
        s.taxes.insert(
            "middle76".into(),
            TokenObservation {
                policy_identity: "synthetic".into(),
                code_id: 202,
                buy_bps: 333,
                sell_bps: 250,
                observed_at_ms: now_ms(),
                max_wallet: None,
                pairs: s.pairs.iter().map(|p| p.contract_address.clone()).collect(),
                actors: BTreeMap::new(),
            },
        );
        let path = vec![("pair7a".into(), 1, 10), ("pair7b".into(), 10, 2)];
        let credit = 10u128.pow(18);
        let quote = score(&s, &path, credit, SCALE / 20).unwrap();
        let wrapper = WrapperSnapshot {
            mapper: "mapper".into(),
            treasury: "treasury".into(),
            observed_at_ms: now_ms(),
            mappings: HashMap::from([(
                "uluna".into(),
                NativeMapping {
                    cw20: "in18".into(),
                    maximum: None,
                    used: 0,
                    tax_cap: None,
                },
            )]),
            fee_wrap_bps: 200,
            fee_unwrap_bps: 0,
            tax_atomics: None,
            router_unwrap_supported: false,
            mapper_floor_version: None,
            treasury_floor_version: None,
        };
        let gross = wrapper.inverse_wrap(credit).unwrap();
        let mut original = super::super::tests::request(gross);
        original.token_in = "uluna".into();
        let context = NativeContext {
            request: original,
            wrapper,
            wrap: true,
            unwrap: false,
        };
        let body = final_response(
            &s,
            &super::super::tests::request(credit),
            &quote,
            credit,
            SCALE / 20,
            1,
            Some(&context),
        )
        .unwrap();
        assert_eq!(body["tax_funding_version"], 1);
        assert_eq!(
            body["execution_metadata"]["input_amount"],
            gross.to_string()
        );
        assert_eq!(
            body["execution_metadata"]["wrapper"]["route_input_amount"],
            credit.to_string()
        );
        assert_eq!(body["estimated_amount_out"], quote.output.to_string());
        assert_eq!(body["spot_amount_out"], gross.to_string());
        s.taxes.get_mut("middle76").unwrap().observed_at_ms -= CONFIG_TTL_MS;
        assert!(score(&s, &path, credit, SCALE / 20).is_none());
    }
}
