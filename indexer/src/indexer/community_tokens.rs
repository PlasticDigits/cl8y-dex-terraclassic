//! Community tax catalog ingest + LCD attestation probe (GitLab #594).
//!
//! Trust LCD `ContractInfo.code_id` / `admin`, not event fields. Attest `launcher_tx`
//! only when the wasm emitter is `COMMUNITY_TOKEN_LAUNCHER`. Adopted tokens (#626) may
//! attest via `GetMigrateOrigin` + CMM admin — never a fake `launcher_tx`.

use std::time::Duration;

use serde::Deserialize;
use serde_json::json;
use tokio_util::sync::CancellationToken;

use crate::config::Config;
use crate::db::queries::community_tokens as db;
use crate::lcd::{Attribute, LcdClient, TxResponse};

pub const COMMUNITY_TAX_PROBE_INTERVAL: Duration = Duration::from_secs(60);

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

#[derive(Debug, Clone)]
pub struct ParsedCommunityEvent {
    pub emitter: String,
    pub action: String,
    pub community_token: Option<String>,
    pub token: Option<String>,
    pub sku: Option<String>,
    pub invoice: Option<String>,
}

pub fn parse_community_event(attrs: &[Attribute]) -> Option<ParsedCommunityEvent> {
    let action = wasm_attr_last(attrs, "action")?;
    match action {
        "create_token_ready" | "enable_feature" | "update_settings" | "mint" | "migrate-adopt" => {}
        _ => return None,
    }
    let emitter = wasm_contract_addr(attrs)?.to_string();
    Some(ParsedCommunityEvent {
        emitter,
        action: action.to_string(),
        community_token: wasm_attr_last(attrs, "community_token").map(str::to_string),
        token: wasm_attr_last(attrs, "token").map(str::to_string),
        sku: wasm_attr_last(attrs, "sku").map(str::to_string),
        invoice: wasm_attr_last(attrs, "invoice").map(str::to_string),
    })
}

pub fn parse_community_events_from_tx(tx: &TxResponse) -> Vec<ParsedCommunityEvent> {
    let events: Vec<&crate::lcd::Event> = if let Some(logs) = &tx.logs {
        logs.iter().flat_map(|l| l.events.iter()).collect()
    } else if let Some(evts) = &tx.events {
        evts.iter().collect()
    } else {
        Vec::new()
    };
    let mut out = Vec::new();
    for ev in events {
        if ev.event_type != "wasm" && ev.event_type != "wasm-wasm" {
            continue;
        }
        if let Some(parsed) = parse_community_event(&ev.attributes) {
            out.push(parsed);
        }
    }
    out
}

pub fn catalog_configured(config: &Config) -> bool {
    config.community_tax_code_id.is_some()
        && config
            .community_token_launcher
            .as_ref()
            .is_some_and(|s| !s.is_empty())
        && config
            .cmm_governance_addr
            .as_ref()
            .is_some_and(|s| !s.is_empty())
}

pub async fn ingest_event(
    pool: &sqlx::PgPool,
    config: &Config,
    ev: &ParsedCommunityEvent,
    txhash: &str,
    height: i64,
) -> Result<(), sqlx::Error> {
    if !catalog_configured(config) {
        return Ok(());
    }
    let launcher = config.community_token_launcher.as_deref().unwrap_or("");

    match ev.action.as_str() {
        "create_token_ready" => {
            if ev.emitter != launcher {
                return Ok(());
            }
            let Some(addr) = ev.community_token.as_deref() else {
                return Ok(());
            };
            db::upsert_from_launcher(pool, addr, &ev.emitter, txhash, height).await?;
            db::insert_event(
                pool,
                addr,
                txhash,
                height,
                "create_token_ready",
                "create",
                None,
                None,
                Some(&json!({ "launcher": ev.emitter })),
            )
            .await?;
        }
        "enable_feature" => {
            let token = ev
                .token
                .as_deref()
                .or(if ev.emitter != launcher {
                    Some(ev.emitter.as_str())
                } else {
                    None
                })
                .unwrap_or("");
            if token.is_empty() {
                return Ok(());
            }
            if ev.emitter != launcher && ev.emitter != token {
                return Ok(());
            }
            if db::get_by_address(pool, token).await?.is_none() {
                return Ok(());
            }
            if ev.emitter == launcher || ev.emitter == token {
                if let Some(sku) = ev.sku.as_deref() {
                    db::merge_feature(pool, token, sku).await?;
                }
                db::insert_event(
                    pool,
                    token,
                    txhash,
                    height,
                    "enable_feature",
                    "sku_unlock",
                    ev.sku.as_deref(),
                    ev.invoice.as_deref(),
                    None,
                )
                .await?;
            }
        }
        "update_settings" => {
            if db::get_by_address(pool, &ev.emitter).await?.is_none() {
                return Ok(());
            }
            db::insert_event(
                pool,
                &ev.emitter,
                txhash,
                height,
                "update_settings",
                "settings_fee",
                None,
                ev.invoice.as_deref(),
                None,
            )
            .await?;
        }
        "migrate-adopt" => {
            let addr = ev
                .community_token
                .as_deref()
                .unwrap_or(ev.emitter.as_str());
            // Do not write launcher_tx — origin is GetMigrateOrigin + CMM admin.
            db::upsert_from_migrate(pool, addr, txhash, height).await?;
            db::insert_event(
                pool,
                addr,
                txhash,
                height,
                "migrate-adopt",
                "adopt",
                None,
                None,
                Some(&json!({ "emitter": ev.emitter })),
            )
            .await?;
        }
        "mint" => {
            if db::get_by_address(pool, &ev.emitter).await?.is_none() {
                return Ok(());
            }
            db::insert_event(
                pool,
                &ev.emitter,
                txhash,
                height,
                "mint",
                "mint",
                None,
                None,
                None,
            )
            .await?;
        }
        _ => {}
    }
    Ok(())
}

#[derive(Debug, Deserialize)]
struct LcdConfig {
    manager: Option<String>,
    treasury: Option<String>,
    buy_bps: Option<u16>,
    sell_bps: Option<u16>,
    transfer_bps: Option<u16>,
}

#[derive(Debug, Deserialize)]
struct LcdFeatures {
    mint_control: Option<bool>,
    transfer_tax: Option<bool>,
    split_router: Option<bool>,
    auto_v2_lp: Option<bool>,
    exemption_directory: Option<bool>,
    variable_rates: Option<bool>,
    launch_guards: Option<bool>,
}

#[derive(Debug, Deserialize)]
struct LcdTokenInfo {
    name: Option<String>,
    symbol: Option<String>,
    decimals: Option<u8>,
}

#[derive(Debug, Deserialize)]
struct LcdLauncherOrigin {
    launcher: Option<String>,
}

#[derive(Debug, Deserialize)]
struct LcdMigrateOrigin {
    source_cw2: Option<String>,
}

/// Honest listed-template cw2 names written by the #626 importer.
pub fn is_allowed_adopt_cw2(name: &str) -> bool {
    matches!(
        name,
        "crates.io:cw20-base" | "crates.io:cw20-mintable" | "crates.io:terraport-token"
    )
}

/// I594-3 + #626: CMM admin + official launcher origin, plus either a real
/// launcher_tx **or** an allowlisted GetMigrateOrigin. Never fake launcher_tx.
pub fn attested_cmm(
    code_id: u64,
    want_code: u64,
    admin: &str,
    cmm: &str,
    launcher_tx_ok: bool,
    origin_launcher: Option<&str>,
    want_launcher: &str,
    migrate_source_cw2: Option<&str>,
) -> bool {
    if code_id != want_code || admin != cmm {
        return false;
    }
    if origin_launcher != Some(want_launcher) {
        return false;
    }
    launcher_tx_ok || migrate_source_cw2.is_some_and(is_allowed_adopt_cw2)
}

pub async fn refresh_one(pool: &sqlx::PgPool, lcd: &LcdClient, config: &Config, addr: &str) {
    let Some(want_code) = config.community_tax_code_id else {
        return;
    };
    let Some(cmm) = config.cmm_governance_addr.as_deref() else {
        return;
    };
    let Some(launcher) = config.community_token_launcher.as_deref() else {
        return;
    };

    let info = match lcd.get_contract_info(addr).await {
        Ok(i) => i,
        Err(e) => {
            tracing::debug!(addr, error = %e, "community tax LCD ContractInfo failed");
            return;
        }
    };
    let Some(code_id) = info.code_id_u64() else {
        return;
    };
    let admin = info.admin.clone().unwrap_or_default();

    let cfg: Option<LcdConfig> = lcd
        .query_contract(addr, &json!({ "get_config": {} }))
        .await
        .ok();
    let feats: Option<LcdFeatures> = lcd
        .query_contract(addr, &json!({ "get_features": {} }))
        .await
        .ok();
    let token_info: Option<LcdTokenInfo> = lcd
        .query_contract(addr, &json!({ "token_info": {} }))
        .await
        .ok();
    let origin: Option<LcdLauncherOrigin> = lcd
        .query_contract(addr, &json!({ "get_launcher_origin": {} }))
        .await
        .ok();
    let migrate_origin: Option<LcdMigrateOrigin> = lcd
        .query_contract(addr, &json!({ "get_migrate_origin": {} }))
        .await
        .ok();

    let row = db::get_by_address(pool, addr).await.ok().flatten();
    let launcher_tx_ok = row
        .as_ref()
        .and_then(|r| r.launcher_tx.as_ref())
        .is_some_and(|t| !t.is_empty());
    let attested = attested_cmm(
        code_id,
        want_code,
        &admin,
        cmm,
        launcher_tx_ok,
        origin.as_ref().and_then(|o| o.launcher.as_deref()),
        launcher,
        migrate_origin
            .as_ref()
            .and_then(|o| o.source_cw2.as_deref()),
    );

    let features = json!({
        "mint_control": feats.as_ref().and_then(|f| f.mint_control).unwrap_or(false),
        "transfer_tax": feats.as_ref().and_then(|f| f.transfer_tax).unwrap_or(false),
        "split_router": feats.as_ref().and_then(|f| f.split_router).unwrap_or(false),
        "auto_v2_lp": feats.as_ref().and_then(|f| f.auto_v2_lp).unwrap_or(false),
        "exemption_directory": feats.as_ref().and_then(|f| f.exemption_directory).unwrap_or(false),
        "variable_rates": feats.as_ref().and_then(|f| f.variable_rates).unwrap_or(false),
        "launch_guards": feats.as_ref().and_then(|f| f.launch_guards).unwrap_or(false),
    });

    if let Err(e) = db::apply_lcd_snapshot(
        pool,
        addr,
        code_id as i64,
        if admin.is_empty() { None } else { Some(admin.as_str()) },
        cfg.as_ref().and_then(|c| c.manager.as_deref()),
        cfg.as_ref().and_then(|c| c.treasury.as_deref()),
        cfg.as_ref().and_then(|c| c.buy_bps.map(|v| v as i16)),
        cfg.as_ref().and_then(|c| c.sell_bps.map(|v| v as i16)),
        cfg.as_ref().and_then(|c| c.transfer_bps.map(|v| v as i16)),
        &features,
        token_info.as_ref().and_then(|t| t.name.as_deref()),
        token_info.as_ref().and_then(|t| t.symbol.as_deref()),
        token_info.as_ref().and_then(|t| t.decimals.map(|d| i16::from(d))),
        attested,
    )
    .await
    {
        tracing::debug!(addr, error = %e, "community tax snapshot upsert failed");
    }
}

pub async fn run_probe_loop(
    pool: sqlx::PgPool,
    lcd: LcdClient,
    config: Config,
    cancel: CancellationToken,
) {
    if !catalog_configured(&config) {
        tracing::info!("Community tax catalog unconfigured — probe loop idle");
        return;
    }
    loop {
        if cancel.is_cancelled() {
            return;
        }
        match db::list_all_addresses(&pool).await {
            Ok(addrs) => {
                for addr in addrs {
                    if cancel.is_cancelled() {
                        return;
                    }
                    refresh_one(&pool, &lcd, &config, &addr).await;
                }
            }
            Err(e) => tracing::debug!(error = %e, "community tax address list failed"),
        }
        tokio::select! {
            _ = cancel.cancelled() => return,
            _ = tokio::time::sleep(COMMUNITY_TAX_PROBE_INTERVAL) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attest_launcher_path() {
        assert!(attested_cmm(
            11619,
            11619,
            "cmm",
            "cmm",
            true,
            Some("launcher"),
            "launcher",
            None,
        ));
    }

    #[test]
    fn attest_adopt_path_without_launcher_tx() {
        assert!(attested_cmm(
            11619,
            11619,
            "cmm",
            "cmm",
            false,
            Some("launcher"),
            "launcher",
            Some("crates.io:cw20-base"),
        ));
        assert!(attested_cmm(
            11619,
            11619,
            "cmm",
            "cmm",
            false,
            Some("launcher"),
            "launcher",
            Some("crates.io:terraport-token"),
        ));
    }

    #[test]
    fn attest_rejects_rogue_without_cmm_or_origin() {
        assert!(!attested_cmm(
            11619,
            11619,
            "not-cmm",
            "cmm",
            false,
            Some("launcher"),
            "launcher",
            Some("crates.io:cw20-base"),
        ));
        assert!(!attested_cmm(
            11619,
            11619,
            "cmm",
            "cmm",
            false,
            Some("launcher"),
            "launcher",
            None,
        ));
        assert!(!attested_cmm(
            11619,
            11619,
            "cmm",
            "cmm",
            true,
            None,
            "launcher",
            Some("crates.io:cw20-base"),
        ));
    }

    #[test]
    fn parse_migrate_adopt_event() {
        let parsed = parse_community_event(&attrs(&[
            ("_contract_address", "terra1token"),
            ("action", "migrate-adopt"),
            ("community_token", "terra1token"),
            ("source_cw2", "crates.io:cw20-base"),
        ]))
        .expect("parse");
        assert_eq!(parsed.action, "migrate-adopt");
        assert_eq!(parsed.emitter, "terra1token");
        assert_eq!(parsed.community_token.as_deref(), Some("terra1token"));
    }

    fn attrs(pairs: &[(&str, &str)]) -> Vec<Attribute> {
        pairs
            .iter()
            .map(|(k, v)| Attribute {
                key: (*k).to_string(),
                value: (*v).to_string(),
            })
            .collect()
    }
}
