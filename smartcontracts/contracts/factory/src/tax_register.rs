//! Autoregister community-tax assets after `CreatePair` (GitLab #633 / **R633-2**).
//!
//! Gate on cw2 contract name — never execute `register_listed_pair` on honest
//! CW20s (they reject the unknown variant and would revert `CreatePair`).
//! Terraport / GDEX / non-factory addrs never reach this path (factory just
//! instantiated the pair). Do not factory-whitelist 8654 / launcher / AutoLP.

use cosmwasm_std::{from_json, to_json_binary, Addr, CosmosMsg, Deps, WasmMsg};
use cw2::ContractVersion;
use dex_common::types::AssetInfo;
use serde::{Deserialize, Serialize};

use crate::error::ContractError;

/// cw2 name written by `cl8y-community-tax-token` (11611 / 11619 / LocalTerra store).
pub const COMMUNITY_TAX_CW2_NAME: &str = "crates.io:cl8y-community-tax-token";

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum TaxTokenExecute {
    RegisterListedPair { pair: String },
}

/// True only when the contract's cw2 name is the community-tax crate.
/// Missing / unreadable cw2 → false (honest templates, unknown wasm).
pub fn is_community_tax_cw2(deps: Deps, addr: &Addr) -> bool {
    let Ok(Some(raw)) = deps
        .querier
        .query_wasm_raw(addr.to_string(), b"contract_info".as_slice())
    else {
        return false;
    };
    let Ok(ver) = from_json::<ContractVersion>(&raw) else {
        return false;
    };
    ver.contract == COMMUNITY_TAX_CW2_NAME
}

/// One `register_listed_pair` execute per tax-side asset. Honest sides omitted.
pub fn register_listed_pair_msgs(
    deps: Deps,
    pair: &Addr,
    asset_infos: &[AssetInfo; 2],
) -> Result<Vec<CosmosMsg>, ContractError> {
    let mut msgs = Vec::new();
    for asset in asset_infos {
        let AssetInfo::Token { contract_addr } = asset else {
            continue;
        };
        let token = deps.api.addr_validate(contract_addr)?;
        if !is_community_tax_cw2(deps, &token) {
            continue;
        }
        msgs.push(
            WasmMsg::Execute {
                contract_addr: token.to_string(),
                msg: to_json_binary(&TaxTokenExecute::RegisterListedPair {
                    pair: pair.to_string(),
                })?,
                funds: vec![],
            }
            .into(),
        );
    }
    Ok(msgs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cosmwasm_std::testing::{mock_dependencies, MockApi, MockQuerier, MockStorage};
    use cosmwasm_std::{
        to_json_binary, ContractResult, OwnedDeps, QuerierResult, SystemError, SystemResult,
        WasmQuery,
    };
    use std::collections::HashMap;

    fn cw2_bin(name: &str) -> Vec<u8> {
        to_json_binary(&ContractVersion {
            contract: name.into(),
            version: "1.0.0".into(),
        })
        .unwrap()
        .to_vec()
    }

    fn deps_with_cw2(
        map: HashMap<String, Vec<u8>>,
    ) -> OwnedDeps<MockStorage, MockApi, MockQuerier> {
        let mut deps = mock_dependencies();
        deps.querier.update_wasm(move |q| match q {
            WasmQuery::Raw { contract_addr, key } => {
                if key.as_slice() != b"contract_info" {
                    return SystemResult::Err(SystemError::Unknown {});
                }
                match map.get(contract_addr) {
                    Some(raw) => SystemResult::Ok(ContractResult::Ok(raw.clone().into())),
                    None => SystemResult::Ok(ContractResult::Ok(Vec::<u8>::new().into())),
                }
            }
            _ => SystemResult::Err(SystemError::Unknown {}),
        });
        deps
    }

    fn token(addr: &str) -> AssetInfo {
        AssetInfo::Token {
            contract_addr: addr.into(),
        }
    }

    #[test]
    fn honest_honest_emits_no_register() {
        let honest = cw2_bin("crates.io:cw20-base");
        let deps = deps_with_cw2(HashMap::from([
            ("token_a".into(), honest.clone()),
            ("token_b".into(), honest),
        ]));
        let msgs = register_listed_pair_msgs(
            deps.as_ref(),
            &Addr::unchecked("pair"),
            &[token("token_a"), token("token_b")],
        )
        .unwrap();
        assert!(msgs.is_empty());
    }

    #[test]
    fn tax_honest_registers_only_tax_side() {
        let deps = deps_with_cw2(HashMap::from([
            ("tax".into(), cw2_bin(COMMUNITY_TAX_CW2_NAME)),
            ("honest".into(), cw2_bin("crates.io:cw20-mintable")),
        ]));
        let msgs = register_listed_pair_msgs(
            deps.as_ref(),
            &Addr::unchecked("pair1"),
            &[token("tax"), token("honest")],
        )
        .unwrap();
        assert_eq!(msgs.len(), 1);
        match &msgs[0] {
            CosmosMsg::Wasm(WasmMsg::Execute { contract_addr, .. }) => {
                assert_eq!(contract_addr, "tax");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn tax_tax_registers_both() {
        let tax = cw2_bin(COMMUNITY_TAX_CW2_NAME);
        let deps = deps_with_cw2(HashMap::from([
            ("tax_a".into(), tax.clone()),
            ("tax_b".into(), tax),
        ]));
        let msgs = register_listed_pair_msgs(
            deps.as_ref(),
            &Addr::unchecked("pair2"),
            &[token("tax_a"), token("tax_b")],
        )
        .unwrap();
        assert_eq!(msgs.len(), 2);
    }

    #[test]
    fn missing_cw2_is_not_tax() {
        let deps = mock_dependencies();
        assert!(!is_community_tax_cw2(
            deps.as_ref(),
            &Addr::unchecked("unknown")
        ));
    }

    #[test]
    fn query_error_is_not_tax() {
        let mut deps = mock_dependencies();
        deps.querier.update_wasm(|_| {
            SystemResult::Err(SystemError::NoSuchContract {
                addr: "gone".into(),
            })
        });
        assert!(!is_community_tax_cw2(
            deps.as_ref(),
            &Addr::unchecked("gone")
        ));
        let _ = QuerierResult::Ok(ContractResult::Ok(Default::default()));
    }
}
