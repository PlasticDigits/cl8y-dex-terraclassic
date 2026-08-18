use std::collections::HashSet;
use std::time::{Duration, Instant};

use bigdecimal::BigDecimal;
use sqlx::PgPool;

use crate::db::queries::traders::{self, TraderRow};
use crate::lcd::{Attribute, LcdClient, TxResponse};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

/// Default daily reconciliation when `TIER_SYNC_RECONCILE_INTERVAL` is unset (GitLab #364).
pub const DEFAULT_TIER_RECONCILE_INTERVAL_SECS: u64 = 86_400;

const FEE_DISCOUNT_REGISTRY_ACTIONS: &[&str] = &[
    "register",
    "register_wallet",
    "deregister",
    "deregister_wallet",
];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ParsedRegistryTierEvent {
    Register { wallet: String, tier_id: i16 },
    Deregister { wallet: String },
}

pub async fn update_trader_on_swap(
    pool: &PgPool,
    lcd: &LcdClient,
    fee_discount_addr: Option<&str>,
    sender: &str,
    trade_volume: &BigDecimal,
    trade_volume_usd: Option<&BigDecimal>,
) -> Result<(), BoxError> {
    let is_new = traders::upsert_trader(pool, sender, trade_volume, trade_volume_usd).await?;
    if is_new {
        if let Some(addr) = fee_discount_addr.filter(|a| !a.is_empty()) {
            hydrate_trader_tier_from_lcd(pool, lcd, addr, sender).await?;
        }
    }
    Ok(())
}

pub async fn run_tier_reconcile_loop(
    pool: PgPool,
    lcd: LcdClient,
    fee_discount_addr: Option<String>,
    reconcile_interval_secs: u64,
) {
    let addr = match fee_discount_addr {
        Some(a) if !a.is_empty() => a,
        _ => {
            tracing::info!("No fee_discount_address configured, tier reconcile disabled");
            return;
        }
    };

    let interval = Duration::from_secs(reconcile_interval_secs.max(60));
    tracing::info!(
        tier_sync_reconcile_interval_secs = reconcile_interval_secs,
        "Trader tier reconcile loop started (event-driven updates are primary; reconcile corrects drift)"
    );

    loop {
        tokio::time::sleep(interval).await;

        let started = Instant::now();
        tracing::info!("Running trader tier reconciliation...");
        match reconcile_trader_tiers(&pool, &lcd, &addr).await {
            Ok(count) => {
                let elapsed = started.elapsed().as_secs();
                tracing::info!(
                    traders_reconciled = count,
                    tier_sync_lag_seconds = elapsed,
                    "Trader tier reconciliation complete"
                );
            }
            Err(e) => tracing::error!("Tier reconciliation failed: {}", e),
        }
    }
}

/// Full-table LCD reconciliation for drift correction (GitLab #364).
pub async fn reconcile_trader_tiers(
    pool: &PgPool,
    lcd: &LcdClient,
    fee_discount_addr: &str,
) -> Result<usize, BoxError> {
    let all_traders: Vec<TraderRow> = sqlx::query_as("SELECT * FROM traders")
        .fetch_all(pool)
        .await?;

    tracing::info!("Reconciling tiers for {} traders", all_traders.len());

    let mut reconciled = 0usize;
    for trader in &all_traders {
        match hydrate_trader_tier_from_lcd(pool, lcd, fee_discount_addr, &trader.address).await {
            Ok(()) => reconciled += 1,
            Err(e) => {
                tracing::warn!(
                    "Failed to reconcile registration for {}: {}",
                    trader.address,
                    e
                );
            }
        }
    }

    Ok(reconciled)
}

pub async fn apply_registry_tier_events(
    pool: &PgPool,
    events: &[ParsedRegistryTierEvent],
) -> Result<(), BoxError> {
    for event in events {
        match event {
            ParsedRegistryTierEvent::Register { wallet, tier_id } => {
                let tier_name = tier_display_name(*tier_id);
                traders::upsert_trader_tier(pool, wallet, *tier_id, &tier_name, true).await?;
                tracing::info!(
                    wallet = %wallet,
                    tier_id = tier_id,
                    "Fee-discount registration indexed (event-driven)"
                );
            }
            ParsedRegistryTierEvent::Deregister { wallet } => {
                traders::upsert_trader_tier(pool, wallet, 0, "None", false).await?;
                tracing::info!(
                    wallet = %wallet,
                    "Fee-discount deregistration indexed (event-driven)"
                );
            }
        }
    }
    Ok(())
}

pub fn parse_fee_discount_registry_events(
    tx: &TxResponse,
    fee_discount_addr: &str,
) -> Vec<ParsedRegistryTierEvent> {
    let mut events = Vec::new();
    let mut seen_wallets: HashSet<String> = HashSet::new();

    for event in collect_wasm_events(tx) {
        let attrs = &event.attributes;
        for (i, attr) in attrs.iter().enumerate() {
            if attr.key != "action" {
                continue;
            }
            if !FEE_DISCOUNT_REGISTRY_ACTIONS.contains(&attr.value.as_str()) {
                continue;
            }
            let Some(contract) = wasm_contract_addr_before(attrs, i)
                .or_else(|| wasm_contract_addr(attrs))
            else {
                continue;
            };
            if contract != fee_discount_addr {
                continue;
            }
            if segment_kv_map_after_action(attrs, i).contains_key("skipped") {
                tracing::debug!(
                    action = %attr.value,
                    "Skipping fee-discount registry event with skipped attribute"
                );
                continue;
            }
            let Some(wallet) = segment_wallet(attrs, i) else {
                tracing::warn!(
                    action = %attr.value,
                    "Malformed fee-discount registry event: missing wallet"
                );
                continue;
            };
            if !seen_wallets.insert(wallet.clone()) {
                // Last occurrence in tx attribute stream wins; drop prior entry for this wallet.
                events.retain(|e| event_wallet(e) != wallet);
            }
            match attr.value.as_str() {
                "register" | "register_wallet" => {
                    let Some(tier_id) = segment_tier_id(attrs, i) else {
                        tracing::warn!(
                            wallet = %wallet,
                            action = %attr.value,
                            "Malformed fee-discount registry event: invalid tier_id"
                        );
                        continue;
                    };
                    events.push(ParsedRegistryTierEvent::Register { wallet, tier_id });
                }
                "deregister" | "deregister_wallet" => {
                    events.push(ParsedRegistryTierEvent::Deregister { wallet });
                }
                _ => {}
            }
        }
    }

    events
}

fn event_wallet(event: &ParsedRegistryTierEvent) -> &str {
    match event {
        ParsedRegistryTierEvent::Register { wallet, .. }
        | ParsedRegistryTierEvent::Deregister { wallet } => wallet,
    }
}

/// Query LCD once and upsert tier fields.
pub async fn hydrate_trader_tier_from_lcd(
    pool: &PgPool,
    lcd: &LcdClient,
    fee_discount_addr: &str,
    address: &str,
) -> Result<(), BoxError> {
    let val = lcd
        .query_contract::<serde_json::Value>(
            fee_discount_addr,
            &serde_json::json!({"get_registration": {"trader": address}}),
        )
        .await?;

    let (tier_id, tier_name, registered) = parse_registration_lcd(&val);
    traders::upsert_trader_tier(pool, address, tier_id, &tier_name, registered).await?;
    Ok(())
}

pub(crate) fn parse_registration_lcd(val: &serde_json::Value) -> (i16, String, bool) {
    let registered = val
        .get("registered")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if !registered {
        return (0, "None".to_string(), false);
    }

    let tier_id = val
        .get("tier_id")
        .and_then(|v| v.as_u64())
        .unwrap_or(0) as i16;

    let tier_name = val
        .get("tier_name")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| tier_display_name(tier_id));

    (tier_id, tier_name, true)
}

pub(crate) fn tier_display_name(tier_id: i16) -> String {
    match tier_id {
        0 => "Tier 0".to_string(),
        255 => "Tier 255".to_string(),
        1..=9 => format!("Tier {}", tier_id),
        _ => "None".to_string(),
    }
}

fn collect_wasm_events(tx: &TxResponse) -> Vec<crate::lcd::Event> {
    if let Some(logs) = &tx.logs {
        logs.iter()
            .flat_map(|l| l.events.iter())
            .filter(|e| e.event_type == "wasm" || e.event_type == "wasm-wasm")
            .cloned()
            .collect()
    } else if let Some(evts) = &tx.events {
        evts.iter()
            .filter(|e| e.event_type == "wasm" || e.event_type == "wasm-wasm")
            .cloned()
            .collect()
    } else {
        Vec::new()
    }
}

fn wasm_attr_last<'a>(attributes: &'a [Attribute], key: &str) -> Option<&'a str> {
    attributes
        .iter()
        .rev()
        .find(|a| a.key == key)
        .map(|a| a.value.as_str())
}

fn wasm_contract_addr(attributes: &[Attribute]) -> Option<&str> {
    wasm_attr_last(attributes, "_contract_address")
}

fn wasm_contract_addr_before(attrs: &[Attribute], idx: usize) -> Option<&str> {
    attrs[..idx]
        .iter()
        .rev()
        .find(|a| a.key == "_contract_address")
        .map(|a| a.value.as_str())
}

fn segment_kv_map_after_action(
    attrs: &[Attribute],
    action_pos: usize,
) -> std::collections::HashMap<&str, &str> {
    let mut m = std::collections::HashMap::new();
    let mut i = action_pos.saturating_add(1);
    while i < attrs.len() {
        let k = attrs[i].key.as_str();
        if k == "action" || k == "_contract_address" {
            break;
        }
        m.insert(k, attrs[i].value.as_str());
        i += 1;
    }
    m
}

fn segment_wallet(attrs: &[Attribute], action_pos: usize) -> Option<String> {
    segment_kv_map_after_action(attrs, action_pos)
        .get("wallet")
        .map(|s| s.to_string())
}

fn segment_tier_id(attrs: &[Attribute], action_pos: usize) -> Option<i16> {
    let raw = segment_kv_map_after_action(attrs, action_pos).get("tier_id").copied()?;
    let parsed = raw.parse::<i16>().ok()?;
    if parsed < 0 || parsed > 255 {
        return None;
    }
    Some(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lcd::{Event, TxLog};

    fn wasm_tx(attrs: Vec<(&str, &str)>) -> TxResponse {
        let attributes: Vec<Attribute> = attrs
            .into_iter()
            .map(|(k, v)| Attribute {
                key: k.to_string(),
                value: v.to_string(),
            })
            .collect();
        TxResponse {
            height: "1".into(),
            txhash: "HASH".into(),
            logs: Some(vec![TxLog {
                events: vec![Event {
                    event_type: "wasm".into(),
                    attributes,
                }],
            }]),
            timestamp: None,
            events: None,
        }
    }

    const FEE_ADDR: &str = "terra1feediscountcontractaddress000000";

    #[test]
    fn parse_register_event() {
        let tx = wasm_tx(vec![
            ("_contract_address", FEE_ADDR),
            ("action", "register"),
            ("wallet", "terra1traderabc"),
            ("tier_id", "5"),
        ]);
        let events = parse_fee_discount_registry_events(&tx, FEE_ADDR);
        assert_eq!(
            events,
            vec![ParsedRegistryTierEvent::Register {
                wallet: "terra1traderabc".into(),
                tier_id: 5,
            }]
        );
    }

    #[test]
    fn parse_deregister_wallet_skipped() {
        let tx = wasm_tx(vec![
            ("_contract_address", FEE_ADDR),
            ("action", "deregister_wallet"),
            ("skipped", "epoch mismatch — registration was renewed"),
            ("wallet", "terra1traderabc"),
        ]);
        assert!(parse_fee_discount_registry_events(&tx, FEE_ADDR).is_empty());
    }

    #[test]
    fn parse_malformed_register_missing_tier_id() {
        let tx = wasm_tx(vec![
            ("_contract_address", FEE_ADDR),
            ("action", "register"),
            ("wallet", "terra1traderabc"),
        ]);
        assert!(parse_fee_discount_registry_events(&tx, FEE_ADDR).is_empty());
    }

    #[test]
    fn parse_wrong_contract_ignored() {
        let tx = wasm_tx(vec![
            ("_contract_address", "terra1other"),
            ("action", "register"),
            ("wallet", "terra1traderabc"),
            ("tier_id", "3"),
        ]);
        assert!(parse_fee_discount_registry_events(&tx, FEE_ADDR).is_empty());
    }

    #[test]
    fn parse_last_register_wins_per_wallet_in_tx() {
        let tx = wasm_tx(vec![
            ("_contract_address", FEE_ADDR),
            ("action", "register"),
            ("wallet", "terra1traderabc"),
            ("tier_id", "2"),
            ("action", "register"),
            ("wallet", "terra1traderabc"),
            ("tier_id", "4"),
        ]);
        let events = parse_fee_discount_registry_events(&tx, FEE_ADDR);
        assert_eq!(
            events,
            vec![ParsedRegistryTierEvent::Register {
                wallet: "terra1traderabc".into(),
                tier_id: 4,
            }]
        );
    }

    #[test]
    fn parse_registration_lcd_shape() {
        let val = serde_json::json!({
            "registered": true,
            "tier_id": 5,
            "tier": { "discount_bps": 5000 }
        });
        assert_eq!(
            parse_registration_lcd(&val),
            (5, "Tier 5".to_string(), true)
        );
        let unreg = serde_json::json!({ "registered": false });
        assert_eq!(parse_registration_lcd(&unreg), (0, "None".to_string(), false));
    }
}
