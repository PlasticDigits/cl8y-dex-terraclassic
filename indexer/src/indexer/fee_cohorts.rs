//! #1211: private fee-payer attribution at an immutable chain position.
//! No live traders join. A provider-certified H-1 snapshot plus ordered registry
//! receipts in H supplies membership. Missing history is unknown, never retail.
use super::protocol_fees::{wasm_contract_addr_before, wasm_kv_map_after_action};
use crate::{
    config::Config,
    lcd::{LcdClient, TxResponse},
};
use serde_json::{json, Value};
use sqlx::{FromRow, PgPool};
use std::{collections::HashMap, time::Duration};

pub struct AttributionBudget {
    deadline: Option<tokio::time::Instant>,
    remaining_reads: usize,
    schema_ready: Option<bool>,
}
impl Default for AttributionBudget {
    fn default() -> Self {
        Self {
            deadline: None,
            remaining_reads: 4,
            schema_ready: None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Registration {
    pub registered: bool,
    pub tier_id: i16,
}
impl Registration {
    pub fn cohort(self) -> &'static str {
        if self.registered && self.tier_id == 0 {
            "mm"
        } else {
            "retail"
        }
    }
}

/// A real Terra account/contract address; no actor bytes enter public aggregates.
fn valid_actor(value: &str) -> bool {
    use bech32::FromBase32;
    if !value.starts_with("terra1") || value.len() > 90 {
        return false;
    }
    let Ok((hrp, data, bech32::Variant::Bech32)) = bech32::decode(value) else {
        return false;
    };
    hrp == "terra"
        && Vec::<u8>::from_base32(&data).is_ok_and(|bytes| matches!(bytes.len(), 20 | 32))
}

pub fn parse_registration(value: &Value) -> Option<Registration> {
    let registered = value.get("registered")?.as_bool()?;
    let tier_id = if registered {
        i16::try_from(value.get("tier_id")?.as_u64()?).ok()?
    } else {
        0
    };
    (0..=255).contains(&tier_id).then_some(Registration {
        registered,
        tier_id,
    })
}

#[derive(Debug)]
struct Action<'a> {
    emitter: &'a str,
    primary_wasm: bool,
    name: &'a str,
    attrs: HashMap<&'a str, &'a str>,
    position: i64,
    // Duplicate identity keys make attribution ambiguous, even though the
    // economic amount parser may independently accept this receipt.
    ambiguous_actor: bool,
}
impl<'a> Action<'a> {
    fn actor(&self, key: &str, config: &Config) -> Option<&'a str> {
        if self.ambiguous_actor {
            return None;
        }
        let actor = *self.attrs.get(key)?;
        if !valid_actor(actor)
            || config.router_address.as_deref() == Some(actor)
            || actor == config.factory_address
            || config.wrap_mapper_address.as_deref() == Some(actor)
            || config.ust1_window_address.as_deref() == Some(actor)
            || config.community_token_launcher.as_deref() == Some(actor)
        {
            return None;
        }
        Some(actor)
    }
    fn swap_actor(&self, config: &Config) -> Option<&'a str> {
        if self.attrs.get("sender").copied() == config.router_address.as_deref() {
            self.actor("trader", config)
        } else {
            self.actor("sender", config)
        }
    }
}

fn actions(tx: &TxResponse) -> Vec<Action<'_>> {
    if tx.code != 0 {
        return vec![];
    }
    let events: Vec<_> = match &tx.logs {
        Some(logs) => logs.iter().flat_map(|l| &l.events).collect(),
        None => tx.events.iter().flatten().collect(),
    };
    let mut out = vec![];
    let mut position = 0;
    for event in events {
        if !matches!(event.event_type.as_str(), "wasm" | "wasm-wasm") {
            continue;
        }
        for (index, attr) in event.attributes.iter().enumerate() {
            if attr.key != "action" {
                continue;
            }
            let current = position;
            position += 1;
            let Some(emitter) = wasm_contract_addr_before(&event.attributes, index) else {
                continue;
            };
            let end = event.attributes[index + 1..]
                .iter()
                .position(|a| matches!(a.key.as_str(), "action" | "_contract_address"))
                .map_or(event.attributes.len(), |n| index + 1 + n);
            let ambiguous_actor = ["sender", "trader", "owner", "wallet", "depositor"]
                .iter()
                .any(|key| {
                    event.attributes[index + 1..end]
                        .iter()
                        .filter(|a| a.key == *key)
                        .count()
                        > 1
                });
            out.push(Action {
                emitter,
                primary_wasm: event.event_type == "wasm",
                name: &attr.value,
                attrs: wasm_kv_map_after_action(&event.attributes, index),
                position: current,
                ambiguous_actor,
            });
        }
    }
    out
}

#[derive(FromRow)]
struct Fee {
    id: i64,
    source: String,
    ordinal: i64,
    pair_address: Option<String>,
}

fn unique<'a>(mut candidates: impl Iterator<Item = &'a Action<'a>>) -> Option<&'a Action<'a>> {
    let first = candidates.next()?;
    candidates.next().is_none().then_some(first)
}

fn swap_action<'a>(
    all: &'a [Action<'a>],
    swaps: &[super::parser::ParsedSwap],
    pair: &str,
    index: i32,
) -> Option<&'a Action<'a>> {
    let swap = swaps
        .iter()
        .find(|s| s.pair_address == pair && s.swap_index == index)?;
    unique(all.iter().filter(|a| {
        a.primary_wasm
            && a.name == "swap"
            && a.emitter == pair
            && a.attrs.get("sender").copied() == Some(swap.sender.as_str())
            && a.attrs.get("offer_asset").copied() == Some(swap.offer_asset.as_str())
            && a.attrs.get("ask_asset").copied() == Some(swap.ask_asset.as_str())
            && a.attrs
                .get("offer_amount")
                .and_then(|v| v.parse::<bigdecimal::BigDecimal>().ok())
                .as_ref()
                == Some(&swap.offer_amount)
            && a.attrs
                .get("return_amount")
                .and_then(|v| v.parse::<bigdecimal::BigDecimal>().ok())
                .as_ref()
                == Some(&swap.return_amount)
    }))
}

fn resolve<'a>(
    fee: &Fee,
    all: &'a [Action<'a>],
    config: &Config,
    tx: &TxResponse,
) -> Option<(&'a str, i64)> {
    let action = match fee.source.as_str() {
        "wrap" | "unwrap" => {
            let mapper = config.wrap_mapper_address.as_deref()?;
            let action = all
                .iter()
                .filter(|a| {
                    a.emitter == mapper && matches!(a.name, "wrap" | "notify_deposit" | "unwrap")
                })
                .nth(usize::try_from(fee.ordinal).ok()?)?;
            if (fee.source == "unwrap") != (action.name == "unwrap") {
                return None;
            }
            action
        }
        "ust1_mint" | "ust1_redeem" => {
            let window = config.ust1_window_address.as_deref()?;
            let action = all
                .iter()
                .filter(|a| a.emitter == window && matches!(a.name, "deposit" | "withdraw"))
                .nth(usize::try_from(fee.ordinal).ok()?)?;
            if (fee.source == "ust1_redeem") != (action.name == "withdraw") {
                return None;
            }
            action
        }
        "pair_creation" => all
            .iter()
            .filter(|a| a.emitter == config.factory_address && a.name == "create_pair")
            .nth(usize::try_from(fee.ordinal).ok()?)?,
        // The fee row already passed invoice emitter attestation. Resolve the
        // same source-local ordinal, reserving invalid/zero candidates as ingest does.
        "sku_unlock" | "settings_fee" => {
            let launcher = config.community_token_launcher.as_deref()?;
            all.iter()
                .filter(|a| match (fee.source.as_str(), a.name) {
                    ("sku_unlock", "create_token") => a.emitter == launcher,
                    ("sku_unlock", "enable_feature") | ("settings_fee", "update_settings") => {
                        a.emitter != launcher
                    }
                    _ => false,
                })
                .nth(usize::try_from(fee.ordinal).ok()?)?
        }
        _ => return None,
    };
    if matches!(
        fee.source.as_str(),
        "pair_creation" | "sku_unlock" | "settings_fee"
    ) {
        let candidates = super::invoice_fees::parse_invoice_fees(
            tx,
            &config.factory_address,
            config.community_token_launcher.as_deref().unwrap_or(""),
            &config.hub_ust1_address,
        );
        let candidate = candidates
            .iter()
            .find(|c| c.fee.source.as_str() == fee.source && c.fee.ordinal == fee.ordinal)?;
        if candidate.emitter != action.emitter {
            return None;
        }
    }
    let actor = if fee.source == "swap_amm" {
        action.swap_actor(config)
    } else if fee.source == "limit_place" {
        action.actor("owner", config)
    } else if fee.source == "wrap" && action.name == "notify_deposit" {
        // The pinned mapper authenticates treasury callbacks and emits the
        // actual native depositor; callback sender is not the fee payer.
        action.actor("depositor", config)
    } else {
        action.actor("sender", config)
    }?;
    Some((actor, action.position))
}

async fn registration_at(
    pool: &PgPool,
    lcd: &LcdClient,
    registry: &str,
    actor: &str,
    height: i64,
    tx_index: i64,
    action_index: i64,
    budget: &mut AttributionBudget,
) -> Result<Option<Registration>, sqlx::Error> {
    // Indexed changes in this block dominate H-1, including deregistration.
    let event = sqlx::query_as::<_, (bool, i16, bool)>(
        "SELECT registered,tier_id,valid_state FROM fee_registry_events WHERE registry=$1 AND (actor=$2 OR actor IS NULL) AND block_height=$3 AND (tx_index,action_index)<=($4,$5) ORDER BY tx_index DESC,action_index DESC LIMIT 1")
        .bind(registry).bind(actor).bind(height).bind(tx_index).bind(action_index).fetch_optional(pool).await?;
    if let Some((registered, tier_id, valid)) = event {
        return Ok(valid.then_some(Registration {
            registered,
            tier_id,
        }));
    }
    if height <= 1 {
        return Ok(None);
    }
    let prior = height - 1;
    let snapshot = sqlx::query_as::<_, (bool,i16)>("SELECT registered,tier_id FROM fee_registry_snapshots WHERE registry=$1 AND actor=$2 AND block_height=$3")
        .bind(registry).bind(actor).bind(prior).fetch_optional(pool).await?;
    if let Some((registered, tier_id)) = snapshot {
        return Ok(Some(Registration {
            registered,
            tier_id,
        }));
    }
    let deadline = *budget
        .deadline
        .get_or_insert_with(|| tokio::time::Instant::now() + Duration::from_secs(2));
    if budget.remaining_reads == 0 || tokio::time::Instant::now() >= deadline {
        return Ok(None);
    }
    budget.remaining_reads -= 1;
    let owned_lcd = lcd.with_observation_deadline(deadline);
    let Ok(Ok(value)) = tokio::time::timeout_at(
        deadline,
        owned_lcd.query_contract_at_height_bounded::<Value>(
            registry,
            &json!({"get_registration":{"trader":actor}}),
            prior,
            4096,
        ),
    )
    .await
    else {
        return Ok(None);
    };
    if tokio::time::Instant::now() >= deadline {
        return Ok(None);
    }
    let Some(registration) = parse_registration(&value) else {
        return Ok(None);
    };
    sqlx::query("INSERT INTO fee_registry_snapshots(registry,actor,block_height,registered,tier_id) VALUES($1,$2,$3,$4,$5) ON CONFLICT DO NOTHING")
        .bind(registry).bind(actor).bind(prior).bind(registration.registered).bind(registration.tier_id).execute(pool).await?;
    Ok(Some(registration))
}

/// Called after this successful transaction's authenticated economic rows exist.
/// Attribution is insert-only; replay never reclassifies a historical fee.
pub async fn attribute_transaction(
    pool: &PgPool,
    lcd: &LcdClient,
    config: &Config,
    tx: &TxResponse,
    height: i64,
    tx_index: i64,
    budget: &mut AttributionBudget,
) -> Result<(), sqlx::Error> {
    if tx.code != 0 {
        return Ok(());
    }
    // One marker decision per block owner. A 67 block cannot start touching
    // cohort tables midway through an online migration; its fees remain unknown.
    let enabled = match budget.schema_ready {
        Some(enabled) => enabled,
        None => {
            let enabled = crate::database_security::cohorts_ready(pool).await?;
            budget.schema_ready = Some(enabled);
            enabled
        }
    };
    if !enabled {
        return Ok(());
    }
    let all = actions(tx);
    let parsed_swaps = super::parser::parse_swaps(tx);
    let registry = config
        .fee_discount_address
        .as_deref()
        .filter(|a| valid_actor(a));
    if let Some(registry) = registry {
        for action in all.iter().filter(|a| a.emitter == registry) {
            let state = match action.name {
                "register" | "register_wallet" => action
                    .attrs
                    .get("tier_id")
                    .and_then(|v| v.parse::<i16>().ok())
                    .filter(|n| (0..=255).contains(n))
                    .map(|tier_id| Registration {
                        registered: true,
                        tier_id,
                    }),
                "deregister" | "deregister_wallet" => Some(Registration {
                    registered: false,
                    tier_id: 0,
                }),
                _ => None,
            };
            if action.attrs.contains_key("skipped") {
                continue;
            }
            if !matches!(
                action.name,
                "register" | "register_wallet" | "deregister" | "deregister_wallet"
            ) {
                continue;
            }
            let actor = action
                .attrs
                .get("wallet")
                .copied()
                .filter(|v| valid_actor(v) && !action.ambiguous_actor);
            let valid = state.is_some() && actor.is_some();
            let state = state.unwrap_or(Registration {
                registered: false,
                tier_id: 0,
            });
            // A malformed pinned transition invalidates the affected wallet.
            // Without a trustworthy wallet, it is a block-wide unknown barrier.
            sqlx::query("INSERT INTO fee_registry_events(registry,block_height,tx_index,action_index,actor,registered,tier_id,valid_state) VALUES($1,$2,$3,$4,$5,$6,$7,$8) ON CONFLICT DO NOTHING")
                .bind(registry).bind(height).bind(tx_index).bind(action.position).bind(actor).bind(state.registered).bind(state.tier_id).bind(valid).execute(pool).await?;
        }
    }
    let fees = sqlx::query_as::<_, Fee>("SELECT e.id,e.source,e.ordinal,p.contract_address AS pair_address FROM protocol_fee_events e LEFT JOIN pairs p ON p.id=e.pair_id LEFT JOIN protocol_fee_attributions a ON a.fee_event_id=e.id WHERE e.tx_hash=$1 AND e.block_height=$2 AND a.fee_event_id IS NULL ORDER BY e.id")
        .bind(&tx.txhash).bind(height).fetch_all(pool).await?;
    for fee in fees {
        let resolved = if fee.source == "swap_amm" {
            fee.pair_address
                .as_deref()
                .and_then(|pair| {
                    i32::try_from(fee.ordinal)
                        .ok()
                        .and_then(|index| swap_action(&all, &parsed_swaps, pair, index))
                })
                .and_then(|a| a.swap_actor(config).map(|actor| (actor, a.position)))
        } else if fee.source == "book_take" {
            // Parent swap is the taker; makers are intentionally never selected.
            let parents = sqlx::query_as::<_, (String,i32)>("SELECT p.contract_address,se.swap_index FROM limit_order_fills f JOIN swap_events se ON se.id=f.swap_event_id JOIN pairs p ON p.id=f.pair_id WHERE f.tx_hash=$1 AND f.order_id=$2")
                .bind(&tx.txhash).bind(fee.ordinal).fetch_all(pool).await?;
            if parents.len() == 1 {
                let (pair, index) = &parents[0];
                swap_action(&all, &parsed_swaps, pair, *index)
                    .and_then(|a| a.swap_actor(config).map(|actor| (actor, a.position)))
            } else {
                None
            }
        } else if fee.source == "limit_place" {
            let placements = sqlx::query_as::<_, (String,)>("SELECT p.contract_address FROM limit_order_placements l JOIN pairs p ON p.id=l.pair_id WHERE l.tx_hash=$1 AND l.order_id=$2")
                .bind(&tx.txhash).bind(fee.ordinal).fetch_all(pool).await?;
            if placements.len() == 1 {
                unique(all.iter().filter(|a| {
                    a.emitter == placements[0].0
                        && a.name == "place_limit_order"
                        && a.attrs.get("order_id").and_then(|v| v.parse::<i64>().ok())
                            == Some(fee.ordinal)
                }))
                .and_then(|a| a.actor("owner", config).map(|actor| (actor, a.position)))
            } else {
                None
            }
        } else {
            resolve(&fee, &all, config, tx)
        };
        let actor = resolved.map(|(actor, _)| actor);
        let state = match (resolved, registry) {
            (Some((actor, position)), Some(registry)) => {
                registration_at(
                    pool, lcd, registry, actor, height, tx_index, position, budget,
                )
                .await?
            }
            _ => None,
        };
        sqlx::query("INSERT INTO protocol_fee_attributions(fee_event_id,actor,cohort) VALUES($1,$2,$3) ON CONFLICT DO NOTHING")
            .bind(fee.id).bind(actor).bind(state.map_or("unknown",Registration::cohort)).execute(pool).await?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use bech32::ToBase32;
    #[test]
    fn issue1211_registry_membership_is_explicit_and_typed() {
        assert_eq!(
            parse_registration(&json!({"registered":false,"tier_id":0}))
                .unwrap()
                .cohort(),
            "retail"
        );
        assert_eq!(
            parse_registration(&json!({"registered":true,"tier_id":0}))
                .unwrap()
                .cohort(),
            "mm"
        );
        for tier in [1, 5, 9, 255] {
            assert_eq!(
                parse_registration(&json!({"registered":true,"tier_id":tier}))
                    .unwrap()
                    .cohort(),
                "retail"
            );
        }
        for invalid in [
            json!({}),
            json!({"registered":true}),
            json!({"registered":true,"tier_id":256}),
            json!({"registered":"false"}),
        ] {
            assert!(parse_registration(&invalid).is_none());
        }
    }
    #[test]
    fn issue1211_actor_requires_checksum_and_terra_prefix() {
        let valid =
            bech32::encode("terra", [7u8; 20].to_base32(), bech32::Variant::Bech32).unwrap();
        assert!(valid_actor(&valid));
        assert!(!valid_actor("terra1not-a-real-wallet"));
        assert!(!valid_actor(
            &bech32::encode("cosmos", [7u8; 20].to_base32(), bech32::Variant::Bech32).unwrap()
        ));
        assert!(!valid_actor(&format!("{valid} ")));
        assert!(!valid_actor(&"t".repeat(4096)));
    }
}
