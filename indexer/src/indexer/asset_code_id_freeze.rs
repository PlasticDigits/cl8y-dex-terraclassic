//! F6 listed-CW20 `code_id` freeze cache (GitLab #585).
//!
//! Pair write paths fail closed on drift / de-whitelist / guard errors. Queries
//! (`Simulation` / `HybridSimulation`) stay ungated, so a frozen pair can still
//! quote. This module:
//!
//! 1. **Evaluates** pin vs live `ContractInfo.code_id` and factory whitelist.
//! 2. **Caches** frozen pair addresses (process-local; LCD probe is off the
//!    request path).
//! 3. Lets `route/solve` **exclude** frozen hops so they are not treated as
//!    executable. Pair list/detail **flag** them (`code_id_frozen`).
//!
//! LCD / `GetAssetCodeIds` failures are **fail-open for routing** (keep last
//! known, or tradable if never seen). On-chain execute still fail-closes.
//! Pre-1.15.0 pairs (`unknown variant` / unpinned) are **not** frozen.

use std::collections::HashSet;
use std::sync::{Mutex, OnceLock};
use std::time::Duration;

use sqlx::PgPool;
use tokio_util::sync::CancellationToken;

use crate::db::queries::{assets, pairs as db_pairs};
use crate::lcd::{LcdClient, LcdError};

/// Background LCD probe interval. Freeze membership is not on the request path.
pub const CODE_ID_FREEZE_PROBE_INTERVAL: Duration = Duration::from_secs(60);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodeIdFreezeVerdict {
    /// Pins match live ids and both live ids are factory-whitelisted.
    Tradable,
    /// Live `code_id` ≠ pin, or live id is not factory-whitelisted.
    Frozen,
    /// LCD / query failed — do not change cached membership.
    Unknown,
}

/// Pure pin/whitelist check (no LCD). Both legs must pass.
pub fn evaluate_live_pins(
    pin0: u64,
    pin1: u64,
    live0: u64,
    live1: u64,
    whitelisted0: bool,
    whitelisted1: bool,
) -> CodeIdFreezeVerdict {
    if live0 != pin0 || live1 != pin1 || !whitelisted0 || !whitelisted1 {
        CodeIdFreezeVerdict::Frozen
    } else {
        CodeIdFreezeVerdict::Tradable
    }
}

fn freeze_set() -> &'static Mutex<HashSet<String>> {
    static SET: OnceLock<Mutex<HashSet<String>>> = OnceLock::new();
    SET.get_or_init(|| Mutex::new(HashSet::new()))
}

/// True when the pair is known F6-frozen (execute blocked). Fail-open if the lock is poisoned.
pub fn is_pair_code_id_frozen(pair_address: &str) -> bool {
    freeze_set()
        .lock()
        .map(|g| g.contains(pair_address))
        .unwrap_or(false)
}

pub fn snapshot_frozen_pair_addresses() -> HashSet<String> {
    freeze_set()
        .lock()
        .map(|g| g.clone())
        .unwrap_or_default()
}

/// Replace the freeze set (integration tests + probe loop retain).
pub fn replace_frozen_pair_addresses(next: HashSet<String>) {
    if let Ok(mut g) = freeze_set().lock() {
        *g = next;
    }
}

pub fn apply_probe_verdicts(verdicts: &[(String, CodeIdFreezeVerdict)]) {
    let Ok(mut g) = freeze_set().lock() else {
        return;
    };
    for (addr, verdict) in verdicts {
        match verdict {
            CodeIdFreezeVerdict::Frozen => {
                g.insert(addr.clone());
            }
            CodeIdFreezeVerdict::Tradable => {
                g.remove(addr);
            }
            CodeIdFreezeVerdict::Unknown => {}
        }
    }
}

fn is_pre_f6_query_reject(err: &LcdError) -> bool {
    let s = err.to_string().to_ascii_lowercase();
    s.contains("unknown variant")
        || s.contains("error parsing into type")
        || s.contains("pins are missing")
        || s.contains("assetcodeidunpinned")
}

#[derive(Debug, serde::Deserialize)]
struct AssetCodeIdsLcd {
    code_ids: [u64; 2],
}

#[derive(Debug, serde::Deserialize)]
struct CodeIdWhitelistedLcd {
    whitelisted: bool,
}

async fn probe_one_pair(
    lcd: &LcdClient,
    factory: &str,
    pair_addr: &str,
    token0: Option<&str>,
    token1: Option<&str>,
) -> CodeIdFreezeVerdict {
    let Some(t0) = token0.filter(|a| a.starts_with("terra1")) else {
        return CodeIdFreezeVerdict::Tradable;
    };
    let Some(t1) = token1.filter(|a| a.starts_with("terra1")) else {
        return CodeIdFreezeVerdict::Tradable;
    };

    let pins = match lcd
        .query_contract::<AssetCodeIdsLcd>(pair_addr, &serde_json::json!({ "get_asset_code_ids": {} }))
        .await
    {
        Ok(r) => r.code_ids,
        Err(e) if is_pre_f6_query_reject(&e) => return CodeIdFreezeVerdict::Tradable,
        Err(_) => return CodeIdFreezeVerdict::Unknown,
    };

    let live0 = match lcd.get_contract_code_id(t0).await {
        Ok(id) => id,
        Err(_) => return CodeIdFreezeVerdict::Unknown,
    };
    let live1 = match lcd.get_contract_code_id(t1).await {
        Ok(id) => id,
        Err(_) => return CodeIdFreezeVerdict::Unknown,
    };

    let wl0 = match query_whitelisted(lcd, factory, live0).await {
        Ok(v) => v,
        Err(_) => return CodeIdFreezeVerdict::Unknown,
    };
    let wl1 = match query_whitelisted(lcd, factory, live1).await {
        Ok(v) => v,
        Err(_) => return CodeIdFreezeVerdict::Unknown,
    };

    evaluate_live_pins(pins[0], pins[1], live0, live1, wl0, wl1)
}

async fn query_whitelisted(lcd: &LcdClient, factory: &str, code_id: u64) -> Result<bool, LcdError> {
    let resp: CodeIdWhitelistedLcd = lcd
        .query_contract(
            factory,
            &serde_json::json!({ "is_code_id_whitelisted": { "code_id": code_id } }),
        )
        .await?;
    Ok(resp.whitelisted)
}

/// Refresh freeze membership from LCD. Request-path code must not call this.
pub async fn refresh_frozen_pairs(pool: &PgPool, lcd: &LcdClient, factory: &str) {
    if factory.trim().is_empty() {
        return;
    }
    let pairs = match db_pairs::get_all_pairs(pool).await {
        Ok(p) => p,
        Err(e) => {
            tracing::warn!(error = %e, "F6 freeze probe: failed to list pairs");
            return;
        }
    };
    let asset_map = match assets::get_all_assets(pool).await {
        Ok(rows) => {
            let mut m = std::collections::HashMap::new();
            for a in rows {
                m.insert(a.id, a);
            }
            m
        }
        Err(e) => {
            tracing::warn!(error = %e, "F6 freeze probe: failed to list assets");
            return;
        }
    };

    let mut known = HashSet::new();
    let mut verdicts = Vec::with_capacity(pairs.len());
    for p in &pairs {
        known.insert(p.contract_address.clone());
        let t0 = asset_map
            .get(&p.asset_0_id)
            .and_then(|a| a.contract_address.as_deref());
        let t1 = asset_map
            .get(&p.asset_1_id)
            .and_then(|a| a.contract_address.as_deref());
        let v = probe_one_pair(lcd, factory, &p.contract_address, t0, t1).await;
        if v == CodeIdFreezeVerdict::Frozen {
            tracing::warn!(
                pair = %p.contract_address,
                "F6 freeze: pair excluded from route/solve (code_id drift or not whitelisted)"
            );
        }
        verdicts.push((p.contract_address.clone(), v));
    }
    apply_probe_verdicts(&verdicts);
    if let Ok(mut g) = freeze_set().lock() {
        g.retain(|addr| known.contains(addr));
    }
}

pub async fn run_code_id_freeze_probe_loop(
    pool: PgPool,
    lcd: LcdClient,
    factory: String,
    cancel: CancellationToken,
) {
    loop {
        if cancel.is_cancelled() {
            return;
        }
        refresh_frozen_pairs(&pool, &lcd, &factory).await;
        tokio::select! {
            _ = cancel.cancelled() => return,
            _ = tokio::time::sleep(CODE_ID_FREEZE_PROBE_INTERVAL) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matching_pins_and_whitelist_are_tradable() {
        assert_eq!(
            evaluate_live_pins(10184, 6036, 10184, 6036, true, true),
            CodeIdFreezeVerdict::Tradable
        );
    }

    #[test]
    fn drift_on_either_leg_is_frozen() {
        assert_eq!(
            evaluate_live_pins(10184, 6036, 9999, 6036, true, true),
            CodeIdFreezeVerdict::Frozen
        );
        assert_eq!(
            evaluate_live_pins(10184, 6036, 10184, 1, true, true),
            CodeIdFreezeVerdict::Frozen
        );
    }

    #[test]
    fn dewhitelist_is_frozen_even_when_pins_match() {
        assert_eq!(
            evaluate_live_pins(10184, 6036, 10184, 6036, true, false),
            CodeIdFreezeVerdict::Frozen
        );
        assert_eq!(
            evaluate_live_pins(10184, 6036, 10184, 6036, false, true),
            CodeIdFreezeVerdict::Frozen
        );
    }

    #[test]
    fn freeze_set_roundtrip_and_unknown_keeps_membership() {
        replace_frozen_pair_addresses(HashSet::new());
        apply_probe_verdicts(&[
            ("terra1frozen".into(), CodeIdFreezeVerdict::Frozen),
            ("terra1ok".into(), CodeIdFreezeVerdict::Tradable),
        ]);
        assert!(is_pair_code_id_frozen("terra1frozen"));
        assert!(!is_pair_code_id_frozen("terra1ok"));
        apply_probe_verdicts(&[("terra1frozen".into(), CodeIdFreezeVerdict::Unknown)]);
        assert!(is_pair_code_id_frozen("terra1frozen"));
        apply_probe_verdicts(&[("terra1frozen".into(), CodeIdFreezeVerdict::Tradable)]);
        assert!(!is_pair_code_id_frozen("terra1frozen"));
        replace_frozen_pair_addresses(HashSet::new());
    }

    #[test]
    fn pre_f6_reject_classifier() {
        let err = LcdError::ContractQueryRejected(
            "unknown variant `get_asset_code_ids`".into(),
        );
        assert!(is_pre_f6_query_reject(&err));
        let err = LcdError::ContractQueryRejected(
            "Asset CW20 code_id pins are missing; migrate the pair contract".into(),
        );
        assert!(is_pre_f6_query_reject(&err));
        let err = LcdError::AllEndpointsFailed("timeout".into());
        assert!(!is_pre_f6_query_reject(&err));
    }

    #[test]
    fn probe_interval_is_one_minute() {
        assert_eq!(CODE_ID_FREEZE_PROBE_INTERVAL, Duration::from_secs(60));
    }
}
