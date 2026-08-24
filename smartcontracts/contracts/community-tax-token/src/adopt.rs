//! Foreign-template adopt onto this wasm (GitLab #626 / S3).
//!
//! Same-crate upgrades stay in [`crate::contract::migrate`] via
//! `ensure_from_older_version`. This module runs only when cw2 is **not**
//! `crates.io:cl8y-community-tax-token`.
//!
//! ## Invariants (M626)
//!
//! - Allowlisted source cw2 only: `cw20-base`, `cw20-mintable`, `terraport-token`.
//! - `tax_map` storage prefix → revert (8654 / inbound FoT stays no-go).
//! - `BALANCES` / `TOKEN_INFO` / allowances are kept; no mint or burn.
//! - Free profile: no paid SKUs. Source minter is **revoked**.
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

/// Honest listed (or Terraport V2) cw2 names. 8654 / unknown names are rejected.
pub const ALLOWED_SOURCE_CW2: &[&str] = &[
    "crates.io:cw20-base",
    "crates.io:cw20-mintable",
    "crates.io:terraport-token",
];

const TAX_MAP_PREFIX: &[u8] = b"tax_map";
const CONTRACT_NAME: &str = "crates.io:cl8y-community-tax-token";
const CONTRACT_VERSION: &str = env!("CARGO_PKG_VERSION");

pub fn is_allowed_source_cw2(name: &str) -> bool {
    ALLOWED_SOURCE_CW2.contains(&name)
}

pub fn looks_like_taxed_cw2(name: &str) -> bool {
    let n = name.to_ascii_lowercase();
    n.contains("taxed") || n.contains("tax_map") || n.contains("cw20_taxed")
}

pub fn storage_has_key_prefix(storage: &dyn Storage, prefix: &[u8]) -> bool {
    match storage.range(Some(prefix), None, Order::Ascending).next() {
        Some((k, _)) => k.starts_with(prefix),
        None => false,
    }
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
    if looks_like_taxed_cw2(&version.contract) || !is_allowed_source_cw2(&version.contract) {
        return Err(ContractError::AdoptCw2NotAllowed {
            name: version.contract,
        });
    }
    if storage_has_key_prefix(deps.storage, TAX_MAP_PREFIX) {
        return Err(ContractError::AdoptTaxMapPresent {});
    }

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

    crate::contract::validate_instantiate_caps(
        msg.max_buy_bps,
        msg.max_sell_bps,
        msg.max_transfer_bps,
    )?;
    crate::contract::validate_bps_at_init(msg.buy_bps, msg.max_buy_bps)?;
    crate::contract::validate_bps_at_init(msg.sell_bps, msg.max_sell_bps)?;
    let transfer = msg.transfer_bps.unwrap_or(0);
    crate::contract::validate_bps_at_init(transfer, msg.max_transfer_bps)?;
    // Adopt is free-profile (no VariableRates SKU) — max must equal current.
    for (field, max, current) in [
        ("max_buy_bps", msg.max_buy_bps, msg.buy_bps),
        ("max_sell_bps", msg.max_sell_bps, msg.sell_bps),
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
            buy_bps: msg.buy_bps,
            sell_bps: msg.sell_bps,
            transfer_bps: transfer,
            max_buy_bps: msg.max_buy_bps,
            max_sell_bps: msg.max_sell_bps,
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
        .add_attribute("mint_revoked", had_minter.to_string()))
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
        assert!(!looks_like_taxed_cw2("crates.io:cw20-base"));
        assert!(looks_like_taxed_cw2("crates.io:cw20-taxed"));
    }

    #[test]
    fn tax_map_prefix_detects_leftover() {
        let mut s = MockStorage::new();
        assert!(!storage_has_key_prefix(&s, TAX_MAP_PREFIX));
        s.set(b"tax_map", b"1");
        assert!(storage_has_key_prefix(&s, TAX_MAP_PREFIX));
        s.set(b"tax_map\0dest", b"x");
        assert!(storage_has_key_prefix(&s, TAX_MAP_PREFIX));
    }
}
