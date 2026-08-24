//! Foreign-template adopt onto this wasm (GitLab #626 / S3 + S4).
//!
//! Same-crate upgrades stay in [`crate::contract::migrate`] via
//! `ensure_from_older_version`. This module runs only when cw2 is **not**
//! `crates.io:cl8y-community-tax-token`.
//!
//! ## Invariants (M626)
//!
//! - Allowlisted source cw2: `cw20-base`, `cw20-mintable`, `terraport-token`,
//!   plus `cw20-taxed` / `cw20_taxed` for the ALPHA wipe path.
//! - 8654 / FoT leftovers (`tax_info`, `tax_map`, `whale_info`) are **wiped**.
//!   Honest templates with those leftovers take the same wipe (ALPHA may
//!   report cw2 `crates.io:cw20-base`). Unknown cw2 still reverts.
//! - After wipe, inbound Transfer is 1:1. Do not whitelist 8654.
//! - `BALANCES` / `TOKEN_INFO` / allowances are kept; no mint or burn.
//! - Free profile: no paid SKUs. Source minter is **revoked**.
//! - Wipe maps ALPHA 1% / 4.5% → sell 100 / buy 450 when the payload is zeros.
//! - `CONFIG.launcher` is the official launcher (`GetLauncherOrigin`).
//! - `GetMigrateOrigin` is written so catalog can attest without faking `launcher_tx`.

use cosmwasm_std::{Order, Response, Storage};
use cw2::{get_contract_version, set_contract_version};
use cw20_base::state::TOKEN_INFO;

use crate::error::ContractError;
use crate::msg::{AdoptMigrateMsg, Sku, MAX_DECIMALS, MIN_DECIMALS};
use crate::state::{
    Config, Features, MigrateOrigin, CONFIG, FEATURES, MIGRATE_ORIGIN, PROTOCOL_EXEMPT,
};

/// Honest listed (or Terraport V2) cw2 names.
pub const ALLOWED_SOURCE_CW2: &[&str] = &[
    "crates.io:cw20-base",
    "crates.io:cw20-mintable",
    "crates.io:terraport-token",
];

/// ALPHA / cw20-taxed cw2 names. Live 8654 may still report `cw20-base`.
pub const ALLOWED_TAXED_CW2: &[&str] = &["crates.io:cw20-taxed", "cw20_taxed"];

/// ALPHA live map: 4.5% buy / 1% sell (combined 550 ≤ 2500).
pub const ALPHA_BUY_BPS: u16 = 450;
pub const ALPHA_SELL_BPS: u16 = 100;

const WIPE_NAMESPACES: &[&[u8]] = &[b"tax_map", b"tax_info", b"whale_info"];
const CONTRACT_NAME: &str = "crates.io:cl8y-community-tax-token";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn is_allowed_source_cw2(name: &str) -> bool {
    ALLOWED_SOURCE_CW2.contains(&name)
}

pub fn is_allowed_taxed_cw2(name: &str) -> bool {
    ALLOWED_TAXED_CW2.contains(&name)
}

pub fn looks_like_taxed_cw2(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("taxed") || n.contains("tax_map") || n.contains("cw20_taxed")
}

pub fn storage_plus_prefix(ns: &[u8]) -> Vec<u8> {
    let mut p = Vec::with_capacity(2 + ns.len());
    p.extend_from_slice(&(ns.len() as u16).to_be_bytes());
    p.extend_from_slice(ns);
    p
}

pub fn storage_has_key_prefix(storage: &dyn Storage, prefix: &[u8]) -> bool {
    match storage.range(Some(prefix), None, Order::Ascending).next() {
        Some((k, _)) => k.starts_with(prefix),
        None => false,
    }
}

pub fn storage_has_namespace(storage: &dyn Storage, ns: &[u8]) -> bool {
    storage_has_key_prefix(storage, ns) || storage_has_key_prefix(storage, &storage_plus_prefix(ns))
}

pub fn has_fot_leftover(storage: &dyn Storage) -> bool {
    WIPE_NAMESPACES
        .iter()
        .any(|ns| storage_has_namespace(storage, ns))
}

fn wipe_prefix(storage: &mut dyn Storage, prefix: &[u8]) -> u32 {
    let mut n = 0u32;
    loop {
        let next = storage
            .range(Some(prefix), None, Order::Ascending)
            .next()
            .and_then(|(k, _)| k.starts_with(prefix).then(|| k.to_vec()));
        match next {
            Some(k) => {
                storage.remove(&k);
                n += 1;
            }
            None => break,
        }
    }
    n
}

/// Drop 8654 `tax_info` / `tax_map` / `whale_info` (raw and cw-storage-plus).
pub fn wipe_fot_leftovers(storage: &mut dyn Storage) -> u32 {
    let mut n = 0u32;
    for ns in WIPE_NAMESPACES {
        n += wipe_prefix(storage, ns);
        n += wipe_prefix(storage, &storage_plus_prefix(ns));
    }
    n
}

pub fn execute_adopt(
    deps: cosmwasm_std::DepsMut,
    env: cosmwasm_std::Env,
    msg: AdoptMigrateMsg,
) -> Result<Response, ContractError> {
    let version =
        get_contract_version(deps.storage).map_err(|_| ContractError::AdoptCw2NotAllowed {
            name: String::new(),
        })?;
    let taxed = is_allowed_taxed_cw2(&version.contract);
    if !is_allowed_source_cw2(&version.contract) && !taxed {
        return Err(ContractError::AdoptCw2NotAllowed {
            name: version.contract,
        });
    }
    let leftovers = has_fot_leftover(deps.storage);
    let wiped = if leftovers {
        let n = wipe_fot_leftovers(deps.storage);
        if has_fot_leftover(deps.storage) {
            return Err(ContractError::AdoptTaxMapPresent {});
        }
        n
    } else {
        0
    };
    let s4 = taxed || wiped > 0;

    match CONFIG.may_load(deps.storage) {
        Ok(Some(_)) => return Err(ContractError::AdoptAlreadyConfigured {}),
        Err(_) => return Err(ContractError::AdoptStorageSmash { key: "cfg".into() }),
        Ok(None) => {}
    }
    match FEATURES.may_load(deps.storage) {
        Ok(Some(_)) => return Err(ContractError::AdoptAlreadyConfigured {}),
        Err(_) => return Err(ContractError::AdoptStorageSmash { key: "feat".into() }),
        Ok(None) => {}
    }

    let mut token_info = TOKEN_INFO
        .may_load(deps.storage)?
        .ok_or(ContractError::AdoptMissingTokenInfo {})?;
    if !(MIN_DECIMALS..=MAX_DECIMALS).contains(&token_info.decimals) {
        return Err(ContractError::DecimalsRange {
            min: MIN_DECIMALS,
            max: MAX_DECIMALS,
            got: token_info.decimals,
        });
    }

    let (buy_bps, sell_bps, max_buy_bps, max_sell_bps) = if s4
        && msg.buy_bps == 0
        && msg.sell_bps == 0
        && msg.max_buy_bps == 0
        && msg.max_sell_bps == 0
    {
        (ALPHA_BUY_BPS, ALPHA_SELL_BPS, ALPHA_BUY_BPS, ALPHA_SELL_BPS)
    } else {
        (msg.buy_bps, msg.sell_bps, msg.max_buy_bps, msg.max_sell_bps)
    };

    crate::contract::validate_instantiate_caps(max_buy_bps, max_sell_bps, msg.max_transfer_bps)?;
    crate::contract::validate_bps_at_init(buy_bps, max_buy_bps)?;
    crate::contract::validate_bps_at_init(sell_bps, max_sell_bps)?;
    let transfer = msg.transfer_bps.unwrap_or(0);
    crate::contract::validate_bps_at_init(transfer, msg.max_transfer_bps)?;
    // Adopt is free-profile (no VariableRates SKU) — max must equal current.
    for (field, max, current) in [
        ("max_buy_bps", max_buy_bps, buy_bps),
        ("max_sell_bps", max_sell_bps, sell_bps),
        ("max_transfer_bps", msg.max_transfer_bps, transfer),
    ] {
        if max != current {
            return Err(ContractError::SkuPayloadWithoutFeature {
                field: field.into(),
                sku: Sku::VariableRates.as_str().to_string(),
            });
        }
    }

    let had_minter = token_info.mint.is_some();
    if had_minter {
        token_info.mint = None;
        TOKEN_INFO.save(deps.storage, &token_info)?;
    }

    let manager = deps.api.addr_validate(&msg.manager)?;
    let treasury = deps.api.addr_validate(&msg.treasury)?;
    let factory = deps.api.addr_validate(&msg.factory)?;
    let ust1 = deps.api.addr_validate(&msg.ust1)?;
    let cmm_treasury = deps.api.addr_validate(&msg.cmm_treasury)?;
    let official_launcher = deps.api.addr_validate(&msg.official_launcher)?;
    let router = msg
        .router
        .as_ref()
        .map(|r| deps.api.addr_validate(r))
        .transpose()?;

    CONFIG.save(
        deps.storage,
        &Config {
            manager: manager.clone(),
            treasury,
            buy_bps,
            sell_bps,
            transfer_bps: transfer,
            max_buy_bps,
            max_sell_bps,
            max_transfer_bps: msg.max_transfer_bps,
            factory: factory.clone(),
            router: router.clone(),
            ust1,
            cmm_treasury,
            autolp: None,
            launcher: Some(official_launcher.clone()),
            mint_revoked: had_minter,
        },
    )?;
    FEATURES.save(deps.storage, &Features::from_skus(&[]))?;

    PROTOCOL_EXEMPT.save(deps.storage, &env.contract.address, &true)?;
    PROTOCOL_EXEMPT.save(deps.storage, &factory, &true)?;
    if let Some(r) = &router {
        PROTOCOL_EXEMPT.save(deps.storage, r, &true)?;
    }

    MIGRATE_ORIGIN.save(
        deps.storage,
        &MigrateOrigin {
            source_cw2: version.contract.clone(),
            source_version: version.version.clone(),
            source_code_id: msg.source_code_id,
            migrated_at_height: env.block.height,
        },
    )?;
    set_contract_version(deps.storage, CONTRACT_NAME, CONTRACT_VERSION)?;

    Ok(Response::new()
        .add_attribute("action", "migrate-adopt")
        .add_attribute("community_token", env.contract.address)
        .add_attribute("source_cw2", version.contract)
        .add_attribute("source_version", version.version)
        .add_attribute("manager", manager)
        .add_attribute("launcher", official_launcher)
        .add_attribute("mint_revoked", had_minter.to_string())
        .add_attribute("fot_wiped", wiped.to_string())
        .add_attribute("s4", s4.to_string()))
}

#[cfg(test)]
mod unit_tests {
    use super::*;
    use cosmwasm_std::testing::MockStorage;

    #[test]
    fn allowlist_honest_only() {
        assert!(is_allowed_source_cw2("crates.io:cw20-base"));
        assert!(is_allowed_source_cw2("crates.io:cw20-mintable"));
        assert!(is_allowed_source_cw2("crates.io:terraport-token"));
        assert!(!is_allowed_source_cw2("crates.io:cw20-taxed"));
        assert!(is_allowed_taxed_cw2("crates.io:cw20-taxed"));
        assert!(!looks_like_taxed_cw2("crates.io:cw20-base"));
        assert!(looks_like_taxed_cw2("crates.io:cw20-taxed"));
    }

    #[test]
    fn tax_info_and_tax_map_namespaces_wipe() {
        let mut s = MockStorage::new();
        assert!(!has_fot_leftover(&s));
        s.set(b"tax_map", b"1");
        s.set(&storage_plus_prefix(b"tax_info"), b"alpha");
        s.set(&storage_plus_prefix(b"whale_info"), b"whale");
        assert!(has_fot_leftover(&s));
        assert!(wipe_fot_leftovers(&mut s) >= 3);
        assert!(!has_fot_leftover(&s));
        s.set(b"balance", b"keep");
        assert!(!has_fot_leftover(&s));
    }
}
